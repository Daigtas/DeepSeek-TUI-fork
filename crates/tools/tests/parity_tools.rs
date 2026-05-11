use std::sync::Arc;

use async_trait::async_trait;
use deepseek_protocol::{ToolKind, ToolOutput, ToolPayload};
use deepseek_tools::{
    FunctionCallError, RetryPolicy, ToolCall, ToolCallSource, ToolHandler, ToolInvocation,
    ToolRegistry, ToolSpec,
};
use serde_json::json;

struct EchoHandler;

#[async_trait]
impl ToolHandler for EchoHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> std::result::Result<ToolOutput, FunctionCallError> {
        Ok(ToolOutput::Function {
            body: Some(json!({
                "tool": invocation.tool_name,
                "call_id": invocation.call_id
            })),
            success: true,
        })
    }
}

#[tokio::test]
async fn dispatches_function_tool_with_parallel_flag() {
    let mut registry = ToolRegistry::default();
    registry
        .register(
            ToolSpec {
                name: "echo".to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                supports_parallel_tool_calls: true,
                timeout_ms: Some(1000),
                retry_policy: RetryPolicy::never(),
            },
            Arc::new(EchoHandler),
        )
        .expect("register tool");

    let output = registry
        .dispatch(
            ToolCall {
                name: "echo".to_string(),
                payload: ToolPayload::Function {
                    arguments: "{\"message\":\"hi\"}".to_string(),
                },
                source: ToolCallSource::Direct,
                raw_tool_call_id: Some("call-1".to_string()),
            },
            true,
        )
        .await
        .expect("dispatch tool");
    match output {
        ToolOutput::Function { success, .. } => assert!(success),
        other => panic!("unexpected output: {other:?}"),
    }
}

#[tokio::test]
async fn dispatches_function_tool_with_retry_policy() {
    let mut registry = ToolRegistry::default();
    registry
        .register(
            ToolSpec {
                name: "echo-retry".to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                supports_parallel_tool_calls: false,
                timeout_ms: Some(500),
                retry_policy: RetryPolicy::with_max_retries(2),
            },
            Arc::new(EchoHandler),
        )
        .expect("register tool");

    let output = registry
        .dispatch(
            ToolCall {
                name: "echo-retry".to_string(),
                payload: ToolPayload::Function {
                    arguments: "{}".to_string(),
                },
                source: ToolCallSource::Direct,
                raw_tool_call_id: Some("call-2".to_string()),
            },
            true,
        )
        .await
        .expect("dispatch tool");
    match output {
        ToolOutput::Function { success, .. } => assert!(success),
        other => panic!("unexpected output: {other:?}"),
    }
}
