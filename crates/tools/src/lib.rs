use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use deepseek_protocol::{ToolKind, ToolOutput, ToolPayload};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// Capabilities that a tool may have or require.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCapability {
    /// Tool only reads data, never modifies state.
    ReadOnly,
    /// Tool writes to the filesystem.
    WritesFiles,
    /// Tool executes arbitrary shell commands.
    ExecutesCode,
    /// Tool makes network requests.
    Network,
    /// Tool can be run in a sandbox.
    Sandboxable,
    /// Tool requires user approval before execution.
    RequiresApproval,
}

/// Approval requirement for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalRequirement {
    /// Never needs approval: safe read-only operations.
    #[default]
    Auto,
    /// Suggest approval but allow user to skip.
    Suggest,
    /// Always require explicit user approval.
    Required,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during tool execution.
#[derive(Debug, Clone)]
pub enum ToolError {
    InvalidInput { message: String },
    MissingField { field: String },
    PathEscape { path: PathBuf },
    ExecutionFailed { message: String },
    Timeout { seconds: u64 },
    NotAvailable { message: String },
    PermissionDenied { message: String },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { message } => {
                write!(f, "Failed to validate input: {message}")
            }
            Self::MissingField { field } => {
                write!(
                    f,
                    "Failed to validate input: missing required field '{field}'"
                )
            }
            Self::PathEscape { path } => {
                write!(
                    f,
                    "Failed to resolve path '{}': path escapes workspace",
                    path.display()
                )
            }
            Self::ExecutionFailed { message } => {
                write!(f, "Failed to execute tool: {message}")
            }
            Self::Timeout { seconds } => {
                write!(
                    f,
                    "Failed to execute tool: operation timed out after {seconds}s"
                )
            }
            Self::NotAvailable { message } => {
                write!(f, "Failed to locate tool: {message}")
            }
            Self::PermissionDenied { message } => {
                write!(f, "Failed to authorize tool execution: {message}")
            }
        }
    }
}

impl std::error::Error for ToolError {}

impl ToolError {
    #[must_use]
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: msg.into(),
        }
    }

    #[must_use]
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    #[must_use]
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            message: msg.into(),
        }
    }

    #[must_use]
    pub fn path_escape(path: impl Into<PathBuf>) -> Self {
        Self::PathEscape { path: path.into() }
    }

    #[must_use]
    pub fn not_available(msg: impl Into<String>) -> Self {
        Self::NotAvailable {
            message: msg.into(),
        }
    }

    #[must_use]
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDenied {
            message: msg.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool result
// ---------------------------------------------------------------------------

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The output content, which may be JSON or plain text.
    pub content: String,
    /// Whether the execution was successful.
    pub success: bool,
    /// Optional structured metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl ToolResult {
    /// Create a successful result with content.
    #[must_use]
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            success: true,
            metadata: None,
        }
    }

    /// Create an error result with message.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            success: false,
            metadata: None,
        }
    }

    /// Create a successful result from JSON. Returns an error instead of
    /// panicking when serialization fails.
    pub fn json<T: Serialize>(value: &T) -> std::result::Result<Self, serde_json::Error> {
        Ok(Self {
            content: serde_json::to_string_pretty(value)?,
            success: true,
            metadata: None,
        })
    }

    /// Create a result from JSON, falling back to a plain-text error message
    /// if serialization fails.
    #[must_use]
    pub fn json_or_error<T: Serialize>(value: &T) -> Self {
        match serde_json::to_string_pretty(value) {
            Ok(content) => Self {
                content,
                success: true,
                metadata: None,
            },
            Err(err) => Self {
                content: format!("(serialization error: {err})"),
                success: false,
                metadata: None,
            },
        }
    }

    /// Add metadata to the result.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ---------------------------------------------------------------------------
// Execution metrics
// ---------------------------------------------------------------------------

/// Per-tool execution metrics collected during dispatch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolExecutionMetrics {
    /// Total number of invocations.
    pub total_calls: u64,
    /// Number of successful invocations.
    pub success_calls: u64,
    /// Number of failed invocations.
    pub failure_calls: u64,
    /// Number of timed-out invocations.
    pub timeout_calls: u64,
    /// Number of retried invocations.
    pub retry_calls: u64,
    /// Cumulative execution time in milliseconds.
    pub total_duration_ms: u64,
    /// Minimum execution time in milliseconds.
    pub min_duration_ms: Option<u64>,
    /// Maximum execution time in milliseconds.
    pub max_duration_ms: Option<u64>,
}

