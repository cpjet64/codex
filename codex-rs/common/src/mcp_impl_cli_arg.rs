use clap::Args;
use clap::ValueEnum;

/// Runtime-selectable MCP implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpImpl {
    Legacy,
    Official,
}

impl McpImpl {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpImpl::Legacy => "legacy",
            McpImpl::Official => "official",
        }
    }
}

/// CLI argument container for selecting the MCP implementation.
#[derive(Debug, Args, Clone, Default)]
pub struct McpImplArg {
    /// Select MCP implementation (overridable by CODEX_MCP_IMPL)
    #[arg(long = "mcp-impl", value_enum)]
    pub mcp_impl: Option<McpImpl>,
}

impl McpImplArg {
    /// Resolve the selected implementation, defaulting to Legacy if unspecified.
    pub fn selected_or_default(&self) -> McpImpl {
        if let Some(v) = self.mcp_impl {
            return v;
        }

        match std::env::var("CODEX_MCP_IMPL") {
            Ok(val) => match val.to_ascii_lowercase().as_str() {
                "official" => McpImpl::Official,
                "legacy" => McpImpl::Legacy,
                _ => McpImpl::Legacy,
            },
            Err(_) => McpImpl::Legacy,
        }
    }
}
