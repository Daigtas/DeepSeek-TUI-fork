use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

// ---------------------------------------------------------------------------
// Panes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pane {
    Chat,
    Diff,
    Tasks,
    Agents,
    Status,
    Jobs,
}

impl Pane {
    /// Human-readable label for the pane.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Chat => "Chat",
            Self::Diff => "Diff",
            Self::Tasks => "Tasks",
            Self::Agents => "Agents",
            Self::Status => "Status",
            Self::Jobs => "Jobs",
        }
    }

    /// Default keybinding for this pane (char).
    #[must_use]
    pub fn default_key(&self) -> char {
        match self {
            Self::Chat => '1',
            Self::Diff => '2',
            Self::Tasks => '3',
            Self::Agents => '4',
            Self::Status => '6',
            Self::Jobs => '5',
        }
    }
}

// ---------------------------------------------------------------------------
// Context budget
// ---------------------------------------------------------------------------

/// Tracks token usage against a total budget and signals when remaining
/// headroom drops below configurable warning and critical thresholds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Total context window size in tokens.
    pub total_tokens: usize,
    /// Tokens currently consumed.
    pub used_tokens: usize,
    /// Fraction of total remaining that triggers a warning (default 0.35).
    pub warning_threshold: f64,
    /// Fraction of total remaining that triggers a critical alert (default 0.25).
    pub critical_threshold: f64,
}

impl ContextBudget {
    /// Create a new budget with the given total token capacity and default thresholds.
    #[must_use]
    pub fn new(total_tokens: usize) -> Self {
        Self {
            total_tokens,
            used_tokens: 0,
            warning_threshold: 0.35,
            critical_threshold: 0.25,
        }
    }

    /// Update the used token count.
    pub fn update(&mut self, used_tokens: usize) {
        self.used_tokens = used_tokens;
    }

    /// Tokens remaining in the budget.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.total_tokens.saturating_sub(self.used_tokens)
    }

    /// Fraction of total tokens still available (0.0 – 1.0).
    #[must_use]
    pub fn pct_remaining(&self) -> f64 {
        if self.total_tokens == 0 {
            return 0.0;
        }
        self.remaining() as f64 / self.total_tokens as f64
    }

    /// True when the remaining fraction is at or below the warning threshold.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.pct_remaining() <= self.warning_threshold
            && self.pct_remaining() > self.critical_threshold
    }

    /// True when the remaining fraction is at or below the critical threshold.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.pct_remaining() <= self.critical_threshold
    }

    /// Gauge color zone.
    #[must_use]
    pub fn zone(&self) -> BudgetZone {
        let pct = self.pct_remaining();
        if pct <= self.critical_threshold {
            BudgetZone::Critical
        } else if pct <= self.warning_threshold {
            BudgetZone::Warning
        } else {
            BudgetZone::Ok
        }
    }

    /// Render an ASCII gauge bar of `width` characters.
    /// Example: `[████████░░░░░░░░] 65%`
    #[must_use]
    pub fn gauge_bar(&self, width: usize) -> String {
        let bar_width = width.saturating_sub(2); // brackets
        let used_pct = 1.0 - self.pct_remaining();
        let filled = (used_pct * bar_width as f64).round() as usize;
        let filled = filled.min(bar_width);
        let marker = match self.zone() {
            BudgetZone::Ok => '█',
            BudgetZone::Warning => '▓',
            BudgetZone::Critical => '░',
        };
        let bar: String = (0..bar_width)
            .map(|i| if i < filled { marker } else { ' ' })
            .collect();
        let pct = (used_pct * 100.0).round() as usize;
        format!("[{bar}] {pct}%")
    }

    /// Short human-readable status for the gauge.
    #[must_use]
    pub fn gauge_label(&self) -> &'static str {
        match self.zone() {
            BudgetZone::Ok => "context OK",
            BudgetZone::Warning => "context WARN",
            BudgetZone::Critical => "CONTEXT CRITICAL",
        }
    }

    /// Detailed description for tooltip/status bar.
    #[must_use]
    pub fn gauge_detail(&self) -> String {
        let used_k = self.used_tokens as f64 / 1000.0;
        let total_k = self.total_tokens as f64 / 1000.0;
        let rem_k = self.remaining() as f64 / 1000.0;
        format!(
            "{}: {:.1}K / {:.1}K used, {:.1}K remaining ({:.0}%)",
            self.gauge_label(),
            used_k,
            total_k,
            rem_k,
            self.pct_remaining() * 100.0
        )
    }
}

/// Color zone for the context budget gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetZone {
    /// Plenty of headroom — green.
    Ok,
    /// Approaching limit — yellow/amber.
    Warning,
    /// Critically low — red.
    Critical,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            total_tokens: 128_000,
            used_tokens: 0,
            warning_threshold: 0.35,
            critical_threshold: 0.25,
        }
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Content type detected when pasting data into the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PasteContentType {
    /// Plain text input.
    Text,
    /// Source code with optional language hint.
    Code { language: Option<String> },
    /// A URL / hyperlink.
    Link { url: String },
    /// An image (raw bytes or base64).
    Image { mime_type: String, size_bytes: usize },
    /// Mixed content (multiple types detected).
    Mixed { components: Vec<PasteContentType> },
}

impl PasteContentType {
    /// Human-readable label for the detected content type.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code { .. } => "code",
            Self::Link { .. } => "link",
            Self::Image { .. } => "image",
            Self::Mixed { .. } => "mixed",
        }
    }

    /// Detect content type from raw pasted text.
    #[must_use]
    pub fn detect(raw: &str) -> Self {
        detect_paste_content(raw)
    }

    /// Detect content type from raw bytes (for binary/image paste).
    #[must_use]
    pub fn detect_bytes(data: &[u8]) -> Self {
        detect_paste_bytes(data)
    }

    /// Suggested syntax highlighter scope for code blocks (syntect/tree-sitter compatible).
    #[must_use]
    pub fn syntax_scope(&self) -> Option<&'static str> {
        match self {
            Self::Code { language } => language.as_deref().and_then(|l| match l {
                "rust" => Some("source.rust"),
                "python" => Some("source.python"),
                "javascript" | "js" => Some("source.js"),
                "typescript" | "ts" => Some("source.ts"),
                "go" => Some("source.go"),
                "java" => Some("source.java"),
                "cpp" | "c" => Some("source.c++"),
                "bash" | "sh" | "shell" => Some("source.shell"),
                "ruby" => Some("source.ruby"),
                "json" => Some("source.json"),
                "xml" | "html" => Some("text.xml"),
                "toml" => Some("source.toml"),
                "yaml" | "yml" => Some("source.yaml"),
                "diff" => Some("source.diff"),
                "perl" => Some("source.perl"),
                "sql" => Some("source.sql"),
                "css" => Some("source.css"),
                _ => None,
            }),
            _ => None,
        }
    }

    /// Line count if this is text-based content.
    #[must_use]
    pub fn line_count(&self, content: &str) -> usize {
        content.lines().count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiEvent {
    KeyPressed(char),
    PromptSubmitted(String),
    ResponseDelta(String),
    ToolStarted(String),
    ToolFinished(String),
    JobQueued(String),
    JobProgress { job_id: String, progress: u8 },
    JobCompleted(String),
    ApprovalRequested(String),
    ApprovalResolved(String),
    PauseRequested,
    ResumeRequested,
    /// Bracketed paste started (terminal sent \e[200~).
    PasteStart,
    /// Bracketed paste ended (terminal sent \e[201~).
    PasteEnd,
    /// Content pasted from clipboard (terminal paste / OSC 52).
    PasteContent {
        /// Text representation (UTF-8 for text, base64 for images).
        content: String,
        /// Raw binary data (set for image paste, None for text).
        raw_bytes: Option<Vec<u8>>,
        /// Detected content type.
        detected_type: PasteContentType,
    },
    /// Context budget dropped below the warning threshold.
    ContextWarning,
    /// Context budget dropped below the critical threshold.
    ContextCritical,
    Tick,
    /// Tab pressed — trigger path/file autocompletion.
    TabPressed,
    /// Tab pressed — trigger path/file/slash autocompletion (does NOT cycle mode).
    CompletionTrigger,
    /// Ctrl+Tab pressed — cycle between Plan / Agent / YOLO modes.
    CtrlTabPressed,
    /// User input sent while agent is executing tools (mid-execution pushback).
    UserPushback(String),
    /// Backspace in the input buffer (for path completion tracking).
    BackspacePressed,
    /// Cycle to next path completion candidate.
    CyclePathCompletion,
    /// Enter key pressed (submit prompt).
    EnterPressed,
    /// Shift+Enter pressed (insert literal newline for multi-line input).
    ShiftEnterPressed,
    /// A slash command was submitted (e.g. /help, /model deepseek-v4-pro).
    SlashCommand(SlashCommand),
    /// An agent was spawned in the swarm.
    AgentSpawned { id: String, role: String, name: String },
    /// An agent completed its task.
    AgentCompleted { id: String, summary: String },
    /// An agent encountered an error.
    AgentErrored { id: String, error: String },
    /// Periodic agent heartbeat (for liveness monitoring).
    AgentHeartbeat { id: String },
    /// Capture a named checkpoint of the current session state.
    CaptureCheckpoint { name: String, description: Option<String> },
    /// Restore a previously saved checkpoint.
    RestoreCheckpoint { name: String },
}

// ---------------------------------------------------------------------------
// Effects (stack-allocated via SmallVec)
// ---------------------------------------------------------------------------

/// SmallVec with inline capacity for 4 effects (covers most reduce() returns).
pub type EffectVec = SmallVec<[UiEffect; 4]>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiEffect {
    Render,
    PersistCheckpoint,
    ScheduleBackgroundRefresh,
    /// Show path/file autocompletion suggestions popup.
    ShowPathSuggestions(Vec<String>),
    /// Replace current path with completed version.
    CompletePath(String),
    /// Execute a slash command (routed to the TUI frontend or daemon).
    ExecuteSlashCommand(SlashCommand),
    /// Attach a file to the current conversation (triggered by `@path`).
    AttachFile(String),
    /// Toggle between Plan / Agent / YOLO modes (triggered by Ctrl+Tab).
    ToggleMode,
    /// Inject user input into the running agent's context for mid-execution rethinking.
    PushbackDraft(String),
    /// Show slash command completion suggestions.
    ShowSlashSuggestions(Vec<String>),
    /// Show the slash command popup menu with structured data.
    ShowSlashPopup(SlashPopup),
    /// Update the context budget gauge in the status bar.
    UpdateContextGauge {
        bar: String,
        label: String,
        detail: String,
        zone: BudgetZone,
    },
    /// Insert a literal newline into the input buffer (multi-line mode).
    InsertNewline,
    /// An agent's status changed — update the agents pane.
    AgentStatusChanged { id: String, status: String },
    /// A checkpoint was captured or restored.
    CheckpointChanged { name: String, action: String },
    /// Request the git diff for the current workspace.
    RequestGitDiff,
    EmitStatusLine(String),
    /// Enable bracketed paste mode on the terminal (\e[?2004h).
    EnableBracketedPaste,
    /// Emit a human-readable context-budget warning message.
    EmitContextWarning(String),
    /// Disable bracketed paste mode on the terminal (\e[?2004l).
    DisableBracketedPaste,
}

// ---------------------------------------------------------------------------
// Bracketed paste buffer (with burst-paste fallback)
// ---------------------------------------------------------------------------

/// Terminal control sequences for bracketed paste mode.
pub mod ansi {
    /// Enable bracketed paste mode: terminal wraps pasted text in \e[200~ … \e[201~.
    pub const BRACKETED_PASTE_ON: &str = "\x1b[?2004h";
    /// Disable bracketed paste mode.
    pub const BRACKETED_PASTE_OFF: &str = "\x1b[?2004l";
    /// Paste start sentinel emitted by terminal.
    pub const PASTE_START: &str = "\x1b[200~";
    /// Paste end sentinel emitted by terminal.
    pub const PASTE_END: &str = "\x1b[201~";
    /// OSC 52 query clipboard request prefix (terminal responds with base64 clipboard data).
    pub const OSC52_QUERY: &str = "\x1b]52;c;?\x1b\\";
    /// OSC 52 set clipboard prefix (for writing to clipboard).
    pub const OSC52_SET_PREFIX: &str = "\x1b]52;c;";
    /// OSC 52 response sentinel (terminal response to query).
    pub const OSC52_RESPONSE_PREFIX: &str = "\x1b]52;";
}

/// Skip a generic ANSI escape sequence starting at `i` in `bytes`.
/// Returns the index after the skipped sequence.
fn skip_escape_sequence(bytes: &[u8], i: usize) -> usize {
    let mut j = i + 1; // skip \x1b
    if j >= bytes.len() { return bytes.len(); }
    match bytes[j] {
        b'[' => {
            // CSI: \x1b[ ... 0x40–0x7e
            j += 1;
            while j < bytes.len() && (bytes[j] < 0x40 || bytes[j] > 0x7e) {
                j += 1;
            }
            if j < bytes.len() { j += 1; }
            j
        }
        b']' => {
            // OSC: \x1b] ... \x07 or \x1b\\
            j += 1;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    j += 1;
                    break;
                }
                if bytes[j] == 0x1b && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                    j += 2;
                    break;
                }
                j += 1;
            }
            j
        }
        b'P' | b'_' | b'^' | b'X' => {
            // DCS, APC, PM, SOS: skip until ST (\x1b\\)
            j += 1;
            while j + 1 < bytes.len() {
                if bytes[j] == 0x1b && bytes[j + 1] == b'\\' {
                    j += 2;
                    break;
                }
                j += 1;
            }
            j
        }
        _ => j, // single-char escape, skip just the \x1b + next char
    }
}

