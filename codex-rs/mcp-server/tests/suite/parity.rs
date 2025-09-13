use mcp_test_support::McpProcess;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn is_valid_tool_name(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

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
    let mut legacy = get_tool_names("legacy").await?;
    let mut official = get_tool_names("official").await?;

    // Ensure official names are valid and sanitized
    assert!(official.iter().all(|n| is_valid_tool_name(n)));

    // Compare parity modulo sanitization by sanitizing legacy names
    legacy = legacy.into_iter().map(|n| sanitize_tool_name(&n)).collect();
    legacy.sort();
    official.sort();
    assert_eq!(legacy, official);
    Ok(())
}

#[tokio::test]
async fn test_official_tools_list_names_are_valid() -> anyhow::Result<()> {
    let official = get_tool_names("official").await?;
    assert!(
        official.iter().all(|n| is_valid_tool_name(n)),
        "all official tool names must match ^[a-zA-Z0-9_-]+$"
    );
    Ok(())
}
