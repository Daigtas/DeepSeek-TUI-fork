use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

// ============================================================================
// Public types
// ============================================================================

/// Discriminates what kind of context data an entry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// A conversation turn (user prompt + assistant response).
    Turn,
    /// A code snippet pasted or referenced by the user.
    Snippet,
    /// A URL / link reference.
    Link,
    /// An architectural or design decision.
    Decision,
    /// Arbitrary workspace-level key-value state.
    WorkspaceState,
    /// A reference to a file in the workspace.
    FileRef,
    /// An image reference (path, metadata).
    Image,
}

impl EntryType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Turn => "turn",
            Self::Snippet => "snippet",
            Self::Link => "link",
            Self::Decision => "decision",
            Self::WorkspaceState => "state",
            Self::FileRef => "file_ref",
            Self::Image => "image",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "turn" => Some(Self::Turn),
            "snippet" => Some(Self::Snippet),
            "link" => Some(Self::Link),
            "decision" => Some(Self::Decision),
            "state" => Some(Self::WorkspaceState),
            "file_ref" => Some(Self::FileRef),
            "image" => Some(Self::Image),
            _ => None,
        }
    }
}

/// A single context entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Discrimination tag.
    pub entry_type: EntryType,
    /// Workspace path this entry belongs to.
    pub workspace: String,
    /// ISO-8601 creation timestamp.
    pub timestamp: String,
    /// Namespaced key for direct lookup (e.g. "turn.42", "decision.auth").
    pub key: String,
    /// JSON-encoded value payload.
    pub value: serde_json::Value,
    /// Tags for categorization and search.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Importance / relevance score (0.0 – 1.0).
    #[serde(default = "default_importance")]
    pub importance: f64,
    /// How many times this entry has been accessed.
    #[serde(default)]
    pub access_count: u64,
    /// ISO-8601 last-access timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<String>,
}

fn default_importance() -> f64 { 0.5 }

/// A single conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Unique identifier.
    pub id: String,
    /// Thread / session identifier.
    pub thread_id: String,
    /// Monotonic index within the thread.
    pub index: u64,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Role: "user", "assistant", or "system".
    pub role: String,
    /// Full text content.
    pub content: String,
    /// Reasoning / thinking content (if available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Model used for this turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Provider used for this turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Tool calls made during this turn (JSON array).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A workspace-scoped key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStateEntry {
    pub workspace: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

/// Result of building a hybrid context window for LLM injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridContext {
    /// Assembled context string ready for prompt injection.
    pub text: String,
    /// Number of turns included (hot + cold).
    pub turn_count: usize,
    /// Number of decisions included.
    pub decision_count: usize,
    /// Number of snippets/links/files included.
    pub reference_count: usize,
    /// Estimated token count of the assembled text.
    pub estimated_tokens: usize,
    /// Whether truncation occurred.
    pub truncated: bool,
}

/// Parameters for building a hybrid context.
#[derive(Debug, Clone)]
pub struct ContextParams {
    /// Thread / session ID to load turns for.
    pub thread_id: Option<String>,
    /// Workspace to load state and decisions for.
    pub workspace: Option<String>,
    /// Maximum tokens for the assembled context (approximate).
    pub max_tokens: usize,
    /// Maximum recent turns to include.
    pub max_recent_turns: usize,
    /// Include workspace state entries.
    pub include_workspace_state: bool,
    /// Include decisions.
    pub include_decisions: bool,
    /// Include snippets and file references.
    pub include_references: bool,
    /// Search query for relevance-ranked context retrieval.
    pub search_query: Option<String>,
}

impl Default for ContextParams {
    fn default() -> Self {
        Self {
            thread_id: None,
            workspace: None,
            max_tokens: 128_000,
            max_recent_turns: 40,
            include_workspace_state: true,
            include_decisions: true,
            include_references: true,
            search_query: None,
        }
    }
}

// ============================================================================
// Relationship graph
// ============================================================================

/// A directed relationship between two context entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLink {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
}

// ============================================================================
// Hybrid context store
// ============================================================================

/// SQLite-backed hybrid context store with in-memory hot cache.
///
/// # Architecture
///
/// - **Hot cache** (in-memory): most recent N turns per thread, frequently
///   accessed entries, kept in an LRU-ish map for zero-latency access.
/// - **Cold storage** (SQLite): full history, searchable with FTS5,
///   durable across restarts.
///
/// # Schema
///
/// ```sql
/// context_entries (id, entry_type, workspace, timestamp, key, value, tags, importance, access_count, last_accessed)
/// conversation_turns (id, thread_id, index, timestamp, role, content, reasoning, model, provider, tool_calls, metadata)
/// workspace_state (workspace, key, value, updated_at)
/// entity_links (source_id, target_id, relation_type)
/// ```
pub struct HybridContextStore {
    /// SQLite connection for durability.
    db: Arc<Mutex<Connection>>,
    /// Path to the database file.
    db_path: PathBuf,
    /// In-memory hot cache: thread_id → Vec of recent turns.
    hot_turns: RwLock<HashMap<String, Vec<ConversationTurn>>>,
    /// In-memory hot cache: key → ContextEntry for frequently accessed entries.
    hot_entries: RwLock<HashMap<String, ContextEntry>>,
    /// Maximum hot turns per thread.
    hot_turn_limit: usize,
    /// Maximum hot entries overall.
    hot_entry_limit: usize,
}