/// Handle an OSC 52 clipboard response.
/// Format: \x1b]52;c;<base64-data>\x07 or \x1b]52;c;<base64-data>\x1b\\
/// Returns the index after the consumed sequence.
fn skip_osc_52(bytes: &[u8], i: usize, events: &mut Vec<UiEvent>) -> usize {
    // Find the data between ';' and the terminator
    let mut data_start = i + 5; // skip \x1b]52;
    while data_start < bytes.len() && bytes[data_start] != b';' {
        data_start += 1;
    }
    if data_start >= bytes.len() { return bytes.len(); }
    data_start += 1; // skip the ';'

    let mut data_end = data_start;
    while data_end < bytes.len() {
        if bytes[data_end] == 0x07 {
            break;
        }
        if bytes[data_end] == 0x1b && data_end + 1 < bytes.len() && bytes[data_end + 1] == b'\\' {
            break;
        }
        data_end += 1;
    }

    if data_end > data_start {
        // We got clipboard data — could be text or image
        let raw_payload = &bytes[data_start..data_end];
        if let Ok(decoded) = base64_decode(raw_payload) {
            let detected_type = PasteContentType::detect_bytes(&decoded);
            let content = match &detected_type {
                PasteContentType::Image { .. } => {
                    String::from_utf8(raw_payload.to_vec()).unwrap_or_default()
                }
                _ => String::from_utf8_lossy(&decoded).into_owned(),
            };
            let raw_bytes = match &detected_type {
                PasteContentType::Image { .. } => Some(decoded),
                _ => None,
            };
            events.push(UiEvent::PasteContent {
                content,
                raw_bytes,
                detected_type,
            });
        }
    }

    // Advance past the terminator
    if data_end < bytes.len() {
        if bytes[data_end] == 0x07 {
            data_end += 1;
        } else {
            data_end += 2; // \x1b\\
        }
    }
    data_end
}

/// Base64 encode helper.
fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(CHARS[((triple >> 18) & 0x3f) as usize] as char);
        output.push(CHARS[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(CHARS[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(CHARS[(triple & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

/// Base64 decode helper.
fn base64_decode(input: &[u8]) -> Result<Vec<u8>, ()> {
    let mut output = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for &byte in input {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' | b' ' | b'\t' => continue,
            _ => return Err(()),
        };
        buf = (buf << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(output)
}

/// Buffers characters between PasteStart and PasteEnd events.
///
/// When PasteEnd is received, the buffer is drained into a PasteContent event
/// with detected content type, and the buffer resets.
///
/// Uses a byte buffer (`Vec<u8>`) to support both text and binary paste
/// content (images, mixed data). Text content is extracted as UTF-8 on drain;
/// binary content is preserved in `raw_bytes`.
#[derive(Debug, Clone)]
pub struct BracketedPasteBuffer {
    /// Accumulated paste content (raw bytes for binary/image support).
    buffer: Vec<u8>,
    /// Whether bracketed paste mode is active in the terminal.
    paste_active: bool,
    /// Whether the terminal supports bracketed paste (set from TerminalCaps).
    pub bracketed_supported: bool,
    /// Burst detection: minimum bytes in a single read to trigger paste.
    pub burst_threshold: usize,
    /// Track whether we are currently in a burst paste (non-bracketed).
    burst_active: bool,
    /// Accumulated burst bytes waiting for the next read to decide.
    burst_buffer: Vec<u8>,
    /// How many consecutive reads had bytes flowing (for burst end detection).
    burst_read_count: u32,
    /// Escape-only reads since last content byte (for menu-paste detection).
    escape_only_reads: u32,
}

impl Default for BracketedPasteBuffer {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            paste_active: false,
            bracketed_supported: true,
            burst_threshold: 20,  // lower threshold catches menu-paste chunks
            burst_active: false,
            burst_buffer: Vec::new(),
            burst_read_count: 0,
            escape_only_reads: 0,
        }
    }
}

impl BracketedPasteBuffer {
    /// Create a new empty paste buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if we are currently inside a bracketed paste region.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.paste_active
    }

    /// Feed a UiEvent into the buffer.
    ///
    /// Returns `Some(UiEvent)` when a complete paste is detected:
    /// - `PasteContent` with detected type when `PasteEnd` arrives
    /// - `None` when buffering is in progress
    /// - Individual `KeyPressed` events are passed through when not in paste mode
    /// - `PasteStart` and `PasteEnd` are consumed by the buffer
    #[must_use]
    pub fn feed(&mut self, event: UiEvent) -> Option<UiEvent> {
        match event {
            UiEvent::PasteStart => {
                self.paste_active = true;
                self.buffer.clear();
                None
            }
            UiEvent::PasteEnd => {
                self.paste_active = false;
                if self.buffer.is_empty() {
                    return None;
                }
                let raw = std::mem::take(&mut self.buffer);
                let detected_type = PasteContentType::detect_bytes(&raw);
                // Convert to text representation (UTF-8 for text, base64 for images)
                let content = match &detected_type {
                    PasteContentType::Image { .. } => {
                        base64_encode(&raw)
                    }
                    _ => String::from_utf8_lossy(&raw).into_owned(),
                };
                let raw_bytes = match &detected_type {
                    PasteContentType::Image { .. } => Some(raw),
                    _ => None,
                };
                Some(UiEvent::PasteContent {
                    content,
                    raw_bytes,
                    detected_type,
                })
            }
            UiEvent::KeyPressed(ch) if self.paste_active => {
                // In paste mode, push the UTF-8 bytes of the character
                let mut buf = [0u8; 4];
                let encoded = ch.encode_utf8(&mut buf);
                self.buffer.extend_from_slice(encoded.as_bytes());
                None
            }
            UiEvent::PasteContent { .. } if self.paste_active => {
                // Nested paste — drain and emit
                self.paste_active = false;
                self.buffer.clear();
                Some(event)
            }
            other => Some(other), // pass through
        }
    }

    /// Feed raw bytes (from terminal stdin) into the buffer.
    ///
    /// This is the low-level entry point for terminal input handling.
    /// It detects escape sequences and builds appropriate events.
    ///
    /// In paste mode, ALL bytes are collected into the raw buffer — no filtering.
    /// This ensures binary data (images, code with special chars) is preserved
    /// and detected correctly when the paste ends.
    /// End the current burst paste, emitting a PasteContent event.
    fn finish_burst(&mut self) -> Option<UiEvent> {
        if self.burst_buffer.is_empty() {
            return None;
        }
        let raw = std::mem::take(&mut self.burst_buffer);
        let detected_type = PasteContentType::detect_bytes(&raw);
        let content = match &detected_type {
            PasteContentType::Image { .. } => base64_encode(&raw),
            _ => String::from_utf8_lossy(&raw).into_owned(),
        };
        let raw_bytes = match &detected_type {
            PasteContentType::Image { .. } => Some(raw),
            _ => None,
        };
        Some(UiEvent::PasteContent { content, raw_bytes, detected_type })
    }

    #[must_use]
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> Vec<UiEvent> {
        // ── Burst paste detection (fallback for terminals without bracketed paste) ──
        // Terminals like PuTTY, screen, tmux (without paste support) don't emit
        // \e[200~ / \e[201~ sentinels. We detect paste via:
        //   1. Large burst in one read (rapid paste)
        //   2. Content after escape-only reads (right-click menu paste)
        //   3. Multi-chunk accumulation (long paste split across reads)
        if !self.bracketed_supported && !self.paste_active {
            // Count content bytes vs escape bytes in this read
            let content_byte_count = bytes.iter().filter(|&&b| b != 0x1b).count();
            let has_only_escapes = content_byte_count == 0 && !bytes.is_empty();

            if bytes.len() >= self.burst_threshold {
                // ── Case 1: Large burst detected ──
                self.burst_active = true;
                self.burst_read_count = 0;
                self.escape_only_reads = 0;
                self.burst_buffer.clear();

                // Collect content bytes, skip escape sequences
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == 0x1b {
                        i = skip_escape_sequence(bytes, i);
                    } else if bytes[i] == b'\r' {
                        self.burst_buffer.push(b'\n');
                        i += 1;
                    } else {
                        self.burst_buffer.push(bytes[i]);
                        i += 1;
                    }
                }
                if !self.burst_buffer.is_empty() {
                    let detected_type = PasteContentType::detect_bytes(&self.burst_buffer);
                    let content = match &detected_type {
                        PasteContentType::Image { .. } => base64_encode(&self.burst_buffer),
                        _ => String::from_utf8_lossy(&self.burst_buffer).into_owned(),
                    };
                    let raw_bytes = match &detected_type {
                        PasteContentType::Image { .. } => Some(std::mem::take(&mut self.burst_buffer)),
                        _ => { self.burst_buffer.clear(); None }
                    };
                    return vec![UiEvent::PasteContent { content, raw_bytes, detected_type }];
                }
            } else if has_only_escapes {
                // ── Case 2: Track escape-only reads (menu interaction) ──
                self.escape_only_reads += 1;
            } else if self.escape_only_reads > 2 && content_byte_count > 0 {
                // ── Case 3: Content after menu dismissal → treat as paste ──
                // After several escape-only reads (menu opened/closed), any
                // content-heavy read is likely a paste operation.
                self.burst_active = true;
                self.escape_only_reads = 0;
                self.burst_buffer.clear();
                for &b in bytes {
                    if b == b'\r' {
                        self.burst_buffer.push(b'\n');
                    } else if b != 0x1b {
                        self.burst_buffer.push(b);
                    }
                }
                if !self.burst_buffer.is_empty() {
                    let detected_type = PasteContentType::detect_bytes(&self.burst_buffer);
                    let content = match &detected_type {
                        PasteContentType::Image { .. } => base64_encode(&self.burst_buffer),
                        _ => String::from_utf8_lossy(&self.burst_buffer).into_owned(),
                    };
                    let raw_bytes = match &detected_type {
                        PasteContentType::Image { .. } => Some(std::mem::take(&mut self.burst_buffer)),
                        _ => { self.burst_buffer.clear(); None }
                    };
                    return vec![UiEvent::PasteContent { content, raw_bytes, detected_type }];
                }
            } else if content_byte_count > 0 {
                // Reset escape counter when we see content during normal typing
                self.escape_only_reads = 0;
                if self.burst_active && bytes.is_empty() {
                    // Empty read ends burst
                    self.burst_active = false;
                    self.escape_only_reads = 0;
                    if let Some(ev) = self.finish_burst() {
                        return vec![ev];
                    }
                } else if self.burst_active && content_byte_count < self.burst_threshold {
                    // ── Case 4: Continue accumulating multi-chunk paste ──
                    self.burst_read_count += 1;
                    for &b in bytes {
                        if b == b'\r' {
                            self.burst_buffer.push(b'\n');
                        } else if b != 0x1b {
                            self.burst_buffer.push(b);
                        }
                    }
                    return Vec::new();
                }
            }
        }

        let mut events = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            if self.paste_active {
                // ── Paste mode: collect ALL bytes until paste end sentinel ──
                if bytes[i..].starts_with(b"\x1b[201~") {
                    events.push(UiEvent::PasteEnd);
                    i += 6;
                } else {
                    self.buffer.push(bytes[i]);
                    i += 1;
                }
            } else {
                // ── Normal mode: detect sequences and key presses ──
                // Check for paste start: \x1b[200~
                if bytes[i..].starts_with(b"\x1b[200~") {
                    events.push(UiEvent::PasteStart);
                    i += 6;
                    continue;
                }
                // Check for paste end (shouldn't normally appear outside paste, but handle gracefully)
                if bytes[i..].starts_with(b"\x1b[201~") {
                    events.push(UiEvent::PasteEnd);
                    i += 6;
                    continue;
                }
                // Ctrl+Tab: \x1b[1;5I (most terminals) or \x1b[27;5;9~ (some)
                if bytes[i..].starts_with(b"\x1b[1;5I") {
                    events.push(UiEvent::CtrlTabPressed);
                    i += 6;
                    continue;
                }
                if bytes[i..].starts_with(b"\x1b[27;5;9~") {
                    events.push(UiEvent::CtrlTabPressed);
                    i += 8;
                    continue;
                }
                // OSC 52 clipboard response: \x1b]52;...\x07 or \x1b]52;...\x1b\\
                if bytes[i..].starts_with(b"\x1b]52;") {
                    i = skip_osc_52(bytes, i, &mut events);
                    continue;
                }
                // Generic escape sequence skipping
                if bytes[i] == 0x1b {
                    i = skip_escape_sequence(bytes, i);
                    continue;
                }
                // Regular character — map special keys to dedicated events
                let byte = bytes[i];
                match byte {
                    b'\t' => events.push(UiEvent::CompletionTrigger),
                    b'\x7f' | b'\x08' => events.push(UiEvent::BackspacePressed),
                    b'\r' => events.push(UiEvent::EnterPressed),
                    b'\n' => events.push(UiEvent::KeyPressed('\n')), // newline for multi-line
                    _ => {
                        let ch = byte as char;
                        if ch.is_ascii_graphic() || ch == ' ' {
                            events.push(UiEvent::KeyPressed(ch));
                        }
                    }
                }
                i += 1;
            }
        }

        // Apply buffer logic to the collected events
        let mut output = Vec::new();
        for ev in events {
            if let Some(out_ev) = self.feed(ev) {
                output.push(out_ev);
            }
        }
        output
    }

    /// Reset the buffer state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.paste_active = false;
        self.burst_active = false;
        self.burst_buffer.clear();
        self.burst_read_count = 0;
        self.escape_only_reads = 0;
    }

    /// Current buffered content (useful for preview).
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    /// Get a preview of the buffered content (first N chars, decoded as UTF-8).
    #[must_use]
    pub fn preview(&self, max_chars: usize) -> String {
        let text = String::from_utf8_lossy(&self.buffer);
        if text.len() <= max_chars {
            text.into_owned()
        } else {
            format!("{}…", &text[..max_chars])
        }
    }

    /// Estimate if the paste is "large" (likely binary or multi-KB text).
    #[must_use]
    pub fn is_large_paste(&self) -> bool {
        self.buffer.len() > 4096
    }
}

// ---------------------------------------------------------------------------
// Keybinding configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    /// Map from key character to pane.
    pub pane_keys: Vec<(char, Pane)>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            pane_keys: vec![
                ('1', Pane::Chat),
                ('2', Pane::Diff),
                ('3', Pane::Tasks),
                ('4', Pane::Agents),
                ('5', Pane::Jobs),
            ],
        }
    }
}

