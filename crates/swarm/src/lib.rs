use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use deepseek_context::HybridContextStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ============================================================================
// Hive Mind — shared context across all agents
// ============================================================================

/// A single entry in the hive mind knowledge store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveEntry {
    /// Unique key (namespaced).
    pub key: String,
    /// JSON value.
    pub value: Value,
    /// Monotonic version number.
    pub version: u64,
    /// Agent that wrote this entry.
    pub author: String,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Optional tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Confidence / relevance score (0.0 – 1.0).
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 { 0.8 }

/// A notification that a hive entry was created or updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveNotification {
    pub key: String,
    pub version: u64,
    pub action: HiveAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HiveAction {
    Created,
    Updated,
    Deleted,
}

/// Shared context store accessible by all agents in the swarm.
///
/// Features:
/// - Key-value store with versioning
/// - Pub/sub notifications on changes
/// - Namespace isolation (`agent.*`, `finding.*`, `decision.*`, `task.*`)
/// - Full snapshot for new agent initialization
pub struct HiveMind {
    /// All entries, keyed by normalized key.
    entries: RwLock<BTreeMap<String, HiveEntry>>,
    /// Global version counter.
    version_counter: Mutex<u64>,
    /// Broadcast channel for change notifications.
    tx: broadcast::Sender<HiveNotification>,
}

impl HiveMind {
    /// Create a new empty hive mind.
    #[must_use]
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            entries: RwLock::new(BTreeMap::new()),
            version_counter: Mutex::new(0),
            tx,
        }
    }

    /// Subscribe to hive notifications. Returns a receiver that gets every
    /// create/update/delete event.
    pub fn subscribe(&self) -> broadcast::Receiver<HiveNotification> {
        self.tx.subscribe()
    }

    /// Inject or update an entry.
    ///
    /// Returns the new version number.
    pub async fn inject(
        &self,
        key: &str,
        value: Value,
        author: &str,
        tags: Vec<String>,
        confidence: f64,
    ) -> Result<u64> {
        let normalized = normalize_key(key);
        let mut version = self.version_counter.lock().await;
        *version += 1;

        let entry = HiveEntry {
            key: normalized.clone(),
            value,
            version: *version,
            author: author.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            tags,
            confidence,
        };

        let action = if self.entries.read().await.contains_key(&normalized) {
            HiveAction::Updated
        } else {
            HiveAction::Created
        };

        self.entries.write().await.insert(normalized.clone(), entry);

        let _ = self.tx.send(HiveNotification {
            key: normalized,
            version: *version,
            action,
        });

        Ok(*version)
    }

    /// Query a single entry by key.
    pub async fn query(&self, key: &str) -> Option<HiveEntry> {
        self.entries.read().await.get(&normalize_key(key)).cloned()
    }

    /// Query all entries matching a key prefix (namespace).
    ///
    /// Example: `query_prefix("finding.")` returns all findings.
    pub async fn query_prefix(&self, prefix: &str) -> Vec<HiveEntry> {
        let normalized = normalize_key(prefix);
        self.entries
            .read()
            .await
            .range(normalized.clone()..)
            .take_while(|(k, _)| k.starts_with(&normalized))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Query entries by tag.
    pub async fn query_by_tag(&self, tag: &str) -> Vec<HiveEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Delete an entry.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let normalized = normalize_key(key);
        if self.entries.write().await.remove(&normalized).is_some() {
            let version = {
                let mut v = self.version_counter.lock().await;
                *v += 1;
                *v
            };
            let _ = self.tx.send(HiveNotification {
                key: normalized,
                version,
                action: HiveAction::Deleted,
            });
        }
        Ok(())
    }

    /// Full snapshot of all entries (for initializing a new agent's context).
    pub async fn snapshot(&self) -> Vec<HiveEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// Compact summary for injecting into an agent's system prompt.
    pub async fn summary(&self) -> String {
        let entries = self.entries.read().await;
        if entries.is_empty() {
            return "(no shared context)".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push("## Shared Context (Hive Mind)".to_string());

        for (key, entry) in entries.iter() {
            let tags = if entry.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", entry.tags.join(", "))
            };
            let conf = if (entry.confidence - 1.0).abs() > f64::EPSILON {
                format!(" (confidence: {:.0}%)", entry.confidence * 100.0)
            } else {
                String::new()
            };
            lines.push(format!(
                "- **{key}** v{} by {}{}{}: {}",
                entry.version,
                entry.author,
                tags,
                conf,
                truncate_value(&entry.value, 200),
            ));
        }

        lines.join("\n")
    }

    /// Total number of entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Whether the hive is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

impl Default for HiveMind {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Agent roles
// ============================================================================

/// Specialized agent role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Read-only investigation: reads files, searches code, explores structure.
    Explorer,
    /// Writes and modifies code, runs tools, makes changes.
    Implementer,
    /// Reviews code for bugs, security issues, quality problems.
    Reviewer,
    /// Runs tests, validates behavior, checks acceptance criteria.
    Tester,
    /// Creates plans, decomposes tasks, writes checklists.
    Planner,
    /// Orchestrates other agents, merges results, resolves conflicts.
    Coordinator,
    /// General-purpose agent (fallback).
    General,
}

