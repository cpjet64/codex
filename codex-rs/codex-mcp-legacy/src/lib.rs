//! Legacy adapter shell. Implementation moves here gradually.

use codex_mcp_types::McpClient as IfaceClient;
use codex_mcp_types::McpError;
use codex_mcp_types::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

/// Placeholder legacy client wrapper.
pub struct LegacyClient;

impl LegacyClient {
    pub fn new_placeholder() -> Self { Self }
}

impl IfaceClient for LegacyClient {
    fn as_name(&self) -> &'static str { "legacy" }

    fn initialize(
        &self,
        _p: InitializeRequestParams,
        _n: Option<serde_json::Value>,
        _t: Option<Duration>,
    ) -> Result<InitializeResult, McpError> {
        Err(anyhow::anyhow!("unimplemented").into())
    }

    fn list_tools(
        &self,
        _p: Option<ListToolsRequestParams>,
        _t: Option<Duration>,
    ) -> Result<ListToolsResult, McpError> {
        Err(anyhow::anyhow!("unimplemented").into())
    }

    fn call_tool(
        &self,
        _name: String,
        _args: Option<serde_json::Value>,
        _t: Option<Duration>,
    ) -> Result<CallToolResult, McpError> {
        Err(anyhow::anyhow!("unimplemented").into())
    }

    fn send_request<R>(
        &self,
        _p: R::Params,
        _t: Option<Duration>,
    ) -> Result<R::Result, McpError>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        Err(anyhow::anyhow!("unimplemented").into())
    }

    fn send_notification<N>(&self, _p: N::Params) -> Result<(), McpError>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize,
    {
        Err(anyhow::anyhow!("unimplemented").into())
    }
}

#[cfg(test)]
mod tests {
    use super::LegacyClient;
    use pretty_assertions::assert_eq;

    #[test]
    fn name_is_legacy() {
        let c = LegacyClient::new_placeholder();
        assert_eq!(c.as_name(), "legacy");
    }
}
