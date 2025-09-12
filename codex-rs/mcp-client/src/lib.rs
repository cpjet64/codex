#[cfg(not(feature = "rmcp_sdk"))]
mod mcp_client;
#[cfg(not(feature = "rmcp_sdk"))]
pub use mcp_client::McpClient;

#[cfg(feature = "rmcp_sdk")]
mod rmcp_wrapper;
#[cfg(feature = "rmcp_sdk")]
pub use rmcp_wrapper::McpClient;
