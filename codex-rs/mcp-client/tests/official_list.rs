#![cfg(feature = "sdk")]

use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;

use anyhow::Context;
use assert_cmd::prelude::*;
use codex_mcp_client::McpClient;
use mcp_types::ClientCapabilities;
use mcp_types::Implementation;
use mcp_types::InitializeRequestParams;
use mcp_types::ListToolsRequestParams;
use mcp_types::MCP_SCHEMA_VERSION;
use tempfile::TempDir;

#[tokio::test]
async fn official_initialize_and_list_tools() -> anyhow::Result<()> {
    // Locate server binary
    let std_cmd = std::process::Command::cargo_bin("codex-mcp-server")?;
    let program: OsString = std_cmd.get_program().to_owned();

    // Temporary CODEX_HOME to avoid touching user config
    let codex_home = TempDir::new().expect("create tempdir");

    // Env for child
    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("CODEX_MCP_IMPL".to_string(), "official".to_string());
    env.insert(
        "CODEX_HOME".to_string(),
        codex_home.path().to_string_lossy().to_string(),
    );
    env.insert("RUST_LOG".to_string(), "info".to_string());

    // Connect client to server via stdio
    let client = McpClient::new_stdio_client(
        program,
        vec![OsString::from("--mcp-impl=official")],
        Some(env),
    )
    .await
    .context("spawn official server")?;

    // Initialize
    let params = InitializeRequestParams {
        capabilities: ClientCapabilities {
            elicitation: None,
            experimental: None,
            roots: None,
            sampling: None,
        },
        client_info: Implementation {
            name: "codex-mcp-client-test".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("Codex".to_string()),
            user_agent: None,
        },
        protocol_version: MCP_SCHEMA_VERSION.to_string(),
    };
    let _init = client
        .initialize(params, None, Some(Duration::from_secs(10)))
        .await
        .context("initialize official")?;

    // List tools
    let tools = client
        .list_tools(None::<ListToolsRequestParams>, Some(Duration::from_secs(5)))
        .await
        .context("list tools")?;
    assert!(
        tools.tools.iter().any(|t| t.name == "codex"),
        "codex tool should be listed"
    );
    assert!(
        tools.tools.iter().any(|t| t.name == "codex-reply"),
        "codex-reply tool should be listed"
    );
    Ok(())
}