impl ToolExecutionMetrics {
    /// Record a successful invocation.
    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_calls += 1;
        self.success_calls += 1;
        self.total_duration_ms += duration_ms;
        self.min_duration_ms = Some(
            self.min_duration_ms
                .map_or(duration_ms, |m| m.min(duration_ms)),
        );
        self.max_duration_ms = Some(
            self.max_duration_ms
                .map_or(duration_ms, |m| m.max(duration_ms)),
        );
    }

    /// Record a failed invocation.
    pub fn record_failure(&mut self) {
        self.total_calls += 1;
        self.failure_calls += 1;
    }

    /// Record a timeout.
    pub fn record_timeout(&mut self) {
        self.total_calls += 1;
        self.timeout_calls += 1;
    }

    /// Record a retry attempt.
    pub fn record_retry(&mut self) {
        self.retry_calls += 1;
    }

    /// Average execution time in milliseconds.
    #[must_use]
    pub fn avg_duration_ms(&self) -> Option<f64> {
        let completed = self.success_calls + self.failure_calls;
        if completed == 0 {
            None
        } else {
            Some(self.total_duration_ms as f64 / completed as f64)
        }
    }

    /// Success rate as a fraction (0.0 – 1.0).
    #[must_use]
    pub fn success_rate(&self) -> Option<f64> {
        if self.total_calls == 0 {
            None
        } else {
            Some(self.success_calls as f64 / self.total_calls as f64)
        }
    }
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

/// Configuration for automatic retry of transient tool failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,
    /// Base delay between retries (applied with exponential backoff).
    pub base_delay_ms: u64,
    /// Maximum delay between retries.
    pub max_delay_ms: u64,
    /// Whether to retry on timeout errors.
    pub retry_on_timeout: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 1,
            base_delay_ms: 500,
            max_delay_ms: 5000,
            retry_on_timeout: true,
        }
    }
}

impl RetryPolicy {
    /// No retries.
    #[must_use]
    pub fn never() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Retry up to `n` times with exponential backoff.
    #[must_use]
    pub fn with_max_retries(n: u32) -> Self {
        Self {
            max_retries: n,
            ..Default::default()
        }
    }

    /// Compute the delay for a given retry attempt (0-indexed).
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let ms = (self.base_delay_ms * 2u64.saturating_pow(attempt)).min(self.max_delay_ms);
        Duration::from_millis(ms)
    }

    /// Whether a specific error is considered transient (retriable).
    fn is_retriable(&self, err: &FunctionCallError) -> bool {
        match err {
            FunctionCallError::TimedOut { .. } => self.retry_on_timeout,
            FunctionCallError::ExecutionFailed { .. } => true,
            FunctionCallError::ToolNotFound { .. }
            | FunctionCallError::KindMismatch { .. }
            | FunctionCallError::MutatingToolRejected { .. }
            | FunctionCallError::Cancelled { .. } => false,
        }
    }
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

/// Helper to extract a required string field from JSON input.
pub fn required_str<'a>(input: &'a Value, field: &str) -> std::result::Result<&'a str, ToolError> {
    input.get(field).and_then(Value::as_str).ok_or_else(|| {
        let provided: Vec<&str> = input
            .as_object()
            .map(|obj| obj.keys().map(|k| k.as_str()).collect())
            .unwrap_or_default();
        if provided.is_empty() {
            ToolError::missing_field(field)
        } else {
            let hint = format!(
                "missing required field '{field}'. Input provided: {}",
                provided.join(", ")
            );
            ToolError::invalid_input(hint)
        }
    })
}

/// Helper to extract an optional string field from JSON input.
#[must_use]
pub fn optional_str<'a>(input: &'a Value, field: &str) -> Option<&'a str> {
    input.get(field).and_then(Value::as_str)
}

/// Helper to extract a required u64 field from JSON input.
pub fn required_u64(input: &Value, field: &str) -> std::result::Result<u64, ToolError> {
    input
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| ToolError::missing_field(field))
}