impl AgentRole {
    /// Human-readable label.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Implementer => "Implementer",
            Self::Reviewer => "Reviewer",
            Self::Tester => "Tester",
            Self::Planner => "Planner",
            Self::Coordinator => "Coordinator",
            Self::General => "General",
        }
    }

    /// Whether this role is read-only (no mutations).
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Explorer | Self::Reviewer | Self::Planner)
    }

    /// Recommended model for this role.
    #[must_use]
    pub fn recommended_model(&self) -> &'static str {
        match self {
            Self::Explorer | Self::Planner => "deepseek-v4-flash",
            Self::Implementer | Self::Coordinator => "deepseek-v4-pro",
            Self::Reviewer | Self::Tester => "deepseek-v4-flash",
            Self::General => "deepseek-v4-pro",
        }
    }

    /// System prompt prefix for this role.
    #[must_use]
    pub fn system_prompt_prefix(&self) -> &'static str {
        match self {
            Self::Explorer => "You are an Explorer agent. Your job is to investigate and report. Read files, search code, and map structure. Do NOT modify anything.",
            Self::Implementer => "You are an Implementer agent. Your job is to write correct, well-tested code. Follow the plan exactly. Make atomic changes.",
            Self::Reviewer => "You are a Reviewer agent. Your job is to find bugs, security issues, and quality problems. Be critical and thorough.",
            Self::Tester => "You are a Tester agent. Your job is to verify behavior. Run tests, check edge cases, and validate acceptance criteria.",
            Self::Planner => "You are a Planner agent. Your job is to create plans, decompose tasks, and write checklists. Think before acting.",
            Self::Coordinator => "You are a Coordinator agent. Your job is to orchestrate other agents, merge results, and resolve conflicts. Keep the swarm aligned.",
            Self::General => "You are a General agent. Adapt to whatever the task requires.",
        }
    }
}

/// Specification for spawning a specialized agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Unique ID for this agent instance.
    pub id: String,
    /// The role this agent fills.
    pub role: AgentRole,
    /// Human-readable name.
    pub name: String,
    /// Model override (uses role default if None).
    pub model: Option<String>,
    /// Tools allowed for this agent.
    pub allowed_tools: Vec<String>,
    /// Whether this agent can access the hive mind.
    pub has_hive_access: bool,
}

impl AgentSpec {
    /// Create a spec for the given role with defaults.
    #[must_use]
    pub fn for_role(role: AgentRole, name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            name: name.to_string(),
            model: None,
            allowed_tools: role_default_tools(role),
            has_hive_access: true,
        }
    }
}

fn role_default_tools(role: AgentRole) -> Vec<String> {
    match role {
        AgentRole::Explorer => vec![
            "read_file".into(), "list_dir".into(), "grep_files".into(),
            "file_search".into(), "web_search".into(), "git_log".into(),
            "git_show".into(), "git_blame".into(),
        ],
        AgentRole::Implementer => vec![
            "read_file".into(), "write_file".into(), "edit_file".into(),
            "apply_patch".into(), "exec_shell".into(), "git_status".into(),
            "git_diff".into(),
        ],
        AgentRole::Reviewer => vec![
            "read_file".into(), "grep_files".into(), "git_diff".into(),
            "git_show".into(), "diagnostics".into(), "review".into(),
        ],
        AgentRole::Tester => vec![
            "run_tests".into(), "exec_shell".into(), "read_file".into(),
            "diagnostics".into(),
        ],
        AgentRole::Planner => vec![
            "read_file".into(), "list_dir".into(), "grep_files".into(),
            "update_plan".into(), "checklist_write".into(),
        ],
        AgentRole::Coordinator => vec![
            "agent_spawn".into(), "agent_result".into(), "agent_list".into(),
            "agent_assign".into(), "agent_wait".into(), "read_file".into(),
        ],
        AgentRole::General => vec![],
    }
}

// ============================================================================
// Swarm task graph
// ============================================================================

/// A node in the task dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// Unique task ID.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// The role best suited for this task.
    pub role: AgentRole,
    /// Task-specific prompt.
    pub prompt: String,
    /// Priority (lower = higher priority).
    pub priority: u32,
    /// Estimated complexity (1-10).
    pub complexity: u32,
    /// Status.
    pub status: TaskStatus,
    /// Result when complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Agent ID that executed this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,
    InProgress,
    Completed,
    Failed,
}

/// A directed acyclic graph of tasks with role assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraph {
    /// All task nodes.
    pub nodes: Vec<TaskNode>,
    /// Edges: node → [dependencies] (dependencies must complete first).
    pub edges: HashMap<String, Vec<String>>,
    /// Name of the overall objective.
    pub objective: String,
    /// When the graph was created.
    pub created_at: String,
}

