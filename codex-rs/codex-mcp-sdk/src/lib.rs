//! Official SDK adapter shell. Minimal, expands later.

use codex_mcp_types::McpClient as IfaceClient;
use codex_mcp_types::McpError;
use codex_mcp_types::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Placeholder SDK client. Wire transports in later tasks.
pub struct SdkClient {
    inner: rmcp_crate::service::RunningService<
        rmcp_crate::service::RoleClient,
        (),
    >,
}

impl SdkClient {
    pub fn new_placeholder() -> Self {
        unreachable!("placeholder not supported once wired")
    }

    pub async fn new_stdio_child(
        program: OsString,
        args: Vec<OsString>,
        env: Option<HashMap<String, String>>,
    ) -> std::io::Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .env_clear()
            .envs(env.unwrap_or_default())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("no child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("no child stdout"))?;

        // rmcp supports serving over generic IO (reader, writer) tuple.
        let transport = (stdout, stdin);
        let inner = ().serve(transport).await.map_err(|e| {
            std::io::Error::other(format!("rmcp serve failed: {e}"))
        })?;
        Ok(Self { inner })
    }
}

impl IfaceClient for SdkClient {
    fn as_name(&self) -> &'static str { "sdk" }

    fn initialize(
        &self,
        _p: InitializeRequestParams,
        n: Option<serde_json::Value>,
        _t: Option<Duration>,
    ) -> Result<InitializeResult, McpError> {
        let peer = self.inner.peer();
        if n.is_some() {
            // Best-effort notify initialized; ignore error mapping for now.
            let _ = futures::executor::block_on(peer.notify_initialized());
        }
        let info = peer
            .peer_info()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("peer not initialized"))?;
        let v = serde_json::to_value(info)?;
        let typed: InitializeResult = serde_json::from_value(v)?;
        Ok(typed)
    }

    fn list_tools(
        &self,
        p: Option<ListToolsRequestParams>,
        t: Option<Duration>,
    ) -> Result<ListToolsResult, McpError> {
        let rmcp_params: Option<rmcp_crate::model::PaginatedRequestParam> = match p {
            Some(pp) => Some(serde_json::from_value(serde_json::to_value(pp)?)?),
            None => None,
        };
        let fut = self.inner.peer().list_tools(rmcp_params);
        let res = match t {
            Some(d) => futures::executor::block_on(async move {
                tokio::time::timeout(d, fut)
                    .await
                    .map_err(|_| anyhow::anyhow!("timeout"))
                    .and_then(|v| v.map_err(|e| anyhow::anyhow!(e)))
            })?,
            None => futures::executor::block_on(async move {
                fut.await.map_err(|e| anyhow::anyhow!(e))
            })?,
        };
        let v = serde_json::to_value(res)?;
        let typed: ListToolsResult = serde_json::from_value(v)?;
        Ok(typed)
    }

    fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
        t: Option<Duration>,
    ) -> Result<CallToolResult, McpError> {
        let args_obj = arguments.and_then(|v| v.as_object().cloned());
        let fut = self
            .inner
            .peer()
            .call_tool(rmcp_crate::model::CallToolRequestParam {
                name: name.into(),
                arguments: args_obj,
            });
        let res = match t {
            Some(d) => futures::executor::block_on(async move {
                tokio::time::timeout(d, fut)
                    .await
                    .map_err(|_| anyhow::anyhow!("timeout"))
                    .and_then(|v| v.map_err(|e| anyhow::anyhow!(e)))
            })?,
            None => futures::executor::block_on(async move {
                fut.await.map_err(|e| anyhow::anyhow!(e))
            })?,
        };
        let v = serde_json::to_value(res)?;
        let typed: CallToolResult = serde_json::from_value(v)?;
        Ok(typed)
    }

    fn send_request<R>(
        &self,
        params: R::Params,
        t: Option<Duration>,
    ) -> Result<R::Result, McpError>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: DeserializeOwned,
    {
        let method = R::METHOD;
        if method == InitializeRequest::METHOD {
            let _ = serde_json::to_value(&params)?;
            return self.initialize(
                InitializeRequestParams {
                    capabilities: ClientCapabilities {
                        experimental: None,
                        roots: None,
                        sampling: None,
                        elicitation: None,
                    },
                    client_info: Implementation {
                        name: "codex-mcp-sdk".to_owned(),
                        version: env!("CARGO_PKG_VERSION").to_owned(),
                        title: Some("Codex".to_string()),
                        user_agent: None,
                    },
                    protocol_version: MCP_SCHEMA_VERSION.to_owned(),
                },
                None,
                t,
            )
            .and_then(|res| {
                let v = serde_json::to_value(res)?;
                let typed: R::Result = serde_json::from_value(v)?;
                Ok(typed)
            });
        }
        if method == ListToolsRequest::METHOD {
            let p: Option<ListToolsRequestParams> =
                serde_json::from_value(serde_json::to_value(params)?)?;
            let res = self.list_tools(p, t)?;
            let v = serde_json::to_value(res)?;
            let typed: R::Result = serde_json::from_value(v)?;
            return Ok(typed);
        }
        if method == CallToolRequest::METHOD {
            let p: CallToolRequestParams =
                serde_json::from_value(serde_json::to_value(params)?)?;
            let res = self.call_tool(p.name, p.arguments, t)?;
            let v = serde_json::to_value(res)?;
            let typed: R::Result = serde_json::from_value(v)?;
            return Ok(typed);
        }
        Err(anyhow::anyhow!(format!(
            "unsupported rmcp request method: {}",
            method
        ))
        .into())
    }

    fn send_notification<N>(&self, params: N::Params) -> Result<(), McpError>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize,
    {
        let method = N::METHOD;
        if method == InitializedNotification::METHOD {
            let _ = serde_json::to_value(&params)?;
            futures::executor::block_on(self.inner.peer().notify_initialized())
                .map_err(|e| anyhow::anyhow!(format!("notify_initialized failed: {e}")))?;
        }
        Ok(())
    }
}

