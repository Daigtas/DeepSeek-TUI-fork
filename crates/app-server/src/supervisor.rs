//! Daemon supervisor: agent lifecycle persistence, progress logging, session resumption.
//!
//! Ensures agents continue working when clients disconnect, and provides
//! progress visibility when reconnecting from another device.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use deepseek_context::HybridContextStore;
use deepseek_session::SessionStore;
use deepseek_swarm::SwarmOrchestrator;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::info;

// ============================================================================
// Progress log entry
// ============================================================================

/// A checkpoint in the daemon's progress log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Event type: agent_started, agent_completed, phase_complete, checkpoint, info
    pub event: String,
    /// Human-readable message.
    pub message: String,
    /// Optional structured data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ============================================================================
// Resume suggestion
// ============================================================================

/// What the user should do when reconnecting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSuggestion {
    /// Whether there's work to resume.
    pub has_work: bool,
    /// Human-readable status summary.
    pub summary: String,
    /// Last 20 progress entries for context.
    pub recent_progress: Vec<ProgressEntry>,
    /// Active agents (if any).
    pub active_agents: Vec<ActiveAgentInfo>,
    /// Suggested next action: "resume", "review", "idle"
    pub suggested_action: String,
    /// The session ID to resume (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Hive mind summary for context.
    #[serde(default)]
    pub hive_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAgentInfo {
    pub id: String,
    pub role: String,
    pub name: String,
}

// ============================================================================
// Daemon supervisor
// ============================================================================

/// Manages daemon lifecycle: progress logging, session resumption state.
pub struct DaemonSupervisor {
    /// Context store for persistent progress logging.
    context_store: Arc<HybridContextStore>,
    /// Session store for listing recent sessions.
    session_store: Arc<SessionStore>,
    /// Swarm orchestrator for agent state.
    swarm: Arc<SwarmOrchestrator>,
    /// In-memory progress buffer (recent entries).
    progress: RwLock<Vec<ProgressEntry>>,
    /// Maximum in-memory progress entries.
    max_progress_entries: usize,
    /// Path to the progress log file (JSONL).
    log_path: Option<PathBuf>,
}

impl DaemonSupervisor {
    /// Create a new daemon supervisor.
    pub fn new(
        context_store: Arc<HybridContextStore>,
        session_store: Arc<SessionStore>,
        swarm: Arc<SwarmOrchestrator>,
        log_path: Option<PathBuf>,
    ) -> Self {
        Self {
            context_store,
            session_store,
            swarm,
            progress: RwLock::new(Vec::new()),
            max_progress_entries: 500,
            log_path,
        }
    }

    /// Log a progress event. Writes to in-memory buffer and optional JSONL log file.
    /// Context store persistence is deferred to avoid blocking tokio workers.
    pub async fn log(&self, event: &str, message: &str, data: Option<Value>) {
        let entry = ProgressEntry {
            timestamp: Utc::now().to_rfc3339(),
            event: event.to_string(),
            message: message.to_string(),
            data,
        };

        // In-memory buffer (non-blocking)
        {
            let mut buf = self.progress.write().await;
            buf.push(entry.clone());
            if buf.len() > self.max_progress_entries {
                buf.remove(0);
            }
        }

        // Write to JSONL log file (non-blocking)
        if let Some(ref path) = self.log_path {
            if let Ok(json) = serde_json::to_string(&entry) {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    use std::io::Write;
                    let _ = writeln!(file, "{json}");
                }
            }
        }

        // Defer context store persistence to checkpoint
        let key = format!("daemon.progress.{}", entry.timestamp);
        let store = self.context_store.clone();
        if let Ok(json) = serde_json::to_string(&entry) {
            tokio::task::spawn_blocking(move || {
                let _ = store.set_workspace_state("_daemon", &key, &json);
            });
        }

        info!(%event, %message, "daemon progress");
    }

    /// Log that an agent started working.
    pub async fn agent_started(&self, agent_id: &str, role: &str, name: &str) {
        self.log(
            "agent_started",
            &format!("Agent {name} ({role}) started"),
            Some(json!({"agent_id": agent_id, "role": role, "name": name})),
        ).await;
    }

    /// Log that an agent completed work.
    pub async fn agent_completed(&self, agent_id: &str, role: &str, summary: &str) {
        self.log(
            "agent_completed",
            &format!("Agent ({role}) completed: {summary}"),
            Some(json!({"agent_id": agent_id, "role": role, "summary": summary})),
        ).await;
    }

    /// Log a phase completion checkpoint.
    pub async fn phase_completed(&self, phase: u32, plans_completed: usize, plans_total: usize) {
        self.log(
            "phase_completed",
            &format!("Phase {phase} completed: {plans_completed}/{plans_total} plans"),
            Some(json!({"phase": phase, "plans_completed": plans_completed, "plans_total": plans_total})),
        ).await;
    }

    /// Get recent progress entries (in-memory).
    pub async fn recent_progress(&self, limit: usize) -> Vec<ProgressEntry> {
        let buf = self.progress.read().await;
        let start = if buf.len() > limit { buf.len() - limit } else { 0 };
        buf[start..].to_vec()
    }

    /// Build a resume suggestion for a reconnecting client.
    pub async fn build_resume_suggestion(&self) -> ResumeSuggestion {
        // Check for active agents
        let active_agents: Vec<ActiveAgentInfo> = self
            .swarm
            .list_agents()
            .await
            .into_iter()
            .map(|a| ActiveAgentInfo {
                id: a.id,
                role: a.role.label().to_string(),
                name: a.name,
            })
            .collect();

        let has_active_work = !active_agents.is_empty();

        // Get hive summary
        let hive_summary = self.swarm.hive_summary().await;

        // Get recent sessions
        let recent_sessions = self.session_store.list().unwrap_or_default();
        let last_session = recent_sessions.first();

        // Build summary
        let summary = if has_active_work {
            format!(
                "{} agent(s) active: {}. Hive has entries.",
                active_agents.len(),
                active_agents.iter()
                    .map(|a| format!("{} ({})", a.name, a.role))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else if let Some(session) = last_session {
            format!(
                "Last session: '{}' ({} turns, updated {}). No active agents.",
                session.name,
                session.turn_count,
                &session.updated_at[..19.min(session.updated_at.len())]
            )
        } else {
            "No active work. Ready for new tasks.".to_string()
        };

        let suggested_action = if has_active_work {
            "resume"
        } else if last_session.is_some() {
            "review"
        } else {
            "idle"
        };

        ResumeSuggestion {
            has_work: has_active_work || last_session.is_some(),
            summary,
            recent_progress: self.recent_progress(20).await,
            active_agents,
            suggested_action: suggested_action.to_string(),
            session_id: last_session.map(|s| s.id.clone()),
            hive_summary,
        }
    }

    /// Persist hive mind to context store (checkpoint before idle/detach).
    pub async fn checkpoint_hive(&self) -> Result<()> {
        self.swarm.persist_to_store(&self.context_store).await?;
        self.log("checkpoint", "Hive mind persisted to context store", None).await;
        Ok(())
    }

    /// Restore hive mind from context store (on startup/resume).
    pub async fn restore_hive(&self) -> Result<usize> {
        let count = self.swarm.restore_from_store(&self.context_store).await?;
        if count > 0 {
            self.log("restore", &format!("Restored {count} hive entries from context store"), None).await;
        }
        Ok(count)
    }
}