impl TaskGraph {
    /// Create a new empty task graph.
    #[must_use]
    pub fn new(objective: impl Into<String>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: HashMap::new(),
            objective: objective.into(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Add a task node.
    pub fn add_node(&mut self, node: TaskNode) -> &mut Self {
        self.nodes.push(node);
        self
    }

    /// Add a dependency: `task_id` depends on `depends_on_id`.
    pub fn add_dependency(&mut self, task_id: &str, depends_on_id: &str) -> &mut Self {
        self.edges
            .entry(task_id.to_string())
            .or_default()
            .push(depends_on_id.to_string());
        self
    }

    /// Get tasks that are ready to execute (all dependencies met).
    #[must_use]
    pub fn ready_tasks(&self) -> Vec<&TaskNode> {
        self.nodes
            .iter()
            .filter(|n| n.status == TaskStatus::Pending || n.status == TaskStatus::Ready)
            .filter(|n| self.dependencies_satisfied(&n.id))
            .collect()
    }

    /// Check if all dependencies for a node are completed.
    fn dependencies_satisfied(&self, node_id: &str) -> bool {
        match self.edges.get(node_id) {
            Some(deps) => deps.iter().all(|dep_id| {
                self.nodes
                    .iter()
                    .any(|n| n.id == *dep_id && n.status == TaskStatus::Completed)
            }),
            None => true,
        }
    }

    /// Mark a task as in-progress.
    pub fn mark_in_progress(&mut self, task_id: &str, agent_id: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == task_id) {
            node.status = TaskStatus::InProgress;
            node.assigned_agent = Some(agent_id.to_string());
        }
    }

    /// Mark a task as completed with a result.
    pub fn mark_completed(&mut self, task_id: &str, result: String) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == task_id) {
            node.status = TaskStatus::Completed;
            node.result = Some(result);
        }
    }

    /// Mark a task as failed.
    pub fn mark_failed(&mut self, task_id: &str, error: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == task_id) {
            node.status = TaskStatus::Failed;
            node.result = Some(error.to_string());
        }
    }

    /// Total task count.
    #[must_use]
    pub fn total(&self) -> usize {
        self.nodes.len()
    }

    /// Completed task count.
    #[must_use]
    pub fn completed(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.status == TaskStatus::Completed)
            .count()
    }

    /// Whether all tasks are complete.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.nodes
            .iter()
            .all(|n| n.status == TaskStatus::Completed || n.status == TaskStatus::Failed)
    }

    /// Progress percentage.
    #[must_use]
    pub fn progress_pct(&self) -> f64 {
        if self.nodes.is_empty() {
            return 100.0;
        }
        self.completed() as f64 / self.nodes.len() as f64 * 100.0
    }

    /// Topological sort for execution order.
    #[must_use]
    pub fn topological_order(&self) -> Vec<&TaskNode> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(&node.id).or_insert(0);
        }

        for (task_id, deps) in &self.edges {
            for dep in deps {
                *in_degree.entry(task_id.as_str()).or_insert(0) += 1;
                adjacency
                    .entry(dep.as_str())
                    .or_default()
                    .push(task_id.as_str());
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();

        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.nodes.iter().find(|n| n.id == id) {
                order.push(node);
            }
            if let Some(neighbors) = adjacency.get(id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        order
    }
}

// ============================================================================
// Swarm orchestrator
// ============================================================================

/// Agent handle returned when spawning into the swarm.
#[derive(Debug)]
pub struct AgentHandle {
    pub id: String,
    pub role: AgentRole,
    pub name: String,
    /// Channel to send tasks to this agent.
    task_tx: mpsc::Sender<SwarmMessage>,
    /// Channel to receive results from this agent.
    result_rx: Arc<Mutex<mpsc::Receiver<AgentResult>>>,
}

impl Clone for AgentHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            role: self.role,
            name: self.name.clone(),
            task_tx: self.task_tx.clone(),
            result_rx: self.result_rx.clone(),
        }
    }
}

impl AgentHandle {
    /// Wait for the agent to complete and return its result.
    /// Returns `None` if the agent's channel is closed without a result.
    pub async fn await_completion(&self) -> Option<AgentResult> {
        let mut rx = self.result_rx.lock().await;
        rx.recv().await
    }
}

/// Message sent to an agent in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmMessage {
    /// Execute a task.
    ExecuteTask { task_id: String, prompt: String },
    /// Broadcast from coordinator to all agents.
    Broadcast { from: String, message: String },
    /// Inject hive context update.
    ContextUpdate { key: String, value: Value },
    /// Shutdown signal.
    Shutdown,
}

/// Result returned by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: String,
    pub task_id: Option<String>,
    pub success: bool,
    pub output: String,
    pub hive_updates: Vec<HiveInjection>,
}

/// An entry the agent wants to inject into the hive mind after completing work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveInjection {
    pub key: String,
    pub value: Value,
    pub tags: Vec<String>,
    pub confidence: f64,
}

/// Manages the swarm: spawning agents, routing tasks, aggregating results.
pub struct SwarmOrchestrator {
    /// Shared hive mind.
    pub hive: Arc<HiveMind>,
    /// Active agent handles.
    agents: RwLock<HashMap<String, AgentHandle>>,
    /// Active task graphs (reserved for future multi-graph orchestration).
    #[allow(dead_code)]
    task_graphs: RwLock<HashMap<String, TaskGraph>>,
    /// Results collected from agents.
    results: RwLock<Vec<AgentResult>>,
    /// Current swarm topology.
    topology: RwLock<SwarmTopology>,
    /// Registered background workers.
    workers: RwLock<Vec<BackgroundWorker>>,
}

