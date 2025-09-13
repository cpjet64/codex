use mcp_test_support::McpProcess;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

/// Helper: start server with given impl (legacy|official), initialize, and return sorted tool names.
async fn get_tool_names(mcp_impl: &str) -> anyhow::Result<Vec<String>> {
    let codex_home = TempDir::new().expect("create tempdir");
    let mut mcp =
        McpProcess::new_with_env(codex_home.path(), &[("CODEX_MCP_IMPL", Some(mcp_impl))]).await?;

    // Initialize handshake
    mcp.initialize().await?;

    // tools/list request
    let req_id = mcp.send_tools_list_request().await?;
    let response = mcp
        .read_stream_until_response_message(mcp_types::RequestId::Integer(req_id))
        .await?;
    let tools_result: mcp_types::ListToolsResult = serde_json::from_value(response.result)?;

    let mut names: Vec<String> = tools_result.tools.into_iter().map(|t| t.name).collect();
    names.sort();
    Ok(names)
}

#[tokio::test]
async fn test_tools_list_names_parity_between_impls() -> anyhow::Result<()> {
    let legacy = get_tool_names("legacy").await?;
    let official = get_tool_names("official").await?;
    assert_eq!(legacy, official);
    Ok(())
}
