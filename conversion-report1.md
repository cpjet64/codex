# Codex MCP Integration Deep Dive

Status and recommendations on integrating the official Rust MCP SDK (rmcp) alongside the legacy implementation, with a focus on parity, interchangeability, and exposure of full SDK capabilities.

## Executive Summary

- Official SDK path is integrated and usable today for the Codex MCP surface: initialize → tools/list → tools/call. The MCP server defaults to the official implementation at compile time; the client has a feature-gated official wrapper and runtime selection.
- Interchangeability is implemented end-to-end. Users can independently select legacy or official for both server and client via CLI, env, or config. Legacy remains the behavioral default.
- No intentional regressions found in the supported surface. The official server bridges to existing Codex tool logic, including elicitation. Tests validate parity of tool listing and basic client/server interop for the official path when compiled with features.
- “Full SDK capabilities” (resources, prompts, generic request pass‑through, richer notifications) are not yet exposed through Codex’s public client API or server handlers. This is an intentional scope constraint; add-on work is listed below.

## Repository Inventory (MCP‑related)

- `codex-rs/mcp-types`
  - Generated schema/types + traits `ModelContextProtocolRequest`/`...Notification` used across the codebase.
- `codex-rs/mcp-client`
  - Legacy: JSON‑RPC over stdio with Tokio.
  - Official wrapper: `src/rmcp_wrapper.rs` (feature `rmcp_sdk`), mirrors legacy public API and adapts to `rmcp::model::*` via serde.
  - Runtime selection shim: `src/lib.rs` (`McpClient` chooses Legacy vs Official based on env/override when feature is enabled).
- `codex-rs/mcp-server`
  - Legacy server and message processor.
  - Official server: `src/rmcp_handlers.rs` (feature `rmcp_sdk`, enabled by default in this crate). Implements `initialize`, `tools/list`, and `tools/call`; bridges Codex tool runner and elicitation flows.
- `codex-rs/core`
  - `mcp_connection_manager.rs`: spawns MCP clients per configured server, handles init, aggregates `tools/list`, routes `tools/call`.
  - Config additions: `mcp_impl`, `mcp_client_impl` options threaded through config and CLI defaults.
- CLI wiring (`codex-rs/cli`)
  - Server impl selection: `--mcp-impl` or `CODEX_MCP_IMPL` with config fallback.
  - Client impl selection: `--mcp-client-impl` or `CODEX_MCP_CLIENT_IMPL` with config fallback. Uses `codex_mcp_client::set_client_impl_override`.

## What Works Today (Official SDK Path)

- Client (official wrapper)
  - `initialize`, `tools/list`, `tools/call` mapped to `rmcp` typed methods; timeouts supported.
  - `initialized` notification supported via `notify_initialized`.
- Server (official)
  - `initialize`: replies with server info and capabilities matching legacy expectations.
  - `tools/list`: returns the same two Codex tools as legacy by converting legacy tool definitions to `rmcp::model::Tool` via serde.
  - `tools/call`: routes to Codex’s legacy tool runner and converts responses back to `rmcp::model::CallToolResult`.
  - Elicitation: forwards Codex’s server‑initiated requests using rmcp typed request for create‑elicitation and relays responses back into the runner.

## Interchangeability & Selection Logic

- Server implementation
  - Build: `codex-mcp-server` enables `rmcp_sdk` by default.
  - Runtime: CLI `--mcp-impl (legacy|official)`, env `CODEX_MCP_IMPL`, config `mcp_impl`.
- Client implementation
  - Build: `codex-mcp-client` official path is behind feature `rmcp_sdk` (not enabled by default at the workspace level).
  - Runtime: CLI `--mcp-client-impl (legacy|official)`, env `CODEX_MCP_CLIENT_IMPL`, config `mcp_client_impl`.
  - Behavior when feature missing: official selection falls back to legacy internally (safe but potentially confusing; see Recommendations).

## Capability Parity