impl SwarmOrchestrator {
    /// Create a new orchestrator with an empty hive mind.
    pub fn new() -> Self {
        Self {
            hive: Arc::new(HiveMind::new()),
            agents: RwLock::new(HashMap::new()),
            task_graphs: RwLock::new(HashMap::new()),
            results: RwLock::new(Vec::new()),
            topology: RwLock::new(SwarmTopology::Hierarchical),
            workers: RwLock::new(BackgroundWorker::predefined()),
        }
    }

    /// Create an orchestrator with a pre-existing hive mind (shared across
    /// multiple orchestrator instances or sessions).
    pub fn with_hive(hive: Arc<HiveMind>) -> Self {
        Self {
            hive,
            agents: RwLock::new(HashMap::new()),
            task_graphs: RwLock::new(HashMap::new()),
            results: RwLock::new(Vec::new()),
            topology: RwLock::new(SwarmTopology::Hierarchical),
            workers: RwLock::new(BackgroundWorker::predefined()),
        }
    }

    /// Spawn a specialized agent into the swarm.
    pub async fn spawn_agent(&self, spec: AgentSpec) -> AgentHandle {
        let id = spec.id.clone();
        let role = spec.role;
        let name = spec.name.clone();

        let (task_tx, mut task_rx) = mpsc::channel::<SwarmMessage>(32);
        let (result_tx, result_rx) = mpsc::channel::<AgentResult>(32);

        let handle = AgentHandle {
            id: id.clone(),
            role,
            name: name.clone(),
            task_tx: task_tx.clone(),
            result_rx: Arc::new(Mutex::new(result_rx)),
        };

        self.agents.write().await.insert(id.clone(), handle.clone());

        // Spawn the agent's event loop
        let hive = self.hive.clone();
        let agent_id = id.clone();
        tokio::spawn(async move {
            info!(%agent_id, role = %role.label(), "swarm agent started");
            while let Some(msg) = task_rx.recv().await {
                match msg {
                    SwarmMessage::ExecuteTask { task_id, prompt } => {
                        debug!(%agent_id, %task_id, "agent executing task");
                        // In a real implementation, this would call the LLM.
                        // Here we simulate with a placeholder result.
                        let output = format!(
                            "[{}] executed task '{}': processed prompt '{}'",
                            role.label(),
                            task_id,
                            truncate_str(&prompt, 80)
                        );
                        let hive_updates = vec![
                            HiveInjection {
                                key: format!("task.{}.result", task_id),
                                value: serde_json::json!({
                                    "status": "completed",
                                    "agent": agent_id,
                                    "role": role.label(),
                                    "summary": truncate_str(&prompt, 200)
                                }),
                                tags: vec!["task-result".into(), role.label().to_lowercase()],
                                confidence: 0.9,
                            },
                        ];
                        let _ = result_tx.send(AgentResult {
                            agent_id: agent_id.clone(),
                            task_id: Some(task_id),
                            success: true,
                            output,
                            hive_updates,
                        }).await;
                    }
                    SwarmMessage::Broadcast { from, message } => {
                        debug!(%agent_id, %from, "agent received broadcast: {}", message);
                    }
                    SwarmMessage::ContextUpdate { key, value } => {
                        debug!(%agent_id, %key, "agent received context update");
                        let _ = hive.inject(
                            &key,
                            value,
                            &format!("broadcast-via-{}", agent_id),
                            vec!["broadcast".into()],
                            0.7,
                        ).await;
                    }
                    SwarmMessage::Shutdown => {
                        info!(%agent_id, "swarm agent shutting down");
                        break;
                    }
                }
            }
        });

        info!(%id, role = %role.label(), name = %name, "agent spawned into swarm");
        handle
    }