impl Keybindings {
    /// Look up the pane bound to a given key.
    #[must_use]
    pub fn pane_for_key(&self, key: char) -> Option<Pane> {
        self.pane_keys
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, p)| *p)
    }

    /// Rebind a key to a pane. Returns the old pane if the key was already bound.
    pub fn rebind(&mut self, key: char, pane: Pane) -> Option<Pane> {
        let old = self.pane_keys.iter().position(|(k, _)| *k == key).map(|i| self.pane_keys[i].1);
        self.pane_keys.retain(|(k, _)| *k != key);
        self.pane_keys.push((key, pane));
        old
    }

    /// Unbind a key.
    pub fn unbind(&mut self, key: char) {
        self.pane_keys.retain(|(k, _)| *k != key);
    }

    /// Check if a key is already bound to any pane.
    #[must_use]
    pub fn is_bound(&self, key: char) -> bool {
        self.pane_keys.iter().any(|(k, _)| *k == key)
    }

    /// Detect conflicts: keys bound to the same pane.
    #[must_use]
    pub fn conflicts(&self) -> Vec<(char, char)> {
        let mut conflicts = Vec::new();
        for (i, (k1, p1)) in self.pane_keys.iter().enumerate() {
            for (k2, p2) in self.pane_keys.iter().skip(i + 1) {
                if p1 == p2 {
                    conflicts.push((*k1, *k2));
                }
            }
        }
        conflicts
    }

    /// List all bindings as (key, pane_label) pairs.
    #[must_use]
    pub fn list_bindings(&self) -> Vec<(char, &'static str)> {
        self.pane_keys.iter().map(|(k, p)| (*k, p.label())).collect()
    }

    /// Build the default keybindings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Slash commands
// ---------------------------------------------------------------------------

/// Commands available via `/` prefix in the input (like Codex CLI slash commands).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashCommand {
    /// Show help and available commands.
    Help,
    /// Request context compaction to free token budget.
    Compact,
    /// Clear the current conversation.
    Clear,
    /// Switch to a different model.
    Model(String),
    /// Show agent swarm status.
    Agents,
    /// Show current git diff.
    Diff,
    /// Attach a file to the conversation.
    File(String),
    /// Switch to a named pane.
    Pane(String),
    /// Show daemon status.
    Status,
    /// Exit the TUI.
    Exit,
    /// Resume a previous session.
    Resume,
    /// Save the current session.
    Save,
}

impl SlashCommand {
    /// Parse a slash command from input text (everything after the `/`).
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let (cmd, arg) = if let Some(space) = trimmed.find(' ') {
            (&trimmed[..space], trimmed[space + 1..].trim())
        } else {
            (trimmed, "")
        };

        match cmd {
            "help" | "h" | "?" => Some(Self::Help),
            "compact" | "cmp" => Some(Self::Compact),
            "clear" | "cls" | "reset" => Some(Self::Clear),
            "model" | "m" => {
                if arg.is_empty() { None } else { Some(Self::Model(arg.to_string())) }
            }
            "agents" | "ag" | "swarm" => Some(Self::Agents),
            "diff" | "d" => Some(Self::Diff),
            "file" | "f" | "attach" => {
                if arg.is_empty() { None } else { Some(Self::File(arg.to_string())) }
            }
            "pane" | "p" => {
                if arg.is_empty() { None } else { Some(Self::Pane(arg.to_string())) }
            }
            "status" | "st" => Some(Self::Status),
            "exit" | "quit" | "q" => Some(Self::Exit),
            "resume" | "res" => Some(Self::Resume),
            "save" | "s" => Some(Self::Save),
            _ => None,
        }
    }

    /// All available commands for autocompletion.
    pub fn completions() -> &'static [(&'static str, &'static str)] {
        &[
            ("help", "Show available commands and usage"),
            ("compact", "Free context token budget"),
            ("clear", "Clear current conversation"),
            ("model <name>", "Switch AI model"),
            ("agents", "Show agent swarm status"),
            ("diff", "Show current git diff"),
            ("file <path>", "Attach a file to conversation"),
            ("pane <name>", "Switch to a named pane"),
            ("status", "Show daemon status"),
            ("exit", "Exit the TUI"),
            ("resume", "Resume previous session"),
            ("save", "Save current session"),
        ]
    }

    /// Human-readable description of this command.
    pub fn describe(&self) -> &'static str {
        match self {
            Self::Help => "Show available commands",
            Self::Compact => "Free context budget",
            Self::Clear => "Clear conversation",
            Self::Model(_) => "Switch AI model",
            Self::Agents => "Agent swarm status",
            Self::Diff => "Git working-tree diff",
            Self::File(_) => "Attach file",
            Self::Pane(_) => "Switch pane",
            Self::Status => "Daemon status",
            Self::Exit => "Exit TUI",
            Self::Resume => "Resume session",
            Self::Save => "Save session",
        }
    }
}

/// Check if text after prefix (`/` or `@`) could be a filesystem path.
///
/// Any text that isn't an exact known slash command is treated as a potential
/// path — the path completer will determine if actual filesystem matches exist.
/// This handles both `@src` (simple names) and `/src/main.rs` (slashed paths).
fn could_be_path(after_prefix: &str) -> bool {
    if after_prefix.is_empty() {
        return false;
    }
    // Known slash commands that should NOT trigger path completion
    let known_commands = ["help", "h", "?", "compact", "cmp", "clear", "cls", "reset",
        "model", "m", "agents", "ag", "swarm", "diff", "d",
        "file", "f", "attach", "pane", "p", "status", "st",
        "exit", "quit", "q", "resume", "res", "save", "s"];
    let first_word = after_prefix.split_whitespace().next().unwrap_or(after_prefix);
    // If it's a known command (without args), don't treat as path
    if known_commands.contains(&first_word) && !after_prefix.contains(' ') {
        return false;
    }
    true
}

/// Structured data for rendering a slash command popup menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashPopup {
    pub visible: bool,
    pub title: String,
    pub items: Vec<SlashPopupItem>,
    pub selected: usize,
    pub filter_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashPopupItem {
    pub label: String,
    pub description: String,
    pub shortcut: Option<String>,
}

impl SlashPopup {
    /// Build a popup from the current filter text.
    pub fn build(filter: &str) -> Self {
        let lower = filter.to_lowercase();
        let items: Vec<SlashPopupItem> = SlashCommand::completions()
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&lower))
            .map(|(name, desc)| SlashPopupItem {
                label: format!("/{name}"),
                description: desc.to_string(),
                shortcut: {
                    let first = name.chars().next().map(|c| c.to_string());
                    first
                },
            })
            .collect();
        Self {
            visible: true,
            title: "Commands".into(),
            items,
            selected: 0,
            filter_text: filter.to_string(),
        }
    }
}

/// Filter slash command completions matching a prefix.
fn match_slash_commands(prefix: &str) -> Vec<String> {
    let lower = prefix.to_lowercase();
    SlashCommand::completions()
        .iter()
        .filter(|(name, _)| name.to_lowercase().starts_with(&lower))
        .map(|(name, desc)| format!("/{name} — {desc}"))
        .collect()
}

// ---------------------------------------------------------------------------
// Session checkpoints
// ---------------------------------------------------------------------------

/// A named save point in a session that can be restored later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// User-provided or auto-generated name.
    pub name: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// The input buffer state at checkpoint time.
    pub input_snapshot: String,
    /// Number of pending tasks at checkpoint time.
    pub pending_tasks: usize,
    /// The active pane at checkpoint time.
    pub active_pane: Pane,
    /// User-provided description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Checkpoint {
    /// Create a new checkpoint from the current UiState.
    pub fn capture(state: &UiState, name: &str, description: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            created_at: chrono_now(),
            input_snapshot: state.input_buffer.clone(),
            pending_tasks: state.pending_tasks,
            active_pane: state.active_pane,
            description: description.map(|s| s.to_string()),
            tags: Vec::new(),
        }
    }

    /// Restore this checkpoint into the given UiState.
    pub fn restore(&self, state: &mut UiState) {
        state.input_buffer = self.input_snapshot.clone();
        state.pending_tasks = self.pending_tasks;
        state.active_pane = self.active_pane;
        state.path_completer = None;
    }

    /// Short display label.
    pub fn display(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("");
        if desc.is_empty() {
            format!("{} ({} tasks, pane {})", self.name, self.pending_tasks, self.active_pane.label())
        } else {
            format!("{} — {} ({} tasks)", self.name, desc, self.pending_tasks)
        }
    }
}

/// Returns an ISO-8601 timestamp for the current moment.
fn chrono_now() -> String {
    // Minimal timestamp without pulling in chrono — use UTC seconds since epoch
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple ISO-8601: YYYY-MM-DDTHH:MM:SSZ
    let days = secs / 86400;
    let time = secs % 86400;
    let hours = time / 3600;
    let mins = (time % 3600) / 60;
    let secs_rem = time % 60;
    // Days since Unix epoch → approximate date (simplified, good enough for display)
    let year = 1970 + (days / 365) as u64;
    let doy = days % 365;
    let month = (doy / 30 + 1).min(12);
    let day = (doy % 30 + 1).min(31);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{secs_rem:02}Z")
}

// ---------------------------------------------------------------------------
// Terminal capabilities
// ---------------------------------------------------------------------------

/// Detected terminal capabilities for feature-gating advanced rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCaps {
    /// Terminal supports true color (24-bit).
    pub true_color: bool,
    /// Terminal supports kitty keyboard protocol.
    pub kitty_keyboard: bool,
    /// Terminal supports Sixel image protocol.
    pub sixel: bool,
    /// Terminal supports iTerm2 inline images.
    pub iterm2_images: bool,
    /// Terminal supports OSC 52 clipboard read/write.
    pub osc52_clipboard: bool,
    /// Terminal supports bracketed paste.
    pub bracketed_paste: bool,
    /// Terminal supports burst-paste detection (always true; we always try).
    pub burst_paste: bool,
    /// Terminal width in cells.
    pub width: u16,
    /// Terminal height in cells.
    pub height: u16,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            true_color: false,
            kitty_keyboard: false,
            sixel: false,
            iterm2_images: false,
            osc52_clipboard: true,   // most modern terminals support this
            bracketed_paste: true,    // enabled by default
            burst_paste: true,        // always attempt burst detection fallback
            width: 80,
            height: 24,
        }
    }
}