/// Helper to extract an optional u64 field with default.
#[must_use]
pub fn optional_u64(input: &Value, field: &str, default: u64) -> u64 {
    input.get(field).and_then(Value::as_u64).unwrap_or(default)
}

/// Helper to extract an optional bool field with default.
#[must_use]
pub fn optional_bool(input: &Value, field: &str, default: bool) -> bool {
    input.get(field).and_then(Value::as_bool).unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Tool spec
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub supports_parallel_tool_calls: bool,
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiguredToolSpec {
    pub spec: ToolSpec,
    pub supports_parallel_tool_calls: bool,
}

// ---------------------------------------------------------------------------
// Tool call
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallSource {
    Direct,
    JsRepl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub payload: ToolPayload,
    pub source: ToolCallSource,
    pub raw_tool_call_id: Option<String>,
}

impl ToolCall {
    pub fn execution_subject(&self, fallback_cwd: &str) -> (String, String, &'static str) {
        match &self.payload {
            ToolPayload::LocalShell { params } => (
                params.command.clone(),
                params
                    .cwd
                    .clone()
                    .unwrap_or_else(|| fallback_cwd.to_string()),
                "shell",
            ),
            _ => (self.name.clone(), fallback_cwd.to_string(), "tool"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool invocation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub call_id: String,
    pub tool_name: String,
    pub payload: ToolPayload,
    pub source: ToolCallSource,
}

// ---------------------------------------------------------------------------
// Function call error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionCallError {
    ToolNotFound { name: String },
    KindMismatch { expected: ToolKind, got: ToolKind },
    MutatingToolRejected { name: String },
    TimedOut { name: String, timeout_ms: u64 },
    Cancelled { name: String },
    ExecutionFailed { name: String, error: String },
}

impl std::fmt::Display for FunctionCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolNotFound { name } => write!(f, "tool not found: {name}"),
            Self::KindMismatch { expected, got } => write!(f, "tool kind mismatch: expected {expected:?}, got {got:?}"),
            Self::MutatingToolRejected { name } => write!(f, "mutating tool rejected: {name}"),
            Self::TimedOut { name, timeout_ms } => write!(f, "tool timed out: {name} ({timeout_ms}ms)"),
            Self::Cancelled { name } => write!(f, "tool cancelled: {name}"),
            Self::ExecutionFailed { name, error } => write!(f, "tool execution failed: {name}: {error}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool handler trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn kind(&self) -> ToolKind;
    fn matches_kind(&self, kind: ToolKind) -> bool {
        self.kind() == kind
    }
    fn is_mutating(&self) -> bool {
        false
    }
    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> std::result::Result<ToolOutput, FunctionCallError>;
}

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

/// Maximum number of tools that can execute in parallel.
const DEFAULT_MAX_PARALLEL_TOOLS: usize = 16;

#[derive(Debug)]
pub struct ToolCallRuntime {
    /// Semaphore limiting concurrent tool executions.
    pub parallel_semaphore: Arc<Semaphore>,
    /// Per-tool execution metrics.
    pub metrics: Arc<RwLock<HashMap<String, ToolExecutionMetrics>>>,
}

impl Default for ToolCallRuntime {
    fn default() -> Self {
        Self {
            parallel_semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_PARALLEL_TOOLS)),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ToolCallRuntime {
    /// Create a runtime with a custom parallelism limit.
    #[must_use]
    pub fn with_max_parallel(max_parallel: usize) -> Self {
        Self {
            parallel_semaphore: Arc::new(Semaphore::new(max_parallel.max(1))),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Snapshot current metrics.
    pub async fn metrics_snapshot(&self) -> HashMap<String, ToolExecutionMetrics> {
        self.metrics.read().await.clone()
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
    specs: HashMap<String, ConfiguredToolSpec>,
    runtime: ToolCallRuntime,
}

impl ToolRegistry {
    pub fn register(&mut self, spec: ToolSpec, handler: Arc<dyn ToolHandler>) -> Result<()> {
        let name = spec.name.clone();
        self.specs.insert(
            name.clone(),
            ConfiguredToolSpec {
                supports_parallel_tool_calls: spec.supports_parallel_tool_calls,
                spec,
            },
        );
        self.handlers.insert(name, handler);
        Ok(())
    }

    pub fn list_specs(&self) -> Vec<ConfiguredToolSpec> {
        self.specs.values().cloned().collect()
    }

    /// Get a reference to the runtime for external metrics inspection.
    #[must_use]
    pub fn runtime(&self) -> &ToolCallRuntime {
        &self.runtime
    }

    /// Dispatch a tool call with retry support.
    pub async fn dispatch(
        &self,
        call: ToolCall,
        allow_mutating: bool,
    ) -> std::result::Result<ToolOutput, FunctionCallError> {
        let handler = self.handlers.get(&call.name).cloned().ok_or_else(|| {
            FunctionCallError::ToolNotFound {
                name: call.name.clone(),
            }
        })?;
        let configured =
            self.specs
                .get(&call.name)
                .cloned()
                .ok_or_else(|| FunctionCallError::ToolNotFound {
                    name: call.name.clone(),
                })?;

        let payload_kind = tool_payload_kind(&call.payload);
        let expected = handler.kind();
        if !handler.matches_kind(payload_kind) {
            return Err(FunctionCallError::KindMismatch {
                expected,
                got: payload_kind,
            });
        }
        if handler.is_mutating() && !allow_mutating {
            return Err(FunctionCallError::MutatingToolRejected { name: call.name });
        }

        let invocation = ToolInvocation {
            call_id: call
                .raw_tool_call_id
                .clone()
                .unwrap_or_else(|| format!("tool-call-{}", uuid::Uuid::new_v4())),
            tool_name: call.name.clone(),
            payload: call.payload,
            source: call.source,
        };

        let retry_policy = &configured.spec.retry_policy;
        let mut last_error = None;

        for attempt in 0..=retry_policy.max_retries {
            if attempt > 0 {
                debug!(
                    tool = %call.name,
                    attempt,
                    "retrying tool execution"
                );
                self.record_metric(&call.name, move |m| m.record_retry());
                tokio::time::sleep(retry_policy.delay_for_attempt(attempt - 1)).await;
            }

            let start = Instant::now();

            // Acquire a permit from the semaphore to limit parallelism.
            let _permit = self.runtime.parallel_semaphore.acquire().await;

            let result = self
                .execute_with_timeout(
                    &*handler,
                    configured.spec.timeout_ms,
                    invocation.clone(),
                )
                .await;

            let elapsed_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    self.record_metric(&call.name, move |m| m.record_success(elapsed_ms));
                    return Ok(output);
                }
                Err(err) => {
                    if retry_policy.is_retriable(&err) && attempt < retry_policy.max_retries {
                        warn!(
                            tool = %call.name,
                            attempt,
                            error = %err,
                            "tool execution failed, will retry"
                        );
                        last_error = Some(err);
                        continue;
                    }
                    match &err {
                        FunctionCallError::TimedOut { .. } => {
                            self.record_metric(&call.name, move |m| m.record_timeout());
                        }
                        _ => {
                            self.record_metric(&call.name, move |m| m.record_failure());
                        }
                    }
                    return Err(err);
                }
            }
        }

        // Unreachable unless retries exhausted — return the last error.
        Err(last_error.unwrap_or_else(|| {
            FunctionCallError::ExecutionFailed {
                name: call.name.clone(),
                error: "retries exhausted".into(),
            }
        }))
    }

    async fn execute_with_timeout(
        &self,
        handler: &dyn ToolHandler,
        timeout_ms: Option<u64>,
        invocation: ToolInvocation,
    ) -> std::result::Result<ToolOutput, FunctionCallError> {
        if let Some(timeout_ms) = timeout_ms {
            let name = invocation.tool_name.clone();
            match tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                handler.handle(invocation),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(FunctionCallError::TimedOut { name, timeout_ms }),
            }
        } else {
            handler.handle(invocation).await
        }
    }

    fn record_metric(&self, tool_name: &str, f: impl FnOnce(&mut ToolExecutionMetrics) + Send + 'static) {
        // Best-effort: spawn a task to avoid holding locks across awaits.
        // In practice this writes a small metric update and returns quickly.
        let metrics = self.runtime.metrics.clone();
        let name = tool_name.to_string();
        tokio::spawn(async move {
            let mut guard = metrics.write().await;
            let entry = guard.entry(name).or_default();
            f(entry);
        });
    }
}

fn tool_payload_kind(payload: &ToolPayload) -> ToolKind {
    match payload {
        ToolPayload::Mcp { .. } => ToolKind::Mcp,
        ToolPayload::Function { .. }
        | ToolPayload::Custom { .. }
        | ToolPayload::LocalShell { .. } => ToolKind::Function,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tool_result_json_round_trips_content() {
        let result = ToolResult::json(&json!({"ok": true})).expect("json");
        assert!(result.success);
        assert!(result.content.contains("\"ok\": true"));
    }

    #[test]
    fn tool_result_json_or_error_does_not_panic() {
        // serde_json can't serialize a raw Value that contains non-serializable types.
        // But this test verifies json_or_error never panics on a normal value.
        let result = ToolResult::json_or_error(&json!({"key": "val"}));
        assert!(result.success);
    }

    #[test]
    fn helper_extractors_validate_shape() {
        let input = json!({"name": "demo", "count": 7, "enabled": true});
        assert_eq!(required_str(&input, "name").expect("name"), "demo");
        assert_eq!(optional_u64(&input, "count", 0), 7);
        assert!(optional_bool(&input, "enabled", false));
        assert!(matches!(
            required_u64(&input, "name"),
            Err(ToolError::MissingField { .. })
        ));
    }

    #[test]
    fn required_str_reports_provided_fields_on_missing_required_field() {
        let input = json!({"path": "src/lib.rs", "content": "new body"});
        let err = required_str(&input, "replace").expect_err("replace is missing");
        let message = err.to_string();
        assert!(message.contains("missing required field 'replace'"));
        assert!(message.contains("Input provided:"));
        assert!(message.contains("path"));
        assert!(message.contains("content"));
    }

    #[test]
    fn tool_error_display_matches_legacy_text() {
        let err = ToolError::missing_field("path");
        assert_eq!(
            err.to_string(),
            "Failed to validate input: missing required field 'path'"
        );
    }

    // -- new: metrics ---------------------------------------------------

    #[test]
    fn metrics_tracks_success_and_failure() {
        let mut m = ToolExecutionMetrics::default();
        m.record_success(100);
        m.record_success(200);
        m.record_failure();
        m.record_timeout();

        assert_eq!(m.total_calls, 4);
        assert_eq!(m.success_calls, 2);
        assert_eq!(m.failure_calls, 1);
        assert_eq!(m.timeout_calls, 1);
        assert_eq!(m.min_duration_ms, Some(100));
        assert_eq!(m.max_duration_ms, Some(200));
        assert!((m.avg_duration_ms().unwrap() - 150.0).abs() < 1.0);
        assert!((m.success_rate().unwrap() - 0.5).abs() < 0.01);
    }

    // -- new: retry policy ----------------------------------------------

    #[test]
    fn retry_policy_delay_grows_exponentially() {
        let policy = RetryPolicy::default();
        let d0 = policy.delay_for_attempt(0);
        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);
        assert!(d1 > d0);
        assert!(d2 > d1);
    }

    #[test]
    fn retry_policy_respects_max_delay() {
        let policy = RetryPolicy {
            max_delay_ms: 1000,
            ..Default::default()
        };
        let d10 = policy.delay_for_attempt(10);
        assert_eq!(d10, Duration::from_millis(1000));
    }

    #[test]
    fn retry_policy_never_disables_retries() {
        let policy = RetryPolicy::never();
        assert_eq!(policy.max_retries, 0);
    }

    #[test]
    fn retriable_errors_classified_correctly() {
        let policy = RetryPolicy::default();
        assert!(policy.is_retriable(&FunctionCallError::TimedOut {
            name: "x".into(),
            timeout_ms: 1000
        }));
        assert!(policy.is_retriable(&FunctionCallError::ExecutionFailed {
            name: "x".into(),
            error: "oops".into()
        }));
        assert!(!policy.is_retriable(&FunctionCallError::ToolNotFound {
            name: "x".into()
        }));
    }
}
