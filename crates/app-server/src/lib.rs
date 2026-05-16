use std::collections::HashSet;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::{Context, Result};
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use chrono::Utc;
use deepseek_agent::ModelRegistry;
use deepseek_config::{CliRuntimeOverrides, ConfigStore};
use deepseek_context::HybridContextStore;
use deepseek_core::Runtime;
use deepseek_execpolicy::{AskForApproval, ExecPolicyEngine};
use async_trait::async_trait;
use deepseek_hooks::{HookDispatcher, HookEvent, HookSink, JsonlHookSink, StdoutHookSink};
use deepseek_mcp::McpManager;
use deepseek_protocol::{
    AppRequest, AppResponse, PromptRequest, PromptResponse, ThreadListParams, ThreadRequest,
    ThreadResponse,
};
use deepseek_session::SessionStore;
use deepseek_state::StateStore;
use deepseek_swarm::{AgentRole, AgentSpec, SwarmOrchestrator};
use deepseek_tools::{ToolCall, ToolRegistry};
use deepseek_tui_core::{Checkpoint, EffectVec, UiEffect, UiEvent, UiState};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{RwLock, mpsc, Notify};
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use uuid::Uuid;

mod terminal;
mod supervisor;
pub mod tui;

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
    #[allow(dead_code)]
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

    let state = build_state(&options, None)?;

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
        "params": {"service": "deepseek-app-server", "version": env!("CARGO_PKG_VERSION")}
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
    let state = build_state(&opts, None)?;
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
// ── Helper: write to stdout in raw mode ───────────────────────────────────
macro_rules! echo {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = write!(out, $($arg)*);
        let _ = out.flush();
    }};
}