impl TerminalCaps {
    /// Best image protocol available, if any.
    pub fn image_protocol(&self) -> Option<&'static str> {
        if self.sixel { Some("sixel") }
        else if self.iterm2_images { Some("iterm2") }
        else if self.kitty_keyboard { Some("kitty") }
        else { None }
    }

    /// Whether any image display is possible.
    pub fn can_display_images(&self) -> bool {
        self.sixel || self.iterm2_images
    }

    /// Detect capabilities from environment variables.
    pub fn detect_from_env() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let ssh_client = std::env::var("SSH_CLIENT").unwrap_or_default();
        let _ssh_tty = std::env::var("SSH_TTY").unwrap_or_default();

        // Detect terminals known to lack bracketed paste support
        let has_bracketed = !term.eq_ignore_ascii_case("putty")
            && !term.eq_ignore_ascii_case("putty-256color")
            && !term_program.eq_ignore_ascii_case("putty")
            && !term.starts_with("screen")   // screen multiplexer doesn't support it
            && !term.starts_with("tmux");     // tmux needs explicit config for bracketed paste

        // Detect PuTTY via SSH client string or TERM
        let is_putty = term.starts_with("putty")
            || term_program.eq_ignore_ascii_case("putty")
            || (!ssh_client.is_empty() && term == "xterm"); // heuristic: SSH xterm often = PuTTY

        Self {
            true_color: colorterm == "truecolor" || colorterm == "24bit"
                || term.contains("256color") || term.contains("truecolor"),
            kitty_keyboard: std::env::var("KITTY_KEYBOARD").is_ok(),
            sixel: term.contains("sixel") || term_program == "iTerm.app",
            iterm2_images: std::env::var("ITERM_SESSION_ID").is_ok()
                || term_program.contains("iTerm"),
            osc52_clipboard: !is_putty,  // PuTTY doesn't support OSC 52
            bracketed_paste: has_bracketed,
            burst_paste: true,           // always attempt burst detection
            width: 80,
            height: 24,
        }
    }
}

// ---------------------------------------------------------------------------
// Path autocompletion
// ---------------------------------------------------------------------------

/// Tracks path autocompletion state when the user types a `/` in the input.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathCompleter {
    pub partial: String,
    pub candidates: Vec<String>,
    pub selected: usize,
    pub active: bool,
}

impl PathCompleter {
    pub fn new(partial: &str) -> Self {
        let mut pc = Self { partial: partial.to_string(), candidates: Vec::new(), selected: 0, active: true };
        pc.refresh_candidates();
        pc
    }
    pub fn refresh_candidates(&mut self) {
        self.candidates = complete_path(&self.partial);
        self.selected = 0;
    }
    pub fn push_char(&mut self, ch: char) {
        self.partial.push(ch);
        self.refresh_candidates();
    }
    pub fn pop_char(&mut self) {
        self.partial.pop();
        self.refresh_candidates();
    }
    pub fn current_candidate(&self) -> Option<&str> {
        self.candidates.get(self.selected).map(|s| s.as_str())
    }
    pub fn next(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = (self.selected + 1) % self.candidates.len();
        }
    }
    pub fn deactivate(&mut self) {
        self.active = false;
        self.candidates.clear();
    }
}

fn complete_path(partial: &str) -> Vec<String> {
    let (dir, prefix) = if let Some(last_slash) = partial.rfind('/') {
        (partial[..=last_slash].to_string(), partial[last_slash + 1..].to_string())
    } else {
        (".".to_string(), partial.to_string())
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&prefix) && !name_str.starts_with('.') {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir {
                    dirs.push(format!("{name_str}/"));
                } else {
                    files.push(name_str.to_string());
                }
            }
        }
    }
    dirs.sort();
    files.sort();
    dirs.extend(files);
    dirs
}

// ---------------------------------------------------------------------------
// UI State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub active_pane: Pane,
    pub paused: bool,
    /// Accumulated input characters (for path detection and completion).
    #[serde(default)]
    pub input_buffer: String,
    /// Active path autocompletion state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_completer: Option<PathCompleter>,
    pub last_response_delta: Option<String>,
    pub active_tool: Option<String>,
    pub pending_tasks: usize,
    pub active_jobs: usize,
    pub pending_approvals: usize,
    pub status_line: String,
    #[serde(default)]
    pub keybindings: Keybindings,
    /// Monotonically increasing event counter (for debugging).
    #[serde(default)]
    pub event_count: u64,
    /// Context budget tracker for token usage warnings.
    #[serde(default)]
    pub context_budget: ContextBudget,
    /// Detected terminal capabilities.
    #[serde(default)]
    pub terminal_caps: TerminalCaps,
    /// Saved session checkpoints (most recent first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<Checkpoint>,
    /// Pending user input to inject into the agent's execution stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_pushback: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_pane: Pane::Chat,
            paused: false,
            input_buffer: String::new(),
            path_completer: None,
            last_response_delta: None,
            active_tool: None,
            pending_tasks: 0,
            active_jobs: 0,
            pending_approvals: 0,
            status_line: "ready".to_string(),
            keybindings: Keybindings::default(),
            event_count: 0,
            context_budget: ContextBudget::default(),
            terminal_caps: TerminalCaps::default(),
            checkpoints: Vec::new(),
            pending_pushback: None,
        }
    }
}

