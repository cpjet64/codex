use assert_cmd::prelude::*;
use codex_mcp_sdk::SdkClient;
use mcp_types::CallToolRequestParams;
use mcp_types::ClientCapabilities;
use mcp_types::Implementation;
use mcp_types::InitializeRequestParams;
use mcp_types::MCP_SCHEMA_VERSION;
use serde_json::json;
use std::process::Command as StdCommand;
use std::time::Duration;

#[tokio::test]
async fn call_tool_timeout_http() -> anyhow::Result<()> {
    // Start official server bound to loopback HTTP.
    let mut cmd = StdCommand::cargo_bin("codex-mcp-server")?;
    cmd.env("CODEX_MCP_IMPL", "official");
    cmd.env("CODEX_MCP_SERVER_HTTP_BIND", "127.0.0.1:18123");
    cmd.env("RUST_LOG", "info");
    let mut child = cmd.spawn()?;

    // Brief wait for bind.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = SdkClient::new_streamable_http_client("http://127.0.0.1:18123/mcp").await?;

    // Initialize first.
    let params = InitializeRequestParams {
        capabilities: ClientCapabilities {
            experimental: None,
            roots: None,
            sampling: None,
            elicitation: None,
        },
        client_info: Implementation {
            name: "timeout-http-test".into(),
            version: "0.0.0".into(),
            title: Some("Timeout HTTP Test".into()),
            user_agent: None,
        },
        protocol_version: MCP_SCHEMA_VERSION.into(),
    };
    let _ = client.initialize(params, None, None)?;

    // Issue a codex tool call but set an extremely small timeout to
    // exercise the adapter's timeout path.
    let p = CallToolRequestParams {
        name: "codex".into(),
        arguments: Some(json!({ "prompt": "Say hi" })),
    };
    let err = client
        .send_request::<mcp_types::CallToolRequest>(p, Some(Duration::from_millis(1)))
        .expect_err("expected timeout error");
    let msg = format!("{err}");
    assert!(msg.to_lowercase().contains("timeout"));

    let _ = child.kill();
    Ok(())
}