    /// Send a task to a specific agent.
    pub async fn assign_task(&self, agent_id: &str, task_id: &str, prompt: &str) -> Result<()> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .with_context(|| format!("agent not found: {agent_id}"))?;
        agent
            .task_tx
            .send(SwarmMessage::ExecuteTask {
                task_id: task_id.to_string(),
                prompt: prompt.to_string(),
            })
            .await
            .context("failed to send task to agent")?;
        Ok(())
    }

    /// Broadcast a message to all agents.
    pub async fn broadcast(&self, from: &str, message: &str) -> Result<()> {
        let agents = self.agents.read().await;
        for agent in agents.values() {
            let _ = agent
                .task_tx
                .send(SwarmMessage::Broadcast {
                    from: from.to_string(),
                    message: message.to_string(),
                })
                .await;
        }
        Ok(())
    }

    /// Shutdown an agent by ID.
    pub async fn shutdown_agent(&self, agent_id: &str) -> Result<()> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(agent_id) {
            let _ = agent.task_tx.send(SwarmMessage::Shutdown).await;
        }
        self.agents.write().await.remove(agent_id);
        Ok(())
    }

    /// Shutdown all agents.
    pub async fn shutdown_all(&self) {
        let agents = self.agents.read().await;
        for agent in agents.values() {
            let _ = agent.task_tx.send(SwarmMessage::Shutdown).await;
        }
        self.agents.write().await.clear();
        info!("all swarm agents shut down");
    }

    /// Execute a task graph: spawn agents per role, respect dependencies,
    /// collect results, update hive mind.
    pub async fn execute_graph(&self, graph: &mut TaskGraph) -> Result<Vec<AgentResult>> {
        info!(
            objective = %graph.objective,
            nodes = graph.total(),
            "executing task graph"
        );

        // Inject the objective into the hive
        self.hive
            .inject(
                "objective.current",
                serde_json::json!({
                    "description": graph.objective,
                    "total_tasks": graph.total(),
                    "graph_id": Uuid::new_v4().to_string(),
                }),
                "orchestrator",
                vec!["objective".into()],
                1.0,
            )
            .await?;

        let mut all_results = Vec::new();

        while !graph.is_done() {
            let ready: Vec<_> = graph
                .ready_tasks()
                .into_iter()
                .map(|n| (n.id.clone(), n.role, n.prompt.clone()))
                .collect();

            if ready.is_empty() && !graph.is_done() {
                // Deadlock — some tasks can never complete
                for node in &graph.nodes {
                    if node.status == TaskStatus::Failed {
                        warn!(task_id = %node.id, "task failed, skipping dependent tasks");
                    }
                }
                // Mark remaining pending tasks as failed
                for node in graph.nodes.iter_mut() {
                    if node.status == TaskStatus::Pending {
                        node.status = TaskStatus::Failed;
                        node.result = Some("blocked by failed dependency".into());
                    }
                }
                break;
            }

            let mut handles = Vec::new();

            for (task_id, role, prompt) in &ready {
                let spec = AgentSpec::for_role(*role, &format!("{}-{}", role.label(), &task_id[..8]));
                let agent_id = spec.id.clone();
                let handle = self.spawn_agent(spec).await;

                graph.mark_in_progress(task_id, &agent_id);

                self.assign_task(&agent_id, task_id, prompt).await?;

                handles.push((task_id.clone(), agent_id, handle));
            }

            // Wait for all spawned agents to complete
            for (task_id, agent_id, handle) in handles {
                let mut rx = handle.result_rx.lock().await;
                if let Some(result) = rx.recv().await {
                    // Inject agent's hive updates
                    for update in &result.hive_updates {
                        let _ = self
                            .hive
                            .inject(
                                &update.key,
                                update.value.clone(),
                                &result.agent_id,
                                update.tags.clone(),
                                update.confidence,
                            )
                            .await;
                    }

                    if result.success {
                        graph.mark_completed(
                            &task_id,
                            result.output.clone(),
                        );
                    } else {
                        graph.mark_failed(&task_id, &result.output);
                    }

                    all_results.push(result);
                }

                self.shutdown_agent(&agent_id).await?;
            }
        }

        // Inject final summary into hive
        self.hive
            .inject(
                "objective.completed",
                serde_json::json!({
                    "objective": graph.objective,
                    "completed": graph.completed(),
                    "total": graph.total(),
                    "progress_pct": graph.progress_pct(),
                }),
                "orchestrator",
                vec!["objective".into(), "completed".into()],
                1.0,
            )
            .await?;

        info!(
            completed = graph.completed(),
            total = graph.total(),
            "task graph execution complete"
        );

        Ok(all_results)
    }

    /// Create a pre-configured task graph from a natural-language objective.
    ///
    /// The coordinator role analyzes the objective and produces a decomposition.
    #[must_use]
    pub fn decompose_objective(objective: &str) -> TaskGraph {
        // In a real implementation, this would call an LLM to decompose.
        // Here we provide a sensible default decomposition pattern.
        let mut graph = TaskGraph::new(objective);

        // Phase 1: Explore (read-only investigation)
        let explore = TaskNode {
            id: "explore".into(),
            description: "Explore the codebase".into(),
            role: AgentRole::Explorer,
            prompt: format!("Investigate the codebase to understand what needs to change for: {objective}"),
            priority: 1,
            complexity: 2,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };

        // Phase 2: Plan
        let plan = TaskNode {
            id: "plan".into(),
            description: "Create implementation plan".into(),
            role: AgentRole::Planner,
            prompt: format!("Create a detailed implementation plan for: {objective}"),
            priority: 1,
            complexity: 3,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };

        // Phase 3: Implement (multiple parallel tasks)
        let implement = TaskNode {
            id: "implement".into(),
            description: "Implement the changes".into(),
            role: AgentRole::Implementer,
            prompt: format!("Implement the changes described in the plan for: {objective}"),
            priority: 2,
            complexity: 6,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };

        // Phase 4: Review
        let review = TaskNode {
            id: "review".into(),
            description: "Review the implementation".into(),
            role: AgentRole::Reviewer,
            prompt: format!("Review the implementation for bugs, security, and quality issues for: {objective}"),
            priority: 3,
            complexity: 3,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };

        // Phase 5: Test
        let test = TaskNode {
            id: "test".into(),
            description: "Run tests and validate".into(),
            role: AgentRole::Tester,
            prompt: format!("Run all tests and validate the implementation for: {objective}"),
            priority: 3,
            complexity: 2,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };

        graph.add_node(explore);
        graph.add_node(plan);
        graph.add_node(implement);
        graph.add_node(review);
        graph.add_node(test);

        // Dependencies: explore → plan → implement → review → test
        graph.add_dependency("plan", "explore");
        graph.add_dependency("implement", "plan");
        graph.add_dependency("review", "implement");
        graph.add_dependency("test", "review");

        graph
    }

    /// List all active agents.
    pub async fn list_agents(&self) -> Vec<AgentSpec> {
        self.agents
            .read()
            .await
            .values()
            .map(|h| AgentSpec {
                id: h.id.clone(),
                role: h.role,
                name: h.name.clone(),
                model: None,
                allowed_tools: role_default_tools(h.role),
                has_hive_access: true,
            })
            .collect()
    }

    /// Get the hive mind summary for injection into prompts.
    pub async fn hive_summary(&self) -> String {
        self.hive.summary().await
    }

    /// Collect all results gathered so far.
    pub async fn collected_results(&self) -> Vec<AgentResult> {
        self.results.read().await.clone()
    }

    // ====================================================================
    // Topology management
    // ====================================================================

    /// Set the swarm communication topology.
    pub async fn set_topology(&self, topology: SwarmTopology) {
        *self.topology.write().await = topology;
        info!(topology = %topology.label(), "swarm topology changed");
    }

    /// Get the current topology.
    pub async fn current_topology(&self) -> SwarmTopology {
        *self.topology.read().await
    }

    // ====================================================================
    // Background worker management
    // ====================================================================

    /// Register a background worker.
    pub async fn register_worker(&self, worker: BackgroundWorker) {
        self.workers.write().await.push(worker);
    }

    /// List all registered workers.
    pub async fn list_workers(&self) -> Vec<BackgroundWorker> {
        self.workers.read().await.clone()
    }

    /// Enable a worker by name.
    pub async fn enable_worker(&self, name: &str) -> Result<()> {
        let mut workers = self.workers.write().await;
        let worker = workers
            .iter_mut()
            .find(|w| w.name == name)
            .with_context(|| format!("worker not found: {name}"))?;
        worker.enabled = true;
        Ok(())
    }

    /// Disable a worker by name.
    pub async fn disable_worker(&self, name: &str) -> Result<()> {
        let mut workers = self.workers.write().await;
        let worker = workers
            .iter_mut()
            .find(|w| w.name == name)
            .with_context(|| format!("worker not found: {name}"))?;
        worker.enabled = false;
        Ok(())
    }

    // ====================================================================
    // Persistence: back hive mind to context store for cross-restart survival
    // ====================================================================

    /// Persist the entire hive mind state to a HybridContextStore.
    ///
    /// Each hive entry is stored as a workspace state entry under "_hive".
    pub async fn persist_to_store(&self, store: &HybridContextStore) -> Result<()> {
        let snapshot = self.hive.snapshot().await;
        for entry in &snapshot {
            let key = format!("hive.{}", entry.key);
            let value = serde_json::to_string(entry)
                .with_context(|| format!("serialize hive entry: {}", entry.key))?;
            store.set_workspace_state("_hive", &key, &value)?;
        }
        let keys: Vec<String> = snapshot.iter().map(|e| e.key.clone()).collect();
        store.set_workspace_state("_hive", "hive._keys", &serde_json::to_string(&keys)?)?;
        info!(entries = snapshot.len(), "hive mind persisted to context store");
        Ok(())
    }

    /// Restore the hive mind state from a HybridContextStore.
    ///
    /// Existing hive entries are NOT cleared — this is an additive restore.
    pub async fn restore_from_store(&self, store: &HybridContextStore) -> Result<usize> {
        let keys_json = store.get_workspace_state("_hive", "hive._keys")?;
        let keys: Vec<String> = match keys_json {
            Some(s) => serde_json::from_str(&s).context("parse hive keys")?,
            None => return Ok(0),
        };

        let mut restored = 0usize;
        for key in &keys {
            let store_key = format!("hive.{key}");
            if let Some(raw) = store.get_workspace_state("_hive", &store_key)? {
                let entry: HiveEntry = serde_json::from_str(&raw)
                    .with_context(|| format!("deserialize hive entry: {key}"))?;
                let _ = self.hive.inject(&entry.key, entry.value, &entry.author, entry.tags, entry.confidence).await?;
                restored += 1;
            }
        }

        info!(restored, "hive mind restored from context store");
        Ok(restored)
    }
}

