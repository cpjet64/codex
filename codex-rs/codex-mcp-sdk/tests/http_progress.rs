#![cfg(feature = "rmcp")]

use std::time::Duration;

use assert_cmd::prelude::*;
use tokio::sync::mpsc;

// Alias rmcp dependency used by the SDK crate.
use rmcp as rmcp_crate;

#[tokio::test]
async fn http_progress_logging_appears() -> anyhow::Result<()> {
    // Start server in HTTP mode on loopback.
    let mut cmd = std::process::Command::cargo_bin("codex-mcp-server")?;
    cmd.env("CODEX_MCP_IMPL", "official");
    cmd.env("CODEX_MCP_SERVER_HTTP_BIND", "127.0.0.1:18123");
    cmd.env("RUST_LOG", "info");
    let mut child = cmd.spawn()?;

    // Let the server bind.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Channel to capture server->client notifications.
    let (tx, mut rx) = mpsc::unbounded_channel::<rmcp_crate::model::ServerNotification>();

    // Client handler that forwards notifications to the channel.
    #[derive(Clone)]
    struct ClientHandler {
        tx: mpsc::UnboundedSender<rmcp_crate::model::ServerNotification>,
    }

    impl rmcp_crate::service::Service<rmcp_crate::service::RoleClient> for ClientHandler {
        fn get_info(&self) -> rmcp_crate::model::ClientInfo {
            rmcp_crate::model::ClientInfo::from_build_env()
        }

        fn handle_notification(
            &self,
            notification: rmcp_crate::model::ServerNotification,
            _ctx: rmcp_crate::service::NotificationContext<
                rmcp_crate::service::RoleClient,
            >,
        ) -> Result<(), rmcp_crate::model::ErrorData> {
            let _ = self.tx.send(notification);
            Ok(())
        }
    }

    let handler = ClientHandler { tx };
    let transport = rmcp_crate::transport::StreamableHttpClientTransport::from_uri(
        "http://127.0.0.1:18123/mcp",
    );
    let running = handler
        .serve(transport)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // Issue a codex tool call to trigger progress notifications.
    let args = serde_json::json!({ "prompt": "Say hello" });
    let _ = running
        .peer()
        .call_tool(rmcp_crate::model::CallToolRequestParam {
            name: "codex".into(),
            arguments: args.as_object().cloned(),
        })
        .await;

    // Scan notifications briefly for logger=="progress".
    let mut found = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(note)) = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            if let rmcp_crate::model::ServerNotification::LoggingMessageNotification(p) = note {
                if p.logger.as_deref() == Some("progress") {
                    found = true;
                    break;
                }
            }
        }
    }

    let _ = child.kill();
    assert!(found, "expected logger=progress notification over HTTP");
    Ok(())
}
