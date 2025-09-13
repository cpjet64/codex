use codex_arg0::arg0_dispatch_or_else;
use codex_common::CliConfigOverrides;
use codex_common::McpImpl;
use codex_mcp_server::run_main_with_impl;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        // Simple arg/env parsing for selecting implementation.
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut selected_impl = match std::env::var("CODEX_MCP_IMPL") {
            Ok(v) if v.eq_ignore_ascii_case("official") => McpImpl::Official,
            _ => McpImpl::Legacy,
        };
        for w in args.windows(2) {
            if w[0] == "--mcp-impl" && w[1].eq_ignore_ascii_case("official") {
                selected_impl = McpImpl::Official;
            }
        }
        for a in &args {
            if let Some(val) = a.strip_prefix("--mcp-impl=")
                && val.eq_ignore_ascii_case("official")
            {
                selected_impl = McpImpl::Official;
            }
        }
        run_main_with_impl(
            codex_linux_sandbox_exe,
            CliConfigOverrides::default(),
            selected_impl,
        )
        .await?;
        Ok(())
    })
}
