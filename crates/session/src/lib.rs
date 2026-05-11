use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Session ID
// ---------------------------------------------------------------------------

/// Unique session identifier.
pub type SessionId = String;

/// Generate a new session ID (UUID v4, hyphenated).
#[must_use]
pub fn new_session_id() -> SessionId {
    Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// Session turn
// ---------------------------------------------------------------------------

/// A single turn in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    /// Monotonic turn index (0-based).
    pub index: u64,
    /// ISO-8601 timestamp of the turn.
    pub timestamp: String,
    /// The model used for this turn.
    pub model: String,
    /// The provider used for this turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// User input / prompt.
    pub user_input: String,
    /// Assistant response text.
    pub assistant_response: String,
    /// Reasoning/thinking content if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Tool calls made during this turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<SerializedToolCall>,
    /// Turn-level metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Serialized representation of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedToolCall {
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub success: bool,
    pub duration_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Session manifest
// ---------------------------------------------------------------------------

/// Lightweight metadata for listing and identifying sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    /// Unique session identifier.
    pub id: SessionId,
    /// Human-readable name (set by user or auto-generated).
    pub name: String,
    /// The workspace path this session was created in.
    pub workspace_path: String,
    /// Hostname of the device that created this session.
    pub hostname: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last-modified timestamp.
    pub updated_at: String,
    /// Number of turns in the session.
    pub turn_count: u64,
    /// The default model used.
    pub model: String,
    /// First line of the first user prompt (for preview).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Tags applied to the session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Whether the session is archived.
    #[serde(default)]
    pub archived: bool,
    /// Checksum of the session data for integrity verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

// ---------------------------------------------------------------------------
// Session data (full)
// ---------------------------------------------------------------------------

/// Complete session data including manifest and turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub manifest: SessionManifest,
    pub turns: Vec<SessionTurn>,
}

// ---------------------------------------------------------------------------
// Session store
// ---------------------------------------------------------------------------

