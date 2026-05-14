use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use chrono::Utc;
use deepseek_agent::ModelRegistry;
use deepseek_config::{CliRuntimeOverrides, ConfigStore};
use deepseek_context::HybridContextStore;
use deepseek_core::Runtime;
use deepseek_execpolicy::{AskForApproval, ExecPolicyEngine};
use deepseek_hooks::{HookDispatcher, JsonlHookSink, StdoutHookSink};
use deepseek_mcp::McpManager;
use deepseek_protocol::{
    AppRequest, AppResponse, PromptRequest, PromptResponse, ThreadListParams, ThreadRequest,
    ThreadResponse,
};
use deepseek_session::SessionStore;
use deepseek_state::StateStore;
use deepseek_swarm::{AgentRole, AgentSpec, SwarmOrchestrator};
use deepseek_tools::{ToolCall, ToolRegistry};
use deepseek_tui_core::UiEvent;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{RwLock, mpsc};
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use uuid::Uuid;

mod terminal;
mod supervisor;

use supervisor::DaemonSupervisor;

// ── Options ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppServerOptions {
    pub listen: SocketAddr,
    pub config_path: Option<PathBuf>,
    pub daemon: bool,
    pub pid_file: Option<PathBuf>,
    pub auto_shutdown_idle: bool,
    pub idle_timeout_secs: u64,
}

// ── Daemon state ───────────────────────────────────────────────────────────

struct DaemonState {
    connected_clients: RwLock<HashSet<String>>,
    detached: AtomicBool,
    active_tasks: AtomicU64,
    started_at: String,
    auto_shutdown_idle: AtomicBool,
    #[allow(dead_code)] idle_timeout_secs: u64,
}

impl DaemonState {
    fn new(auto_shutdown_idle: bool, idle_timeout_secs: u64) -> Self {
        Self {
            connected_clients: RwLock::new(HashSet::new()),
            detached: AtomicBool::new(false),
            active_tasks: AtomicU64::new(0),
            started_at: Utc::now().to_rfc3339(),
            auto_shutdown_idle: AtomicBool::new(auto_shutdown_idle),
            idle_timeout_secs,
        }
    }
    async fn client_connect(&self) -> String {
        let id = Uuid::new_v4().to_string();
        self.connected_clients.write().await.insert(id.clone());
        id
    }
    #[allow(dead_code)]
    async fn client_disconnect(&self, id: &str) {
        self.connected_clients.write().await.remove(id);
    }
    #[allow(dead_code)]
    async fn connected_count(&self) -> usize {
        self.connected_clients.read().await.len()
    }
    fn task_started(&self) { self.active_tasks.fetch_add(1, Ordering::SeqCst); }
    #[allow(dead_code)]
    fn task_finished(&self) { self.active_tasks.fetch_sub(1, Ordering::SeqCst); }
    fn active_task_count(&self) -> u64 { self.active_tasks.load(Ordering::SeqCst) }
    fn should_auto_shutdown(&self) -> bool { self.auto_shutdown_idle.load(Ordering::SeqCst) }
    fn set_detached(&self, v: bool) { self.detached.store(v, Ordering::SeqCst); }
    fn is_detached(&self) -> bool { self.detached.load(Ordering::SeqCst) }
}

// ── App state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    config_path: Option<PathBuf>,
    config: Arc<RwLock<deepseek_config::ConfigToml>>,
    runtime: Arc<tokio::sync::Mutex<Runtime>>,
    registry: ModelRegistry,
    session_store: Arc<SessionStore>,
    daemon: Arc<DaemonState>,
    swarm: Arc<SwarmOrchestrator>,
    context_store: Arc<HybridContextStore>,
    supervisor: Arc<DaemonSupervisor>,
}