impl Default for SwarmOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Swarm topologies
// ============================================================================

/// Swarm communication topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmTopology {
    /// Single coordinator delegates to workers (default).
    Hierarchical,
    /// Agents communicate peer-to-peer.
    Mesh,
    /// Topology changes based on task complexity.
    Adaptive,
    /// Agents arranged in sequential pipeline stages.
    Pipeline,
}

impl SwarmTopology {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hierarchical => "hierarchical",
            Self::Mesh => "mesh",
            Self::Adaptive => "adaptive",
            Self::Pipeline => "pipeline",
        }
    }
}

// ============================================================================
// Background workers
// ============================================================================

/// Trigger condition for a background worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerTrigger {
    /// Run on a fixed interval.
    OnInterval { secs: u64 },
    /// Run when a phase completes.
    OnPhaseComplete,
    /// Run when a file changes.
    OnFileChange { path: String },
    /// Run once at startup.
    OnStartup,
}

/// A background worker that runs automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundWorker {
    pub name: String,
    pub trigger: WorkerTrigger,
    pub task: String,
    pub role: AgentRole,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_run: Option<String>,
}

fn default_true() -> bool { true }

/// Predefined workers matching GSD/Ruflo patterns.
impl BackgroundWorker {
    #[must_use]
    pub fn predefined() -> Vec<Self> {
        vec![
            Self {
                name: "audit".into(),
                trigger: WorkerTrigger::OnInterval { secs: 3600 },
                task: "Audit recent changes for code quality issues, security concerns, and architectural drift".into(),
                role: AgentRole::Reviewer,
                enabled: true,
                last_run: None,
            },
            Self {
                name: "optimize".into(),
                trigger: WorkerTrigger::OnPhaseComplete,
                task: "Find optimization opportunities in recently changed code".into(),
                role: AgentRole::Implementer,
                enabled: true,
                last_run: None,
            },
            Self {
                name: "testgaps".into(),
                trigger: WorkerTrigger::OnPhaseComplete,
                task: "Find untested code paths and suggest test coverage improvements".into(),
                role: AgentRole::Tester,
                enabled: true,
                last_run: None,
            },
            Self {
                name: "docs".into(),
                trigger: WorkerTrigger::OnPhaseComplete,
                task: "Update documentation for recent changes — README, API docs, inline comments".into(),
                role: AgentRole::Planner,
                enabled: false,
                last_run: None,
            },
            Self {
                name: "security-scan".into(),
                trigger: WorkerTrigger::OnInterval { secs: 86400 },
                task: "Scan codebase for security vulnerabilities, exposed secrets, and unsafe patterns".into(),
                role: AgentRole::Reviewer,
                enabled: true,
                last_run: None,
            },
        ]
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn normalize_key(key: &str) -> String {
    key.trim().to_lowercase().replace(' ', "_")
}

fn truncate_value(value: &Value, max_chars: usize) -> String {
    let s = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.len() <= max_chars {
        s
    } else {
        format!("{}…", &s[..max_chars])
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}…", &s[..max_chars])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    // -- HiveMind --------------------------------------------------------

    #[tokio::test]
    async fn hive_inject_and_query() {
        let hive = HiveMind::new();
        let v = hive
            .inject(
                "finding.architecture",
                serde_json::json!({"pattern": "layered", "layers": 3}),
                "explorer-1",
                vec!["architecture".into()],
                0.9,
            )
            .await
            .expect("inject");
        assert_eq!(v, 1);

        let entry = hive.query("finding.architecture").await.expect("exists");
        assert_eq!(entry.author, "explorer-1");
        assert_eq!(entry.tags, vec!["architecture"]);
        assert!((entry.confidence - 0.9).abs() < 0.001);
    }

    #[tokio::test]
    async fn hive_prefix_query() {
        let hive = HiveMind::new();
        hive.inject("finding.one", serde_json::json!(1), "a", vec![], 0.5).await.unwrap();
        hive.inject("finding.two", serde_json::json!(2), "a", vec![], 0.5).await.unwrap();
        hive.inject("decision.one", serde_json::json!(3), "b", vec![], 0.5).await.unwrap();

        let findings = hive.query_prefix("finding.").await;
        assert_eq!(findings.len(), 2);
    }

    #[tokio::test]
    async fn hive_subscriptions() {
        let hive = HiveMind::new();
        let mut rx = hive.subscribe();

        hive.inject("key1", serde_json::json!("v1"), "a", vec![], 0.5).await.unwrap();

        let notification = rx.try_recv().expect("notification");
        assert_eq!(notification.key, "key1");
        assert_eq!(notification.action, HiveAction::Created);
    }

    #[tokio::test]
    async fn hive_delete() {
        let hive = HiveMind::new();
        hive.inject("tmp", serde_json::json!("x"), "a", vec![], 0.5).await.unwrap();
        assert!(hive.query("tmp").await.is_some());

        hive.delete("tmp").await.unwrap();
        assert!(hive.query("tmp").await.is_none());
    }

    // -- TaskGraph -------------------------------------------------------

    #[test]
    fn task_graph_topological_order() {
        let mut graph = TaskGraph::new("test objective");
        let a = TaskNode { id: "a".into(), description: "A".into(), role: AgentRole::Explorer, prompt: "a".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };
        let b = TaskNode { id: "b".into(), description: "B".into(), role: AgentRole::Implementer, prompt: "b".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };
        let c = TaskNode { id: "c".into(), description: "C".into(), role: AgentRole::Reviewer, prompt: "c".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };

        graph.add_node(a);
        graph.add_node(b);
        graph.add_node(c);
        graph.add_dependency("b", "a");
        graph.add_dependency("c", "b");

        let order = graph.topological_order();
        let ids: Vec<&str> = order.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn task_graph_ready_tasks() {
        let mut graph = TaskGraph::new("test");
        let a = TaskNode { id: "a".into(), description: "A".into(), role: AgentRole::Explorer, prompt: "a".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };
        let b = TaskNode { id: "b".into(), description: "B".into(), role: AgentRole::Implementer, prompt: "b".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };

        graph.add_node(a);
        graph.add_node(b);
        graph.add_dependency("b", "a");

        let ready = graph.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "a");
    }

    #[test]
    fn task_graph_progress() {
        let mut graph = TaskGraph::new("test");
        let a = TaskNode { id: "a".into(), description: "A".into(), role: AgentRole::General, prompt: "".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };
        let b = TaskNode { id: "b".into(), description: "B".into(), role: AgentRole::General, prompt: "".into(), priority: 1, complexity: 1, status: TaskStatus::Pending, result: None, assigned_agent: None };
        graph.add_node(a);
        graph.add_node(b);

        assert_eq!(graph.progress_pct(), 0.0);
        graph.mark_completed("a", "done".into());
        assert_eq!(graph.progress_pct(), 50.0);
        graph.mark_completed("b", "done".into());
        assert_eq!(graph.progress_pct(), 100.0);
        assert!(graph.is_done());
    }

    // -- Agent roles -----------------------------------------------------

    #[test]
    fn agent_roles_have_unique_labels() {
        let roles = [
            AgentRole::Explorer,
            AgentRole::Implementer,
            AgentRole::Reviewer,
            AgentRole::Tester,
            AgentRole::Planner,
            AgentRole::Coordinator,
            AgentRole::General,
        ];
        let labels: HashSet<&str> = roles.iter().map(|r| r.label()).collect();
        assert_eq!(labels.len(), roles.len());
    }

    #[test]
    fn read_only_roles_correctly_identified() {
        assert!(AgentRole::Explorer.is_read_only());
        assert!(AgentRole::Reviewer.is_read_only());
        assert!(AgentRole::Planner.is_read_only());
        assert!(!AgentRole::Implementer.is_read_only());
        assert!(!AgentRole::Tester.is_read_only());
    }

    #[test]
    fn agent_spec_generates_unique_ids() {
        let a = AgentSpec::for_role(AgentRole::Explorer, "e1");
        let b = AgentSpec::for_role(AgentRole::Explorer, "e2");
        assert_ne!(a.id, b.id);
    }

    // -- Swarm orchestrator ----------------------------------------------

    #[tokio::test]
    async fn orchestrator_spawns_and_runs_agent() {
        let swarm = SwarmOrchestrator::new();
        let spec = AgentSpec::for_role(AgentRole::Explorer, "test-explorer");
        let agent_id = spec.id.clone();

        let handle = swarm.spawn_agent(spec).await;
        assert_eq!(handle.id, agent_id);

        swarm.assign_task(&agent_id, "task-1", "explore the code").await.unwrap();

        let mut rx = handle.result_rx.lock().await;
        let result = rx.recv().await.expect("result");
        assert!(result.success);
        assert_eq!(result.task_id, Some("task-1".into()));

        swarm.shutdown_agent(&agent_id).await.unwrap();
    }

    #[tokio::test]
    async fn orchestrator_executes_task_graph() {
        let swarm = SwarmOrchestrator::new();
        let mut graph = TaskGraph::new("simple test");

        let task = TaskNode {
            id: "t1".into(),
            description: "simple task".into(),
            role: AgentRole::General,
            prompt: "do something".into(),
            priority: 1,
            complexity: 1,
            status: TaskStatus::Pending,
            result: None,
            assigned_agent: None,
        };
        graph.add_node(task);

        let results = swarm.execute_graph(&mut graph).await.expect("execute");
        assert!(!results.is_empty());
        assert!(results[0].success);
        assert!(graph.is_done());

        // Hive should contain the objective and results
        let hive = swarm.hive.query("objective.current").await;
        assert!(hive.is_some());
    }

    #[tokio::test]
    async fn hive_mind_survives_between_orchestrators() {
        let hive = Arc::new(HiveMind::new());
        hive.inject("shared.key", serde_json::json!("persistent"), "test", vec![], 1.0)
            .await
            .unwrap();

        let swarm = SwarmOrchestrator::with_hive(hive.clone());
        let entry = swarm.hive.query("shared.key").await;
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, serde_json::json!("persistent"));
    }

    #[test]
    fn decompose_objective_creates_valid_graph() {
        let graph = SwarmOrchestrator::decompose_objective("add dark mode support");
        assert_eq!(graph.total(), 5);
        assert!(!graph.is_done());

        let ready = graph.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "explore");
    }
}