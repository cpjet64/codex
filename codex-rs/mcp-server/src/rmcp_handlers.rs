//! rmcp-based server implementation (feature: `rmcp_sdk`).
//!
//! This defines a minimal RMCP server that exposes the same tools as the
//! legacy JSON-RPC path. Initially, the handlers return a placeholder result
//! for `tools/call`; the list of tools and schemas match the legacy path.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use codex_core::AuthManager;
use codex_core::ConversationManager;
use codex_core::config::Config;
use mcp_types::ModelContextProtocolRequest;
use serde_json::json;
use tracing::debug;
use tracing::info;

// Use rmcp via codex-mcp-sdk reexports to avoid importing rmcp directly here.
use codex_mcp_sdk::rmcp_reexports as rmcp;

/// RMCP server that handles MCP requests using the official SDK.
#[derive(Clone)]
pub struct CodexRmcpService {
    _codex_linux_sandbox_exe: Option<PathBuf>,
    _config: Arc<Config>,
    conversation_manager: Arc<ConversationManager>,
}

impl CodexRmcpService {
    pub fn new(codex_linux_sandbox_exe: Option<PathBuf>, config: Arc<Config>) -> Self {
        let auth = AuthManager::shared(config.codex_home.clone());
        let conversation_manager = Arc::new(ConversationManager::new(auth.clone()));
        Self {
            _codex_linux_sandbox_exe: codex_linux_sandbox_exe,
            _config: config,
            conversation_manager,
        }
    }

    /// Sanitize a tool name to match ^[a-zA-Z0-9_-]+$
    fn sanitize_tool_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect()
    }

    fn legacy_tools_as_rmcp() -> Vec<rmcp::model::Tool> {
        let mut tools = Vec::new();
        let legacy_codex = crate::codex_tool_config::create_tool_for_codex_tool_call_param();
        if let Ok(mut v) = serde_json::to_value(&legacy_codex) {
            if let Some(name_str) = v.get("name").and_then(|s| s.as_str()) {
                let sanitized = Self::sanitize_tool_name(name_str);
                v["name"] = serde_json::Value::String(sanitized);
            }
            if let Ok(tool) = serde_json::from_value::<rmcp::model::Tool>(v) {
                tools.push(tool);
            }
        }
        let legacy_reply = crate::codex_tool_config::create_tool_for_codex_tool_call_reply_param();
        if let Ok(mut v) = serde_json::to_value(&legacy_reply) {
            if let Some(name_str) = v.get("name").and_then(|s| s.as_str()) {
                let sanitized = Self::sanitize_tool_name(name_str);
                v["name"] = serde_json::Value::String(sanitized);
            }
            if let Ok(tool) = serde_json::from_value::<rmcp::model::Tool>(v) {
                tools.push(tool);
            }
        }
        tools
    }
}