// ── Request types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallRequest {
    call: ToolCall,
    #[serde(default)]
    cwd: Option<PathBuf>,
}
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)] jsonrpc: Option<String>,
    #[serde(default)] id: Option<Value>,
    method: String,
    #[serde(default)] params: Value,
}
#[derive(Debug)]
struct JsonRpcError { code: i64, message: String, data: Option<Value> }
#[derive(Debug)]
struct StdioDispatchResult { result: Value, should_exit: bool }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct ConfigGetParams { key: String }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct ConfigSetParams { key: String, value: String }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct ThreadIdParams { thread_id: String }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct ThreadMessageParams { thread_id: String, input: String }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct SessionExportParams { session_id: String, #[serde(default)] output_path: Option<String> }
#[derive(Debug, Deserialize)]
struct SessionImportParams { archive_path: String, #[serde(default)] overwrite: bool }
#[derive(Debug, Deserialize)] #[allow(dead_code)]
struct SessionSearchParams { query: String }

// ── Entry points ───────────────────────────────────────────────────────────

pub async fn run(options: AppServerOptions) -> Result<()> {
    run_foreground(options).await
}

async fn run_foreground(options: AppServerOptions) -> Result<()> {
    info!(listen=%options.listen, daemon=options.daemon, "starting");

    let state = build_state(&options)?;

    // Log startup (hive mind restores lazily on first /daemon/resume call)
    state.supervisor.log("startup", "Daemon started — hive mind will restore on first resume", None).await;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/thread", post(thread_handler))
        .route("/app", post(app_handler))
        .route("/prompt", post(prompt_handler))
        .route("/tool", post(tool_handler))
        .route("/jobs", get(jobs_handler))
        .route("/mcp/startup", post(mcp_startup_handler))
        .route("/sessions", get(session_list_handler))
        .route("/sessions/{id}", get(session_read_handler))
        .route("/sessions/{id}", delete(session_delete_handler))
        .route("/sessions/{id}/export", post(session_export_handler))
        .route("/sessions/import", post(session_import_handler))
        .route("/daemon/detach", post(daemon_detach_handler))
        .route("/daemon/attach", post(daemon_attach_handler))
        .route("/daemon/status", get(daemon_status_handler))
        .route("/swarm/agents", get(swarm_agents_handler))
        .route("/swarm/spawn", post(swarm_spawn_handler))
        .route("/hive/query/{key}", get(hive_query_handler))
        .route("/hive/inject", post(hive_inject_handler))
        .route("/hive/summary", get(hive_summary_handler))
        .route("/hive/snapshot", get(hive_snapshot_handler))
        .route("/daemon/resume", get(daemon_resume_handler))
        .route("/daemon/progress", get(daemon_progress_handler))
        .route("/daemon/checkpoint", post(daemon_checkpoint_handler))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(options.listen).await?;
    info!("listening on {}", options.listen);

    let shutdown_tx = if options.auto_shutdown_idle {
        let s = state.clone(); let t = options.idle_timeout_secs;
        let (tx, rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move { auto_shutdown_watcher(s, t, rx).await; });
        Some(tx)
    } else { None };

    axum::serve(listener, app)
        .with_graceful_shutdown(daemon_shutdown(state.clone()))
        .await?;

    if let Some(tx) = shutdown_tx { let _ = tx.send(()).await; }
    if let Some(ref pf) = options.pid_file { let _ = fs::remove_file(pf); }
    info!("shut down cleanly");
    Ok(())
}

// ── Shutdown / lifecycle ───────────────────────────────────────────────────

async fn daemon_shutdown(state: AppState) {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ctrl-c"); };
    #[cfg(unix)] let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("sigterm").recv().await;
    };
    #[cfg(unix)] let hangup = {
        let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .expect("sighup");
        async move { loop { sig.recv().await; state.daemon.set_detached(true); } }
    };
    #[cfg(not(unix))] let terminate = std::future::pending::<()>();
    #[cfg(not(unix))] let hangup = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => { info!("Ctrl+C"); } _ = terminate => { info!("SIGTERM"); } _ = hangup => {} }
}

async fn auto_shutdown_watcher(state: AppState, timeout_secs: u64, mut cancel: mpsc::Receiver<()>) {
    let dur = tokio::time::Duration::from_secs(timeout_secs);
    loop {
        tokio::select! { _ = cancel.recv() => { return; } _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {} }
        if state.daemon.connected_count().await == 0 && state.daemon.active_task_count() == 0 {
            tokio::select! { _ = cancel.recv() => { return; } _ = tokio::time::sleep(dur) => {} }
            if state.daemon.connected_count().await == 0 && state.daemon.active_task_count() == 0 {
                info!("auto-shutdown: idle, all tasks complete");
                process::exit(0);
            }
        }
    }
}

// ── HTTP handlers ──────────────────────────────────────────────────────────

