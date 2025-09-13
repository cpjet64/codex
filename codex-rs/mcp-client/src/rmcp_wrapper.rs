//! rmcp-based MCP client wrapper (feature: `rmcp_sdk`).
//!
//! This mirrors the public API of the legacy `McpClient` so callers do not
//! change. Internally, it uses rmcp transports.

#![allow(dead_code)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use mcp_types::CallToolRequest;
use mcp_types::CallToolRequestParams;
use mcp_types::InitializeRequest;
use mcp_types::InitializeRequestParams;
use mcp_types::InitializedNotification;
use mcp_types::ListToolsRequest;
use mcp_types::ListToolsRequestParams;
use mcp_types::ListToolsResult;
use mcp_types::ModelContextProtocolNotification;
use mcp_types::ModelContextProtocolRequest;
//
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::process::Command as TokioCommand;
use tokio::time;

// rmcp imports are present to document intended integration; avoid heavy
// compile errors by not using them beyond type visibility here.
use rmcp::service::RoleClient;
use rmcp::service::RunningService;
use rmcp::service::ServiceExt;
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::TokioChildProcess;

pub struct RmcpClient {
    inner: RunningService<RoleClient, ()>,
    // Map sanitized tool names -> original tool names as advertised by the server.
    // This lets us safely present sanitized names to callers while still calling
    // the correct original name on the wire.
    tool_name_map: Mutex<std::collections::HashMap<String, String>>,
}

/// Sanitize a tool name to a safe identifier: ^[a-zA-Z0-9_-]+$
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