- Parity for Codex’s current MCP surface:
  - Initialize handshake, list tools, call tools: Yes (official = legacy behavior). Parity test exists for tool names.
  - Elicitation/approvals: Official server bridges to legacy mechanism; no functional loss detected within supported flows.
- Not yet exposed (official SDK supports, Codex not wired):
  - `resources/*` (list/read/templates), `prompts/*`, generic request/notification pass‑through for future methods, extended transports.

## Tests & Validation Hooks

- Server parity test: `codex-rs/mcp-server/tests/suite/parity.rs` compares `tools/list` names across legacy vs official.
- Client official test: `codex-rs/mcp-client/tests/official_list.rs` (gated by `--features rmcp_sdk`) initializes official server and validates `tools/list` content.
- Manual validation suggestions:
  - Server (official): `codex mcp --mcp-impl=official`
  - Client (official): run Codex with `--mcp-client-impl=official` and an external MCP server.

## Observed Gaps / Risks

- Feature UX mismatch on client build flags
  - Selecting official client at runtime when `rmcp_sdk` was not compiled silently falls back to legacy. Functionally safe, but confusing to users.
- Environment forwarding differences
  - Legacy client’s stdio child env baseline differs from rmcp wrapper’s `merge_base_env` (Unix: legacy includes `__CF_USER_TEXT_ENCODING`, `TZ`; Windows: wrapper adds `COMSPEC`, `SYSTEMROOT`, `WINDIR`). Potential behavioral drift for some servers.
- Limited method coverage in official client wrapper
  - `send_request` supports only initialize, list tools, and call tool; other methods return an error. This is sufficient for Codex today but not the full rmcp surface.
- Server unimplemented endpoints
  - Non‑tool endpoints return empty/default responses in official server; parity with legacy’s non‑support, but not full SDK exposure.

## Security & Safety Considerations

- No weakening of sandbox or approval semantics in official server path; the server bridges to legacy tool runner and elicitation logic.
- Child process env remains curated; however, align env baselines (see Recommendations) to avoid accidental secret leakage or behavior changes.
- Supply chain: `rmcp` pinned to `=0.6.4` in both client and server crates.

## Recommendations

1) Enable official client by default (build‑time)
- Add `features = ["rmcp_sdk"]` to `codex-mcp-client` in dependent crates or a workspace feature to standardize builds.

2) Add a runtime warning when official client is selected but not compiled
- If `McpClient` official path is unavailable (feature off), log a clear warning before falling back to legacy.

3) Align environment baselines
- Update `rmcp_wrapper::merge_base_env` to match legacy `DEFAULT_ENV_VARS` per OS to minimize drift.

4) Expand official client API coverage (as needed)
- Either implement a generic passthrough onto rmcp’s client for arbitrary MCP requests or add typed wrappers for `resources/*` and `prompts/*`.

5) Expand official server handlers (as needed)
- Implement `resources/*` and `prompts/*` handlers, even if initially returning clear, typed errors, to keep schema parity and future‑proofing.

6) CI and tests
- Add CI job that builds `codex-mcp-client` with `--features rmcp_sdk` and runs its tests.
- Keep server parity tests as guardrails; extend with resource/prompt tests once implemented.

## How to Validate Locally

- Official client+server tests (requires Rust toolchain and feature enabled):
  - `cargo test -p codex-mcp-client --features rmcp_sdk`
- Server tests (both implementations):
  - `cargo test -p codex-mcp-server`
- Manual checks:
  - Start official server: `codex mcp --mcp-impl=official`
  - Run Codex with official client (ensure `rmcp_sdk` built): `codex --mcp-client-impl=official ...`

## Conclusion

- The official Rust MCP SDK is integrated and interchangeable with the legacy implementation for the subset of MCP used by Codex today. The migration is effectively complete for initialize/list/call flows, and parity controls exist.
- To claim “full SDK capability exposure” within Codex, implement the remaining non‑tool endpoints and broaden the official client wrapper. The recommended build/runtime UX improvements will reduce confusion and lock in reliability as more capability is adopted.