async fn healthz() -> Json<Value> {
    Json(json!({"status":"ok","protocol":"v2","service":"deepseek-app-server","version":env!("CARGO_PKG_VERSION")}))
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket))
}

async fn handle_ws(mut socket: WebSocket) {
    tracing::info!("WebSocket client connected");
    let welcome = serde_json::json!({
        "jsonrpc": "2.0", "method": "connected",
        "params": {"service": "deepseek-app-server", "version": "0.8.26"}
    });
    let _ = socket.send(Message::Text(welcome.to_string().into())).await;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if parsed.get("method").and_then(|v| v.as_str()) == Some("prompt") {
                        let ack = serde_json::json!({
                            "jsonrpc": "2.0", "method": "response.delta",
                            "params": {"delta": "Connected.", "conversation_id": parsed["params"]["conversation_id"]}
                        });
                        let _ = socket.send(Message::Text(ack.to_string().into())).await;
                        let done = serde_json::json!({
                            "jsonrpc": "2.0", "method": "response.complete",
                            "params": {"full_text": "WebSocket active.", "conversation_id": parsed["params"]["conversation_id"]},
                            "id": parsed["id"]
                        });
                        let _ = socket.send(Message::Text(done.to_string().into())).await;
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    tracing::info!("WebSocket client disconnected");
}

async fn thread_handler(State(state): State<AppState>, Json(req): Json<ThreadRequest>) -> Json<ThreadResponse> {
    let mut rt = state.runtime.lock().await;
    match rt.handle_thread(req).await {
        Ok(r) => Json(r),
        Err(e) => Json(ThreadResponse { thread_id:"error".into(),status:format!("error:{e}"),thread:None,threads:vec![],model:None,model_provider:None,cwd:None,approval_policy:None,sandbox:None,events:vec![],data:json!({})}),
    }
}

async fn prompt_handler(State(state): State<AppState>, Json(req): Json<PromptRequest>) -> Json<PromptResponse> {
    let mut rt = state.runtime.lock().await;
    match rt.handle_prompt(req, &CliRuntimeOverrides::default()).await {
        Ok(r) => Json(r),
        Err(e) => Json(PromptResponse { output:e.to_string(), model:"unknown".into(), events:vec![] }),
    }
}

async fn tool_handler(State(state): State<AppState>, Json(req): Json<ToolCallRequest>) -> Json<Value> {
    let rt = state.runtime.lock().await;
    let cwd = req.cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    match rt.invoke_tool(req.call, AskForApproval::OnRequest, &cwd).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}

async fn jobs_handler(State(state): State<AppState>) -> Json<AppResponse> {
    let rt = state.runtime.lock().await;
    Json(rt.app_status())
}

async fn mcp_startup_handler(State(state): State<AppState>) -> Json<Value> {
    let rt = state.runtime.lock().await;
    let s = rt.mcp_startup().await;
    Json(json!({"ok":true,"summary":s}))
}

async fn app_handler(State(state): State<AppState>, Json(req): Json<AppRequest>) -> Json<AppResponse> {
    Json(process_app_request(&state, req).await)
}

// ── Session handlers ───────────────────────────────────────────────────────

async fn session_list_handler(State(state): State<AppState>) -> Json<Value> {
    match state.session_store.list() {
        Ok(s) => Json(json!({"ok":true,"sessions":s})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}
async fn session_read_handler(State(state): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    match state.session_store.load(&id) {
        Ok(d) => Json(json!({"ok":true,"session":d})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}
async fn session_delete_handler(State(state): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    match state.session_store.delete(&id) {
        Ok(()) => Json(json!({"ok":true,"deleted":id})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}
async fn session_export_handler(State(state): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>, Json(p): Json<SessionExportParams>) -> Json<Value> {
    let out = p.output_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("{}.ds-session",&id[..8.min(id.len())])));
    match deepseek_session::export_session(&state.session_store, &id, &out) {
        Ok(()) => Json(json!({"ok":true,"session_id":id,"output_path":out.display().to_string()})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}
async fn session_import_handler(State(state): State<AppState>, Json(p): Json<SessionImportParams>) -> Json<Value> {
    match deepseek_session::import_session(&state.session_store, &PathBuf::from(&p.archive_path), p.overwrite) {
        Ok(d) => Json(json!({"ok":true,"session_id":d.manifest.id,"name":d.manifest.name,"turn_count":d.turns.len()})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}

// ── Daemon handlers ────────────────────────────────────────────────────────

async fn daemon_detach_handler(State(state): State<AppState>) -> Json<Value> {
    state.daemon.set_detached(true);
    info!("detached via API");
    Json(json!({"ok":true,"detached":true}))
}
async fn daemon_attach_handler(State(state): State<AppState>) -> Json<Value> {
    let cid = state.daemon.client_connect().await;
    Json(json!({"ok":true,"client_id":cid,"connected_clients":state.daemon.connected_count().await}))
}
async fn daemon_status_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"ok":true,"detached":state.daemon.is_detached(),"connected_clients":state.daemon.connected_count().await,"active_tasks":state.daemon.active_task_count(),"auto_shutdown_idle":state.daemon.should_auto_shutdown(),"started_at":state.daemon.started_at}))
}

// ── Daemon resume / progress handlers ──────────────────────────────────────

async fn daemon_resume_handler(State(state): State<AppState>) -> Json<Value> {
    let suggestion = state.supervisor.build_resume_suggestion().await;
    Json(json!({"ok":true,"resume":suggestion}))
}

async fn daemon_progress_handler(State(state): State<AppState>) -> Json<Value> {
    let progress = state.supervisor.recent_progress(50).await;
    Json(json!({"ok":true,"progress":progress,"count":progress.len()}))
}

async fn daemon_checkpoint_handler(State(state): State<AppState>) -> Json<Value> {
    match state.supervisor.checkpoint_hive().await {
        Ok(()) => Json(json!({"ok":true,"message":"checkpoint saved"})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}

// ── Swarm / hive handlers ──────────────────────────────────────────────────

async fn swarm_agents_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"ok":true,"agents":state.swarm.list_agents().await}))
}
async fn swarm_spawn_handler(State(state): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let role = match body.get("role").and_then(|v| v.as_str()).unwrap_or("general").to_lowercase().as_str() {
        "explorer" => AgentRole::Explorer, "implementer" => AgentRole::Implementer,
        "reviewer" => AgentRole::Reviewer, "tester" => AgentRole::Tester,
        "planner" => AgentRole::Planner, "coordinator" => AgentRole::Coordinator,
        _ => AgentRole::General,
    };
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
    let h = state.swarm.spawn_agent(AgentSpec::for_role(role, name)).await;
    let agent_id = h.id.clone();
    let agent_role_label = h.role.label().to_string();
    let agent_name = h.name.clone();
    let supervisor = state.supervisor.clone();
    let daemon = state.daemon.clone();
    state.daemon.task_started();
    state.supervisor.agent_started(&agent_id, &agent_role_label, &agent_name).await;

    // Background listener: log when agent completes
    let role_for_log = agent_role_label.clone();
    let agent_id_for_log = agent_id.clone();
    tokio::spawn(async move {
        let result = h.await_completion().await;
        daemon.task_finished();
        let summary = result.as_ref()
            .map(|r| r.output.as_str())
            .unwrap_or("agent finished");
        supervisor.agent_completed(&agent_id_for_log, &role_for_log, summary).await;
    });

    Json(json!({"ok":true,"agent_id":agent_id,"role":agent_role_label,"name":agent_name}))
}
async fn hive_query_handler(State(state): State<AppState>, axum::extract::Path(key): axum::extract::Path<String>) -> Json<Value> {
    match state.swarm.hive.query(&key).await {
        Some(e) => Json(json!({"ok":true,"entry":e})),
        None => Json(json!({"ok":false,"error":"not found"})),
    }
}
async fn hive_inject_handler(State(state): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let key = body.get("key").and_then(|v| v.as_str()).unwrap_or("");
    let value = body.get("value").cloned().unwrap_or(Value::Null);
    let author = body.get("author").and_then(|v| v.as_str()).unwrap_or("api");
    let tags: Vec<String> = body.get("tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    let conf = body.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.8);
    match state.swarm.hive.inject(key, value, author, tags, conf).await {
        Ok(v) => Json(json!({"ok":true,"key":key,"version":v})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}
async fn hive_summary_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"ok":true,"summary":state.swarm.hive.summary().await,"entry_count":state.swarm.hive.len().await}))
}
async fn hive_snapshot_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"ok":true,"entries":state.swarm.hive.snapshot().await}))
}