/// Manages persistent session storage on disk.
///
/// Sessions are stored as directories under `base_dir/<session-id>/`:
/// - `manifest.json` — session metadata
/// - `turns.jsonl` — one JSON object per line, one line per turn
pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    /// Create a new session store rooted at `base_dir`.
    #[must_use]
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Return the base directory path.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Default session directory: `~/.deepseek/sessions/`
    pub fn default_store() -> Result<Self> {
        let home = dirs_fallback()?;
        Ok(Self::new(home.join(".deepseek").join("sessions")))
    }

    /// Workspace-local session directory: `<workspace>/.deepseek/sessions/`
    pub fn workspace_store(workspace: &Path) -> Self {
        Self::new(workspace.join(".deepseek").join("sessions"))
    }

    /// Ensure the session directory exists.
    pub fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir)
            .with_context(|| format!("failed to create session dir: {}", self.base_dir.display()))
    }

    /// Path to a specific session directory.
    fn session_dir(&self, id: &SessionId) -> PathBuf {
        self.base_dir.join(id)
    }

    /// Save a full session to disk.
    pub fn save(&self, data: &SessionData) -> Result<()> {
        self.ensure_dir()?;
        let dir = self.session_dir(&data.manifest.id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create session dir: {}", dir.display()))?;

        // Write manifest
        let manifest_path = dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&data.manifest)
            .context("failed to serialize session manifest")?;
        std::fs::write(&manifest_path, manifest_json)
            .with_context(|| format!("failed to write manifest: {}", manifest_path.display()))?;

        // Write turns as JSONL
        let turns_path = dir.join("turns.jsonl");
        let mut file = std::fs::File::create(&turns_path)
            .with_context(|| format!("failed to create turns file: {}", turns_path.display()))?;
        for turn in &data.turns {
            let line = serde_json::to_string(turn)
                .context("failed to serialize turn")?;
            writeln!(file, "{line}")?;
        }

        Ok(())
    }

    /// Load a full session from disk.
    pub fn load(&self, id: &SessionId) -> Result<SessionData> {
        let dir = self.session_dir(id);
        if !dir.exists() {
            bail!("session not found: {id}");
        }

        let manifest_path = dir.join("manifest.json");
        let manifest_json = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read manifest: {}", manifest_path.display()))?;
        let manifest: SessionManifest = serde_json::from_str(&manifest_json)
            .context("failed to parse session manifest")?;

        let turns_path = dir.join("turns.jsonl");
        let turns = if turns_path.exists() {
            let file = std::fs::File::open(&turns_path)
                .with_context(|| format!("failed to open turns: {}", turns_path.display()))?;
            let reader = BufReader::new(file);
            let mut turns = Vec::new();
            for line in reader.lines() {
                let line = line.context("failed to read turn line")?;
                if line.trim().is_empty() {
                    continue;
                }
                let turn: SessionTurn = serde_json::from_str(&line)
                    .with_context(|| format!("failed to parse turn: {line}"))?;
                turns.push(turn);
            }
            turns
        } else {
            Vec::new()
        };

        Ok(SessionData { manifest, turns })
    }

    /// Delete a session from disk.
    pub fn delete(&self, id: &SessionId) -> Result<()> {
        let dir = self.session_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .with_context(|| format!("failed to delete session: {}", dir.display()))?;
        }
        Ok(())
    }

    /// List all sessions, sorted by updated_at descending.
    pub fn list(&self) -> Result<Vec<SessionManifest>> {
        self.ensure_dir()?;
        let mut manifests = Vec::new();

        let entries = std::fs::read_dir(&self.base_dir)
            .with_context(|| format!("failed to read session dir: {}", self.base_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            match std::fs::read_to_string(&manifest_path) {
                Ok(json) => {
                    if let Ok(manifest) = serde_json::from_str::<SessionManifest>(&json) {
                        manifests.push(manifest);
                    }
                }
                Err(_) => continue,
            }
        }

        // Sort by updated_at descending
        manifests.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(manifests)
    }

    /// List sessions for a specific workspace.
    pub fn list_by_workspace(&self, workspace: &str) -> Result<Vec<SessionManifest>> {
        let all = self.list()?;
        Ok(all
            .into_iter()
            .filter(|m| m.workspace_path == workspace)
            .collect())
    }

    /// Find sessions matching a search query (name, preview, tags).
    pub fn search(&self, query: &str) -> Result<Vec<SessionManifest>> {
        let all = self.list()?;
        let lower = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&lower)
                    || m.preview
                        .as_ref()
                        .is_some_and(|p| p.to_lowercase().contains(&lower))
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&lower))
            })
            .collect())
    }

    /// Check if a session exists.
    #[must_use]
    pub fn exists(&self, id: &SessionId) -> bool {
        self.session_dir(id).join("manifest.json").exists()
    }

    /// Get the total number of sessions.
    pub fn count(&self) -> Result<usize> {
        Ok(self.list()?.len())
    }
}

// ---------------------------------------------------------------------------
// Session export
// ---------------------------------------------------------------------------

/// Export format version embedded in archives.
const EXPORT_FORMAT_VERSION: &str = "1.0";

/// Header written at the start of an exported session archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportHeader {
    format_version: String,
    session_id: SessionId,
    exported_at: String,
    export_hostname: String,
    source_workspace: String,
}

/// Creates a portable `.ds-session` archive (tar.gz) from a session.
///
/// Archive structure:
/// ```text
/// header.json       — ExportHeader metadata
/// manifest.json     — SessionManifest
/// turns.jsonl       — SessionTurn entries, one per line
/// ```
pub fn export_session(store: &SessionStore, id: &SessionId, output_path: &Path) -> Result<()> {
    let data = store.load(id)?;

    let header = ExportHeader {
        format_version: EXPORT_FORMAT_VERSION.to_string(),
        session_id: data.manifest.id.clone(),
        exported_at: now_iso(),
        export_hostname: hostname(),
        source_workspace: data.manifest.workspace_path.clone(),
    };

    let file = std::fs::File::create(output_path)
        .with_context(|| format!("failed to create export file: {}", output_path.display()))?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(gz);

    // header.json
    let header_json = serde_json::to_string_pretty(&header)
        .context("failed to serialize export header")?;
    add_tar_entry(&mut archive, "header.json", header_json.as_bytes())?;

    // manifest.json
    let manifest_json = serde_json::to_string_pretty(&data.manifest)
        .context("failed to serialize manifest")?;
    add_tar_entry(&mut archive, "manifest.json", manifest_json.as_bytes())?;

    // turns.jsonl
    let mut turns_buf = Vec::new();
    for turn in &data.turns {
        let line = serde_json::to_string(turn).context("failed to serialize turn")?;
        turns_buf.extend_from_slice(line.as_bytes());
        turns_buf.push(b'\n');
    }
    add_tar_entry(&mut archive, "turns.jsonl", &turns_buf)?;

    let gz = archive
        .into_inner()
        .context("failed to finalize tar archive")?;
    gz.finish().context("failed to finalize gzip")?;

    Ok(())
}