impl HybridContextStore {
    /// Open or create the context database at `db_path`.
    pub fn open(db_path: impl Into<PathBuf>) -> Result<Self> {
        let db_path = db_path.into();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create context dir: {}", parent.display()))?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("open context db: {}", db_path.display()))?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let store = Self {
            db: Arc::new(Mutex::new(conn)),
            db_path: db_path.clone(),
            hot_turns: RwLock::new(HashMap::new()),
            hot_entries: RwLock::new(HashMap::new()),
            hot_turn_limit: 200,
            hot_entry_limit: 1000,
        };

        store.migrate()?;
        info!(path = %db_path.display(), "context store opened");

        Ok(store)
    }

    /// Open an in-memory store (for testing).
    #[doc(hidden)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            db: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
            hot_turns: RwLock::new(HashMap::new()),
            hot_entries: RwLock::new(HashMap::new()),
            hot_turn_limit: 200,
            hot_entry_limit: 1000,
        };
        store.migrate()?;
        Ok(store)
    }

    /// Return the database path.
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // -- Schema migration -----------------------------------------------

    fn migrate(&self) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS context_entries (
                id          TEXT PRIMARY KEY,
                entry_type  TEXT NOT NULL,
                workspace   TEXT NOT NULL DEFAULT '',
                timestamp   TEXT NOT NULL,
                key         TEXT NOT NULL,
                value       TEXT NOT NULL,
                tags        TEXT NOT NULL DEFAULT '[]',
                importance  REAL NOT NULL DEFAULT 0.5,
                access_count INTEGER NOT NULL DEFAULT 0,
                last_accessed TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_entries_type ON context_entries(entry_type);
            CREATE INDEX IF NOT EXISTS idx_entries_workspace ON context_entries(workspace);
            CREATE INDEX IF NOT EXISTS idx_entries_key ON context_entries(key);
            CREATE INDEX IF NOT EXISTS idx_entries_timestamp ON context_entries(timestamp);

            CREATE TABLE IF NOT EXISTS conversation_turns (
                id          TEXT PRIMARY KEY,
                thread_id   TEXT NOT NULL,
                idx         INTEGER NOT NULL,
                timestamp   TEXT NOT NULL,
                role        TEXT NOT NULL,
                content     TEXT NOT NULL,
                reasoning   TEXT,
                model       TEXT,
                provider    TEXT,
                tool_calls  TEXT,
                metadata    TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_turns_thread ON conversation_turns(thread_id, idx);
            CREATE INDEX IF NOT EXISTS idx_turns_timestamp ON conversation_turns(timestamp);

            CREATE TABLE IF NOT EXISTS workspace_state (
                workspace   TEXT NOT NULL,
                key         TEXT NOT NULL,
                value       TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                PRIMARY KEY (workspace, key)
            );

            CREATE TABLE IF NOT EXISTS entity_links (
                source_id     TEXT NOT NULL,
                target_id     TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id, relation_type)
            );

            CREATE INDEX IF NOT EXISTS idx_links_target ON entity_links(target_id);

            -- FTS5 virtual table for full-text search on turns
            CREATE VIRTUAL TABLE IF NOT EXISTS turns_fts USING fts5(
                content, reasoning, role,
                content='conversation_turns',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS turns_fts_insert AFTER INSERT ON conversation_turns BEGIN
                INSERT INTO turns_fts(rowid, content, reasoning, role)
                VALUES (new.rowid, new.content, new.reasoning, new.role);
            END;

            CREATE TRIGGER IF NOT EXISTS turns_fts_delete AFTER DELETE ON conversation_turns BEGIN
                INSERT INTO turns_fts(turns_fts, rowid, content, reasoning, role)
                VALUES ('delete', old.rowid, old.content, old.reasoning, old.role);
            END;

            CREATE TRIGGER IF NOT EXISTS turns_fts_update AFTER UPDATE ON conversation_turns BEGIN
                INSERT INTO turns_fts(turns_fts, rowid, content, reasoning, role)
                VALUES ('delete', old.rowid, old.content, old.reasoning, old.role);
                INSERT INTO turns_fts(rowid, content, reasoning, role)
                VALUES (new.rowid, new.content, new.reasoning, new.role);
            END;
            ",
        )?;
        Ok(())
    }

    // ====================================================================
    // Conversation turns
    // ====================================================================

    /// Insert a conversation turn. Updates the hot cache.
    pub fn insert_turn(&self, turn: ConversationTurn) -> Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO conversation_turns (id, thread_id, idx, timestamp, role, content, reasoning, model, provider, tool_calls, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                turn.id,
                turn.thread_id,
                turn.index,
                turn.timestamp,
                turn.role,
                turn.content,
                turn.reasoning,
                turn.model,
                turn.provider,
                turn.tool_calls.as_ref().map(|v| v.to_string()),
                turn.metadata.as_ref().map(|v| v.to_string()),
            ],
        )?;

        // Update hot cache (non-async, using block_in_place or spawn_blocking)
        self.cache_turn_hot(turn);

        Ok(())
    }

    /// Batch-insert conversation turns in a single transaction.
    pub fn insert_turns_batch(&self, turns: &[ConversationTurn]) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        for turn in turns {
            tx.execute(
                "INSERT OR REPLACE INTO conversation_turns (id, thread_id, idx, timestamp, role, content, reasoning, model, provider, tool_calls, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    turn.id,
                    turn.thread_id,
                    turn.index,
                    turn.timestamp,
                    turn.role,
                    turn.content,
                    turn.reasoning,
                    turn.model,
                    turn.provider,
                    turn.tool_calls.as_ref().map(|v| v.to_string()),
                    turn.metadata.as_ref().map(|v| v.to_string()),
                ],
            )?;
            self.cache_turn_hot(turn.clone());
        }

        tx.commit()?;
        Ok(())
    }

    /// Get recent turns for a thread (most recent first).
    pub fn get_recent_turns(&self, thread_id: &str, limit: usize) -> Result<Vec<ConversationTurn>> {
        // Try hot cache first
        {
            let hot = self.hot_turns.blocking_read();
            if let Some(cached) = hot.get(thread_id) {
                let turns: Vec<_> = cached.iter().rev().take(limit).cloned().collect();
                if turns.len() >= limit || cached.len() >= self.hot_turn_limit {
                    return Ok(turns);
                }
            }
        }

        // Fall back to DB
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, thread_id, idx, timestamp, role, content, reasoning, model, provider, tool_calls, metadata
             FROM conversation_turns
             WHERE thread_id = ?1
             ORDER BY idx DESC
             LIMIT ?2",
        )?;

        let turns: Vec<ConversationTurn> = stmt
            .query_map(params![thread_id, limit as i64], |row| {
                Ok(ConversationTurn {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    index: row.get(2)?,
                    timestamp: row.get(3)?,
                    role: row.get(4)?,
                    content: row.get(5)?,
                    reasoning: row.get(6)?,
                    model: row.get(7)?,
                    provider: row.get(8)?,
                    tool_calls: row
                        .get::<_, Option<String>>(9)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    metadata: row
                        .get::<_, Option<String>>(10)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(turns)
    }

    /// Get the total turn count for a thread.
    pub fn turn_count(&self, thread_id: &str) -> Result<u64> {
        let conn = self.db.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM conversation_turns WHERE thread_id = ?1",
            params![thread_id],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// Full-text search across turns.
    pub fn search_turns(&self, query: &str, limit: usize) -> Result<Vec<ConversationTurn>> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ct.id, ct.thread_id, ct.idx, ct.timestamp, ct.role, ct.content, ct.reasoning, ct.model, ct.provider, ct.tool_calls, ct.metadata
             FROM turns_fts fts
             JOIN conversation_turns ct ON ct.rowid = fts.rowid
             WHERE turns_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let turns: Vec<ConversationTurn> = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(ConversationTurn {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    index: row.get(2)?,
                    timestamp: row.get(3)?,
                    role: row.get(4)?,
                    content: row.get(5)?,
                    reasoning: row.get(6)?,
                    model: row.get(7)?,
                    provider: row.get(8)?,
                    tool_calls: row
                        .get::<_, Option<String>>(9)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    metadata: row
                        .get::<_, Option<String>>(10)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(turns)
    }

    /// Delete all turns for a thread.
    pub fn delete_thread_turns(&self, thread_id: &str) -> Result<usize> {
        let conn = self.db.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM conversation_turns WHERE thread_id = ?1",
            params![thread_id],
        )?;

        // Clear hot cache
        let mut hot = self.hot_turns.blocking_write();
        hot.remove(thread_id);

        Ok(count)
    }

    // ====================================================================
    // Context entries (generic)
    // ====================================================================

    /// Store a generic context entry.
    pub fn store_entry(&self, entry: ContextEntry) -> Result<()> {
        let value_json = serde_json::to_string(&entry.value)?;
        let tags_json = serde_json::to_string(&entry.tags)?;

        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO context_entries (id, entry_type, workspace, timestamp, key, value, tags, importance, access_count, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.id,
                entry.entry_type.as_str(),
                entry.workspace,
                entry.timestamp,
                entry.key,
                value_json,
                tags_json,
                entry.importance,
                entry.access_count,
                entry.last_accessed,
            ],
        )?;

        // Update hot cache
        self.cache_entry_hot(entry);

        Ok(())
    }

    /// Retrieve a context entry by key.
    pub fn get_entry(&self, key: &str) -> Result<Option<ContextEntry>> {
        // Try hot cache
        {
            let hot = self.hot_entries.blocking_read();
            if let Some(entry) = hot.get(key) {
                return Ok(Some(entry.clone()));
            }
        }

        // Fall back to DB
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, workspace, timestamp, key, value, tags, importance, access_count, last_accessed
             FROM context_entries WHERE key = ?1",
        )?;

        let entry = stmt
            .query_row(params![key], |row| {
                let entry_type_str: String = row.get(1)?;
                let value_str: String = row.get(5)?;
                let tags_str: String = row.get(6)?;
                Ok(ContextEntry {
                    id: row.get(0)?,
                    entry_type: EntryType::from_str(&entry_type_str).unwrap_or(EntryType::Turn),
                    workspace: row.get(2)?,
                    timestamp: row.get(3)?,
                    key: row.get(4)?,
                    value: serde_json::from_str(&value_str).unwrap_or_default(),
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    importance: row.get(7)?,
                    access_count: row.get(8)?,
                    last_accessed: row.get(9)?,
                })
            })
            .optional()?;

        // Update access count in background
        if let Some(ref e) = entry {
            let _ = self.touch_entry(&e.id);
        }

        Ok(entry)
    }

    /// Query entries by type and optional workspace filter.
    pub fn query_entries(
        &self,
        entry_type: Option<EntryType>,
        workspace: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextEntry>> {
        let conn = self.db.lock().unwrap();

        let type_filter = entry_type.map(|t| t.as_str().to_string());
        let mut sql = String::from(
            "SELECT id, entry_type, workspace, timestamp, key, value, tags, importance, access_count, last_accessed
             FROM context_entries WHERE 1=1",
        );

        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref t) = type_filter {
            sql.push_str(" AND entry_type = ?");
            param_values.push(Box::new(t.clone()));
        }
        if let Some(w) = workspace {
            sql.push_str(" AND workspace = ?");
            param_values.push(Box::new(w.to_string()));
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");
        param_values.push(Box::new(limit as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let entries: Vec<ContextEntry> = stmt
            .query_map(params_ref.as_slice(), |row| {
                let entry_type_str: String = row.get(1)?;
                let value_str: String = row.get(5)?;
                let tags_str: String = row.get(6)?;
                Ok(ContextEntry {
                    id: row.get(0)?,
                    entry_type: EntryType::from_str(&entry_type_str).unwrap_or(EntryType::Turn),
                    workspace: row.get(2)?,
                    timestamp: row.get(3)?,
                    key: row.get(4)?,
                    value: serde_json::from_str(&value_str).unwrap_or_default(),
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    importance: row.get(7)?,
                    access_count: row.get(8)?,
                    last_accessed: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Search entries by tags.
    pub fn search_by_tags(&self, tags: &[String], limit: usize) -> Result<Vec<ContextEntry>> {
        let conn = self.db.lock().unwrap();
        let mut entries = Vec::new();

        for tag in tags {
            let pattern = format!("%\"{}\"%", tag.replace('"', "\\\""));
            let mut stmt = conn.prepare(
                "SELECT id, entry_type, workspace, timestamp, key, value, tags, importance, access_count, last_accessed
                 FROM context_entries WHERE tags LIKE ?1
                 ORDER BY importance DESC, timestamp DESC
                 LIMIT ?2",
            )?;

            let partial: Vec<ContextEntry> = stmt
                .query_map(params![pattern, limit as i64], |row| {
                    let entry_type_str: String = row.get(1)?;
                    let value_str: String = row.get(5)?;
                    let tags_str: String = row.get(6)?;
                    Ok(ContextEntry {
                        id: row.get(0)?,
                        entry_type: EntryType::from_str(&entry_type_str).unwrap_or(EntryType::Turn),
                        workspace: row.get(2)?,
                        timestamp: row.get(3)?,
                        key: row.get(4)?,
                        value: serde_json::from_str(&value_str).unwrap_or_default(),
                        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                        importance: row.get(7)?,
                        access_count: row.get(8)?,
                        last_accessed: row.get(9)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            for e in partial {
                if !entries.iter().any(|existing: &ContextEntry| existing.id == e.id) {
                    entries.push(e);
                }
                if entries.len() >= limit {
                    return Ok(entries);
                }
            }
        }

        Ok(entries)
    }

    /// Delete an entry by key.
    pub fn delete_entry(&self, key: &str) -> Result<bool> {
        let conn = self.db.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM context_entries WHERE key = ?1",
            params![key],
        )?;

        let mut hot = self.hot_entries.blocking_write();
        hot.remove(key);

        Ok(count > 0)
    }

    // ====================================================================
    // Workspace state
    // ====================================================================

    /// Get workspace-scoped state.
    pub fn get_workspace_state(&self, workspace: &str, key: &str) -> Result<Option<String>> {
        let conn = self.db.lock().unwrap();
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM workspace_state WHERE workspace = ?1 AND key = ?2",
                params![workspace, key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }

    /// Set workspace-scoped state.
    pub fn set_workspace_state(&self, workspace: &str, key: &str, value: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO workspace_state (workspace, key, value, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![workspace, key, value, now],
        )?;
        Ok(())
    }

    /// List all state keys for a workspace.
    pub fn list_workspace_state(&self, workspace: &str) -> Result<Vec<WorkspaceStateEntry>> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT workspace, key, value, updated_at FROM workspace_state WHERE workspace = ?1 ORDER BY key",
        )?;

        let entries = stmt
            .query_map(params![workspace], |row| {
                Ok(WorkspaceStateEntry {
                    workspace: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Delete workspace state key.
    pub fn delete_workspace_state(&self, workspace: &str, key: &str) -> Result<bool> {
        let conn = self.db.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM workspace_state WHERE workspace = ?1 AND key = ?2",
            params![workspace, key],
        )?;
        Ok(count > 0)
    }

    // ====================================================================
    // Entity links (relationship graph)
    // ====================================================================

    /// Link two context entries.
    pub fn link_entities(&self, source_id: &str, target_id: &str, relation_type: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO entity_links (source_id, target_id, relation_type)
             VALUES (?1, ?2, ?3)",
            params![source_id, target_id, relation_type],
        )?;
        Ok(())
    }

    /// Get all entries linked to a target.
    pub fn get_linked(&self, target_id: &str, relation_type: Option<&str>) -> Result<Vec<EntityLink>> {
        let conn = self.db.lock().unwrap();
        let mut sql = String::from(
            "SELECT source_id, target_id, relation_type FROM entity_links WHERE target_id = ?1",
        );
        if relation_type.is_some() {
            sql.push_str(" AND relation_type = ?2");
        }

        let mut stmt = conn.prepare(&sql)?;
        let links: Vec<EntityLink> = if let Some(rt) = relation_type {
            stmt.query_map(params![target_id, rt], |row| {
                Ok(EntityLink {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relation_type: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map(params![target_id], |row| {
                Ok(EntityLink {
                    source_id: row.get(0)?,
                    target_id: row.get(1)?,
                    relation_type: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect()
        };

        Ok(links)
    }

    // ====================================================================
    // Hybrid context assembly (the key feature)
    // ====================================================================

    /// Build a hybrid context string for LLM prompt injection.
    ///
    /// Combines:
    /// 1. Recent turns (hot cache + cold DB) — most recent first
    /// 2. Relevant decisions for the workspace
    /// 3. Workspace state (key-value)
    /// 4. Referenced snippets, links, files
    /// 5. Full-text search results (if query provided)
    ///
    /// Truncates to `max_tokens` (approximate, using char count / 4).
    pub fn build_hybrid_context(&self, params: &ContextParams) -> Result<HybridContext> {
        let mut sections: Vec<String> = Vec::new();
        let mut turn_count = 0usize;
        let mut decision_count = 0usize;
        let mut reference_count = 0usize;
        let mut token_budget = params.max_tokens;
        let mut truncated = false;

        // Helper: roughly estimate tokens
        let estimate_tokens = |s: &str| s.len() / 4;

        // 1. Recent conversation turns
        if let Some(ref thread_id) = params.thread_id {
            let turns = self.get_recent_turns(thread_id, params.max_recent_turns)?;
            if !turns.is_empty() {
                let mut turn_text = String::from("## Recent Conversation\n\n");
                for turn in &turns {
                    let role_label = match turn.role.as_str() {
                        "user" => "User",
                        "assistant" => "Assistant",
                        "system" => "System",
                        _ => &turn.role,
                    };

                    let mut block = String::new();
                    block.push_str(&format!("### {role_label} (turn {})\n", turn.index));

                    // Truncate very long turns
                    let content = if turn.content.len() > 4000 {
                        format!("{}…[truncated]", &turn.content[..4000])
                    } else {
                        turn.content.clone()
                    };
                    block.push_str(&content);
                    block.push('\n');

                    if let Some(ref reasoning) = turn.reasoning {
                        if reasoning.len() > 2000 {
                            block.push_str(&format!("\n<thinking>{}</thinking>\n", &reasoning[..2000]));
                        } else {
                            block.push_str(&format!("\n<thinking>{reasoning}</thinking>\n"));
                        }
                    }

                    if let Some(ref tool_calls) = turn.tool_calls {
                        block.push_str(&format!("\n<tool_calls>{tool_calls}</tool_calls>\n"));
                    }

                    let block_tokens = estimate_tokens(&block);
                    if token_budget < block_tokens {
                        truncated = true;
                        break;
                    }
                    token_budget = token_budget.saturating_sub(block_tokens);
                    turn_text.push_str(&block);
                    turn_text.push_str("\n---\n");
                    turn_count += 1;
                }
                sections.push(turn_text);
            }
        }

        // 2. Decisions
        if params.include_decisions {
            let workspace = params.workspace.as_deref().unwrap_or("");
            let decisions = self.query_entries(Some(EntryType::Decision), Some(workspace), 20)?;
            if !decisions.is_empty() {
                let mut dec_text = String::from("## Key Decisions\n\n");
                for d in &decisions {
                    let line = format!(
                        "- **{}**: {} (importance: {:.0}%)\n",
                        d.key,
                        truncate_value(&d.value, 300),
                        d.importance * 100.0
                    );
                    let line_tokens = estimate_tokens(&line);
                    if token_budget < line_tokens {
                        truncated = true;
                        break;
                    }
                    token_budget = token_budget.saturating_sub(line_tokens);
                    dec_text.push_str(&line);
                    decision_count += 1;
                }
                sections.push(dec_text);
            }
        }

        // 3. Workspace state
        if params.include_workspace_state {
            let workspace = params.workspace.as_deref().unwrap_or("");
            let state = self.list_workspace_state(workspace)?;
            if !state.is_empty() {
                let mut state_text = String::from("## Workspace State\n\n");
                for s in &state {
                    let line = format!("- **{}**: {}\n", s.key, s.value);
                    let line_tokens = estimate_tokens(&line);
                    if token_budget < line_tokens {
                        truncated = true;
                        break;
                    }
                    token_budget = token_budget.saturating_sub(line_tokens);
                    state_text.push_str(&line);
                }
                sections.push(state_text);
            }
        }

        // 4. References (snippets, links, files)
        if params.include_references {
            let workspace = params.workspace.as_deref().unwrap_or("");
            for etype in &[EntryType::Snippet, EntryType::Link, EntryType::FileRef, EntryType::Image] {
                let refs = self.query_entries(Some(*etype), Some(workspace), 10)?;
                if !refs.is_empty() {
                    let label = match etype {
                        EntryType::Snippet => "Code Snippets",
                        EntryType::Link => "Links",
                        EntryType::FileRef => "File References",
                        EntryType::Image => "Images",
                        _ => "References",
                    };
                    let mut ref_text = format!("## {label}\n\n");
                    for r in &refs {
                        let line = format!("- **{}**: {}\n", r.key, truncate_value(&r.value, 200));
                        let line_tokens = estimate_tokens(&line);
                        if token_budget < line_tokens {
                            truncated = true;
                            break;
                        }
                        token_budget = token_budget.saturating_sub(line_tokens);
                        ref_text.push_str(&line);
                        reference_count += 1;
                    }
                    if !ref_text.ends_with("## References\n\n") {
                        sections.push(ref_text);
                    }
                }
            }
        }

        // 5. Search results (if query provided)
        if let Some(ref query) = params.search_query {
            if !query.trim().is_empty() {
                let results = self.search_turns(query, 10)?;
                if !results.is_empty() {
                    let mut search_text = format!("## Relevant History (query: \"{query}\")\n\n");
                    for turn in &results {
                        let role = &turn.role;
                        let preview = if turn.content.len() > 300 {
                            format!("{}…", &turn.content[..300])
                        } else {
                            turn.content.clone()
                        };
                        let line = format!("- [{role}] turn {}: {preview}\n", turn.index);
                        let line_tokens = estimate_tokens(&line);
                        if token_budget < line_tokens {
                            truncated = true;
                            break;
                        }
                        token_budget = token_budget.saturating_sub(line_tokens);
                        search_text.push_str(&line);
                    }
                    sections.push(search_text);
                }
            }
        }

        let text = sections.join("\n");
        let estimated_tokens = params.max_tokens - token_budget;

        Ok(HybridContext {
            text,
            turn_count,
            decision_count,
            reference_count,
            estimated_tokens,
            truncated,
        })
    }

    /// Quickly store a decision (convenience method).
    pub fn record_decision(&self, workspace: &str, key: &str, decision: &str, importance: f64) -> Result<()> {
        let entry = ContextEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Decision,
            workspace: workspace.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            key: format!("decision.{key}"),
            value: serde_json::json!({"decision": decision}),
            tags: vec!["decision".into(), key.to_string()],
            importance,
            access_count: 0,
            last_accessed: None,
        };
        self.store_entry(entry)
    }

    /// Store a pasted snippet (code, link, file reference).
    pub fn store_pasted_content(
        &self,
        workspace: &str,
        content_type: EntryType,
        key: &str,
        value: serde_json::Value,
        tags: Vec<String>,
    ) -> Result<()> {
        let entry = ContextEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: content_type,
            workspace: workspace.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            key: key.to_string(),
            value,
            tags,
            importance: 0.7,
            access_count: 0,
            last_accessed: None,
        };
        self.store_entry(entry)
    }

    // ====================================================================
    // Maintenance
    // ====================================================================

    /// Compact the database (VACUUM, optimize FTS).
    pub fn compact(&self) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO turns_fts(turns_fts) VALUES ('optimize');
             PRAGMA optimize;
             VACUUM;",
        )?;
        info!("context store compacted");
        Ok(())
    }

    /// Get database statistics.
    pub fn stats(&self) -> Result<ContextStats> {
        let conn = self.db.lock().unwrap();
        let turn_count: i64 = conn.query_row("SELECT COUNT(*) FROM conversation_turns", [], |r| r.get(0))?;
        let entry_count: i64 = conn.query_row("SELECT COUNT(*) FROM context_entries", [], |r| r.get(0))?;
        let state_count: i64 = conn.query_row("SELECT COUNT(*) FROM workspace_state", [], |r| r.get(0))?;
        let link_count: i64 = conn.query_row("SELECT COUNT(*) FROM entity_links", [], |r| r.get(0))?;

        // DB file size
        let db_size = if self.db_path.as_os_str() != ":memory:" {
            std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        Ok(ContextStats {
            turn_count: turn_count as u64,
            entry_count: entry_count as u64,
            state_count: state_count as u64,
            link_count: link_count as u64,
            db_size_bytes: db_size,
        })
    }

    /// Close the store (flush WAL, clean up).
    pub fn close(&self) -> Result<()> {
        let conn = self.db.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        info!("context store closed");
        Ok(())
    }

    // ====================================================================
    // Hot cache internals
    // ====================================================================

    fn cache_turn_hot(&self, turn: ConversationTurn) {
        let mut hot = match self.hot_turns.try_write() {
            Ok(h) => h,
            Err(_) => return,
        };
        let entry = hot.entry(turn.thread_id.clone()).or_default();
        // Insert sorted by index
        if let Some(pos) = entry.iter().position(|t| t.index >= turn.index) {
            if entry[pos].index == turn.index {
                entry[pos] = turn;
            } else {
                entry.insert(pos, turn);
            }
        } else {
            entry.push(turn);
        }
        // Trim if over limit
        if entry.len() > self.hot_turn_limit {
            entry.drain(0..(entry.len() - self.hot_turn_limit));
        }
    }

    fn cache_entry_hot(&self, entry: ContextEntry) {
        let mut hot = match self.hot_entries.try_write() {
            Ok(h) => h,
            Err(_) => return,
        };
        hot.insert(entry.key.clone(), entry);
        if hot.len() > self.hot_entry_limit {
            // Simple eviction: remove oldest entries
            let excess = hot.len() - self.hot_entry_limit;
            let keys: Vec<String> = hot.keys().take(excess).cloned().collect();
            for k in keys {
                hot.remove(&k);
            }
        }
    }

    fn touch_entry(&self, _id: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE context_entries SET access_count = access_count + 1, last_accessed = ?1 WHERE id = ?2",
            params![now, _id],
        )?;
        Ok(())
    }
}

// ============================================================================
// Stats
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub turn_count: u64,
    pub entry_count: u64,
    pub state_count: u64,
    pub link_count: u64,
    pub db_size_bytes: u64,
}

// ============================================================================
// Helpers
// ============================================================================

fn truncate_value(value: &serde_json::Value, max_chars: usize) -> String {
    let s = match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.len() <= max_chars {
        s
    } else {
        format!("{}…", &s[..max_chars])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_turn(thread_id: &str, index: u64, role: &str, content: &str) -> ConversationTurn {
        ConversationTurn {
            id: Uuid::new_v4().to_string(),
            thread_id: thread_id.to_string(),
            index,
            timestamp: Utc::now().to_rfc3339(),
            role: role.to_string(),
            content: content.to_string(),
            reasoning: None,
            model: Some("deepseek-v4-flash".into()),
            provider: Some("deepseek".into()),
            tool_calls: None,
            metadata: None,
        }
    }

    #[test]
    fn open_and_migrate_in_memory() {
        let store = HybridContextStore::open_in_memory().expect("open");
        let stats = store.stats().expect("stats");
        assert_eq!(stats.turn_count, 0);
    }

    #[test]
    fn insert_and_retrieve_turns() {
        let store = HybridContextStore::open_in_memory().expect("open");

        store.insert_turn(make_turn("thread-1", 0, "user", "Hello")).expect("insert");
        store.insert_turn(make_turn("thread-1", 1, "assistant", "Hi there!")).expect("insert");
        store.insert_turn(make_turn("thread-1", 2, "user", "How are you?")).expect("insert");

        let turns = store.get_recent_turns("thread-1", 10).expect("get");
        assert_eq!(turns.len(), 3);
        // Most recent first
        assert_eq!(turns[0].index, 2);
        assert_eq!(turns[2].index, 0);
        assert_eq!(turns[2].content, "Hello");
    }

    #[test]
    fn turn_count_is_accurate() {
        let store = HybridContextStore::open_in_memory().expect("open");
        store.insert_turn(make_turn("t1", 0, "user", "a")).unwrap();
        store.insert_turn(make_turn("t1", 1, "user", "b")).unwrap();
        store.insert_turn(make_turn("t2", 0, "user", "c")).unwrap();

        assert_eq!(store.turn_count("t1").unwrap(), 2);
        assert_eq!(store.turn_count("t2").unwrap(), 1);
        assert_eq!(store.turn_count("t3").unwrap(), 0);
    }

    #[test]
    fn store_and_query_context_entries() {
        let store = HybridContextStore::open_in_memory().expect("open");

        let entry = ContextEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Decision,
            workspace: "/home/user/project".into(),
            timestamp: Utc::now().to_rfc3339(),
            key: "decision.use-sqlite".into(),
            value: serde_json::json!({"decision": "Use SQLite for persistence", "rationale": "embedded, fast, no server needed"}),
            tags: vec!["architecture".into(), "persistence".into()],
            importance: 0.9,
            access_count: 0,
            last_accessed: None,
        };

        store.store_entry(entry).expect("store");

        let retrieved = store.get_entry("decision.use-sqlite").expect("get");
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.entry_type, EntryType::Decision);
        assert_eq!(r.importance, 0.9);
    }

    #[test]
    fn workspace_state_persistence() {
        let store = HybridContextStore::open_in_memory().expect("open");

        store.set_workspace_state("/ws", "last_branch", "feature/hybrid-context").unwrap();
        store.set_workspace_state("/ws", "preferred_model", "deepseek-v4-pro").unwrap();

        let branch = store.get_workspace_state("/ws", "last_branch").unwrap();
        assert_eq!(branch, Some("feature/hybrid-context".into()));

        let model = store.get_workspace_state("/ws", "preferred_model").unwrap();
        assert_eq!(model, Some("deepseek-v4-pro".into()));

        let missing = store.get_workspace_state("/ws", "nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn delete_workspace_state() {
        let store = HybridContextStore::open_in_memory().expect("open");
        store.set_workspace_state("/ws", "tmp", "value").unwrap();
        assert!(store.get_workspace_state("/ws", "tmp").unwrap().is_some());

        let deleted = store.delete_workspace_state("/ws", "tmp").unwrap();
        assert!(deleted);
        assert!(store.get_workspace_state("/ws", "tmp").unwrap().is_none());
    }

    #[test]
    fn entity_linking() {
        let store = HybridContextStore::open_in_memory().expect("open");

        store.link_entities("turn-1", "decision-1", "references").unwrap();
        store.link_entities("turn-2", "decision-1", "references").unwrap();

        let linked = store.get_linked("decision-1", Some("references")).unwrap();
        assert_eq!(linked.len(), 2);
    }

    #[test]
    fn hybrid_context_assembly() {
        let store = HybridContextStore::open_in_memory().expect("open");

        // Insert turns
        for i in 0..5 {
            store.insert_turn(make_turn("thread-hybrid", i, "user", &format!("message {i}"))).unwrap();
        }

        // Insert a decision
        store.record_decision("/test-ws", "use-rust", "Using Rust for the core", 0.9).unwrap();

        // Set workspace state
        store.set_workspace_state("/test-ws", "language", "Rust").unwrap();

        let ctx = store.build_hybrid_context(&ContextParams {
            thread_id: Some("thread-hybrid".into()),
            workspace: Some("/test-ws".into()),
            max_tokens: 100_000,
            max_recent_turns: 10,
            include_workspace_state: true,
            include_decisions: true,
            include_references: true,
            search_query: None,
        }).expect("build context");

        assert!(ctx.text.contains("Recent Conversation"));
        assert!(ctx.text.contains("message 4")); // most recent
        assert!(ctx.text.contains("Key Decisions"));
        assert!(ctx.text.contains("Workspace State"));
        assert!(ctx.turn_count > 0);
        assert!(ctx.decision_count > 0);
        assert!(!ctx.truncated);
    }

    #[test]
    fn full_text_search() {
        let store = HybridContextStore::open_in_memory().expect("open");

        store.insert_turn(make_turn("fts-test", 0, "user", "I need to implement a database layer")).unwrap();
        store.insert_turn(make_turn("fts-test", 1, "assistant", "Let's use SQLite with WAL mode")).unwrap();
        store.insert_turn(make_turn("fts-test", 2, "user", "What about PostgreSQL?")).unwrap();

        let results = store.search_turns("SQLite", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.content.contains("SQLite")));

        let results = store.search_turns("PostgreSQL", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.content.contains("PostgreSQL")));
    }

    #[test]
    fn batch_insert_performance() {
        let store = HybridContextStore::open_in_memory().expect("open");

        let turns: Vec<_> = (0..100)
            .map(|i| make_turn("batch-test", i, "user", &format!("turn {i}")))
            .collect();

        store.insert_turns_batch(&turns).expect("batch insert");
        assert_eq!(store.turn_count("batch-test").unwrap(), 100);
    }

    #[test]
    fn entry_type_round_trip() {
        for original in &[
            EntryType::Turn,
            EntryType::Snippet,
            EntryType::Link,
            EntryType::Decision,
            EntryType::WorkspaceState,
            EntryType::FileRef,
            EntryType::Image,
        ] {
            let s = original.as_str();
            let parsed = EntryType::from_str(s).expect("parse");
            assert_eq!(*original, parsed, "round-trip failed for {s}");
        }
    }

    #[test]
    fn search_by_tags() {
        let store = HybridContextStore::open_in_memory().expect("open");

        let e1 = ContextEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Snippet,
            workspace: "/ws".into(),
            timestamp: Utc::now().to_rfc3339(),
            key: "snippet.1".into(),
            value: serde_json::json!({"code": "fn main() {}"}),
            tags: vec!["rust".into(), "example".into()],
            importance: 0.8,
            access_count: 0,
            last_accessed: None,
        };
        let e2 = ContextEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Snippet,
            workspace: "/ws".into(),
            timestamp: Utc::now().to_rfc3339(),
            key: "snippet.2".into(),
            value: serde_json::json!({"code": "print('hello')"}),
            tags: vec!["python".into(), "example".into()],
            importance: 0.6,
            access_count: 0,
            last_accessed: None,
        };

        store.store_entry(e1).unwrap();
        store.store_entry(e2).unwrap();

        let results = store.search_by_tags(&["rust".into()], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "snippet.1");

        let results = store.search_by_tags(&["example".into()], 10).unwrap();
        assert_eq!(results.len(), 2);
    }
}
