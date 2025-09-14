use assert_cmd::prelude::*;
use codex_mcp_sdk::SdkClient;
use mcp_types::ClientCapabilities;
use mcp_types::Implementation;
use mcp_types::InitializeRequestParams;
use mcp_types::MCP_SCHEMA_VERSION;
use std::process::Command as StdCommand;
use std::time::Duration;

#[tokio::test]
async fn cancel_smoke_http() -> anyhow::Result<()> {
    // Start official server bound to loopback HTTP.
    let mut cmd = StdCommand::cargo_bin("codex-mcp-server")?;
    cmd.env("CODEX_MCP_IMPL", "official");
    cmd.env("CODEX_MCP_SERVER_HTTP_BIND", "127.0.0.1:18123");
    cmd.env("RUST_LOG", "info");
    let mut child = cmd.spawn()?;

    // Brief wait for bind.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = SdkClient::new_streamable_http_client("http://127.0.0.1:18123/mcp").await?;

    // Initialize then cancel.
    let params = InitializeRequestParams {
        capabilities: ClientCapabilities {
            experimental: None,
            roots: None,
            sampling: None,
            elicitation: None,
        },
        client_info: Implementation {
            name: "cancel-http-test".into(),
            version: "0.0.0".into(),
            title: Some("Cancel HTTP Test".into()),
            user_agent: None,
        },
        protocol_version: MCP_SCHEMA_VERSION.into(),
    };
    let _ = client.initialize(params, None, None)?;

    client.cancel().await?;

    let _ = child.kill();
    Ok(())
}
