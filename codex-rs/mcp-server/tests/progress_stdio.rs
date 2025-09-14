use codex_mcp_server::CodexToolCallParam;
use mcp_test_support::McpProcess;
use mcp_types::JSONRPCMessage;
use mcp_types::JSONRPCNotification;
use serde_json::Value;

#[tokio::test]
async fn progress_logging_notifications_stdio() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut proc = McpProcess::new_with_env(
        tmp.path(),
        &[("CODEX_MCP_IMPL", Some("official"))],
    )
    .await?;

    proc.initialize().await?;

    let params = CodexToolCallParam {
        prompt: "Say hello".into(),
        model: None,
        profile: None,
        cwd: None,
        approval_policy: None,
        sandbox: None,
        config: None,
        base_instructions: None,
        include_plan_tool: None,
    };
    let _id = proc.send_codex_tool_call(params).await?;

    // Scan a bounded number of messages for a logging/message with
    // logger == "progress".
    let mut found = false;
    for _ in 0..200 {
        let msg = proc.read_jsonrpc_message().await?;
        if let JSONRPCMessage::Notification(JSONRPCNotification { method, params, .. }) = msg {
            if method == "logging/message" {
                if let Some(Value::Object(obj)) = params {
                    let logger = obj.get("logger").and_then(|v| v.as_str());
                    if logger == Some("progress") {
                        found = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(found, "expected at least one progress logging message");
    Ok(())
}