impl rmcp::service::Service<rmcp::service::RoleServer> for CodexRmcpService {
    async fn handle_request(
        &self,
        request: rmcp::model::ClientRequest,
        context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::ServerResult, rmcp::model::ErrorData> {
        use rmcp::model::ClientRequest;
        use rmcp::model::ServerResult;

        match request {
            ClientRequest::InitializeRequest(_req) => {
                // Align with legacy initialize expectations where feasible.
                let response_json = json!({
                    "protocolVersion": mcp_types::MCP_SCHEMA_VERSION,
                    "capabilities": {
                        "tools": { "listChanged": true }
                    },
                    "serverInfo": {
                        "name": "codex-mcp-server",
                        "title": "Codex",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                });
                let info: rmcp::model::InitializeResult = serde_json::from_value(response_json)
                    .map_err(|e| {
                        rmcp::model::ErrorData::new(
                            rmcp::model::ErrorCode(-32010),
                            format!("initialize response schema error: {e}"),
                            None,
                        )
                    })?;
                Ok(ServerResult::InitializeResult(info))
            }
            ClientRequest::ListToolsRequest(_req) => {
                let tools = Self::legacy_tools_as_rmcp();
                // Use serde to construct the paginated result shape.
                let value = json!({ "tools": tools });
                let result: rmcp::model::ListToolsResult =
                    serde_json::from_value(value).map_err(|e| {
                        rmcp::model::ErrorData::new(
                            rmcp::model::ErrorCode(-32001),
                            format!("schema error: {e}"),
                            None,
                        )
                    })?;
                Ok(ServerResult::ListToolsResult(result))
            }
            ClientRequest::CallToolRequest(req) => {
                let name = req.params.name.as_ref();
                info!("rmcp tools/call -> name: {name}");

                // Bridge to legacy Codex runner to execute properly.
                use crate::outgoing_message::OutgoingMessage;
                use crate::outgoing_message::OutgoingMessageSender;
                use std::collections::HashMap;
                use tokio::sync::Mutex;
                use tokio::sync::mpsc;
                let (out_tx, mut out_rx) = mpsc::unbounded_channel::<OutgoingMessage>();
                let outgoing = Arc::new(OutgoingMessageSender::new(out_tx));
                let running_map: Arc<
                    Mutex<
                        HashMap<mcp_types::RequestId, codex_protocol::mcp_protocol::ConversationId>,
                    >,
                > = Arc::new(Mutex::new(HashMap::new()));

                // Convert rmcp request id -> mcp_types::RequestId
                let req_id_val = serde_json::to_value(&context.id).map_err(|e| {
                    rmcp::model::ErrorData::new(
                        rmcp::model::ErrorCode(-32004),
                        format!("id ser error: {e}"),
                        None,
                    )
                })?;
                let mcp_req_id: mcp_types::RequestId =
                    serde_json::from_value(req_id_val).map_err(|e| {
                        rmcp::model::ErrorData::new(
                            rmcp::model::ErrorCode(-32005),
                            format!("id conv error: {e}"),
                            None,
                        )
                    })?;
                let mcp_req_id_clone = mcp_req_id.clone();

                match name {
                    "codex" => {
                        let args_map = req.params.arguments.unwrap_or_default();
                        let args_value = serde_json::Value::Object(args_map);
                        let param: crate::codex_tool_config::CodexToolCallParam =
                            serde_json::from_value(args_value).map_err(|e| {
                                rmcp::model::ErrorData::new(
                                    rmcp::model::ErrorCode(-32602),
                                    format!("invalid params: {e}"),
                                    None,
                                )
                            })?;

                        // Build config from params
                        let (initial_prompt, cfg) = param
                            .into_config(self._codex_linux_sandbox_exe.clone())
                            .map_err(|e| {
                                rmcp::model::ErrorData::new(
                                    rmcp::model::ErrorCode(-32006),
                                    format!("config error: {e}"),
                                    None,
                                )
                            })?;

                        // Helpful diagnostics to validate provider wiring during tests.
                        debug!(
                            model = %cfg.model,
                            provider_id = %cfg.model_provider_id,
                            base_url = %cfg.model_provider.base_url.clone().unwrap_or_default(),
                            wire_api = ?cfg.model_provider.wire_api,
                            "effective Codex config for rmcp tools/call"
                        );

                        // Run via legacy runner
                        let outgoing_clone = outgoing.clone();
                        let cm = self.conversation_manager.clone();
                        tokio::spawn(async move {
                            crate::codex_tool_runner::run_codex_tool_session(
                                mcp_req_id_clone,
                                initial_prompt,
                                cfg,
                                outgoing_clone,
                                cm,
                                running_map,
                            )
                            .await;
                        });
                    }
                    "codex-reply" => {
                        let args_map = req.params.arguments.unwrap_or_default();
                        let args_value = serde_json::Value::Object(args_map);
                        let param: crate::codex_tool_config::CodexToolCallReplyParam =
                            serde_json::from_value(args_value).map_err(|e| {
                                rmcp::model::ErrorData::new(
                                    rmcp::model::ErrorCode(-32602),
                                    format!("invalid params: {e}"),
                                    None,
                                )
                            })?;

                        // Parse conversation id
                        let conv_uuid =
                            uuid::Uuid::parse_str(&param.conversation_id).map_err(|e| {
                                rmcp::model::ErrorData::new(
                                    rmcp::model::ErrorCode(-32602),
                                    format!("invalid conversation_id: {e}"),
                                    None,
                                )
                            })?;
                        let conv_id = codex_protocol::mcp_protocol::ConversationId::from(conv_uuid);
                        let conversation = self
                            .conversation_manager
                            .get_conversation(conv_id)
                            .await
                            .map_err(|e| {
                                rmcp::model::ErrorData::new(
                                    rmcp::model::ErrorCode(-32008),
                                    format!("conversation not found: {e}"),
                                    None,
                                )
                            })?;

                        let outgoing_clone = outgoing.clone();
                        let prompt = param.prompt.clone();
                        tokio::spawn(async move {
                            crate::codex_tool_runner::run_codex_tool_session_reply(
                                conversation,
                                outgoing_clone,
                                mcp_req_id_clone,
                                prompt,
                                running_map,
                                conv_id,
                            )
                            .await;
                        });
                    }
                    _ => {
                        let err = rmcp::model::ErrorData::new(
                            rmcp::model::ErrorCode(-32601),
                            format!("unknown tool: {name}"),
                            None,
                        );
                        return Err(err);
                    }
                }

                // Await the first response matching this id from the runner and return as CallToolResult
                let peer = context.peer.clone();
                let result: rmcp::model::CallToolResult = loop {
                    match out_rx.recv().await {
                        Some(OutgoingMessage::Response(resp)) => {
                            if resp.id == mcp_req_id {
                                let val = serde_json::to_value(resp.result).map_err(|e| {
                                    rmcp::model::ErrorData::new(
                                        rmcp::model::ErrorCode(-32002),
                                        format!("serde error: {e}"),
                                        None,
                                    )
                                })?;
                                let call: mcp_types::CallToolResult = serde_json::from_value(val)
                                    .map_err(|e| {
                                    rmcp::model::ErrorData::new(
                                        rmcp::model::ErrorCode(-32003),
                                        format!("schema error: {e}"),
                                        None,
                                    )
                                })?;
                                let val2 = serde_json::to_value(call).map_err(|e| {
                                    rmcp::model::ErrorData::new(
                                        rmcp::model::ErrorCode(-32002),
                                        format!("serde error: {e}"),
                                        None,
                                    )
                                })?;
                                let rmcp_call: rmcp::model::CallToolResult =
                                    serde_json::from_value(val2).map_err(|e| {
                                        rmcp::model::ErrorData::new(
                                            rmcp::model::ErrorCode(-32003),
                                            format!("schema error: {e}"),
                                            None,
                                        )
                                    })?;
                                break rmcp_call;
                            }
                        }
                        Some(OutgoingMessage::Request(req0)) => {
                            // Forward server-initiated requests (e.g., elicitation) to the RMCP client.
                            // Since rmcp 0.6.x does not expose a raw JSON-RPC request API for arbitrary
                            // params, forward via the typed `create_elicitation` and attach Codex-specific
                            // fields under `_meta` so they are preserved on the wire.
                            let peer = peer.clone();
                            let outgoing_for_cb = outgoing.clone();
                            tokio::spawn(async move {
                                if req0.method == mcp_types::ElicitRequest::METHOD {
                                    // Extract the required typed fields and move the rest into `_meta`.
                                    let (message, requested_schema, meta_opt) = match &req0.params {
                                        Some(p) => {
                                            let msg = p
                                                .get("message")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            let schema_obj = p
                                                .get("requestedSchema")
                                                .and_then(|v| v.as_object())
                                                .cloned()
                                                .unwrap_or_default();

                                            let mut extras = serde_json::Map::new();
                                            if let Some(obj) = p.as_object() {
                                                for (k, v) in obj {
                                                    if k != "message" && k != "requestedSchema" {
                                                        extras.insert(k.clone(), v.clone());
                                                    }
                                                }
                                            }
                                            let meta = if extras.is_empty() {
                                                None
                                            } else {
                                                Some(rmcp::model::Meta(extras))
                                            };
                                            (msg, schema_obj, meta)
                                        }
                                        None => ("".to_string(), serde_json::Map::new(), None),
                                    };

                                    let request =
                                        rmcp::model::ServerRequest::CreateElicitationRequest(
                                            rmcp::model::CreateElicitationRequest {
                                                method: Default::default(),
                                                params:
                                                    rmcp::model::CreateElicitationRequestParam {
                                                        message,
                                                        requested_schema,
                                                    },
                                                extensions: Default::default(),
                                            },
                                        );
                                    let options = rmcp::service::PeerRequestOptions {
                                        timeout: None,
                                        meta: meta_opt,
                                    };
                                    let handle = match peer
                                        .send_request_with_option(request, options)
                                        .await
                                    {
                                        Ok(h) => h,
                                        Err(e) => {
                                            let err = mcp_types::JSONRPCErrorError {
                                                code: -32603,
                                                message: format!("{e}"),
                                                data: None,
                                            };
                                            outgoing_for_cb.send_error(req0.id.clone(), err).await;
                                            return;
                                        }
                                    };

                                    match handle.await_response().await {
                                        Ok(rmcp::model::ClientResult::CreateElicitationResult(
                                            res,
                                        )) => {
                                            let decision = match res.action {
                                                rmcp::model::ElicitationAction::Accept => {
                                                    codex_core::protocol::ReviewDecision::Approved
                                                }
                                                rmcp::model::ElicitationAction::Decline
                                                | rmcp::model::ElicitationAction::Cancel => {
                                                    codex_core::protocol::ReviewDecision::Denied
                                                }
                                            };
                                            let back = serde_json::json!({
                                                "decision": decision,
                                            });
                                            outgoing_for_cb
                                                .notify_client_response(req0.id.clone(), back)
                                                .await;
                                        }
                                        Ok(other) => {
                                            let err = mcp_types::JSONRPCErrorError {
                                                code: -32603,
                                                message: format!(
                                                    "unexpected elicitation result variant: {other:?}"
                                                ),
                                                data: None,
                                            };
                                            outgoing_for_cb.send_error(req0.id.clone(), err).await;
                                        }
                                        Err(e) => {
                                            let err = mcp_types::JSONRPCErrorError {
                                                code: -32603,
                                                message: format!("{e}"),
                                                data: None,
                                            };
                                            outgoing_for_cb.send_error(req0.id.clone(), err).await;
                                        }
                                    }
                                } else {
                                    let err = mcp_types::JSONRPCErrorError {
                                        code: -32601,
                                        message: format!(
                                            "unsupported server request method: {}",
                                            req0.method
                                        ),
                                        data: None,
                                    };
                                    outgoing_for_cb.send_error(req0.id.clone(), err).await;
                                }
                            });
                        }
                        Some(OutgoingMessage::Notification(note)) => {
                            // Map Codex notification to rmcp logging message.
                            let mut logger = note.method.clone();
                            if logger == "progress/update" {
                                logger = "progress".to_string();
                            }
                            let data = note.params.unwrap_or_else(|| json!({}));
                            let _ = peer
                                .notify_logging_message(
                                    rmcp::model::LoggingMessageNotificationParam {
                                        level: rmcp::model::LoggingLevel::Info,
                                        logger: Some(logger),
                                        data,
                                    },
                                )
                                .await;
                        }
                        Some(OutgoingMessage::Error(_)) => {
                            // TODO: Consider converting to JSON-RPC error.
                        }
                        None => {
                            return Err(rmcp::model::ErrorData::new(
                                rmcp::model::ErrorCode(-32007),
                                "runner channel closed".to_string(),
                                None,
                            ));
                        }
                        _ => {}
                    }
                };
                Ok(ServerResult::CallToolResult(result))
            }
            // Unused requests in our server today.
            _ => Ok(rmcp::model::ServerResult::EmptyResult(
                rmcp::model::EmptyObject {},
            )),
        }
    }

    async fn handle_notification(
        &self,
        _notification: rmcp::model::ClientNotification,
        _context: rmcp::service::NotificationContext<rmcp::service::RoleServer>,
    ) -> Result<(), rmcp::model::ErrorData> {
        Ok(())
    }

    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: rmcp::model::ServerCapabilities::default(),
            server_info: rmcp::model::Implementation::from_build_env(),
            instructions: None,
        }
    }
}

/// Start the RMCP server on stdio transport and wait until it terminates.
pub async fn run_official_server(
    codex_linux_sandbox_exe: Option<PathBuf>,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let service = CodexRmcpService::new(codex_linux_sandbox_exe, config);
    let running: rmcp::service::RunningService<rmcp::service::RoleServer, _> = service
        .serve(rmcp::transport::stdio())
        .await?;
    tokio::select! {
        _ = running.waiting() => {},
        _ = tokio::signal::ctrl_c() => { let _ = running.shutdown().await; },
    }
    Ok(())
}

/// Start the RMCP server on HTTP transport and wait until it terminates.
pub async fn run_official_server_http(
    bind: &str,
    codex_linux_sandbox_exe: Option<PathBuf>,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let service = CodexRmcpService::new(codex_linux_sandbox_exe, config);
    let transport = rmcp::transport::StreamableHttpServerTransport::bind(bind)
        .await?;
    let running: rmcp::service::RunningService<rmcp::service::RoleServer, _> = service
        .serve(transport)
        .await?;
    tokio::select! {
        _ = running.waiting() => {},
        _ = tokio::signal::ctrl_c() => { let _ = running.shutdown().await; },
    }
    Ok(())
}
