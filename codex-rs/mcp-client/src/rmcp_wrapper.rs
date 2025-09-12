//! rmcp-based MCP client wrapper (feature: `rmcp_sdk`).
//!
//! This mirrors the public API of the legacy `McpClient` so callers do not
//! change. Internally, it uses rmcp transports.

#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use mcp_types::CallToolRequest;
use mcp_types::CallToolRequestParams;
use mcp_types::InitializeRequest;
use mcp_types::InitializeRequestParams;
use mcp_types::InitializedNotification;
use mcp_types::JSONRPCMessage;
use mcp_types::JSONRPCNotification;
use mcp_types::JSONRPCRequest;
use mcp_types::JSONRPCResponse;
use mcp_types::ListToolsRequest;
use mcp_types::ListToolsRequestParams;
use mcp_types::ListToolsResult;
use mcp_types::ModelContextProtocolNotification;
use mcp_types::ModelContextProtocolRequest;
use mcp_types::RequestId;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time;

// rmcp imports are present to document intended integration; avoid heavy
// compile errors by not using them beyond type visibility here.
#[allow(unused_imports)]
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
#[allow(unused_imports)]
use tokio::process::Command as TokioCommand;

pub struct McpClient {
    // Placeholder for future rmcp client handle
    _marker: (),
}

impl McpClient {
    pub async fn new_stdio_client(
        program: OsString,
        args: Vec<OsString>,
        env: Option<HashMap<String, String>>,
    ) -> std::io::Result<Self> {
        // For now, spawn nothing. When rmcp integration lands, this will use
        // TokioChildProcess::new(TokioCommand::new(program).configure(|cmd| { ... }))
        let _ = (program, args, env);
        Ok(Self { _marker: () })
    }

    pub async fn send_request<R>(
        &self,
        params: R::Params,
        _timeout: Option<Duration>,
    ) -> Result<R::Result>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: DeserializeOwned,
    {
        // Minimal stub to keep dual-path compile-time selectable without
        // changing callers. Replace with rmcp request once wired.
        let _ = serde_json::to_value(&params)?;
        Err(anyhow!("rmcp path not yet wired: enable legacy or complete adapter"))
    }

    pub async fn send_notification<N>(&self, params: N::Params) -> Result<()>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize,
    {
        let _ = serde_json::to_value(&params)?;
        Ok(())
    }

    pub async fn initialize(
        &self,
        initialize_params: InitializeRequestParams,
        initialize_notification_params: Option<serde_json::Value>,
        timeout: Option<Duration>,
    ) -> Result<mcp_types::InitializeResult> {
        let _ = (initialize_params, initialize_notification_params, timeout);
        Err(anyhow!("rmcp path not yet wired: enable legacy or complete adapter"))
    }

    pub async fn list_tools(
        &self,
        _params: Option<ListToolsRequestParams>,
        _timeout: Option<Duration>,
    ) -> Result<ListToolsResult> {
        Err(anyhow!("rmcp path not yet wired: enable legacy or complete adapter"))
    }

    pub async fn call_tool(
        &self,
        _name: String,
        _arguments: Option<serde_json::Value>,
        _timeout: Option<Duration>,
    ) -> Result<mcp_types::CallToolResult> {
        Err(anyhow!("rmcp path not yet wired: enable legacy or complete adapter"))
    }
}

