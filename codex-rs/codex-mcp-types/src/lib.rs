//! Neutral DTO re-exports and simple interface traits.
//! Keep files short (<200 lines) and narrow.

pub use mcp_types::*;

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

/// Minimal error type used by adapters.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("anyhow: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Client trait used by core without binding to SDK types.
pub trait McpClient {
    fn as_name(&self) -> &'static str;

    fn initialize(
        &self,
        p: InitializeRequestParams,
        n: Option<serde_json::Value>,
        t: Option<Duration>,
    ) -> Result<InitializeResult, McpError>;

    fn list_tools(
        &self,
        p: Option<ListToolsRequestParams>,
        t: Option<Duration>,
    ) -> Result<ListToolsResult, McpError>;

    fn call_tool(
        &self,
        name: String,
        args: Option<serde_json::Value>,
        t: Option<Duration>,
    ) -> Result<CallToolResult, McpError>;

    fn send_request<R>(
        &self,
        p: R::Params,
        t: Option<Duration>,
    ) -> Result<R::Result, McpError>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: DeserializeOwned;

    fn send_notification<N>(&self, p: N::Params) -> Result<(), McpError>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize;
}

/// Marker for future server-side adapter abstraction.
pub trait McpServer {
    fn as_name(&self) -> &'static str;
}

// Neutral DTO aliases for clarity in core.
pub type ToolSpec = mcp_types::Tool;
pub type ToolCall = mcp_types::CallToolRequestParams;
pub type ToolResult = mcp_types::CallToolResult;