// Async API used by the façade in codex-mcp-client. These methods will be
// wired to the official SDK transports (stdio/http/sse) in later steps.
impl SdkClient {
    pub async fn new_stdio_child(
        _program: std::ffi::OsString,
        _args: Vec<std::ffi::OsString>,
        _env: Option<std::collections::HashMap<String, String>>,
    ) -> std::io::Result<Self> {
        Err(std::io::Error::other("sdk stdio transport not implemented"))
    }

    pub async fn send_request<R>(
        &self,
        _params: R::Params,
        _timeout: Option<Duration>,
    ) -> anyhow::Result<R::Result>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: DeserializeOwned,
    {
        Err(anyhow::anyhow!("unimplemented"))
    }

    pub async fn send_notification<N>(&self, _params: N::Params) -> anyhow::Result<()>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize,
    {
        Err(anyhow::anyhow!("unimplemented"))
    }

    pub async fn initialize(
        &self,
        _p: InitializeRequestParams,
        _n: Option<serde_json::Value>,
        _t: Option<Duration>,
    ) -> anyhow::Result<InitializeResult> {
        Err(anyhow::anyhow!("unimplemented"))
    }

    pub async fn list_tools(
        &self,
        _p: Option<ListToolsRequestParams>,
        _t: Option<Duration>,
    ) -> anyhow::Result<ListToolsResult> {
        Err(anyhow::anyhow!("unimplemented"))
    }

    pub async fn call_tool(
        &self,
        _name: String,
        _args: Option<serde_json::Value>,
        _t: Option<Duration>,
    ) -> anyhow::Result<CallToolResult> {
        Err(anyhow::anyhow!("unimplemented"))
    }
}
#[cfg(test)]
mod tests {
    use super::SdkClient;
    use pretty_assertions::assert_eq;

    #[test]
    fn name_is_sdk() {
        let c = SdkClient::new_placeholder();
        assert_eq!(c.as_name(), "sdk");
    }
}