pub async fn run_tui(config_path: Option<PathBuf>) -> Result<()> {
    let opts = AppServerOptions {
        listen: "127.0.0.1:0".parse().unwrap(),
        config_path,
        daemon: false,
        pid_file: None,
        auto_shutdown_idle: false,
        idle_timeout_secs: 300,
    };
    let state = build_state(&opts, None)?;
    let mut ui_state = UiState::default();

    let mut terminal = terminal::TerminalInput::new();
    terminal.enable_raw_mode().context("failed to enable raw terminal mode")?;

    // ── Channels ──────────────────────────────────────────────────────────
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<(String, Option<String>)>(32);
    let (output_tx, mut output_rx) = mpsc::channel::<(String, bool)>(64);
    // Mid-execution pushback channel: user types correction while agent works
    let (pushback_tx, mut pushback_rx) = mpsc::channel::<String>(16);
    // Async stdin events: blocking thread feeds UiEvents into this channel
    let (event_tx, mut event_rx) = mpsc::channel::<UiEvent>(64);

    // ── Cancellation signal for mid-execution interruption ────────────────
    let cancel_signal: Arc<Notify> = Arc::new(Notify::new());

    // ── Stdin reader thread (blocking read → async channel) ───────────────
    // Raw-mode stdin doesn't play well with tokio::io::stdin(), so we use a
    // dedicated blocking thread that feeds UiEvents into an mpsc channel.
    let mut stdin_terminal = terminal::TerminalInput::new();
    stdin_terminal.enable_raw_mode().context("failed to enable raw mode in stdin thread")?;
    let stdin_tx = event_tx.clone();
    let stdin_shutdown = Arc::new(AtomicBool::new(false));
    let stdin_shutdown_flag = stdin_shutdown.clone();
    std::thread::spawn(move || {
        loop {
            if stdin_shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            match stdin_terminal.read_events() {
                Ok(events) => {
                    for ev in events {
                        if stdin_tx.blocking_send(ev).is_err() {
                            return; // receiver dropped
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // ── Background prompt processor with cancellation + pushback support ──
    // Architecture:
    //   1. Receives prompt from main event loop (prompt_rx)
    //   2. Keeps `current_prompt` across retries so pushbacks can merge
    //   3. Spawns handle_prompt in a cancellable tokio::spawn task
    //   4. Races the spawned task against pushback_rx — on pushback: abort, merge, re-spawn
    //   5. On completion: clears current_prompt, waits for next prompt
    let bg_state = Arc::clone(&state.runtime);
    tokio::spawn(async move {
        // Current executing prompt (base text + thread_id) — persists across pushback restarts
        let mut current_prompt: Option<(String, Option<String>)> = None;
        // Handle to cancellable spawned handle_prompt task (kept outside select! for abort access)
        let mut spawn_handle: Option<tokio::task::JoinHandle<()>>;
        // Pushbacks accumulated between checks
        let mut accumulated_pushbacks: Vec<String> = Vec::new();

        loop {
            // ── Step 1: Drain any pushbacks that arrived since last iteration ──
            while let Ok(pb) = pushback_rx.try_recv() {
                accumulated_pushbacks.push(pb);
            }

            // ── Step 2: Determine the prompt to execute ──
            // If we have pushbacks for the current prompt, restart it with corrections.
            // Otherwise, wait for a new prompt from the user.
            let (mut prompt, thread_id) = if let Some((ref base_prompt, ref tid)) = current_prompt {
                if accumulated_pushbacks.is_empty() {
                    // No pushbacks — wait for a fresh prompt from main loop
                    match prompt_rx.recv().await {
                        Some(p) => {
                            current_prompt = Some((p.0.clone(), p.1.clone()));
                            p
                        }
                        None => break,
                    }
                } else {
                    // Pushbacks arrived — merge into current prompt and retry
                    let pb_text = accumulated_pushbacks.join("\n");
                    accumulated_pushbacks.clear();
                    let merged = merge_prompt_with_pushbacks(base_prompt.clone(), &[pb_text]);
                    (merged, tid.clone())
                }
            } else {
                match prompt_rx.recv().await {
                    Some(p) => {
                        current_prompt = Some((p.0.clone(), p.1.clone()));
                        p
                    }
                    None => break,
                }
            };

            // ── Step 3: Drain pushbacks that arrived during recv ──
            while let Ok(pb) = pushback_rx.try_recv() {
                accumulated_pushbacks.push(pb);
            }

            // Merge any pushbacks that arrived before we start executing
            if !accumulated_pushbacks.is_empty() {
                let pb_text = accumulated_pushbacks.join("\n");
                accumulated_pushbacks.clear();
                prompt = merge_prompt_with_pushbacks(prompt, &[pb_text]);
            }

            let _ = output_tx.send(("\r\x1b[K⏳ thinking…".to_string(), false)).await;

            // ── Step 4: Spawn handle_prompt in a cancellable task ──
            // This is the critical change: instead of calling handle_prompt inline
            // (which blocks the processor for minutes), we spawn it so we can abort
            // it when the user sends a correction.
            let req = PromptRequest {
                thread_id: thread_id.clone(),
                prompt: prompt.clone(),
                model: None,
            };
            let rt_clone = Arc::clone(&bg_state);
            let output_clone = output_tx.clone();

            // ── Bridge: oneshot channel to receive result without moving JoinHandle ──
            // Using oneshot lets us keep spawn_handle accessible in the pushback branch
            // for abort(), since tokio::select! would move a direct JoinHandle.
            let (done_tx, mut done_rx) = tokio::sync::oneshot::channel();
            spawn_handle = Some(tokio::spawn(async move {
                let mut rt = rt_clone.lock().await;
                let result = rt.handle_prompt(req, &CliRuntimeOverrides::default()).await;
                // Send result through oneshot — ignore error if receiver dropped (aborted)
                let _ = done_tx.send(result);
            }));

            // ── Step 5: Race — prompt completion vs pushback arrival ──
            let completed = tokio::select! {
                // Prompt finished normally (or errored)
                result = (&mut done_rx) => {
                    match result {
                        Ok(Ok(resp)) => {
                            let _ = output_clone.send((
                                format!("\r\x1b[K[model: {}]\n{}", resp.model, resp.output),
                                false,
                            )).await;
                        }
                        Ok(Err(e)) => {
                            let _ = output_clone.send((
                                format!("\r\x1b[K[error: {e}]"), true,
                            )).await;
                        }
                        Err(_recv_err) => {
                            // oneshot sender was dropped — task was aborted or panicked
                            let _ = output_clone.send((
                                "\r\x1b[K[agent task cancelled or panicked]".to_string(),
                                true,
                            )).await;
                        }
                    }
                    true // completed
                }
                // Pushback arrived mid-execution — abort the agent and rethink
                pb = pushback_rx.recv() => {
                    if let Some(pb_text) = pb {
                        accumulated_pushbacks.push(pb_text);
                    }
                    // Abort the in-progress handle_prompt — this drops the future,
                    // releases the runtime mutex, and cancels any in-flight HTTP requests
                    if let Some(h) = spawn_handle.take() {
                        h.abort();
                    }
                    let _ = output_clone.send((
                        "\r\x1b[K[correction received — rethinking…]".to_string(),
                        false,
                    )).await;
                    false // not completed — will loop and retry with merged prompt
                }
            };

            if completed {
                // Prompt finished — clear current_prompt so next iteration waits for fresh input
                current_prompt = None;
            }
            // If !completed, current_prompt stays set — next iteration merges and retries
        }
    });

    // ── Welcome banner ────────────────────────────────────────────────────
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║        DeepSeek TUI v{}               ║", env!("CARGO_PKG_VERSION"));
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!("║  Type a prompt and press Enter.          ║");
    eprintln!("║  Paste with Ctrl+V or Shift+Insert.      ║");
    eprintln!("║  Mid-exec: type corrections anytime.     ║");
    eprintln!("║  Auto-sends on period (.) or Enter.      ║");
    eprintln!("║  /save to persist.  /help for commands.  ║");
    eprintln!("║  Ctrl+C to exit (auto-saves session).    ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!();

    let mut thread_id: Option<String> = None;

    // ── Session auto-resume ───────────────────────────────────────────────
    // Check for a saved checkpoint from a previous session (Claude Code parity)
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let checkpoint_path = home.join(".deepseek").join("tui_checkpoint.json");
    if checkpoint_path.exists() {
        if let Ok(data) = std::fs::read_to_string(&checkpoint_path) {
            if let Ok(cp) = serde_json::from_str::<Checkpoint>(&data) {
                echo!("\r\n╔══════════════════════════════════════════╗\r\n");
                echo!("║  Previous session found:                 ║\r\n");
                echo!("║  {}  ║\r\n", cp.display());
                echo!("║  Press 'r' to resume, any key to skip    ║\r\n");
                echo!("╚══════════════════════════════════════════╝\r\n");
                // Read a single key
                if let Ok(events) = terminal.read_events() {
                    if let Some(UiEvent::KeyPressed('r')) = events.first() {
                        cp.restore(&mut ui_state);
                        thread_id = cp.tags.first().cloned();
                        echo!("[session resumed]\r\n\r\n");
                    } else {
                        echo!("[starting fresh]\r\n\r\n");
                    }
                }
                // Remove checkpoint after attempt (resume or skip)
                let _ = std::fs::remove_file(&checkpoint_path);
            }
        }
    }

    // ── Main async event loop ─────────────────────────────────────────────
    loop {
        tokio::select! {
            // ── Incoming UI events from stdin ──────────────────────────
            maybe_event = event_rx.recv() => {
                let Some(event) = maybe_event else { return Ok(()); };

                // ── Capture prompt text BEFORE reduce clears the buffer ──
                let prompt_before = if matches!(event, UiEvent::EnterPressed) {
                    Some(ui_state.input_buffer.trim().to_string())
                } else {
                    None
                };

                let effects = ui_state.reduce(event.clone());
                process_effects(&effects, &mut ui_state);

                // ── Handle key events (echo + special keys) ──────────────
                match event {
                    UiEvent::KeyPressed(ch) => {
                        match ch {
                            '\x03' | '\x04' => {
                                echo!("\r\nexiting.\r\n");
                                // Auto-save session checkpoint before exit (Claude Code parity)
                                let cp = Checkpoint::capture(&ui_state, "auto-save", Some("autosave on exit"));
                                if let Some(ref tid) = thread_id {
                                    let mut cp_with_tag = cp;
                                    cp_with_tag.tags.push(tid.clone());
                                    if let Ok(json) = serde_json::to_string(&cp_with_tag) {
                                        let _ = std::fs::write(&checkpoint_path, json);
                                    }
                                } else {
                                    if let Ok(json) = serde_json::to_string(&cp) {
                                        let _ = std::fs::write(&checkpoint_path, json);
                                    }
                                }
                                stdin_shutdown.store(true, Ordering::Relaxed);
                                let _ = terminal.disable_raw_mode();
                                return Ok(());
                            }
                            '\x7f' | '\x08' => {
                                // Backspace: reduce already pushed the char; undo it
                                if !ui_state.input_buffer.is_empty() {
                                    ui_state.input_buffer.pop();
                                    echo!("\x08 \x08");
                                }
                            }
                            '\t' => {
                                echo!("[tab]");
                                // Undo tab char pushed by reduce
                                if ui_state.input_buffer.ends_with('\t') {
                                    ui_state.input_buffer.pop();
                                }
                            }
                            _ => {
                                // Echo printable characters in raw mode
                                if ch.is_ascii_graphic() || ch == ' ' {
                                    let mut buf = [0u8; 4];
                                    let s = ch.encode_utf8(&mut buf);
                                    echo!("{}", s);
                                }
                                // Auto-pushback: during execution, send correction on sentence boundaries
                                // without requiring Enter. Claude Code-style continuous feedback.
                                if ui_state.pending_tasks > 0 && ch == '.' {
                                    let trimmed = ui_state.input_buffer.trim();
                                    if !trimmed.is_empty() && trimmed.len() > 3 {
                                        let correction = trimmed.to_string();
                                        // Non-blocking send — channel has capacity 16
                                        let _ = pushback_tx.try_send(correction);
                                        echo!(" \x1b[33m↻\x1b[0m");
                                    }
                                }
                            }
                        }
                    }
                    UiEvent::EnterPressed => {
                        let prompt = prompt_before.unwrap_or_default();
                        if prompt.is_empty() {
                            continue;
                        }
                        echo!("\r\n");

                        // ── Local slash commands ────────────────────────
                        match prompt.as_str() {
                            "/save" => {
                                let cp = Checkpoint::capture(&ui_state, "manual-save", Some("user-requested save"));
                                let mut cp_with_tag = cp;
                                if let Some(ref tid) = thread_id {
                                    cp_with_tag.tags.push(tid.clone());
                                }
                                if let Ok(json) = serde_json::to_string(&cp_with_tag) {
                                    if let Err(e) = std::fs::write(&checkpoint_path, json) {
                                        echo!("[save failed: {e}]\r\n\r\n");
                                    } else {
                                        echo!("[session saved — resume with 'deepseek tui']\r\n\r\n");
                                    }
                                }
                                continue;
                            }
                            "/exit" | "/quit" | "/q" => {
                                echo!("exiting.\r\n");
                                // Auto-save on clean exit
                                let cp = Checkpoint::capture(&ui_state, "auto-save", Some("autosave on exit"));
                                if let Some(ref tid) = thread_id {
                                    let mut cp_with_tag = cp;
                                    cp_with_tag.tags.push(tid.clone());
                                    if let Ok(json) = serde_json::to_string(&cp_with_tag) {
                                        let _ = std::fs::write(&checkpoint_path, json);
                                    }
                                } else {
                                    if let Ok(json) = serde_json::to_string(&cp) {
                                        let _ = std::fs::write(&checkpoint_path, json);
                                    }
                                }
                                stdin_shutdown.store(true, Ordering::Relaxed);
                                let _ = terminal.disable_raw_mode();
                                return Ok(());
                            }
                            "/clear" => {
                                thread_id = None;
                                ui_state.pending_tasks = 0;
                                ui_state.pending_pushback = None;
                                echo!("[conversation cleared]\r\n\r\n");
                                continue;
                            }
                            "/status" => {
                                let status = format!(
                                    "thread: {} | tasks: {} | pushback: {}\r\n\r\n",
                                    thread_id.as_deref().unwrap_or("none"),
                                    ui_state.pending_tasks,
                                    ui_state.pending_pushback.as_deref().unwrap_or("none"),
                                );
                                echo!("{}", status);
                                continue;
                            }
                            "/help" => {
                                echo!("/exit /quit /q — exit\r\n");
                                echo!("/clear — clear conversation\r\n");
                                echo!("/save — save session for resume\r\n");
                                echo!("/status — show status\r\n");
                                echo!("/paste — test paste detection\r\n");
                                echo!("Type anytime during agent execution to correct it.\r\n");
                                echo!("Corrections auto-send on period (.) or Enter.\r\n");
                                echo!("\r\n");
                                continue;
                            }
                            _ => {}
                        }

                        // ── Pushback: mid-execution correction ──────────
                        if ui_state.pending_pushback.is_some() {
                            let _ = pushback_tx.send(prompt.clone()).await;
                            cancel_signal.notify_one();
                            echo!("[pushback sent — agent will rethink]\r\n");
                            continue;
                        }

                        // ── Normal prompt: submit to background task ────
                        let tid = thread_id.clone();
                        if let Err(_) = prompt_tx.send((prompt.clone(), tid)).await {
                            echo!("[error: prompt queue full]\r\n\r\n");
                        } else {
                            ui_state.pending_tasks = ui_state.pending_tasks.saturating_add(1);
                            echo!("[processing…]\r\n");
                        }
                    }
                    UiEvent::PasteContent { content, .. } => {
                        echo!("{}", content);
                    }
                    _ => {}
                }
            }

            // ── Output from background task ──────────────────────────────
            maybe_output = output_rx.recv() => {
                let Some((text, is_error)) = maybe_output else { return Ok(()); };
                ui_state.pending_tasks = ui_state.pending_tasks.saturating_sub(1);
                if is_error {
                    eprintln!("{}", text);
                } else {
                    println!("{}", text);
                }
                // Show quick status after response
                let status = format!(
                    "\r\x1b[K[{} tasks remaining]",
                    ui_state.pending_tasks
                );
                echo!("{}", status);
                println!();
            }
        }
    }
}

// ── Rich TUI (ratatui) ─────────────────────────────────────────────────
// Drop-in replacement for run_tui using ratatui for proper terminal rendering.
// Integrates the same channel-based pushback/processing backend with a
// rich multi-pane terminal UI.
pub async fn run_tui_rich(config_path: Option<PathBuf>) -> Result<()> {
    use std::io;
    use std::time::{Duration, Instant};
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use deepseek_tui_core::Pane;
    use ratatui::{Terminal, backend::CrosstermBackend};

    let opts = AppServerOptions {
        listen: "127.0.0.1:0".parse().unwrap(),
        config_path,
        daemon: false,
        pid_file: None,
        auto_shutdown_idle: false,
        idle_timeout_secs: 300,
    };
    // ── Streaming hook channel for real-time output ─────────────────────
    let (stream_tx, mut stream_rx) = tokio::sync::mpsc::unbounded_channel::<HookEvent>();

    let state = build_state(&opts, Some(stream_tx))?;
    let mut ui_state = UiState::default();

    // ── Channels (same as run_tui) ──────────────────────────────────────
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<(String, Option<String>)>(32);
    let (output_tx, mut output_rx) = mpsc::channel::<(String, bool)>(64);
    let (pushback_tx, mut pushback_rx) = mpsc::channel::<String>(16);
    // Swarm agent status updates (Vec<AgentDisplay> or empty)
    let (agents_tx, mut agents_rx) = mpsc::channel::<Vec<tui::widgets::agents::AgentDisplay>>(8);

    // ── Periodic swarm status poller ─────────────────────────────────────
    let swarm_ref = Arc::clone(&state.swarm);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let specs = swarm_ref.list_agents().await;
            let displays: Vec<tui::widgets::agents::AgentDisplay> = specs.into_iter().map(|s| {
                use tui::widgets::agents::AgentDisplayStatus;
                tui::widgets::agents::AgentDisplay {
                    id: s.id,
                    role: s.role.label().to_string(),
                    name: s.name,
                    status: AgentDisplayStatus::Idle, // TODO: map from swarm agent status
                    task: None,
                    model: s.model,
                }
            }).collect();
            if agents_tx.send(displays).await.is_err() { break; }
        }
    });

    // ── Background prompt processor (same cancellable architecture) ─────
    let bg_state = Arc::clone(&state.runtime);
    tokio::spawn(async move {
        let mut current_prompt: Option<(String, Option<String>)> = None;
        let mut spawn_handle: Option<tokio::task::JoinHandle<()>>;
        let mut accumulated_pushbacks: Vec<String> = Vec::new();

        loop {
            while let Ok(pb) = pushback_rx.try_recv() {
                accumulated_pushbacks.push(pb);
            }

            let (mut prompt, thread_id) = if let Some((ref base_prompt, ref tid)) = current_prompt {
                if accumulated_pushbacks.is_empty() {
                    match prompt_rx.recv().await {
                        Some(p) => { current_prompt = Some((p.0.clone(), p.1.clone())); p }
                        None => break,
                    }
                } else {
                    let pb_text = accumulated_pushbacks.join("\n");
                    accumulated_pushbacks.clear();
                    let merged = merge_prompt_with_pushbacks(base_prompt.clone(), &[pb_text]);
                    (merged, tid.clone())
                }
            } else {
                match prompt_rx.recv().await {
                    Some(p) => { current_prompt = Some((p.0.clone(), p.1.clone())); p }
                    None => break,
                }
            };

            while let Ok(pb) = pushback_rx.try_recv() {
                accumulated_pushbacks.push(pb);
            }
            if !accumulated_pushbacks.is_empty() {
                let pb_text = accumulated_pushbacks.join("\n");
                accumulated_pushbacks.clear();
                prompt = merge_prompt_with_pushbacks(prompt, &[pb_text]);
            }

            let _ = output_tx.send(("⏳ thinking…".to_string(), false)).await;

            let req = PromptRequest { thread_id: thread_id.clone(), prompt: prompt.clone(), model: None };
            let rt_clone = Arc::clone(&bg_state);
            let output_clone = output_tx.clone();

            let (done_tx, mut done_rx) = tokio::sync::oneshot::channel();
            spawn_handle = Some(tokio::spawn(async move {
                let mut rt = rt_clone.lock().await;
                let result = rt.handle_prompt(req, &CliRuntimeOverrides::default()).await;
                let _ = done_tx.send(result);
            }));

            let completed = tokio::select! {
                result = (&mut done_rx) => {
                    match result {
                        Ok(Ok(resp)) => {
                            let _ = output_clone.send((
                                format!("[model: {}]\n{}", resp.model, resp.output), false,
                            )).await;
                        }
                        Ok(Err(e)) => {
                            let _ = output_clone.send((format!("[error: {e}]"), true)).await;
                        }
                        Err(_) => {
                            let _ = output_clone.send(("[agent task cancelled]".to_string(), true)).await;
                        }
                    }
                    true
                }
                pb = pushback_rx.recv() => {
                    if let Some(pb_text) = pb { accumulated_pushbacks.push(pb_text); }
                    if let Some(h) = spawn_handle.take() { h.abort(); }
                    let _ = output_clone.send(("[correction received — rethinking…]".to_string(), false)).await;
                    false
                }
            };

            if completed { current_prompt = None; }
        }
    });

    // ── Session auto-resume ─────────────────────────────────────────────
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let checkpoint_path = home.join(".deepseek").join("tui_checkpoint.json");
    if checkpoint_path.exists() {
        if let Ok(data) = std::fs::read_to_string(&checkpoint_path) {
            if let Ok(cp) = serde_json::from_str::<Checkpoint>(&data) {
                // Auto-resume in rich mode: restore state silently
                cp.restore(&mut ui_state);
                let _ = std::fs::remove_file(&checkpoint_path);
            }
        }
    }

    let mut thread_id: Option<String> = None;

    // ── Ratatui terminal setup ──────────────────────────────────────────
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── TUI app state ───────────────────────────────────────────────────
    let mut tui = tui::app::TuiApp::new();
    let tick = Duration::from_millis(100); // 10 FPS
    let mut last_tick = Instant::now();

    // ── Ratatui event loop ──────────────────────────────────────────────
    let running = true;
    let result: Result<()> = loop {
        if !running { break Ok(()); }

        // Check for output from background processor
        while let Ok((text, is_error)) = output_rx.try_recv() {
            if text.starts_with("[correction received") || text.starts_with("⏳ thinking") {
                tui.streaming = true;
                continue;
            }
            let role = if is_error { tui::widgets::chat::MessageRole::Error }
                else { tui::widgets::chat::MessageRole::Assistant };
            tui.add_message(role, text);
            tui.streaming = false;
            tui.scroll_to_bottom();
            ui_state.pending_tasks = ui_state.pending_tasks.saturating_sub(1);
        }

        // Check for swarm agent status updates
        while let Ok(agents) = agents_rx.try_recv() {
            tui.agents.agents = agents;
        }

        // Process streaming hook events (real-time text deltas + tool calls)
        while let Ok(event) = stream_rx.try_recv() {
            match event {
                HookEvent::ResponseStart { .. } => {
                    tui.streaming = true;
                }
                HookEvent::ResponseDelta { delta, .. } => {
                    // Append delta to the last assistant message or create one
                    tui.chat.streaming_text.push_str(&delta);
                }
                HookEvent::ToolLifecycle { tool_name, phase, .. } => {
                    if phase == "start" {
                        tui.add_message(
                            tui::widgets::chat::MessageRole::Tool,
                            format!("🔧 running {}", tool_name),
                        );
                    } else if phase == "end" {
                        tui.add_message(
                            tui::widgets::chat::MessageRole::Tool,
                            format!("✅ completed {}", tool_name),
                        );
                    }
                }
                HookEvent::ResponseEnd { .. } => {
                    // Flush streaming text to a real message
                    let streamed = std::mem::take(&mut tui.chat.streaming_text);
                    if !streamed.is_empty() {
                        tui.add_message(
                            tui::widgets::chat::MessageRole::Assistant,
                            streamed,
                        );
                    }
                    tui.streaming = false;
                    tui.scroll_to_bottom();
                }
                _ => {}
            }
        }

        // Poll for input
        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Auto-save and exit
                            let cp = Checkpoint::capture(&ui_state, "auto-save", Some("autosave on exit"));
                            if let Some(ref tid) = thread_id {
                                let mut c = cp; c.tags.push(tid.clone());
                                if let Ok(json) = serde_json::to_string(&c) { let _ = std::fs::write(&checkpoint_path, json); }
                            } else {
                                if let Ok(json) = serde_json::to_string(&cp) { let _ = std::fs::write(&checkpoint_path, json); }
                            }
                            break Ok(());
                        }
                        KeyCode::Enter => {
                            // Only process input in Chat pane; in other panes, Enter switches to Chat
                            if tui.active_pane != Pane::Chat {
                                tui.switch_pane(Pane::Chat);
                                continue;
                            }
                            let prompt = tui.chat.input.trim().to_string();
                            tui.chat.input.clear();
                            if prompt.is_empty() { continue; }

                            // Slash commands
                            match prompt.as_str() {
                                "/exit" | "/quit" | "/q" => {
                                    let cp = Checkpoint::capture(&ui_state, "auto-save", Some("autosave on exit"));
                                    if let Some(ref tid) = thread_id {
                                        let mut c = cp; c.tags.push(tid.clone());
                                        if let Ok(json) = serde_json::to_string(&c) { let _ = std::fs::write(&checkpoint_path, json); }
                                    }
                                    break Ok(());
                                }
                                "/clear" => {
                                    thread_id = None;
                                    tui.chat.messages.clear();
                                    tui.chat.input.clear();
                                    ui_state.pending_tasks = 0;
                                    continue;
                                }
                                "/diff" => {
                                    if let Ok(output) = std::process::Command::new("git")
                                        .args(["diff", "--color=never"]).output()
                                    {
                                        tui.diff.load_diff(&String::from_utf8_lossy(&output.stdout));
                                    }
                                    tui.switch_pane(Pane::Diff);
                                    continue;
                                }
                                "/swarm" => {
                                    // Trigger immediate agent list refresh
                                    let specs = state.swarm.list_agents().await;
                                    let displays: Vec<tui::widgets::agents::AgentDisplay> = specs.into_iter().map(|s| {
                                        use tui::widgets::agents::AgentDisplayStatus;
                                        tui::widgets::agents::AgentDisplay {
                                            id: s.id, role: s.role.label().to_string(), name: s.name,
                                            status: AgentDisplayStatus::Idle, task: None, model: s.model,
                                        }
                                    }).collect();
                                    tui.agents.agents = displays;
                                    tui.switch_pane(Pane::Agents);
                                    continue;
                                }
                                "/tasks" => {
                                    load_gsd_tasks(&mut tui);
                                    tui.switch_pane(Pane::Tasks);
                                    continue;
                                }
                                "/save" => {
                                    let cp = Checkpoint::capture(&ui_state, "manual-save", Some("user save"));
                                    let mut c = cp;
                                    if let Some(ref tid) = thread_id { c.tags.push(tid.clone()); }
                                    if let Ok(json) = serde_json::to_string(&c) { let _ = std::fs::write(&checkpoint_path, json); }
                                    tui.add_message(tui::widgets::chat::MessageRole::System, "Session saved.".into());
                                    continue;
                                }
                                _ => {}
                            }

                            // Pushback or new prompt
                            if ui_state.pending_pushback.is_some() || tui.streaming {
                                let _ = pushback_tx.send(prompt.clone()).await;
                                tui.add_message(tui::widgets::chat::MessageRole::System, format!("Correction sent: {}", prompt));
                            } else {
                                tui.add_message(tui::widgets::chat::MessageRole::User, prompt.clone());
                                let tid = thread_id.clone();
                                if prompt_tx.send((prompt, tid)).await.is_err() {
                                    tui.add_message(tui::widgets::chat::MessageRole::Error, "Prompt queue full.".into());
                                } else {
                                    ui_state.pending_tasks = ui_state.pending_tasks.saturating_add(1);
                                    tui.streaming = true;
                                }
                            }
                            tui.round_count += 1;
                            tui.scroll_to_bottom();
                        }
                        KeyCode::Backspace => {
                            tui.chat.input.pop();
                        }
                        // ── Pane switching via number keys ──────────────────
                        KeyCode::Char('1') => tui.switch_pane(Pane::Chat),
                        KeyCode::Char('2') => {
                            // Load git diff on switch to Diff pane
                            if tui.active_pane != Pane::Diff {
                                if let Ok(output) = std::process::Command::new("git")
                                    .args(["diff", "--color=never"])
                                    .output()
                                {
                                    let diff_text = String::from_utf8_lossy(&output.stdout);
                                    tui.diff.load_diff(&diff_text);
                                }
                            }
                            tui.switch_pane(Pane::Diff);
                        }
                        KeyCode::Char('3') => {
                            // Load tasks from GSD planning on switch
                            if tui.active_pane != Pane::Tasks && tui.tasks.tasks.is_empty() {
                                load_gsd_tasks(&mut tui);
                            }
                            tui.switch_pane(Pane::Tasks);
                        }
                        KeyCode::Char('4') => tui.switch_pane(Pane::Agents),
                        KeyCode::Char('5') => tui.switch_pane(Pane::Jobs),
                        // ── Text input (Chat pane only) ────────────────────
                        KeyCode::Char(ch) => {
                            if tui.active_pane == Pane::Chat {
                                tui.chat.input.push(ch);
                                // Auto-pushback on period during execution
                                if tui.streaming && ch == '.' && tui.chat.input.trim().len() > 3 {
                                    let correction = tui.chat.input.trim().to_string();
                                    let _ = pushback_tx.try_send(correction);
                                }
                            }
                        }
                        // ── Scroll (routes to active pane) ────────────────
                        KeyCode::Up => { let o = tui.active_scroll_offset_mut(); *o = o.saturating_add(1); }
                        KeyCode::Down => { let o = tui.active_scroll_offset_mut(); *o = o.saturating_sub(1); }
                        KeyCode::PageUp => { let o = tui.active_scroll_offset_mut(); *o = o.saturating_add(10); }
                        KeyCode::PageDown => { let o = tui.active_scroll_offset_mut(); *o = o.saturating_sub(10); }
                        _ => {}
                    }
                }
            }
        }

        // Periodic UI state sync
        if last_tick.elapsed() >= tick {
            tui.mode = ui_state.mode;
            tui.budget.used_tokens = ((1.0 - ui_state.context_budget.pct_remaining()) * ui_state.context_budget.total_tokens as f64) as usize;
            last_tick = Instant::now();
        }

        // Render frame
        terminal.draw(|f| tui.render(f))?;
    };

    // ── Restore terminal ────────────────────────────────────────────────
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ── Helper: merge pushbacks into prompt ───────────────────────────────────
fn merge_prompt_with_pushbacks(prompt: String, pushbacks: &[String]) -> String {
    if pushbacks.is_empty() {
        return prompt;
    }
    let corrections = pushbacks.join("\n");
    format!("{}\n\n[USER CORRECTION (mid-execution)]: {}", prompt, corrections)
}

// ── Helper: process UiEffects from the state machine ──────────────────────
fn process_effects(effects: &EffectVec, ui_state: &mut UiState) {
    for effect in effects {
        match effect {
            UiEffect::Render => { /* no-op: live-echo handles rendering */ }
            UiEffect::EmitStatusLine(line) => {
                ui_state.status_line = line.clone();
            }
            UiEffect::PushbackDraft(pb) => {
                ui_state.pending_pushback = Some(pb.clone());
            }
            _ => {}
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


// ── Channel-based hook sink for real-time TUI streaming ──────────────────

struct ChannelHookSink {
    tx: tokio::sync::mpsc::UnboundedSender<HookEvent>,
}

#[async_trait]
impl HookSink for ChannelHookSink {
    async fn emit(&self, event: &HookEvent) -> anyhow::Result<()> {
        // Ignore send errors — receiver was dropped (TUI closed)
        let _ = self.tx.send(event.clone());
        Ok(())
    }
}

// ── State construction ─────────────────────────────────────────────────────

fn build_state(options: &AppServerOptions, hook_stream_tx: Option<tokio::sync::mpsc::UnboundedSender<deepseek_hooks::HookEvent>>) -> Result<AppState> {
    let store = ConfigStore::load(options.config_path.clone())?;
    let config = store.config.clone();
    let registry = ModelRegistry::default();
    let state_db = options.config_path.as_ref().and_then(|p| p.parent().map(|x| x.join("state.db")));
    let state_store = StateStore::open(state_db)?;
    let mut hooks = HookDispatcher::default();
    hooks.add_sink(Arc::new(StdoutHookSink));
    let hl = options.config_path.as_ref().and_then(|p| p.parent().map(|x| x.join("events.jsonl"))).unwrap_or_else(|| PathBuf::from(".deepseek/events.jsonl"));
    hooks.add_sink(Arc::new(JsonlHookSink::new(hl)));
    // Optional streaming hook sink for TUI real-time output
    if let Some(tx) = hook_stream_tx {
        hooks.add_sink(Arc::new(ChannelHookSink { tx }));
    }
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

// ── Helper: load GSD tasks from .planning/ directory ────────────────────
#[allow(dead_code)]
fn load_gsd_tasks(tui: &mut tui::app::TuiApp) {
    // Try to load tasks from the GSD planning directory
    let planning_dir = PathBuf::from(".planning");
    if !planning_dir.exists() {
        // No planning dir — show empty state
        tui.tasks.load_tasks(vec![]);
        return;
    }

    // Try to read ROIADMAP.md or STATE.md for task information
    let mut items = Vec::new();

    // Check for ROADMAP.md
    let roadmap_path = planning_dir.join("ROADMAP.md");
    if roadmap_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&roadmap_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("- [ ]") {
                    let title = trimmed.replacen("- [ ]", "", 1).trim().to_string();
                    items.push((title, "pending".to_string(), "roadmap".to_string()));
                } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
                    let title = trimmed.replacen("- [x]", "", 1).replacen("- [X]", "", 1).trim().to_string();
                    items.push((title, "completed".to_string(), "roadmap".to_string()));
                }
            }
        }
    }

    // Check for STATE.md blockers
    let state_path = planning_dir.join("STATE.md");
    if state_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&state_path) {
            let mut in_blockers = false;
            for line in content.lines() {
                if line.contains("Blockers") || line.contains("blockers") {
                    in_blockers = true;
                    continue;
                }
                if in_blockers && (line.starts_with('-') || line.starts_with('*')) {
                    if line.is_empty() { in_blockers = false; continue; }
                    let title = line.trim_start_matches(&['-', '*', ' '][..]).trim().to_string();
                    if !title.is_empty() {
                        items.push((title, "blocked".to_string(), "blocker".to_string()));
                    }
                }
            }
        }
    }

    tui.tasks.load_tasks(items);
}