/// Import a `.ds-session` archive into a session store.
///
/// Returns the imported session data. If a session with the same ID already
/// exists and `overwrite` is false, the import is rejected.
pub fn import_session(
    store: &SessionStore,
    archive_path: &Path,
    overwrite: bool,
) -> Result<SessionData> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("failed to open archive: {}", archive_path.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut header: Option<ExportHeader> = None;
    let mut manifest: Option<SessionManifest> = None;
    let mut turns_jsonl = Vec::new();

    for entry in archive.entries().context("failed to read archive entries")? {
        let mut entry = entry.context("failed to read entry")?;
        let path = entry.path()?.to_path_buf();
        let path_str = path.to_string_lossy();

        match path_str.as_ref() {
            "header.json" => {
                let mut buf = Vec::new();
                std::io::copy(&mut entry, &mut buf).context("failed to read header.json")?;
                header = Some(serde_json::from_slice(&buf).context("failed to parse header.json")?);
            }
            "manifest.json" => {
                let mut buf = Vec::new();
                std::io::copy(&mut entry, &mut buf).context("failed to read manifest.json")?;
                manifest =
                    Some(serde_json::from_slice(&buf).context("failed to parse manifest.json")?);
            }
            "turns.jsonl" => {
                std::io::copy(&mut entry, &mut turns_jsonl)
                    .context("failed to read turns.jsonl")?;
            }
            _ => {}
        }
    }

    let header = header.context("archive missing header.json")?;
    let manifest = manifest.context("archive missing manifest.json")?;

    if header.format_version != EXPORT_FORMAT_VERSION {
        bail!(
            "unsupported archive format version: {} (expected {})",
            header.format_version,
            EXPORT_FORMAT_VERSION
        );
    }

    if !overwrite && store.exists(&manifest.id) {
        bail!(
            "session '{}' already exists; use --overwrite to replace",
            manifest.id
        );
    }

    // Parse turns
    let turns: Vec<SessionTurn> = if turns_jsonl.is_empty() {
        Vec::new()
    } else {
        let text = String::from_utf8(turns_jsonl).context("turns.jsonl is not valid UTF-8")?;
        let mut turns = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let turn: SessionTurn = serde_json::from_str(trimmed)
                .with_context(|| format!("failed to parse turn: {trimmed}"))?;
            turns.push(turn);
        }
        turns
    };

    let data = SessionData {
        manifest: SessionManifest {
            updated_at: now_iso(),
            ..manifest
        },
        turns,
    };

    store.save(&data)?;
    Ok(data)
}