// ── Stdio ──────────────────────────────────────────────────────────────────

pub async fn run_stdio(config_path: Option<PathBuf>) -> Result<()> {
    let opts = AppServerOptions { listen: "127.0.0.1:0".parse().unwrap(), config_path, daemon: false, pid_file: None, auto_shutdown_idle: false, idle_timeout_secs: 300 };
    let state = build_state(&opts)?;
    let mut reader = BufReader::new(tokio::io::stdin()).lines();
    let mut writer = tokio::io::BufWriter::new(tokio::io::stdout());
    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() { continue; }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let r = jsonrpc_error(None, JsonRpcError::parse_error(format!("invalid json: {e}")));
                writer.write_all(r.to_string().as_bytes()).await?; writer.write_all(b"\n").await?; writer.flush().await?; continue;
            }
        };
        if req.jsonrpc.as_deref().is_some_and(|v| v != "2.0") {
            let r = jsonrpc_error(req.id, JsonRpcError::invalid_request("jsonrpc version must be 2.0"));
            writer.write_all(r.to_string().as_bytes()).await?; writer.write_all(b"\n").await?; writer.flush().await?; continue;
        }
        match dispatch_stdio(&state, &req.method, req.params).await {
            Ok(d) => {
                let enc = jsonrpc_result(req.id, d.result);
                writer.write_all(enc.to_string().as_bytes()).await?; writer.write_all(b"\n").await?; writer.flush().await?;
                if d.should_exit { break; }
            }
            Err(e) => {
                let r = jsonrpc_error(req.id, e);
                writer.write_all(r.to_string().as_bytes()).await?; writer.write_all(b"\n").await?; writer.flush().await?;
            }
        }
    }
    Ok(())
}