impl RmcpClient {
    pub async fn new_stdio_client(
        program: OsString,
        args: Vec<OsString>,
        env: Option<HashMap<String, String>>,
    ) -> std::io::Result<Self> {
        let merged_env = merge_base_env(env);
        let transport = TokioChildProcess::new(TokioCommand::new(program).configure(|c| {
            c.args(args)
                .env_clear()
                .envs(merged_env.clone())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true);
        }))?;
        let inner = ().serve(transport).await.map_err(|e| {
            std::io::Error::other(format!("rmcp client initialization failed: {e}"))
        })?;
        Ok(Self {
            inner,
            tool_name_map: Mutex::new(std::collections::HashMap::new()),
        })
    }

    pub async fn send_request<R>(
        &self,
        params: R::Params,
        timeout: Option<Duration>,
    ) -> Result<R::Result>
    where
        R: ModelContextProtocolRequest,
        R::Params: Serialize,
        R::Result: DeserializeOwned,
    {
        // Use generic request path when available from rmcp client.
        // Fallback to mapping known methods.
        let method = R::METHOD;
        match method {
            m if m == InitializeRequest::METHOD => {
                let _ = serde_json::to_value(params)?; // touch param for type checking
                // Already initialized in serve(); return stored info.
                let res = self
                    .inner
                    .peer()
                    .peer_info()
                    .cloned()
                    .ok_or_else(|| anyhow!("peer info not initialized"))?;
                let v = serde_json::to_value(res)?;
                let typed: R::Result = serde_json::from_value(v)?;
                Ok(typed)
            }
            m if m == ListToolsRequest::METHOD => {
                let p: Option<ListToolsRequestParams> =
                    serde_json::from_value(serde_json::to_value(params)?)?;
                let rmcp_params: Option<rmcp::model::PaginatedRequestParam> = match p {
                    Some(pp) => Some(serde_json::from_value(serde_json::to_value(pp)?)?),
                    None => None,
                };
                let fut = self.inner.list_tools(rmcp_params);
                let res = match timeout {
                    Some(d) => time::timeout(d, fut)
                        .await
                        .map_err(|_| anyhow!("request timed out"))??,
                    None => fut.await?,
                };
                let v = serde_json::to_value(res)?;
                let typed: R::Result = serde_json::from_value(v)?;
                Ok(typed)
            }
            m if m == CallToolRequest::METHOD => {
                let p: CallToolRequestParams =
                    serde_json::from_value(serde_json::to_value(params)?)?;
                let args_obj = p.arguments.and_then(|v| v.as_object().cloned());
                let fut = self.inner.call_tool(rmcp::model::CallToolRequestParam {
                    name: p.name.into(),
                    arguments: args_obj,
                });
                let res = match timeout {
                    Some(d) => time::timeout(d, fut)
                        .await
                        .map_err(|_| anyhow!("request timed out"))??,
                    None => fut.await?,
                };
                let v = serde_json::to_value(res)?;
                let typed: R::Result = serde_json::from_value(v)?;
                Ok(typed)
            }
            other => Err(anyhow!(format!("unsupported rmcp request method: {other}"))),
        }
    }

    pub async fn send_notification<N>(&self, params: N::Params) -> Result<()>
    where
        N: ModelContextProtocolNotification,
        N::Params: Serialize,
    {
        let method = N::METHOD;
        if method == InitializedNotification::METHOD {
            // SDK provides a notification helper without params.
            let _ = serde_json::to_value(&params)?;
            self.inner
                .notify_initialized()
                .await
                .map_err(|e| anyhow!(format!("notify_initialized failed: {e}")))?;
            Ok(())
        } else {
            // No-op for other notifications (not used yet).
            Ok(())
        }
    }

    pub async fn initialize(
        &self,
        _initialize_params: InitializeRequestParams,
        initialize_notification_params: Option<serde_json::Value>,
        _timeout: Option<Duration>,
    ) -> Result<mcp_types::InitializeResult> {
        // The official SDK completes initialization during `serve()`. Return
        // the stored peer info and send the optional notification for parity.
        let response: rmcp::model::InitializeResult = self
            .inner
            .peer()
            .peer_info()
            .cloned()
            .ok_or_else(|| anyhow!("peer info not initialized"))?;
        if initialize_notification_params.is_some() {
            self.inner
                .notify_initialized()
                .await
                .map_err(|e| anyhow!(format!("notify_initialized failed: {e}")))?;
        }
        let v = serde_json::to_value(response)?;
        let typed: mcp_types::InitializeResult = serde_json::from_value(v)?;
        Ok(typed)
    }

    pub async fn list_tools(
        &self,
        _params: Option<ListToolsRequestParams>,
        _timeout: Option<Duration>,
    ) -> Result<ListToolsResult> {
        // Convert mcp_types::ListToolsRequestParams -> rmcp::model::PaginatedRequestParam
        // via serde for compatibility.
        let rmcp_params: Option<rmcp::model::PaginatedRequestParam> = match _params {
            Some(p) => Some(serde_json::from_value(serde_json::to_value(p)?)?),
            None => None,
        };
        let fut = self.inner.list_tools(rmcp_params);
        let res = match _timeout {
            Some(d) => time::timeout(d, fut)
                .await
                .map_err(|_| anyhow!("request timed out"))??,
            None => fut.await?,
        };
        // Convert rmcp -> mcp_types via serde
        // Convert rmcp -> mcp_types via serde first, then sanitize tool names
        // for presentation while remembering the original names for call_tool.
        let v = serde_json::to_value(res)?;
        let mut typed: mcp_types::ListToolsResult = serde_json::from_value(v)?;

        // Build/refresh mapping: sanitized -> original, ensuring uniqueness.
        let mut map = self.tool_name_map.lock().expect("tool_name_map poisoned");
        map.clear();

        // Track used sanitized names to avoid collisions.
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for tool in &mut typed.tools {
            let original = tool.name.clone();
            let mut sanitized = sanitize_tool_name(&original);
            if sanitized.is_empty() {
                sanitized = "_".to_string();
            }
            // Ensure uniqueness if two different originals collide after sanitization
            if used.contains(&sanitized)
                && map.get(&sanitized).map(|o| o != &original).unwrap_or(false)
            {
                let base = sanitized.clone();
                let mut idx: usize = 2;
                loop {
                    let candidate = format!("{}_{}", base, idx);
                    if !used.contains(&candidate) {
                        sanitized = candidate;
                        break;
                    }
                    idx += 1;
                }
            }
            used.insert(sanitized.clone());
            map.insert(sanitized.clone(), original);
            tool.name = sanitized;
        }
        Ok(typed)
    }

    pub async fn call_tool(
        &self,
        _name: String,
        _arguments: Option<serde_json::Value>,
        _timeout: Option<Duration>,
    ) -> Result<mcp_types::CallToolResult> {
        // Translate sanitized name (if used) back to original before sending.
        let (effective_name, fallback_name) = {
            let map = self.tool_name_map.lock().expect("tool_name_map poisoned");
            let mapped = map.get(&_name).cloned();
            match mapped {
                Some(orig) => (orig, _name.clone()),
                None => (_name.clone(), _name.clone()),
            }
        };

        // rmcp expects arguments as Option<Map<String, Value>>
        let args_obj = _arguments.and_then(|v| v.as_object().cloned());
        // First attempt: mapped original name (if present). On failure, fall back to the
        // sanitized name we were given, to support servers that already sanitize their tools.
        let attempt = |name: String| async {
            let fut = self.inner.call_tool(rmcp::model::CallToolRequestParam {
                name: name.into(),
                arguments: args_obj.clone(),
            });
            let res = match _timeout {
                Some(d) => time::timeout(d, fut)
                    .await
                    .map_err(|_| anyhow!("request timed out"))??,
                None => fut.await?,
            };
            let v = serde_json::to_value(res)?;
            let typed: mcp_types::CallToolResult = serde_json::from_value(v)?;
            Ok::<mcp_types::CallToolResult, anyhow::Error>(typed)
        };

        match attempt(effective_name.clone()).await {
            Ok(ok) => Ok(ok),
            Err(first_err) => {
                if effective_name != fallback_name {
                    attempt(fallback_name).await.map_err(|second_err| {
                        anyhow!(format!(
                            "first attempt failed: {first_err}; fallback failed: {second_err}"
                        ))
                    })
                } else {
                    Err(first_err)
                }
            }
        }
    }
}

fn merge_base_env(extra: Option<HashMap<String, String>>) -> HashMap<String, String> {
    // Minimal baseline env forwarding to keep child functional (arg0, PATH, temp, etc.).
    #[cfg(windows)]
    let keys = [
        "PATH",
        "PATHEXT",
        "COMSPEC",
        "SYSTEMROOT",
        "WINDIR",
        "USERNAME",
        "USERDOMAIN",
        "USERPROFILE",
        "TEMP",
        "TMP",
    ];
    #[cfg(not(windows))]
    let keys = [
        "HOME", "LOGNAME", "PATH", "SHELL", "USER", "TMPDIR", "LANG", "LC_ALL", "TERM",
    ];

    let mut map: HashMap<String, String> = keys
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
        .collect();
    if let Some(extra) = extra {
        for (k, v) in extra {
            map.insert(k, v);
        }
    }
    map
}
