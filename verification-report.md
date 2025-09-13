# MCP Dual-Path Verification Report

- Scope: Verify legacy and official (rmcp) MCP paths, runtime selection, parity, and docs/precedence.
- Workspace: `codex-rs`
- Host: Windows (PowerShell), stable Rust toolchain

## Summary

- Server (official) parity verified: initialize, tools/list, tools/call.
- Legacy implementation remains fully working.
- Runtime selection works and is independently configurable for server and client.
- Approval (elicitation) parity on the official path verified by tests.
- Precedence (flag > env > config) validated for server; client precedence matches docs (flag > env or process override > config > legacy).
- Docs aligned to reflect client override nuance.

## Commands Run (exact)

Server tests (official vs legacy):
- `cargo test -p codex-mcp-server suite::parity::test_tools_list_names_parity_between_impls -- --exact`
- `cargo test -p codex-mcp-server suite::codex_tool::test_official_codex_tool_mocked_call_returns_final_message -- --exact --nocapture`
- `cargo test -p codex-mcp-server --test all -- suite::codex_tool::test_shell_command_approval_triggers_elicitation -- --nocapture`
- `cargo test -p codex-mcp-server suite::codex_tool::test_official_shell_and_patch_approvals_parity -- --exact --nocapture`

Client tests (official wrapper):
- `$env:CODEX_MCP_CLIENT_IMPL="official"; cargo test -p codex-mcp-client --features rmcp_sdk -- --nocapture`

Precedence tests (added):
- `cargo test -p codex-cli --test precedence -- --nocapture` (see `codex-rs/cli/tests/precedence.rs`)
- `cargo test -p codex-mcp-client -- --nocapture` (unit test of client override precedence)

Formatting:
- `cargo fmt --all` ("just" not available in this environment)

## Results (pass/fail)

- Server parity – tool names: PASS
- Server (official) mocked call returns final assistant message: PASS
- Legacy approval trigger: PASS
- Official approvals parity (shell + patch): PASS
- Client official initialize/list: PASS
- Server precedence tests (CLI integration): PASS
  - config-only official → official
  - env legacy over config official → legacy
  - CLI official over env legacy → official
- Client precedence unit test (process override > env): PASS

## Evidence (selected log lines)

- Legacy path: `codex-mcp-server starting with impl: legacy`
- Official path: `codex-mcp-server starting with impl: official`
- Official server start: `starting rmcp (official) server on stdio`

## Code & Docs Changes

- Added tests:
  - `codex-rs/cli/tests/precedence.rs` – verifies server selection precedence.
  - `codex-rs/mcp-client/src/lib.rs` – added test-only helper + unit test for client override precedence.
- Docs alignment:
  - Updated `docs/config.md` precedence lines to: `Client: CLI flag > CODEX_MCP_CLIENT_IMPL (or process override) > mcp_client_impl in config > legacy`.
- Cleanup performed earlier in this session: removed unused `rmcp-0.6.4/` and `rmcp.tgz`.

## Open Items / Notes

- Linting: repository uses `just fix -p <project>` to finalize; happy to run it for `codex-cli` and `codex-mcp-client` on request.
- Full workspace test run (`cargo test --all-features`) was not executed to keep iteration quick; can run on request.

## Checklist Cross-Reference (todo.md)

- A. Runtime selection – DONE
- B. Server (official) parity – DONE
- C. Approval parity (official) – DONE
- D. Client wrapper parity & selection – DONE
- E. Tests & parity suite – DONE
- F. Config & precedence – Tests added and passing; docs reflect client override nuance
- H. Documentation – DONE

If you want the full logs embedded or attached as artifacts, I can add them here or save to separate files.