// ── TUI mode: raw terminal input with full paste support ───────────────────

/// Run the TUI in raw terminal mode with bracketed paste, burst paste, and
/// CTRL+V detection. Reads stdin bytes through `TerminalInput` which feeds
/// `BracketedPasteBuffer` for proper multi-line paste handling.
///
/// Prompts are processed asynchronously: typing Enter submits the current
/// input line to a background task. While processing, you can continue typing
/// — new input is merged into the active prompt queue and injected on each
/// tool-use cycle, enabling "rethink and continue" workflows.
pub async fn run_tui(config_path: Option<PathBuf>) -> Result<()> {
    let opts = AppServerOptions {
        listen: "127.0.0.1:0".parse().unwrap(),
        config_path,
        daemon: false,
        pid_file: None,
        auto_shutdown_idle: false,
        idle_timeout_secs: 300,
    };
    let state = build_state(&opts)?;

    let mut terminal = terminal::TerminalInput::new();
    terminal.enable_raw_mode().context("failed to enable raw terminal mode")?;

    // ── Prompt queue: background channel for submitting prompts ──
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<(String, Option<String>)>(32);
    // ── Response channel: background task sends output back to main loop ──
    let (output_tx, mut output_rx) = mpsc::channel::<(String, bool)>(64); // (text, is_error)

    // Spawn the prompt processing background task
    let bg_state = state.clone();
    tokio::spawn(async move {
        while let Some((prompt, thread_id)) = prompt_rx.recv().await {
            let req = PromptRequest {
                thread_id,
                prompt: prompt.clone(),
                model: None,
            };

            let mut rt = bg_state.runtime.lock().await;
            match rt.handle_prompt(req, &CliRuntimeOverrides::default()).await {
                Ok(resp) => {
                    let _ = output_tx.send((format!("[model: {}]\n{}", resp.model, resp.output), false)).await;
                }
                Err(e) => {
                    let _ = output_tx.send((format!("[error: {e}]"), true)).await;
                }
            }
        }
    });

    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║        DeepSeek TUI v{}               ║", env!("CARGO_PKG_VERSION"));
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!("║  Type a prompt and press Enter.          ║");
    eprintln!("║  Paste with Ctrl+V or Shift+Insert.      ║");
    eprintln!("║  Type /help for commands.  Ctrl+C exit.  ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!();

    let mut input_line = String::new();
    let mut thread_id: Option<String> = None;
    let mut active_prompts: usize = 0;

    // Helper: write to stdout in raw mode
    macro_rules! echo {
        ($($arg:tt)*) => {{
            use std::io::Write;
            let mut out = std::io::stdout();
            let _ = write!(out, $($arg)*);
            let _ = out.flush();
        }};
    }

    // ── Main event loop: interleave stdin reads with output processing ──
    loop {
        // Drain any pending output from background tasks (non-blocking)
        while let Ok((text, is_error)) = output_rx.try_recv() {
            active_prompts = active_prompts.saturating_sub(1);
            if is_error {
                echo!("\r\x1b[K"); // clear current line
                eprintln!("{}", text);
            } else {
                echo!("\r\x1b[K"); // clear the "thinking…" or input line
                println!("{}", text);
            }
            println!();
        }

        // Read stdin events (blocking, but interrupted by output above)
        let events = match terminal.read_events() {
            Ok(ev) => ev,
            Err(e) => {
                echo!("\r\n[input error: {e}]\r\n");
                break Ok(());
            }
        };

        for event in events {
            match event {
                UiEvent::KeyPressed(ch) => {
                    if ch == '\x03' || ch == '\x04' {
                        echo!("\r\nexiting.\r\n");
                        let _ = terminal.disable_raw_mode();
                        return Ok(());
                    }
                    if ch == '\x7f' || ch == '\x08' {
                        if !input_line.is_empty() {
                            input_line.pop();
                            echo!("\x08 \x08");
                        }
                        continue;
                    }
                    if ch == '\t' {
                        // Tab completion placeholder
                        echo!("[tab]");
                        continue;
                    }
                    if ch.is_ascii_graphic() || ch == ' ' {
                        input_line.push(ch);
                        let mut buf = [0u8; 4];
                        let s = ch.encode_utf8(&mut buf);
                        echo!("{}", s);
                    }
                }
                UiEvent::EnterPressed => {
                    let prompt = input_line.trim().to_string();
                    echo!("\r\n"); // newline after input
                    input_line.clear();

                    if prompt.is_empty() {
                        continue;
                    }

                    // Local slash commands
                    if prompt == "/exit" || prompt == "/quit" || prompt == "/q" {
                        echo!("exiting.\r\n");
                        let _ = terminal.disable_raw_mode();
                        return Ok(());
                    }
                    if prompt == "/clear" {
                        thread_id = None;
                        echo!("[conversation cleared]\r\n\r\n");
                        continue;
                    }
                    if prompt == "/status" {
                        echo!("thread: {} | queued: {}\r\n\r\n",
                            thread_id.as_deref().unwrap_or("none"), active_prompts);
                        continue;
                    }
                    if prompt == "/help" {
                        echo!("/exit /quit /q — exit\r\n");
                        echo!("/clear — clear conversation\r\n");
                        echo!("/status — show status\r\n");
                        echo!("/paste — test paste detection\r\n");
                        echo!("\r\n");
                        continue;
                    }
                    if prompt == "/paste" {
                        echo!("[paste detection active — try Ctrl+V, Shift+Insert, or middle-click]\r\n\r\n");
                        continue;
                    }

                    // Submit to background prompt processor
                    let tid = thread_id.clone();
                    if let Err(_) = prompt_tx.send((prompt.clone(), tid)).await {
                        echo!("[error: prompt queue full]\r\n\r\n");
                        continue;
                    }
                    active_prompts += 1;
                    echo!("[submitted — {} queued]\r\n", active_prompts);
                }
                UiEvent::PasteContent { content, .. } => {
                    // Paste detected — echo and insert into input line
                    input_line.push_str(&content);
                    echo!("{}", content);
                }
                UiEvent::CtrlVPressed => {
                    // Visual feedback during paste capture
                    echo!("[pasting…]");
                }
                _ => {}
            }
        }
    }
}

async fn dispatch_stdio(state: &AppState, method: &str, params: Value) -> std::result::Result<StdioDispatchResult, JsonRpcError> {
    match method {
        "healthz" | "app/healthz" => Ok(StdioDispatchResult { result: json!({"status":"ok","service":"deepseek-app-server","transport":"stdio"}), should_exit: false }),
        "shutdown" => Ok(StdioDispatchResult { result: json!({"ok":true}), should_exit: true }),
        _ => {
            let mut rt = state.runtime.lock().await;
            match method {
                "thread/request" => { let r: ThreadRequest = parse_params(params)?; Ok(StdioDispatchResult { result: serde_json::to_value(rt.handle_thread(r).await.map_err(|e| JsonRpcError::internal(e.to_string()))?).unwrap(), should_exit: false }) }
                "thread/create" => { let r = ThreadRequest::Create { metadata: params_or_object(params) }; Ok(StdioDispatchResult { result: serde_json::to_value(rt.handle_thread(r).await.map_err(|e| JsonRpcError::internal(e.to_string()))?).unwrap(), should_exit: false }) }
                "thread/list" => { let r = ThreadRequest::List(parse_params(params_or_object(params))?); Ok(StdioDispatchResult { result: serde_json::to_value(rt.handle_thread(r).await.map_err(|e| JsonRpcError::internal(e.to_string()))?).unwrap(), should_exit: false }) }
                "thread/read" => { let r = ThreadRequest::Read(parse_params(params_or_object(params))?); Ok(StdioDispatchResult { result: serde_json::to_value(rt.handle_thread(r).await.map_err(|e| JsonRpcError::internal(e.to_string()))?).unwrap(), should_exit: false }) }
                "prompt/request" | "prompt/run" => { let r: PromptRequest = parse_params(params)?; Ok(StdioDispatchResult { result: serde_json::to_value(rt.handle_prompt(r, &CliRuntimeOverrides::default()).await.map_err(|e| JsonRpcError::internal(e.to_string()))?).unwrap(), should_exit: false }) }
                "daemon/detach" => { state.daemon.set_detached(true); Ok(StdioDispatchResult { result: json!({"ok":true}), should_exit: false }) }
                "daemon/status" => { Ok(StdioDispatchResult { result: json!({"ok":true,"detached":state.daemon.is_detached(),"active_tasks":state.daemon.active_task_count()}), should_exit: false }) }
                "hive/summary" => { Ok(StdioDispatchResult { result: json!({"ok":true,"summary":state.swarm.hive.summary().await}), should_exit: false }) }
                "hive/snapshot" => { Ok(StdioDispatchResult { result: json!({"ok":true,"entries":state.swarm.hive.snapshot().await}), should_exit: false }) }
                _ => Err(JsonRpcError::method_not_found(method)),
            }
        }
    }
}

// ── App request processing ─────────────────────────────────────────────────

async fn process_app_request(state: &AppState, req: AppRequest) -> AppResponse {
    match req {
        AppRequest::Capabilities => AppResponse {
            ok: true,
            data: json!({"routes":["/thread","/app","/prompt","/tool","/jobs","/mcp/startup","/sessions","/daemon/detach","/daemon/status","/swarm/agents","/swarm/spawn","/hive/query","/hive/inject","/hive/summary","/hive/snapshot","/daemon/resume","/daemon/progress","/daemon/checkpoint"],"config":["get","set","unset","list"],"transport":"stdio+http"}),
            events: vec![],
        },
        AppRequest::ConfigGet { key } => {
            let cfg = state.config.read().await;
            AppResponse { ok: true, data: json!({"key":key,"value":cfg.get_value(&key)}), events: vec![] }
        }
        AppRequest::ConfigSet { key, value } => {
            let mut cfg = state.config.write().await;
            let r = cfg.set_value(&key, &value);
            let ok = r.is_ok(); let msg = r.err().map(|e| e.to_string());
            let snap = cfg.clone(); drop(cfg);
            if let Err(e) = persist_config(state, snap).await { warn!(%e, "config persist failed"); }
            AppResponse { ok, data: json!({"key":key,"value":value,"error":msg}), events: vec![] }
        }
        AppRequest::ConfigUnset { key } => {
            let mut cfg = state.config.write().await;
            let r = cfg.unset_value(&key);
            let ok = r.is_ok(); let msg = r.err().map(|e| e.to_string());
            let snap = cfg.clone(); drop(cfg);
            if let Err(e) = persist_config(state, snap).await { warn!(%e, "config persist failed"); }
            AppResponse { ok, data: json!({"key":key,"error":msg}), events: vec![] }
        }
        AppRequest::ConfigList => {
            let cfg = state.config.read().await;
            AppResponse { ok: true, data: json!({"values":cfg.list_values()}), events: vec![] }
        }
        AppRequest::Models => AppResponse { ok: true, data: json!({"models":state.registry.list()}), events: vec![] },
        AppRequest::ThreadLoadedList => {
            let mut rt = state.runtime.lock().await;
            match rt.handle_thread(ThreadRequest::List(ThreadListParams { include_archived: false, limit: Some(50) })).await {
                Ok(r) => AppResponse { ok: true, data: json!({"threads":r.threads}), events: r.events },
                Err(e) => AppResponse { ok: false, data: json!({"error":e.to_string()}), events: vec![] },
            }
        }
    }
}

// ── State construction ─────────────────────────────────────────────────────

fn build_state(options: &AppServerOptions) -> Result<AppState> {
    let store = ConfigStore::load(options.config_path.clone())?;
    let config = store.config.clone();
    let registry = ModelRegistry::default();
    let state_db = options.config_path.as_ref().and_then(|p| p.parent().map(|x| x.join("state.db")));
    let state_store = StateStore::open(state_db)?;
    let mut hooks = HookDispatcher::default();
    hooks.add_sink(Arc::new(StdoutHookSink));
    let hl = options.config_path.as_ref().and_then(|p| p.parent().map(|x| x.join("events.jsonl"))).unwrap_or_else(|| PathBuf::from(".deepseek/events.jsonl"));
    hooks.add_sink(Arc::new(JsonlHookSink::new(hl)));
    let runtime = Runtime::new(config.clone(), registry.clone(), state_store, Arc::new(ToolRegistry::default()), Arc::new(McpManager::default()), ExecPolicyEngine::new(vec![], vec![]), hooks);
    let session_store = match SessionStore::default_store() {
        Ok(s) => Arc::new(s),
        Err(e) => { warn!(%e, "session store fallback"); Arc::new(SessionStore::new(std::env::temp_dir().join("deepseek-sessions"))) }
    };
    let daemon = Arc::new(DaemonState::new(options.auto_shutdown_idle, options.idle_timeout_secs));
    let swarm = Arc::new(SwarmOrchestrator::new());
    let context_store = {
        let db_path = options.config_path.as_ref()
            .and_then(|p| p.parent().map(|x| x.join("context.db")))
            .unwrap_or_else(|| PathBuf::from(".deepseek/context.db"));
        Arc::new(HybridContextStore::open(&db_path).unwrap_or_else(|e| {
            warn!(%e, "context store fallback to in-memory");
            HybridContextStore::open_in_memory().expect("in-memory context store")
        }))
    };
    let supervisor = {
        let log_path = options.config_path.as_ref()
            .and_then(|p| p.parent().map(|x| x.join("daemon.log")))
            .unwrap_or_else(|| PathBuf::from(".deepseek/daemon.log"));
        Arc::new(DaemonSupervisor::new(
            context_store.clone(),
            session_store.clone(),
            swarm.clone(),
            Some(log_path),
        ))
    };
    Ok(AppState { config_path: options.config_path.clone(), config: Arc::new(RwLock::new(config)), runtime: Arc::new(tokio::sync::Mutex::new(runtime)), registry, session_store, daemon, swarm, context_store, supervisor })
}

async fn persist_config(state: &AppState, config: deepseek_config::ConfigToml) -> Result<()> {
    if state.config_path.is_none() { return Ok(()); }
    let mut store = ConfigStore::load(state.config_path.clone())?;
    store.config = config;
    store.save()
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn params_or_object(params: Value) -> Value { if params.is_null() { json!({}) } else { params } }
fn parse_params<T: DeserializeOwned>(params: Value) -> std::result::Result<T, JsonRpcError> {
    serde_json::from_value(params).map_err(|e| JsonRpcError::invalid_params(e.to_string()))
}
fn jsonrpc_result(id: Option<Value>, result: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id.unwrap_or(Value::Null),"result":result})
}
fn jsonrpc_error(id: Option<Value>, err: JsonRpcError) -> Value {
    json!({"jsonrpc":"2.0","id":id.unwrap_or(Value::Null),"error":{"code":err.code,"message":err.message,"data":err.data}})
}
impl JsonRpcError {
    fn parse_error(m: impl Into<String>) -> Self { Self { code: -32700, message: m.into(), data: None } }
    fn invalid_request(m: impl Into<String>) -> Self { Self { code: -32600, message: m.into(), data: None } }
    fn method_not_found(m: &str) -> Self { Self { code: -32601, message: format!("unsupported method: {m}"), data: None } }
    fn invalid_params(m: impl Into<String>) -> Self { Self { code: -32602, message: m.into(), data: None } }
    fn internal(m: impl Into<String>) -> Self { Self { code: -32603, message: m.into(), data: None } }
}