/// Validate an archive without importing it.
pub fn validate_archive(archive_path: &Path) -> Result<ValidationReport> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("failed to open archive: {}", archive_path.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut has_header = false;
    let mut has_manifest = false;
    let mut has_turns = false;
    let mut turn_count = 0u64;
    let mut session_name = String::new();
    let mut errors = Vec::new();

    let entries = match archive.entries() {
        Ok(e) => e,
        Err(e) => {
            return Ok(ValidationReport {
                valid: false,
                has_header: false,
                has_manifest: false,
                has_turns: false,
                turn_count: 0,
                session_name: String::new(),
                errors: vec![format!("failed to read archive entries: {e}")],
            });
        }
    };

    for entry in entries {
        let mut entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("entry error: {e}"));
                continue;
            }
        };
        let path = match entry.path() {
            Ok(p) => p.to_path_buf(),
            Err(e) => {
                errors.push(format!("path error: {e}"));
                continue;
            }
        };
        let path_str = path.to_string_lossy();

        match path_str.as_ref() {
            "header.json" => has_header = true,
            "manifest.json" => {
                has_manifest = true;
                let mut buf = Vec::new();
                if let Err(e) = std::io::copy(&mut entry, &mut buf) {
                    errors.push(format!("manifest read error: {e}"));
                } else if let Ok(m) = serde_json::from_slice::<SessionManifest>(&buf) {
                    session_name = m.name;
                }
            }
            "turns.jsonl" => {
                has_turns = true;
                let mut buf = Vec::new();
                if let Err(e) = std::io::copy(&mut entry, &mut buf) {
                    errors.push(format!("turns read error: {e}"));
                } else {
                    turn_count = buf.iter().filter(|&&b| b == b'\n').count() as u64;
                }
            }
            _ => {}
        }
    }

    Ok(ValidationReport {
        valid: errors.is_empty() && has_header && has_manifest && has_turns,
        has_header,
        has_manifest,
        has_turns,
        turn_count,
        session_name,
        errors,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub has_header: bool,
    pub has_manifest: bool,
    pub has_turns: bool,
    pub turn_count: u64,
    pub session_name: String,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Session builder (convenience API)
// ---------------------------------------------------------------------------

/// Builder for constructing SessionData programmatically.
#[derive(Debug, Default)]
pub struct SessionBuilder {
    name: Option<String>,
    workspace_path: Option<String>,
    model: Option<String>,
    turns: Vec<SessionTurn>,
    tags: Vec<String>,
}

impl SessionBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn add_turn(&mut self, turn: SessionTurn) -> &mut Self {
        self.turns.push(turn);
        self
    }

    #[must_use]
    pub fn with_turn(mut self, turn: SessionTurn) -> Self {
        self.turns.push(turn);
        self
    }

    /// Build the SessionData.
    pub fn build(self) -> Result<SessionData> {
        let now = now_iso();
        let id = new_session_id();
        let name = self.name.unwrap_or_else(|| format!("session-{}", &id[..8]));
        let workspace = self
            .workspace_path
            .unwrap_or_else(|| std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string()));
        let model = self.model.unwrap_or_else(|| "deepseek-v4-pro".to_string());
        let preview = self.turns.first().map(|t| {
            t.user_input
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(120)
                .collect()
        });

        let manifest = SessionManifest {
            id: id.clone(),
            name,
            workspace_path: workspace,
            hostname: hostname(),
            created_at: now.clone(),
            updated_at: now,
            turn_count: self.turns.len() as u64,
            model,
            preview,
            tags: self.tags,
            archived: false,
            checksum: None,
        };

        Ok(SessionData {
            manifest,
            turns: self.turns,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn dirs_fallback() -> Result<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .context("could not determine home directory")
}

fn add_tar_entry<W: Write>(
    archive: &mut tar::Builder<W>,
    name: &str,
    data: &[u8],
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_path(name).context("invalid tar entry path")?;
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header.clone(), name, data)
        .with_context(|| format!("failed to add tar entry: {name}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, SessionStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = SessionStore::new(dir.path().to_path_buf());
        (dir, store)
    }

    fn sample_session(name: &str) -> SessionData {
        SessionBuilder::new()
            .name(name)
            .workspace("/tmp/test-workspace")
            .model("deepseek-v4-flash")
            .with_turn(SessionTurn {
                index: 0,
                timestamp: now_iso(),
                model: "deepseek-v4-flash".into(),
                provider: Some("deepseek".into()),
                user_input: "Hello, what is Rust?".into(),
                assistant_response: "Rust is a systems programming language...".into(),
                reasoning: None,
                tool_calls: vec![],
                metadata: None,
            })
            .with_turn(SessionTurn {
                index: 1,
                timestamp: now_iso(),
                model: "deepseek-v4-flash".into(),
                provider: Some("deepseek".into()),
                user_input: "Show me an example.".into(),
                assistant_response: "Here's a simple program:".into(),
                reasoning: Some("User wants code example".into()),
                tool_calls: vec![SerializedToolCall {
                    name: "write_file".into(),
                    arguments: r#"{"path":"main.rs","content":"fn main() {}"}"#.into(),
                    result: Some("file written".into()),
                    success: true,
                    duration_ms: Some(42),
                }],
                metadata: None,
            })
            .build()
            .expect("build session")
    }

    #[test]
    fn save_and_load_session() {
        let (_dir, store) = temp_store();
        let data = sample_session("test-session");

        store.save(&data).expect("save");
        assert!(store.exists(&data.manifest.id));

        let loaded = store.load(&data.manifest.id).expect("load");
        assert_eq!(loaded.manifest.name, "test-session");
        assert_eq!(loaded.turns.len(), 2);
        assert_eq!(loaded.turns[0].user_input, "Hello, what is Rust?");
        assert_eq!(loaded.turns[1].tool_calls.len(), 1);
    }

    #[test]
    fn list_sessions_sorted_by_date() {
        let (_dir, store) = temp_store();
        let mut data1 = sample_session("alpha");
        let mut data2 = sample_session("beta");

        // Ensure different timestamps
        data2.manifest.updated_at = "2026-01-01T00:00:00Z".into();
        data1.manifest.updated_at = "2026-06-01T00:00:00Z".into();

        store.save(&data2).expect("save beta");
        store.save(&data1).expect("save alpha");

        let list = store.list().expect("list");
        assert_eq!(list.len(), 2);
        // alpha (newer) should be first
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[1].name, "beta");
    }

    #[test]
    fn delete_session() {
        let (_dir, store) = temp_store();
        let data = sample_session("to-delete");
        let id = data.manifest.id.clone();

        store.save(&data).expect("save");
        assert!(store.exists(&id));

        store.delete(&id).expect("delete");
        assert!(!store.exists(&id));
    }

    #[test]
    fn export_and_import_round_trip() {
        let (_dir, store) = temp_store();
        let data = sample_session("portable-session");
        store.save(&data).expect("save");

        let archive_path = _dir.path().join("export.ds-session");
        export_session(&store, &data.manifest.id, &archive_path).expect("export");
        assert!(archive_path.exists());

        // Delete original, then import
        store.delete(&data.manifest.id).expect("delete");

        let imported = import_session(&store, &archive_path, false).expect("import");
        assert_eq!(imported.manifest.name, "portable-session");
        assert_eq!(imported.turns.len(), 2);
        assert_eq!(imported.turns[0].user_input, "Hello, what is Rust?");
    }

    #[test]
    fn import_rejects_duplicate_without_overwrite() {
        let (_dir, store) = temp_store();
        let data = sample_session("conflict-test");
        store.save(&data).expect("save");

        let archive_path = _dir.path().join("conflict.ds-session");
        export_session(&store, &data.manifest.id, &archive_path).expect("export");

        let result = import_session(&store, &archive_path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn import_overwrites_when_requested() {
        let (_dir, store) = temp_store();
        let data = sample_session("overwrite-test");
        store.save(&data).expect("save");

        let archive_path = _dir.path().join("overwrite.ds-session");
        export_session(&store, &data.manifest.id, &archive_path).expect("export");

        let result = import_session(&store, &archive_path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_archive_reports_structure() {
        let (_dir, store) = temp_store();
        let data = sample_session("validate-test");
        store.save(&data).expect("save");

        let archive_path = _dir.path().join("validate.ds-session");
        export_session(&store, &data.manifest.id, &archive_path).expect("export");

        let report = validate_archive(&archive_path).expect("validate");
        assert!(report.valid);
        assert!(report.has_header);
        assert!(report.has_manifest);
        assert!(report.has_turns);
        assert_eq!(report.turn_count, 2);
        assert_eq!(report.session_name, "validate-test");
    }

    #[test]
    fn list_by_workspace_filters_correctly() {
        let (_dir, store) = temp_store();
        let mut data1 = sample_session("ws-a-1");
        data1.manifest.workspace_path = "/home/user/project-a".into();
        let mut data2 = sample_session("ws-b-1");
        data2.manifest.workspace_path = "/home/user/project-b".into();
        let mut data3 = sample_session("ws-a-2");
        data3.manifest.workspace_path = "/home/user/project-a".into();

        store.save(&data1).expect("save");
        store.save(&data2).expect("save");
        store.save(&data3).expect("save");

        let results = store
            .list_by_workspace("/home/user/project-a")
            .expect("list");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.workspace_path == "/home/user/project-a"));
    }

    #[test]
    fn search_finds_by_name_and_preview() {
        let (_dir, store) = temp_store();
        let data = sample_session("rust-tutorial");
        store.save(&data).expect("save");

        let results = store.search("rust").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-tutorial");
    }

    #[test]
    fn builder_produces_valid_session() {
        let data = sample_session("builder-test");
        assert!(!data.manifest.id.is_empty());
        assert_eq!(data.manifest.name, "builder-test");
        assert_eq!(data.turns.len(), 2);
        assert!(data.manifest.preview.is_some());
    }
}
