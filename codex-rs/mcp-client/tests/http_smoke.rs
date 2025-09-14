#![cfg(feature = "sdk")]

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use codex_mcp_client::set_client_impl_override;
use codex_mcp_client::McpClient;
use mcp_types::ClientCapabilities;
use mcp_types::Implementation;
use mcp_types::InitializeRequestParams;
use mcp_types::MCP_SCHEMA_VERSION;
use tempfile::TempDir;

#[tokio::test]
async fn http_initialize_and_list_tools() -> anyhow::Result<()> {
    // Start server in HTTP mode on loopback.
    let codex_home = TempDir::new().expect("create tempdir");
    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("CODEX_MCP_IMPL".into(), "official".into());
    env.insert(
        "CODEX_HOME".into(),
        codex_home.path().to_string_lossy().to_string(),
    );
    env.insert(
        "CODEX_MCP_SERVER_HTTP_BIND".into(),
        "127.0.0.1:18123".into(),
    );

    let mut child = std::process::Command::cargo_bin("codex-mcp-server")?
        .env_clear()
        .envs(env)
        .arg("--mcp-impl=official")
        .spawn()
        .context("spawn official server http")?;

    // Give the server a brief moment to bind.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Use official SDK client over HTTP.
    set_client_impl_override("official");
    let client = McpClient::new_http_client("http://127.0.0.1:18123/mcp")
        .await
        .context("create http client")?;

    // Initialize.
    let params = InitializeRequestParams {
        capabilities: ClientCapabilities {
            elicitation: None,
            experimental: None,
            roots: None,
            sampling: None,
        },
        client_info: Implementation {
            name: "codex-mcp-client-http-test".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Codex".into()),
            user_agent: None,
        },
        protocol_version: MCP_SCHEMA_VERSION.to_string(),
    };
    let _info = client
        .initialize(params, None, Some(Duration::from_secs(5)))
        .await
        .context("initialize http")?;

    // List tools.
    let list = client
        .list_tools(None, Some(Duration::from_secs(5)))
        .await
        .context("list tools http")?;
    assert!(list.tools.iter().any(|t| t.name == "codex"));

    // Cleanup.
    let _ = child.kill();
    Ok(())
}
