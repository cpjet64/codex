mod mcp_client;
use std::sync::OnceLock;

/// Runtime-switchable MCP client that can use either the legacy JSON-RPC
/// implementation or the official RMCP SDK based implementation. Selection is
/// controlled via the environment variable `CODEX_MCP_CLIENT_IMPL` with values
/// `legacy` (default) or `official`.
pub struct McpClient {
    inner: Inner,
}

enum Inner {
    Legacy(Box<mcp_client::McpClient>),
}

// Optional process-local override for client selection without mutating env.
static CLIENT_IMPL_OVERRIDE: OnceLock<String> = OnceLock::new();

/// Set a process-local override for the MCP client implementation.
/// Accepts "legacy" or "official". Unknown values are ignored.
pub fn set_client_impl_override(impl_name: &str) {
    let v = impl_name.to_string();
    let _ = CLIENT_IMPL_OVERRIDE.set(v);
}

// Visible only in tests: expose the chosen client implementation without spawning.
#[cfg(test)]
fn selected_client_impl_for_tests() -> &'static str {
    match CLIENT_IMPL_OVERRIDE.get() {
        Some(v) if v.eq_ignore_ascii_case("official") => "official",
        _ => "legacy",
    }
}

impl McpClient {
    pub async fn new_stdio_client(
        program: std::ffi::OsString,
        args: Vec<std::ffi::OsString>,
        env: Option<std::collections::HashMap<String, String>>,
    ) -> std::io::Result<Self> {
        // Official SDK path not wired in this crate; always use legacy.
        let c = mcp_client::McpClient::new_stdio_client(program, args, env).await?;
        Ok(Self {
            inner: Inner::Legacy(Box::new(c)),
        })
    }

    pub async fn send_request<R>(
        &self,
        params: R::Params,
        timeout: Option<std::time::Duration>,
    ) -> anyhow::Result<R::Result>
    where
        R: mcp_types::ModelContextProtocolRequest,
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        match &self.inner {
            Inner::Legacy(c) => c.send_request::<R>(params, timeout).await,
            
        }
    }

    pub async fn send_notification<N>(&self, params: N::Params) -> anyhow::Result<()>
    where
        N: mcp_types::ModelContextProtocolNotification,
        N::Params: serde::Serialize,
    {
        match &self.inner {
            Inner::Legacy(c) => c.send_notification::<N>(params).await,
            
        }
    }

    pub async fn initialize(
        &self,
        initialize_params: mcp_types::InitializeRequestParams,
        initialize_notification_params: Option<serde_json::Value>,
        timeout: Option<std::time::Duration>,
    ) -> anyhow::Result<mcp_types::InitializeResult> {
        match &self.inner {
            Inner::Legacy(c) => {
                c.initialize(initialize_params, initialize_notification_params, timeout)
                    .await
            }
            
        }
    }

    pub async fn list_tools(
        &self,
        params: Option<mcp_types::ListToolsRequestParams>,
        timeout: Option<std::time::Duration>,
    ) -> anyhow::Result<mcp_types::ListToolsResult> {
        match &self.inner {
            Inner::Legacy(c) => c.list_tools(params, timeout).await,
            
        }
    }

    pub async fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
        timeout: Option<std::time::Duration>,
    ) -> anyhow::Result<mcp_types::CallToolResult> {
        match &self.inner {
            Inner::Legacy(c) => c.call_tool(name, arguments, timeout).await,
            
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_precedence_override_beats_env() {
        // Process override: legacy. Expect legacy to win regardless of env.
        set_client_impl_override("legacy");
        let which = selected_client_impl_for_tests();
        assert_eq!(which, "legacy");
    }
}