impl UiState {
    /// Reduce an event into effects. Uses stack-allocated SmallVec to avoid
    /// heap allocation for the common case (≤ 4 effects).
    #[must_use]
    pub fn reduce(&mut self, event: UiEvent) -> EffectVec {
        self.event_count = self.event_count.wrapping_add(1);

        match event {
            UiEvent::KeyPressed(key) => {
                // Handle literal newline (multi-line input)
                if key == '\n' {
                    self.input_buffer.push('\n');
                    if let Some(ref mut pc) = self.path_completer {
                        pc.deactivate();
                    }
                    self.status_line = "newline".into();
                    return smallvec![UiEffect::Render, UiEffect::InsertNewline];
                }

                // Track input for path detection
                self.input_buffer.push(key);

                // Detect path/command prefix: `/` or `@` at word boundary
                // `/` = slash command or filesystem path
                // `@` = file attachment path
                if key == '/' || key == '@' {
                    let prev_is_boundary = self.input_buffer.len() <= 1
                        || self.input_buffer.as_bytes().get(self.input_buffer.len().wrapping_sub(2))
                            .map_or(true, |&b| b == b' ' || b == b'\n');
                    if prev_is_boundary {
                        self.path_completer = None; // clear any stale path completer
                        if key == '/' {
                            // Show slash command suggestions initially for `/`
                            let popup = SlashPopup::build("");
                            return smallvec![
                                UiEffect::Render,
                                UiEffect::ShowSlashPopup(popup),
                            ];
                        } else {
                            // `@` starts path completion immediately
                            return smallvec![
                                UiEffect::Render,
                            ];
                        }
                    }
                }

                // If we just typed after `/` or `@`, update suggestions
                let prefix_char = self.input_buffer.rfind('/')
                    .or_else(|| self.input_buffer.rfind('@'));
                if let Some(prefix_idx) = prefix_char {
                    let after_prefix = &self.input_buffer[prefix_idx + 1..];
                    let is_boundary = prefix_idx == 0
                        || self.input_buffer.as_bytes().get(prefix_idx.wrapping_sub(1))
                            .map_or(true, |&b| b == b' ' || b == b'\n');
                    if is_boundary && !after_prefix.is_empty() {
                        let is_slash = self.input_buffer.as_bytes()[prefix_idx] == b'/';
                        if could_be_path(after_prefix) {
                            // Try path completion first
                            if let Some(ref mut pc) = self.path_completer {
                                if pc.active {
                                    pc.push_char(key);
                                    return smallvec![
                                        UiEffect::Render,
                                        UiEffect::ShowPathSuggestions(pc.candidates.clone()),
                                    ];
                                }
                            }
                            let pc = PathCompleter::new(after_prefix);
                            let has_candidates = !pc.candidates.is_empty();
                            self.path_completer = Some(pc);
                            if has_candidates {
                                let candidates = self.path_completer.as_ref().unwrap().candidates.clone();
                                return smallvec![
                                    UiEffect::Render,
                                    UiEffect::ShowPathSuggestions(candidates),
                                ];
                            }
                            // No filesystem matches — for `/`, fall back to slash command popup
                            if is_slash {
                                let popup = SlashPopup::build(after_prefix);
                                return smallvec![
                                    UiEffect::Render,
                                    UiEffect::ShowSlashPopup(popup),
                                ];
                            }
                            // For `@`, just render (no suggestions if nothing matches)
                            return smallvec![UiEffect::Render];
                        } else if is_slash {
                            // Known command — show command suggestions
                            let popup = SlashPopup::build(after_prefix);
                            return smallvec![
                                UiEffect::Render,
                                UiEffect::ShowSlashPopup(popup),
                            ];
                        }
                    }
                }

                // If in path completion mode, update completer
                if let Some(ref mut pc) = self.path_completer {
                    if pc.active {
                        pc.push_char(key);
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::ShowPathSuggestions(pc.candidates.clone()),
                        ];
                    }
                }

                // Normal key handling
                if let Some(pane) = self.keybindings.pane_for_key(key) {
                    self.active_pane = pane;
                    smallvec![UiEffect::Render]
                } else {
                    smallvec![]
                }
            }
            UiEvent::PromptSubmitted(prompt) => {
                // Check if this is a slash command or file attachment
                let trimmed = prompt.trim();
                if let Some(after_prefix) = trimmed.strip_prefix('/') {
                    if let Some(cmd) = SlashCommand::parse(after_prefix) {
                        self.status_line = format!("slash command: {}", cmd.describe());
                        self.input_buffer.clear();
                        self.path_completer = None;
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::ExecuteSlashCommand(cmd),
                            UiEffect::EmitStatusLine(self.status_line.clone()),
                        ];
                    }
                    // Non-command `/` — treat as path prompt
                }
                if let Some(path) = trimmed.strip_prefix('@') {
                    if !path.is_empty() {
                        self.status_line = format!("attached file: {path}");
                        self.input_buffer.clear();
                        self.path_completer = None;
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::AttachFile(path.to_string()),
                            UiEffect::EmitStatusLine(self.status_line.clone()),
                        ];
                    }
                }
                // ── Mid-execution pushback detection ────────────────────────
                if self.active_tool.is_some() || self.pending_tasks > 0 {
                    let pushback = trimmed.to_string();
                    self.pending_pushback = Some(pushback.clone());
                    self.status_line = format!(
                        "pushback: {}…",
                        &pushback[..pushback.len().min(50)]
                    );
                    self.input_buffer.clear();
                    self.path_completer = None;
                    return smallvec![
                        UiEffect::Render,
                        UiEffect::PushbackDraft(pushback),
                        UiEffect::EmitStatusLine(self.status_line.clone()),
                    ];
                }
                // ── Normal prompt submission ──────────────────────────────────
                self.pending_tasks = self.pending_tasks.saturating_add(1);
                self.status_line = "prompt submitted".into();
                self.input_buffer.clear();
                self.path_completer = None;
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ResponseDelta(delta) => {
                self.last_response_delta = Some(delta);
                self.status_line = "streaming response".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ToolStarted(name) => {
                self.active_tool = Some(name.clone());
                self.status_line = format!("tool running: {name}");
                let mut effects = smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ];
                // Flush any pending pushback so agent sees it before tool results
                if let Some(pushback) = self.pending_pushback.take() {
                    effects.push(UiEffect::PushbackDraft(pushback));
                }
                effects
            }
            UiEvent::ToolFinished(name) => {
                self.active_tool = None;
                self.pending_tasks = self.pending_tasks.saturating_sub(1);
                self.status_line = format!("tool finished: {name}");
                let mut effects = smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ];
                // Flush any pending pushback so the agent rethinks before next tool
                if let Some(pushback) = self.pending_pushback.take() {
                    effects.push(UiEffect::PushbackDraft(pushback));
                }
                effects
            }
            UiEvent::JobQueued(_) => {
                self.active_jobs = self.active_jobs.saturating_add(1);
                self.status_line = "job queued".into();
                smallvec![UiEffect::Render, UiEffect::PersistCheckpoint]
            }
            UiEvent::JobProgress { progress, .. } => {
                // Clamp progress to [0, 100] instead of just min(100).
                let p = progress.clamp(0, 100);
                self.status_line = format!("job progress: {p}%");
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::JobCompleted(_) => {
                self.active_jobs = self.active_jobs.saturating_sub(1);
                self.status_line = "job completed".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ApprovalRequested(_) => {
                self.pending_approvals = self.pending_approvals.saturating_add(1);
                self.status_line = "approval requested".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ApprovalResolved(_) => {
                self.pending_approvals = self.pending_approvals.saturating_sub(1);
                self.status_line = "approval resolved".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::PauseRequested => {
                self.paused = true;
                self.status_line = "paused".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ResumeRequested => {
                self.paused = false;
                self.status_line = "resumed".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::PasteStart => {
                self.status_line = "pasting…".into();
                smallvec![UiEffect::EmitStatusLine(self.status_line.clone())]
            }
            UiEvent::PasteEnd => {
                // PasteEnd alone without buffered content is a no-op;
                // the buffered content arrives as PasteContent via the buffer.
                smallvec![]
            }
            UiEvent::PasteContent { ref content, ref detected_type, ref raw_bytes } => {
                let line_count = content.lines().count();
                let byte_size = raw_bytes.as_ref().map(|b| b.len()).unwrap_or(content.len());

                // Folded paste notation like Claude Code: [Pasted text N lines, X KB]
                let folded = match detected_type {
                    PasteContentType::Image { mime_type, .. } => {
                        let kb = byte_size as f64 / 1024.0;
                        format!("[Pasted {mime_type} image, {kb:.1} KB]")
                    }
                    PasteContentType::Code { language } => {
                        let lang = language.as_deref().unwrap_or("code");
                        if line_count > 3 {
                            format!("[Pasted {lang} snippet, {line_count} lines, {} B]", byte_size)
                        } else {
                            format!("[Pasted {lang}]: {}", &content[..60.min(content.len())])
                        }
                    }
                    PasteContentType::Link { url } => {
                        format!("[Pasted link]: {url}")
                    }
                    _ => {
                        if line_count > 3 || byte_size > 200 {
                            let kb = byte_size as f64 / 1024.0;
                            if kb >= 1.0 {
                                format!("[Pasted text, {line_count} lines, {kb:.1} KB]")
                            } else {
                                format!("[Pasted text, {line_count} lines, {byte_size} B]")
                            }
                        } else {
                            format!("[Pasted]: {}", &content[..60.min(content.len())])
                        }
                    }
                };
                self.status_line = folded;
                self.pending_tasks = self.pending_tasks.saturating_add(1);
                match detected_type {
                    PasteContentType::Link { .. } | PasteContentType::Code { .. } => {
                        self.active_pane = Pane::Chat;
                    }
                    PasteContentType::Image { .. } => {
                        self.active_pane = Pane::Chat;
                    }
                    _ => {}
                }
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ContextWarning => {
                let pct = self.context_budget.pct_remaining() * 100.0;
                self.status_line = format!("context warning: {pct:.0}% remaining");
                let msg = format!(
                    "Context budget warning: only {pct:.0}% of the context window remains ({}/{} tokens used).",
                    self.context_budget.used_tokens,
                    self.context_budget.total_tokens,
                );
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitContextWarning(msg),
                ]
            }
            UiEvent::ContextCritical => {
                let pct = self.context_budget.pct_remaining() * 100.0;
                self.status_line = format!("CONTEXT CRITICAL: {pct:.0}% remaining");
                let msg = format!(
                    "Context budget CRITICAL: only {pct:.0}% of the context window remains ({}/{} tokens used).",
                    self.context_budget.used_tokens,
                    self.context_budget.total_tokens,
                );
                smallvec![
                    UiEffect::Render,
                    UiEffect::EmitContextWarning(msg),
                ]
            }
            // Tab now triggers ONLY completion (Ctrl+Tab cycles mode).
            // The renderer intercepts CtrlTabPressed for mode cycling.
            UiEvent::TabPressed | UiEvent::CompletionTrigger => {
                // First check path completer
                if let Some(ref mut pc) = self.path_completer {
                    if pc.active {
                        if let Some(candidate) = pc.current_candidate() {
                            let completed = candidate.to_string();
                            pc.deactivate();
                            self.status_line = format!("completed path: {completed}");
                            return smallvec![
                                UiEffect::Render,
                                UiEffect::CompletePath(completed),
                                UiEffect::EmitStatusLine(self.status_line.clone()),
                            ];
                        }
                    }
                }
                // Check if we're in a slash-command (`/`) or file-attach (`@`) context
                let prefix_char = self.input_buffer.rfind('/')
                    .or_else(|| self.input_buffer.rfind('@'));
                if let Some(prefix_idx) = prefix_char {
                    let after_prefix = &self.input_buffer[prefix_idx + 1..];
                    let is_boundary = prefix_idx == 0
                        || self.input_buffer.as_bytes().get(prefix_idx.wrapping_sub(1))
                            .map_or(true, |&b| b == b' ' || b == b'\n');
                    if is_boundary {
                        let is_slash = self.input_buffer.as_bytes()[prefix_idx] == b'/';

                        // Try path completion first
                        if could_be_path(after_prefix) {
                            let pc = PathCompleter::new(after_prefix);
                            if let Some(candidate) = pc.current_candidate() {
                                let completed = candidate.to_string();
                                self.status_line = format!("completed path: {completed}");
                                self.input_buffer = {
                                    let prefix = &self.input_buffer[..=prefix_idx];
                                    format!("{prefix}{completed}")
                                };
                                self.path_completer = None;
                                return smallvec![
                                    UiEffect::Render,
                                    UiEffect::CompletePath(completed),
                                    UiEffect::EmitStatusLine(self.status_line.clone()),
                                ];
                            }
                            // No path matches — if `/`, fall through to command completion
                            if !is_slash {
                                return smallvec![];
                            }
                        }

                        // Slash command completion (only for `/`)
                        if is_slash {
                            let completions = SlashCommand::completions();
                            let lower = after_prefix.to_lowercase();
                            let matching: Vec<&str> = completions
                                .iter()
                                .filter(|(name, _)| name.to_lowercase().starts_with(&lower))
                                .map(|(name, _)| *name)
                                .collect();
                            if let Some(&first) = matching.first() {
                                let completed = format!("/{first} ");
                                self.status_line = format!("command: {first}");
                                self.input_buffer = {
                                    let prefix = &self.input_buffer[..=prefix_idx];
                                    format!("{prefix}{first} ")
                                };
                                return smallvec![
                                    UiEffect::Render,
                                    UiEffect::CompletePath(completed),
                                    UiEffect::EmitStatusLine(self.status_line.clone()),
                                ];
                            }
                        }
                    }
                }
                smallvec![]
            }
            UiEvent::CtrlTabPressed => {
                self.status_line = "mode toggled".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::ToggleMode,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::BackspacePressed => {
                self.input_buffer.pop();
                if let Some(ref mut pc) = self.path_completer {
                    if pc.active {
                        if pc.partial.is_empty() {
                            pc.deactivate();
                            return smallvec![UiEffect::Render];
                        }
                        pc.pop_char();
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::ShowPathSuggestions(pc.candidates.clone()),
                        ];
                    }
                }
                smallvec![]
            }
            UiEvent::CyclePathCompletion => {
                if let Some(ref mut pc) = self.path_completer {
                    if pc.active {
                        pc.next();
                        if let Some(candidate) = pc.current_candidate() {
                            self.status_line = format!("path: {candidate}");
                            return smallvec![
                                UiEffect::Render,
                                UiEffect::ShowPathSuggestions(pc.candidates.clone()),
                                UiEffect::EmitStatusLine(self.status_line.clone()),
                            ];
                        }
                    }
                }
                smallvec![]
            }
            UiEvent::SlashCommand(cmd) => {
                self.status_line = format!("executing: {}", cmd.describe());
                self.input_buffer.clear();
                self.path_completer = None;
                let mut effects = smallvec![UiEffect::Render, UiEffect::ExecuteSlashCommand(cmd.clone())];
                match &cmd {
                    SlashCommand::Help => {
                        let help_text = SlashCommand::completions()
                            .iter()
                            .map(|(n, d)| format!("/{n} — {d}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        self.status_line = format!("commands available:\n{help_text}");
                    }
                    SlashCommand::Clear => {
                        self.status_line = "conversation cleared".into();
                        self.pending_tasks = 0;
                        self.pending_approvals = 0;
                        self.active_jobs = 0;
                    }
                    SlashCommand::Diff => {
                        self.status_line = "fetching git diff…".into();
                        self.active_pane = Pane::Diff;
                        effects.push(UiEffect::RequestGitDiff);
                    }
                    SlashCommand::Agents => {
                        self.status_line = "showing agent swarm…".into();
                        self.active_pane = Pane::Agents;
                    }
                    SlashCommand::Status => {
                        self.status_line = "showing daemon status…".into();
                        self.active_pane = Pane::Status;
                    }
                    SlashCommand::Exit => {
                        self.status_line = "exiting…".into();
                    }
                    SlashCommand::Save => {
                        let cp = Checkpoint::capture(self, "auto-save", Some("slash-command save"));
                        let name = cp.name.clone();
                        self.checkpoints.push(cp);
                        self.status_line = format!("checkpoint saved: {name}");
                        effects.push(UiEffect::CheckpointChanged { name, action: "saved".into() });
                    }
                    SlashCommand::Resume => {
                        if let Some(cp) = self.checkpoints.last().cloned() {
                            let name = cp.name.clone();
                            cp.restore(self);
                            self.status_line = format!("restored checkpoint: {name}");
                            effects.push(UiEffect::CheckpointChanged { name, action: "restored".into() });
                        } else {
                            self.status_line = "no checkpoints to restore".into();
                        }
                    }
                    _ => {}
                }
                effects.push(UiEffect::EmitStatusLine(self.status_line.clone()));
                effects
            }
            UiEvent::EnterPressed => {
                // Submit the current input buffer as a prompt
                let prompt = std::mem::take(&mut self.input_buffer);
                if prompt.trim().is_empty() {
                    return smallvec![];
                }
                // Check for slash command or file attachment
                if let Some(after_prefix) = prompt.trim().strip_prefix('/') {
                    if let Some(cmd) = SlashCommand::parse(after_prefix) {
                        self.status_line = format!("slash command: {}", cmd.describe());
                        self.path_completer = None;
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::ExecuteSlashCommand(cmd),
                            UiEffect::EmitStatusLine(self.status_line.clone()),
                        ];
                    }
                    // Non-command `/` — treat as path prompt (falls through)
                }
                if let Some(path) = prompt.trim().strip_prefix('@') {
                    if !path.is_empty() {
                        self.status_line = format!("attached file: {path}");
                        self.path_completer = None;
                        return smallvec![
                            UiEffect::Render,
                            UiEffect::AttachFile(path.to_string()),
                            UiEffect::EmitStatusLine(self.status_line.clone()),
                        ];
                    }
                }
                // ── Mid-execution pushback detection ────────────────────────
                // If a tool is running or tasks are pending, inject the text
                // into the agent's context instead of starting a new prompt.
                if self.active_tool.is_some() || self.pending_tasks > 0 {
                    let pushback = prompt.trim().to_string();
                    self.pending_pushback = Some(pushback.clone());
                    self.status_line = format!(
                        "pushback: {}…",
                        &pushback[..pushback.len().min(50)]
                    );
                    self.path_completer = None;
                    return smallvec![
                        UiEffect::Render,
                        UiEffect::PushbackDraft(pushback),
                        UiEffect::EmitStatusLine(self.status_line.clone()),
                    ];
                }
                // ── Normal prompt submission ──────────────────────────────────
                self.pending_tasks = self.pending_tasks.saturating_add(1);
                self.status_line = "prompt submitted".into();
                self.path_completer = None;
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::ShiftEnterPressed => {
                self.input_buffer.push('\n');
                if let Some(ref mut pc) = self.path_completer {
                    pc.deactivate();
                }
                self.status_line = "multi-line mode".into();
                smallvec![
                    UiEffect::Render,
                    UiEffect::InsertNewline,
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::AgentSpawned { ref id, ref role, ref name } => {
                self.status_line = format!("agent spawned: {name} ({role})");
                self.active_jobs = self.active_jobs.saturating_add(1);
                smallvec![
                    UiEffect::Render,
                    UiEffect::AgentStatusChanged { id: id.clone(), status: "spawned".into() },
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::AgentCompleted { ref id, ref summary } => {
                self.status_line = format!("agent completed: {summary}");
                self.active_jobs = self.active_jobs.saturating_sub(1);
                self.pending_tasks = self.pending_tasks.saturating_sub(1);
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::AgentStatusChanged { id: id.clone(), status: "completed".into() },
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::AgentErrored { ref id, ref error } => {
                self.status_line = format!("agent error: {error}");
                self.active_jobs = self.active_jobs.saturating_sub(1);
                smallvec![
                    UiEffect::Render,
                    UiEffect::AgentStatusChanged { id: id.clone(), status: "errored".into() },
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::UserPushback(ref pushback) => {
                self.pending_pushback = Some(pushback.clone());
                self.status_line = format!("pushback: {}…", &pushback[..pushback.len().min(50)]);
                smallvec![
                    UiEffect::Render,
                    UiEffect::PushbackDraft(pushback.clone()),
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::AgentHeartbeat { ref id } => {
                smallvec![
                    UiEffect::AgentStatusChanged { id: id.clone(), status: "heartbeat".into() },
                ]
            }
            UiEvent::CaptureCheckpoint { ref name, ref description } => {
                let cp = Checkpoint::capture(self, name, description.as_deref());
                let name = cp.name.clone();
                self.checkpoints.push(cp);
                self.status_line = format!("checkpoint captured: {name}");
                smallvec![
                    UiEffect::Render,
                    UiEffect::PersistCheckpoint,
                    UiEffect::CheckpointChanged { name, action: "captured".into() },
                    UiEffect::EmitStatusLine(self.status_line.clone()),
                ]
            }
            UiEvent::RestoreCheckpoint { ref name } => {
                if let Some(idx) = self.checkpoints.iter().position(|c| c.name == *name) {
                    let cp = self.checkpoints[idx].clone();
                    cp.restore(self);
                    self.status_line = format!("restored checkpoint: {name}");
                    smallvec![
                        UiEffect::Render,
                        UiEffect::CheckpointChanged { name: name.clone(), action: "restored".into() },
                        UiEffect::EmitStatusLine(self.status_line.clone()),
                    ]
                } else {
                    self.status_line = format!("checkpoint not found: {name}");
                    smallvec![
                        UiEffect::Render,
                        UiEffect::EmitStatusLine(self.status_line.clone()),
                    ]
                }
            }
            UiEvent::Tick => {
                let gauge = self.context_budget.gauge_bar(20);
                let label = self.context_budget.gauge_label().to_string();
                let detail = self.context_budget.gauge_detail();
                let zone = self.context_budget.zone();
                smallvec![
                    UiEffect::ScheduleBackgroundRefresh,
                    UiEffect::UpdateContextGauge { bar: gauge, label, detail, zone },
                ]
            }
        }
    }

    /// Machine-parseable snapshot for logging and debugging.
    #[must_use]
    pub fn snapshot(&self) -> String {
        format!(
            "pane={};paused={};pending_tasks={};active_jobs={};pending_approvals={};active_tool={};status={};events={}",
            self.active_pane.label().to_lowercase(),
            self.paused,
            self.pending_tasks,
            self.active_jobs,
            self.pending_approvals,
            self.active_tool.as_deref().unwrap_or(""),
            self.status_line,
            self.event_count,
        )
    }

    /// Serialize full state to JSON for checkpointing.
    ///
    /// Returns an error if serialization fails (shouldn't happen in practice).
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize full state from JSON.
    pub fn from_json(json: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ---------------------------------------------------------------------------
// Paste content detection
// ---------------------------------------------------------------------------

/// Detect content type from raw pasted text.
#[must_use]
pub fn detect_paste_content(text: &str) -> PasteContentType {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return PasteContentType::Text;
    }

    // Check for base64-encoded image (data:image/...;base64,...)
    if let Some(image_type) = looks_like_base64_image(trimmed) {
        return image_type;
    }

    // Check for URL
    if looks_like_url(trimmed) {
        return PasteContentType::Link { url: trimmed.to_string() };
    }

    // Check for code (heuristic: multiple lines with indentation, or common code patterns)
    if looks_like_code(trimmed) {
        let lang = guess_language(trimmed);
        return PasteContentType::Code { language: lang };
    }

    PasteContentType::Text
}

/// Check if text looks like a base64-encoded image (data URI or raw base64).
fn looks_like_base64_image(s: &str) -> Option<PasteContentType> {
    // data:image/png;base64,iVBORw0KGgo...
    if s.starts_with("data:image/") {
        let mime_end = s.find(';').unwrap_or(s.len());
        let mime = &s[5..mime_end]; // strip "data:"
        let size_estimate = s.len().saturating_sub(mime_end).saturating_sub(12); // rough
        return Some(PasteContentType::Image {
            mime_type: mime.to_string(),
            size_bytes: size_estimate * 3 / 4, // base64 → bytes estimate
        });
    }
    // Raw base64 that looks like image: starts with image-specific base64 signatures
    // PNG: iVBORw0KGgo, JPEG: /9j/, GIF: R0lGOD, WebP: UklGR
    let b64_start = &s[..8.min(s.len())];
    if b64_start.starts_with("iVBORw0") {
        return Some(PasteContentType::Image {
            mime_type: "image/png".into(),
            size_bytes: s.len() * 3 / 4,
        });
    }
    if b64_start.starts_with("/9j/") {
        return Some(PasteContentType::Image {
            mime_type: "image/jpeg".into(),
            size_bytes: s.len() * 3 / 4,
        });
    }
    if b64_start.starts_with("R0lGOD") {
        return Some(PasteContentType::Image {
            mime_type: "image/gif".into(),
            size_bytes: s.len() * 3 / 4,
        });
    }
    None
}

/// Detect content type from raw bytes (binary/image paste).
#[must_use]
pub fn detect_paste_bytes(data: &[u8]) -> PasteContentType {
    if data.is_empty() {
        return PasteContentType::Text;
    }

    // Check for image magic bytes
    if data.len() >= 8 {
        if &data[0..4] == b"\x89PNG" {
            return PasteContentType::Image {
                mime_type: "image/png".into(),
                size_bytes: data.len(),
            };
        }
        if &data[0..3] == b"\xff\xd8\xff" {
            return PasteContentType::Image {
                mime_type: "image/jpeg".into(),
                size_bytes: data.len(),
            };
        }
        if &data[0..4] == b"GIF8" {
            return PasteContentType::Image {
                mime_type: "image/gif".into(),
                size_bytes: data.len(),
            };
        }
        if &data[0..4] == b"RIFF" && data.len() >= 12 && &data[8..12] == b"WEBP" {
            return PasteContentType::Image {
                mime_type: "image/webp".into(),
                size_bytes: data.len(),
            };
        }
    }

    // Try interpreting as UTF-8 text
    if let Ok(text) = std::str::from_utf8(data) {
        return detect_paste_content(text);
    }

    PasteContentType::Text
}

fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://")
        || (s.contains("://") && !s.contains(' ') && s.len() < 2048)
}

fn looks_like_code(s: &str) -> bool {
    // Shebang line
    if s.starts_with("#!") {
        return true;
    }
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() < 2 {
        // Single line could be JSON
        if looks_like_json(s) || looks_like_xml(s) || looks_like_diff(s) {
            return true;
        }
        return false;
    }
    let indented = lines.iter().filter(|l| l.starts_with(' ') || l.starts_with('\t')).count();
    let has_braces = s.contains('{') && s.contains('}');
    let has_semicolons = s.matches(';').count() >= 2;
    let has_keywords = s.contains("fn ") || s.contains("def ") || s.contains("function ")
        || s.contains("class ") || s.contains("import ") || s.contains("use ")
        || s.contains("let ") || s.contains("const ") || s.contains("var ")
        || s.contains("pub ") || s.contains("mod ") || s.contains("impl ")
        || s.contains("func ") || s.contains("package ") || s.contains("require(");

    // Also detect JSON, XML, diff, TOML
    looks_like_json(s) || looks_like_xml(s) || looks_like_diff(s)
        || looks_like_toml(s)
        || indented >= 2 || has_braces || has_semicolons || has_keywords
}

fn looks_like_json(s: &str) -> bool {
    let trimmed = s.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

fn looks_like_xml(s: &str) -> bool {
    let trimmed = s.trim();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<!DOCTYPE")
        || (trimmed.starts_with('<') && trimmed.contains("</") && trimmed.ends_with('>'))
}

fn looks_like_diff(s: &str) -> bool {
    let has_diff_header = s.starts_with("diff ") || s.starts_with("--- ") || s.starts_with("+++ ");
    let has_hunks = s.contains("@@ -") || s.contains("@@ +");
    let has_markers = s.lines().any(|l| l.starts_with('+') || l.starts_with('-'))
        && !s.lines().all(|l| l.starts_with('+'));
    has_diff_header || has_hunks || has_markers
}

fn looks_like_toml(s: &str) -> bool {
    let has_section = s.lines().any(|l| l.trim().starts_with('[') && l.trim().ends_with(']'));
    let has_keyval = s.lines().any(|l| l.contains('=') && !l.contains("==") && !l.contains("!=") && !l.contains("<=") && !l.contains(">="));
    has_section && has_keyval
}

fn guess_language(s: &str) -> Option<String> {
    // Shebang detection
    if let Some(first_line) = s.lines().next() {
        if first_line.starts_with("#!") {
            if first_line.contains("python") || first_line.contains("python3") {
                return Some("python".into());
            }
            if first_line.contains("node") || first_line.contains("nodejs") {
                return Some("javascript".into());
            }
            if first_line.contains("bash") || first_line.contains("sh") {
                return Some("bash".into());
            }
            if first_line.contains("ruby") {
                return Some("ruby".into());
            }
            if first_line.contains("perl") {
                return Some("perl".into());
            }
        }
    }
    // Rust
    if (s.contains("fn ") && s.contains("->")) || (s.contains("use ") && s.contains("::"))
        || (s.contains("let mut ") && s.contains(';')) || s.contains("impl ") {
        return Some("rust".into());
    }
    // Go
    if s.contains("func ") && s.contains("error") || s.contains("package main") {
        return Some("go".into());
    }
    // TypeScript
    if s.contains("interface ") || s.contains(": string") || s.contains(": number")
        || (s.contains("export ") && s.contains("from ")) {
        return Some("typescript".into());
    }
    // Python
    if (s.contains("def ") && s.contains(':')) || (s.contains("import ") && !s.contains(';'))
        || s.contains("print(") {
        return Some("python".into());
    }
    // JavaScript
    if s.contains("function ") || s.contains("const ") && s.contains("=>") {
        return Some("javascript".into());
    }
    // Java
    if s.contains("package ") || s.contains("public class ") || s.contains("public static void") {
        return Some("java".into());
    }
    // C/C++
    if s.contains("#include") || s.contains("int main") {
        return Some("cpp".into());
    }
    // Ruby
    if s.contains("def ") && s.contains("end") || s.contains("require ") && !s.contains(';') {
        return Some("ruby".into());
    }
    // Shell
    if s.trim().starts_with("#!/") || (s.contains("echo ") && s.contains("$")) {
        return Some("bash".into());
    }
    // JSON
    if looks_like_json(s) {
        return Some("json".into());
    }
    // XML/HTML
    if looks_like_xml(s) {
        return Some("xml".into());
    }
    // TOML
    if looks_like_toml(s) {
        return Some("toml".into());
    }
    // Diff
    if looks_like_diff(s) {
        return Some("diff".into());
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(unused_must_use)]
    use super::*;

    #[test]
    fn reducer_produces_stable_snapshot_for_core_workflow() {
        let mut state = UiState::default();
        state.reduce(UiEvent::PromptSubmitted("hello".into()));
        state.reduce(UiEvent::ToolStarted("web.search".into()));
        state.reduce(UiEvent::ResponseDelta("partial".into()));
        state.reduce(UiEvent::ToolFinished("web.search".into()));
        state.reduce(UiEvent::ApprovalRequested("approval-1".into()));
        state.reduce(UiEvent::ApprovalResolved("approval-1".into()));
        state.reduce(UiEvent::JobQueued("job-1".into()));
        state.reduce(UiEvent::JobProgress {
            job_id: "job-1".into(),
            progress: 60,
        });
        state.reduce(UiEvent::JobCompleted("job-1".into()));
        state.reduce(UiEvent::KeyPressed('5'));

        assert_eq!(state.active_pane, Pane::Jobs);
        assert_eq!(
            state.snapshot(),
            "pane=jobs;paused=false;pending_tasks=0;active_jobs=0;pending_approvals=0;active_tool=;status=job completed;events=10"
        );
    }

    #[test]
    fn keybinding_lookup_maps_default_keys() {
        let kb = Keybindings::default();
        assert_eq!(kb.pane_for_key('1'), Some(Pane::Chat));
        assert_eq!(kb.pane_for_key('5'), Some(Pane::Jobs));
        assert_eq!(kb.pane_for_key('9'), None);
    }

    #[test]
    fn job_progress_clamps_to_range() {
        let mut state = UiState::default();
        state.reduce(UiEvent::JobProgress {
            job_id: "j".into(),
            progress: 150,
        });
        assert!(state.status_line.contains("100%"));
        state.reduce(UiEvent::JobProgress {
            job_id: "j".into(),
            progress: 0,
        });
        assert!(state.status_line.contains("0%"));
    }

    #[test]
    fn state_round_trips_through_json() {
        let mut state = UiState::default();
        state.reduce(UiEvent::PromptSubmitted("test".into()));
        state.reduce(UiEvent::ToolStarted("echo".into()));

        let json = state.to_json().expect("serialize");
        let restored = UiState::from_json(&json).expect("deserialize");

        assert_eq!(restored.active_pane, state.active_pane);
        assert_eq!(restored.pending_tasks, state.pending_tasks);
        assert_eq!(restored.status_line, state.status_line);
        assert_eq!(restored.event_count, state.event_count);
    }

    #[test]
    fn pane_labels_are_stable() {
        assert_eq!(Pane::Chat.label(), "Chat");
        assert_eq!(Pane::Diff.label(), "Diff");
        assert_eq!(Pane::Tasks.label(), "Tasks");
        assert_eq!(Pane::Agents.label(), "Agents");
        assert_eq!(Pane::Jobs.label(), "Jobs");
    }

    #[test]
    fn effect_vec_is_stack_allocated_for_common_cases() {
        // Verify SmallVec uses inline storage for ≤ 4 effects.
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PromptSubmitted("x".into()));
        assert_eq!(effects.len(), 3);
        assert!(!effects.spilled()); // still on stack
    }

    #[test]
    fn event_count_is_monotonic() {
        let mut state = UiState::default();
        assert_eq!(state.event_count, 0);
        state.reduce(UiEvent::Tick);
        assert_eq!(state.event_count, 1);
        state.reduce(UiEvent::Tick);
        state.reduce(UiEvent::Tick);
        assert_eq!(state.event_count, 3);
    }

    #[test]
    fn paste_text_is_detected() {
        let content = "hello world";
        let detected = PasteContentType::detect(content);
        assert_eq!(detected, PasteContentType::Text);
    }

    #[test]
    fn paste_url_is_detected() {
        let content = "https://github.com/DeepSeek-TUI/DeepSeek-TUI";
        let detected = PasteContentType::detect(content);
        assert!(matches!(detected, PasteContentType::Link { .. }));
    }

    #[test]
    fn paste_code_is_detected() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        let detected = PasteContentType::detect(content);
        assert!(matches!(detected, PasteContentType::Code { .. }));
    }

    #[test]
    fn paste_png_bytes_is_detected() {
        let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        let detected = PasteContentType::detect_bytes(png_header);
        assert!(matches!(detected, PasteContentType::Image { mime_type, .. } if mime_type == "image/png"));
    }

    #[test]
    fn paste_jpeg_bytes_is_detected() {
        let jpeg_header = b"\xff\xd8\xff\xe0\x00\x10JFIF";
        let detected = PasteContentType::detect_bytes(jpeg_header);
        assert!(matches!(detected, PasteContentType::Image { mime_type, .. } if mime_type == "image/jpeg"));
    }

    #[test]
    fn paste_event_updates_state() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PasteContent {
            content: "https://example.com".into(),
            raw_bytes: None,
            detected_type: PasteContentType::Link { url: "https://example.com".into() },
        });
        assert!(state.status_line.contains("Pasted link"));
        assert_eq!(state.pending_tasks, 1);
        assert!(!effects.is_empty());
    }

    // -- Bracketed paste buffer tests ---------------------------------

    #[test]
    fn paste_buffer_buffers_between_start_and_end() {
        let mut buf = BracketedPasteBuffer::new();
        assert!(!buf.is_active());

        assert!(buf.feed(UiEvent::PasteStart).is_none());
        assert!(buf.is_active());

        // These KeyPressed events should be buffered
        assert!(buf.feed(UiEvent::KeyPressed('h')).is_none());
        assert!(buf.feed(UiEvent::KeyPressed('i')).is_none());
        assert!(buf.feed(UiEvent::KeyPressed('\n')).is_none());

        // PasteEnd should emit PasteContent with detected type
        let result = buf.feed(UiEvent::PasteEnd);
        assert!(result.is_some());
        assert!(!buf.is_active());

        match result.unwrap() {
            UiEvent::PasteContent { content, detected_type, .. } => {
                assert_eq!(content, "hi\n");
                assert_eq!(detected_type, PasteContentType::Text);
            }
            other => panic!("expected PasteContent, got {other:?}"),
        }
    }

    #[test]
    fn paste_buffer_feeds_raw_bytes() {
        let mut buf = BracketedPasteBuffer::new();

        // Simulate terminal sending: \e[200~hello world\e[201~
        let mut input = vec![0x1b, b'[', b'2', b'0', b'0', b'~'];
        input.extend_from_slice(b"hello world");
        input.extend_from_slice(&[0x1b, b'[', b'2', b'0', b'1', b'~']);

        let events = buf.feed_bytes(&input);
        assert_eq!(events.len(), 1);

        match &events[0] {
            UiEvent::PasteContent { content, detected_type, .. } => {
                assert_eq!(content, "hello world");
                assert_eq!(*detected_type, PasteContentType::Text);
            }
            other => panic!("expected PasteContent, got {other:?}"),
        }
    }

    #[test]
    fn paste_buffer_detects_url_in_raw_bytes() {
        let mut buf = BracketedPasteBuffer::new();

        let url = b"https://github.com/rust-lang/rust";
        let mut input = vec![0x1b, b'[', b'2', b'0', b'0', b'~'];
        input.extend_from_slice(url);
        input.extend_from_slice(&[0x1b, b'[', b'2', b'0', b'1', b'~']);

        let events = buf.feed_bytes(&input);
        assert_eq!(events.len(), 1);

        match &events[0] {
            UiEvent::PasteContent { detected_type, .. } => {
                assert!(matches!(detected_type, PasteContentType::Link { .. }));
            }
            other => panic!("expected PasteContent with Link, got {other:?}"),
        }
    }

    #[test]
    fn paste_buffer_resets_correctly() {
        let mut buf = BracketedPasteBuffer::new();
        buf.feed(UiEvent::PasteStart);
        buf.feed(UiEvent::KeyPressed('x'));
        assert!(buf.is_active());
        assert_eq!(buf.buffered_len(), 1);

        buf.reset();
        assert!(!buf.is_active());
        assert_eq!(buf.buffered_len(), 0);
    }

    #[test]
    fn paste_buffer_passes_through_non_paste_events() {
        let mut buf = BracketedPasteBuffer::new();

        // Regular events pass through unchanged when not in paste mode
        let result = buf.feed(UiEvent::KeyPressed('1'));
        assert_eq!(result, Some(UiEvent::KeyPressed('1')));

        let result = buf.feed(UiEvent::PromptSubmitted("hello".into()));
        assert_eq!(result, Some(UiEvent::PromptSubmitted("hello".into())));

        let result = buf.feed(UiEvent::Tick);
        assert_eq!(result, Some(UiEvent::Tick));
    }

    #[test]
    fn ansi_constants_are_correct() {
        assert_eq!(ansi::BRACKETED_PASTE_ON, "\x1b[?2004h");
        assert_eq!(ansi::BRACKETED_PASTE_OFF, "\x1b[?2004l");
        assert_eq!(ansi::PASTE_START, "\x1b[200~");
        assert_eq!(ansi::PASTE_END, "\x1b[201~");
    }

    #[test]
    fn paste_content_labels() {
        assert_eq!(PasteContentType::Text.label(), "text");
        assert_eq!(PasteContentType::Code { language: None }.label(), "code");
        assert_eq!(PasteContentType::Link { url: "x".into() }.label(), "link");
        assert_eq!(PasteContentType::Image { mime_type: "image/png".into(), size_bytes: 100 }.label(), "image");
        assert_eq!(PasteContentType::Mixed { components: vec![] }.label(), "mixed");
    }

    // -- Context budget tests ------------------------------------------

    #[test]
    fn context_budget_thresholds() {
        let mut budget = ContextBudget::new(1000);

        // Above warning threshold (50% remaining): no warning, no critical
        budget.update(500);
        assert!(!budget.is_warning());
        assert!(!budget.is_critical());
        assert!((budget.pct_remaining() - 0.5).abs() < 0.001);

        // At warning threshold (35% remaining = 650 used, 350 remaining)
        budget.update(650);
        assert!(budget.is_warning());
        assert!(!budget.is_critical());
        assert!((budget.pct_remaining() - 0.35).abs() < 0.001);

        // At critical threshold (25% remaining = 750 used, 250 remaining)
        budget.update(750);
        assert!(!budget.is_warning()); // not warning because critical takes precedence
        assert!(budget.is_critical());
        assert!((budget.pct_remaining() - 0.25).abs() < 0.001);

        // Well below critical (5% remaining)
        budget.update(950);
        assert!(!budget.is_warning());
        assert!(budget.is_critical());
    }

    #[test]
    fn context_budget_remaining() {
        let mut budget = ContextBudget::new(100);
        assert_eq!(budget.remaining(), 100);
        assert!((budget.pct_remaining() - 1.0).abs() < 0.001);

        budget.update(30);
        assert_eq!(budget.remaining(), 70);
        assert!((budget.pct_remaining() - 0.7).abs() < 0.001);

        budget.update(100);
        assert_eq!(budget.remaining(), 0);
        assert!((budget.pct_remaining() - 0.0).abs() < 0.001);

        // Saturate: used > total should clamp remaining to 0
        budget.update(150);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn context_warning_event() {
        let mut state = UiState::default();
        state.context_budget = ContextBudget::new(1000);
        state.context_budget.update(700); // 30% remaining → warning zone

        let effects = state.reduce(UiEvent::ContextWarning);
        assert!(state.status_line.contains("context warning"));
        assert!(state.status_line.contains("30%"));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::Render)));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::EmitContextWarning(_))));
    }

    #[test]
    fn context_critical_event() {
        let mut state = UiState::default();
        state.context_budget = ContextBudget::new(1000);
        state.context_budget.update(800); // 20% remaining → critical zone

        let effects = state.reduce(UiEvent::ContextCritical);
        assert!(state.status_line.contains("CONTEXT CRITICAL"));
        assert!(state.status_line.contains("20%"));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::Render)));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::EmitContextWarning(_))));
    }

    // -- Slash command tests -------------------------------------------

    #[test]
    fn slash_command_parse_all() {
        assert_eq!(SlashCommand::parse("help"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("h"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("compact"), Some(SlashCommand::Compact));
        assert_eq!(SlashCommand::parse("clear"), Some(SlashCommand::Clear));
        assert_eq!(SlashCommand::parse("agents"), Some(SlashCommand::Agents));
        assert_eq!(SlashCommand::parse("diff"), Some(SlashCommand::Diff));
        assert_eq!(SlashCommand::parse("status"), Some(SlashCommand::Status));
        assert_eq!(SlashCommand::parse("exit"), Some(SlashCommand::Exit));
        assert_eq!(SlashCommand::parse("quit"), Some(SlashCommand::Exit));
        assert_eq!(SlashCommand::parse("resume"), Some(SlashCommand::Resume));
        assert_eq!(SlashCommand::parse("save"), Some(SlashCommand::Save));
    }

    #[test]
    fn slash_command_parse_with_args() {
        assert_eq!(
            SlashCommand::parse("model deepseek-v4-pro"),
            Some(SlashCommand::Model("deepseek-v4-pro".into()))
        );
        assert_eq!(
            SlashCommand::parse("file /home/user/test.rs"),
            Some(SlashCommand::File("/home/user/test.rs".into()))
        );
        assert_eq!(
            SlashCommand::parse("pane chat"),
            Some(SlashCommand::Pane("chat".into()))
        );
    }

    #[test]
    fn slash_command_parse_unknown() {
        assert_eq!(SlashCommand::parse("nonexistent"), None);
        assert_eq!(SlashCommand::parse(""), None);
    }

    #[test]
    fn slash_command_model_needs_arg() {
        assert_eq!(SlashCommand::parse("model"), None);
    }

    #[test]
    fn slash_command_submit_effect() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PromptSubmitted("/help".into()));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::ExecuteSlashCommand(SlashCommand::Help))));
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn slash_command_clear_resets_state() {
        let mut state = UiState::default();
        state.pending_tasks = 5;
        state.pending_approvals = 3;
        state.active_jobs = 2;

        let effects = state.reduce(UiEvent::SlashCommand(SlashCommand::Clear));
        assert_eq!(state.pending_tasks, 0);
        assert_eq!(state.pending_approvals, 0);
        assert_eq!(state.active_jobs, 0);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::ExecuteSlashCommand(SlashCommand::Clear))));
    }

    // -- Path completion tests -----------------------------------------

    #[test]
    fn could_be_path_detection() {
        // Paths should be detected
        assert!(could_be_path("home/user/file"));
        assert!(could_be_path("./src/main.rs"));
        assert!(could_be_path("~/Documents"));
        assert!(could_be_path("../parent"));
        // Simple names should also be treated as potential paths
        assert!(could_be_path("src"));
        assert!(could_be_path("Cargo.toml"));
        // Known slash commands should NOT be treated as paths
        assert!(!could_be_path("help"));
        assert!(!could_be_path("compact"));
        assert!(!could_be_path("agents"));
        assert!(!could_be_path("status"));
        // Empty should not
        assert!(!could_be_path(""));
    }

    #[test]
    fn path_completer_creates_and_cycles() {
        let mut pc = PathCompleter::new("src");
        assert!(pc.active);
        assert!(!pc.candidates.is_empty());
        // Should have at least one candidate
        assert!(pc.current_candidate().is_some());
        pc.next();
        // Should still be valid after cycling
        assert!(pc.candidates.is_empty() || pc.current_candidate().is_some());
        pc.deactivate();
        assert!(!pc.active);
    }

    #[test]
    fn path_completer_push_pop() {
        let mut pc = PathCompleter::new("sr");
        let before = pc.candidates.len();
        pc.push_char('c');
        // After "src", should have fewer or equal candidates
        assert!(pc.candidates.len() <= before || before == 0);
        pc.pop_char();
        // After pop, back to "sr"
        assert_eq!(pc.partial, "sr");
    }

    // -- Paste folded notation tests -----------------------------------

    #[test]
    fn paste_folded_text_short() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PasteContent {
            content: "hello".into(),
            raw_bytes: None,
            detected_type: PasteContentType::Text,
        });
        assert!(state.status_line.contains("hello"));
        assert!(!state.status_line.contains("lines"));
    }

    #[test]
    fn paste_folded_text_multi_line() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PasteContent {
            content: "line1\nline2\nline3\nline4\nline5".into(),
            raw_bytes: None,
            detected_type: PasteContentType::Text,
        });
        assert!(state.status_line.contains("Pasted text"));
        assert!(state.status_line.contains("lines"));
    }

    #[test]
    fn paste_folded_code() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PasteContent {
            content: "fn main() {\n    println!(\"hi\");\n}".into(),
            raw_bytes: None,
            detected_type: PasteContentType::Code { language: Some("rust".into()) },
        });
        assert!(state.status_line.contains("rust"));
    }

    #[test]
    fn paste_folded_image() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::PasteContent {
            content: "base64stuff".into(),
            raw_bytes: Some({
                let mut v = vec![0x89, b'P', b'N', b'G'];
                v.resize(1004, 0);
                v
            }),
            detected_type: PasteContentType::Image {
                mime_type: "image/png".into(),
                size_bytes: 1004,
            },
        });
        assert!(state.status_line.contains("image/png"));
        assert!(state.status_line.contains("KB"));
    }

    // -- Content detection v2 tests ------------------------------------

    #[test]
    fn detect_shebang_scripts() {
        let python = "#!/usr/bin/env python3\nprint('hello')";
        let t = detect_paste_content(python);
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "python"));

        let bash = "#!/bin/bash\necho hello";
        let t = detect_paste_content(bash);
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "bash"));
    }

    #[test]
    fn detect_json() {
        let t = detect_paste_content("{\"key\": \"value\"}");
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "json"));
    }

    #[test]
    fn detect_diff() {
        let t = detect_paste_content("--- a/file.rs\n+++ b/file.rs\n@@ -1,3 +1,4 @@\n+new line\n old line");
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "diff"));
    }

    #[test]
    fn detect_base64_image_data_uri() {
        let t = detect_paste_content("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk");
        assert!(matches!(t, PasteContentType::Image { ref mime_type, .. } if mime_type == "image/png"));
    }

    #[test]
    fn detect_base64_image_raw() {
        let t = detect_paste_content("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk");
        assert!(matches!(t, PasteContentType::Image { ref mime_type, .. } if mime_type == "image/png"));
    }

    #[test]
    fn detect_go_code() {
        let t = detect_paste_content("package main\n\nfunc main() {\n    fmt.Println(\"hello\")\n}");
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "go"));
    }

    #[test]
    fn detect_typescript() {
        let t = detect_paste_content("interface User {\n    name: string;\n    age: number;\n}");
        assert!(matches!(t, PasteContentType::Code { language: Some(ref l) } if l == "typescript"));
    }

    // -- Context budget gauge tests ------------------------------------

    #[test]
    fn gauge_bar_renders_correctly() {
        let mut budget = ContextBudget::new(1000);
        budget.update(650); // 65% used, 35% remaining → warning zone
        let bar = budget.gauge_bar(20);
        assert!(bar.contains('%'));
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']') || bar.contains("] "));
        assert_eq!(budget.zone(), BudgetZone::Warning);
        assert_eq!(budget.gauge_label(), "context WARN");
    }

    #[test]
    fn gauge_zone_transitions() {
        let mut budget = ContextBudget::new(1000);
        budget.update(500); // 50% remaining
        assert_eq!(budget.zone(), BudgetZone::Ok);

        budget.update(700); // 30% remaining → warning
        assert_eq!(budget.zone(), BudgetZone::Warning);

        budget.update(800); // 20% remaining → critical
        assert_eq!(budget.zone(), BudgetZone::Critical);
    }

    #[test]
    fn gauge_detail_includes_metrics() {
        let mut budget = ContextBudget::new(100_000);
        budget.update(30_000);
        let detail = budget.gauge_detail();
        assert!(detail.contains("30.0K"));
        assert!(detail.contains("100.0K"));
        assert!(detail.contains("70.0K"));
        assert!(detail.contains('%'));
    }

    // -- Multi-line input tests ----------------------------------------

    #[test]
    fn multiline_newline_deactivates_path_completer() {
        let mut state = UiState::default();
        state.path_completer = Some(PathCompleter::new("src"));
        let effects = state.reduce(UiEvent::KeyPressed('\n'));
        assert!(state.input_buffer.contains('\n'));
        assert!(state.path_completer.is_none() || !state.path_completer.as_ref().unwrap().active);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::InsertNewline)));
    }

    #[test]
    fn enter_submits_input_buffer() {
        let mut state = UiState::default();
        state.input_buffer = "hello world".into();
        let effects = state.reduce(UiEvent::EnterPressed);
        assert_eq!(state.pending_tasks, 1);
        assert!(state.input_buffer.is_empty());
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PersistCheckpoint)));
    }

    #[test]
    fn enter_empty_buffer_does_nothing() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::EnterPressed);
        assert_eq!(state.pending_tasks, 0);
        assert!(effects.is_empty());
    }

    #[test]
    fn enter_with_slash_command() {
        let mut state = UiState::default();
        state.input_buffer = "/help".into();
        let effects = state.reduce(UiEvent::EnterPressed);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::ExecuteSlashCommand(SlashCommand::Help))));
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn enter_during_tool_execution_becomes_pushback() {
        let mut state = UiState::default();
        state.active_tool = Some("read_file".into());
        state.input_buffer = "also check error handling".into();
        let effects = state.reduce(UiEvent::EnterPressed);
        // Should NOT increment pending_tasks (this is pushback, not new prompt)
        assert_eq!(state.pending_tasks, 0);
        assert!(state.input_buffer.is_empty());
        assert!(state.pending_pushback.is_some());
        assert_eq!(state.pending_pushback.as_deref().unwrap(), "also check error handling");
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p == "also check error handling")));
    }

    #[test]
    fn enter_with_pending_tasks_becomes_pushback() {
        let mut state = UiState::default();
        state.pending_tasks = 2;  // agent has queued tasks
        state.input_buffer = "stop using sed, use edit_file instead".into();
        let effects = state.reduce(UiEvent::EnterPressed);
        // Should NOT increment pending_tasks
        assert_eq!(state.pending_tasks, 2);
        assert!(state.input_buffer.is_empty());
        assert!(state.pending_pushback.is_some());
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p.contains("edit_file"))));
    }

    #[test]
    fn tool_finished_flushes_pending_pushback() {
        let mut state = UiState::default();
        state.active_tool = Some("read_file".into());
        state.pending_tasks = 1;
        state.pending_pushback = Some("check the error handling too".into());
        let effects = state.reduce(UiEvent::ToolFinished("read_file".into()));
        assert!(state.active_tool.is_none());
        assert!(state.pending_pushback.is_none());
        assert_eq!(state.pending_tasks, 0);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p == "check the error handling too")));
    }

    #[test]
    fn tool_started_flushes_pending_pushback() {
        let mut state = UiState::default();
        state.pending_pushback = Some("use grep instead".into());
        let effects = state.reduce(UiEvent::ToolStarted("write_file".into()));
        assert!(state.active_tool.is_some());
        assert!(state.pending_pushback.is_none());
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p == "use grep instead")));
    }

    #[test]
    fn multiple_pushbacks_overwrite() {
        let mut state = UiState::default();
        state.active_tool = Some("read_file".into());
        state.input_buffer = "first thought".into();
        state.reduce(UiEvent::EnterPressed);
        assert_eq!(state.pending_pushback.as_deref().unwrap(), "first thought");
        state.input_buffer = "actually, second thought".into();
        let effects = state.reduce(UiEvent::EnterPressed);
        // Second pushback overwrites first
        assert_eq!(state.pending_pushback.as_deref().unwrap(), "actually, second thought");
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p == "actually, second thought")));
    }

    #[test]
    fn user_pushback_event_works() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::UserPushback("fix all bugs".into()));
        assert_eq!(state.pending_pushback.as_deref().unwrap(), "fix all bugs");
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PushbackDraft(p) if p == "fix all bugs")));
    }

    #[test]
    fn shift_enter_inserts_newline() {
        let mut state = UiState::default();
        state.input_buffer = "line1".into();
        let effects = state.reduce(UiEvent::ShiftEnterPressed);
        assert!(state.input_buffer.contains('\n'));
        assert!(state.input_buffer.starts_with("line1"));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::InsertNewline)));
    }

    #[test]
    fn tick_emits_context_gauge() {
        let mut state = UiState::default();
        state.context_budget.update(500);
        let effects = state.reduce(UiEvent::Tick);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::UpdateContextGauge { .. })));
        assert!(effects.iter().any(|e| matches!(e, UiEffect::ScheduleBackgroundRefresh)));
    }

    // -- Keybinding customization tests --------------------------------

    #[test]
    fn keybinding_rebind_replaces_old() {
        let mut kb = Keybindings::default();
        assert_eq!(kb.pane_for_key('1'), Some(Pane::Chat));
        let old = kb.rebind('1', Pane::Diff);
        assert_eq!(old, Some(Pane::Chat));
        assert_eq!(kb.pane_for_key('1'), Some(Pane::Diff));
    }

    #[test]
    fn keybinding_unbind_removes() {
        let mut kb = Keybindings::default();
        kb.unbind('1');
        assert_eq!(kb.pane_for_key('1'), None);
    }

    #[test]
    fn keybinding_conflicts_detected() {
        let mut kb = Keybindings::default();
        kb.rebind('9', Pane::Chat); // '1' and '9' both → Chat
        let conflicts = kb.conflicts();
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn keybinding_list_bindings() {
        let kb = Keybindings::default();
        let bindings = kb.list_bindings();
        assert!(bindings.iter().any(|(k, l)| *k == '1' && *l == "Chat"));
    }

    // -- Checkpoint tests ----------------------------------------------

    #[test]
    fn checkpoint_capture_and_restore() {
        let mut state = UiState::default();
        state.input_buffer = "work in progress".into();
        state.pending_tasks = 3;
        state.active_pane = Pane::Diff;

        let cp = Checkpoint::capture(&state, "test-ckpt", Some("testing"));
        assert_eq!(cp.name, "test-ckpt");
        assert_eq!(cp.input_snapshot, "work in progress");
        assert_eq!(cp.pending_tasks, 3);

        // Mutate state
        state.input_buffer.clear();
        state.pending_tasks = 0;
        cp.restore(&mut state);
        assert_eq!(state.input_buffer, "work in progress");
        assert_eq!(state.pending_tasks, 3);
        assert_eq!(state.active_pane, Pane::Diff);
    }

    #[test]
    fn checkpoint_display_format() {
        let mut state = UiState::default();
        let cp = Checkpoint::capture(&state, "my-ckpt", Some("before refactor"));
        assert!(cp.display().contains("my-ckpt"));
        assert!(cp.display().contains("before refactor"));
    }

    #[test]
    fn capture_checkpoint_event() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::CaptureCheckpoint {
            name: "test".into(),
            description: Some("desc".into()),
        });
        assert_eq!(state.checkpoints.len(), 1);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::CheckpointChanged { .. })));
    }

    #[test]
    fn restore_checkpoint_not_found() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::RestoreCheckpoint { name: "nonexistent".into() });
        assert!(state.status_line.contains("not found"));
    }

    // -- Agent lifecycle tests -----------------------------------------

    #[test]
    fn agent_spawned_increments_jobs() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::AgentSpawned {
            id: "a1".into(), role: "explorer".into(), name: "test-agent".into(),
        });
        assert_eq!(state.active_jobs, 1);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::AgentStatusChanged { .. })));
    }

    #[test]
    fn agent_completed_decrements_and_persists() {
        let mut state = UiState::default();
        state.active_jobs = 2;
        state.pending_tasks = 3;
        let effects = state.reduce(UiEvent::AgentCompleted {
            id: "a1".into(), summary: "done".into(),
        });
        assert_eq!(state.active_jobs, 1);
        assert_eq!(state.pending_tasks, 2);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::PersistCheckpoint)));
    }

    #[test]
    fn agent_errored_decrements_jobs() {
        let mut state = UiState::default();
        state.active_jobs = 1;
        let effects = state.reduce(UiEvent::AgentErrored {
            id: "a1".into(), error: "timeout".into(),
        });
        assert_eq!(state.active_jobs, 0);
        assert!(state.status_line.contains("timeout"));
    }

    // -- Terminal capabilities tests -----------------------------------

    #[test]
    fn terminal_caps_default_no_images() {
        let caps = TerminalCaps::default();
        assert!(!caps.can_display_images());
        assert!(caps.image_protocol().is_none());
    }

    #[test]
    fn terminal_caps_sixel_enables_images() {
        let mut caps = TerminalCaps::default();
        caps.sixel = true;
        assert!(caps.can_display_images());
        assert_eq!(caps.image_protocol(), Some("sixel"));
    }

    // -- Slash command diff test ---------------------------------------

    #[test]
    fn slash_diff_switches_pane_and_requests_diff() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::SlashCommand(SlashCommand::Diff));
        assert_eq!(state.active_pane, Pane::Diff);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::RequestGitDiff)));
    }

    #[test]
    fn slash_save_creates_checkpoint() {
        let mut state = UiState::default();
        state.input_buffer = "save me".into();
        let effects = state.reduce(UiEvent::SlashCommand(SlashCommand::Save));
        assert_eq!(state.checkpoints.len(), 1);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::CheckpointChanged { .. })));
    }

    #[test]
    fn slash_agents_switches_pane() {
        let mut state = UiState::default();
        let effects = state.reduce(UiEvent::SlashCommand(SlashCommand::Agents));
        assert_eq!(state.active_pane, Pane::Agents);
    }
}