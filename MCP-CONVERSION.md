# MCP Replacement Write‑Up — Codex → Official Rust MCP SDK (rmcp)

Objective

- Replace Codex’s custom MCP implementation (mcp-types, mcp-client, mcp-server and their integrations) with the official Rust MCP SDK from https://github.com/modelcontextprotocol/rust-sdk (crate: `rmcp`).
- Preserve all existing capabilities (safety, security, functionality). Add new capabilities only if risk-free and gated.
- Deliver a reversible, verification‑driven migration with zero regressions.

Notes

- This document is written in Markdown but is saved as `MCP-CONVERSION.doc` per the prompt.
- Deliverables are listed in section 11 and provided under `docs/` alongside this file.

## 1) Scope & Inventory

An exhaustive inventory of MCP usage across code, configs, CI, and docs. A machine‑readable inventory is provided at `docs/mcp-inventory.json`.

Search commands executed

- git grep (case‑insensitive) across the repo:
  - `git grep -n -I -i mcp`
  - `git grep -n -I -i -E "modelcontextprotocol|jsonrpc|tools/call|tools/list|resources/list|resources/read|capabilities|transport|websocket|sse|prompt_context|provider|capabilities_changed|call_tool|tool_call|list_tools|list_resources"`

Language‑aware cross‑references (recommended)

- For deeper call graphs, use ctags or LSP-based tools (not executed here):
  - `ctags -R --languages=Rust --fields=+n -f tags codex-rs`
  - Use rust‑analyzer “Find all references” on: `McpClient::new_stdio_client`, `MessageProcessor::process_request`, and `McpConnectionManager::new`.

Runtime confirmation (recommended)

- Enable structured tracing around MCP startup and tool calls to capture dynamic paths missed by static analysis. Suggested: `RUST_LOG=codex_core=debug,codex_mcp_server=debug,codex_mcp_client=debug` and log the resolved `mcp_servers` and each `tools/call` invocation including duration and result code.

Inventory summary (key items)

- codex-rs/mcp-types
  - Purpose: Generated MCP schema/types, JSON‑RPC envelopes, traits `ModelContextProtocolRequest`/`...Notification`.
  - Usage: Referenced widely (core, server, client, protocol, tui, tests).

- codex-rs/mcp-client
  - `McpClient` implements stdio transport to spawn external MCP servers, send requests, and match responses; small helpers for `tools/list` and `tools/call`.
  - JSON‑RPC framing done manually over Tokio pipes.

- codex-rs/mcp-server
  - Stdio server reading JSON lines, `MessageProcessor` handlers for `initialize`, `tools/list`, `tools/call`, etc.; bridges to Codex core via `codex_message_processor`, streams Codex events out as MCP notifications.
  - Includes elicitation hooks for approvals (exec/patch) and Codex-specific server notifications.

- codex-rs/core
  - `mcp_connection_manager`: spawns each configured server via `McpClient`, performs `initialize` handshake, aggregates `tools/list`, routes `tools/call` with timeouts; qualifies names `<server>__<tool>` with length/sha1 logic.
  - `mcp_tool_call`: emits begin/end events around `McpConnectionManager::call_tool`.
  - `codex.rs`: wires `McpConnectionManager` into session startup; surfaces client start failures.

- codex-rs/protocol(+protocol-ts)
  - Wraps/uses `mcp_types` for message/request/ID types. TS export references `mcp_types` (e.g., `InitializeResult::export_all_to`).

- codex-rs/tui
  - Renders MCP tool results in history (`history_cell.rs`), depends on `mcp_types` content enums.

- codex-rs/cli
  - Adds `codex mcp` (experimental) to launch the custom server: `codex_mcp_server::run_main`.

- Docs / Config / CI
  - docs/advanced.md and docs/config.md document `mcp_servers` (stdio transport).
  - .github/workflows/rust-ci.yml verifies `mcp-types` codegen.
  - codex-rs/mcp-types/generate_mcp_types.py and schema/ are codegen assets.

See full structured list: `docs/mcp-inventory.json`.

## 2) Capability Parity Matrix

The CSV form is at `docs/mcp-parity.csv`. Summary below (selected rows):

- Tool discovery/execution
  - current_impl: `mcp-client` + `mcp_connection_manager` over stdio; `tools/list` aggregate, `tools/call` routed.
  - sdk_feature/api: `rmcp` client with `TokioChildProcess` transport and typed handlers; supports stdio and can be extended via transports.
  - gap?: No (feature‑parity achievable). Adapter: Thin wrapper to preserve current FQ tool naming and timeouts.

- Resource listing/fetch
  - current_impl: Server stubs/logs; not implemented.
  - sdk_feature/api: Supported by SDK types/handlers.
  - gap?: Yes — feature currently absent in server; implement using SDK server handlers; tests required.

- Streaming and notifications
  - current_impl: Server sends progress and Codex events; client only logs notifications.
  - sdk_feature/api: `rmcp` surfaces notifications cleanly.
  - gap?: Partial — add client‑side notification dispatchers and back‑pressure via bounded channels.

- Concurrency/back‑pressure
  - current_impl: Tokio tasks + bounded channels; largely ad‑hoc.
  - sdk_feature/api: Built‑in async handler patterns.
  - gap?: Align handlers to SDK’s concurrency contracts.

- Cancellation/timeouts/retries
  - current_impl: Per‑call timeout in client; no standard cancellation path; limited retry.
  - sdk_feature/api: Request futures + cancellation + room for retries at call sites.
  - gap?: Add cancellation paths and retry policy helpers.

- Error taxonomy/JSON‑RPC compliance
  - current_impl: Manual JSON‑RPC envelopes; custom error codes in server.
  - sdk_feature/api: Centralized JSON‑RPC framing.
  - gap?: Migrate error mapping to SDK conventions; keep user‑facing messages.

- Transports (HTTP/SSE/WS)
  - current_impl: stdio only; docs suggest SSE via external proxy.
  - sdk_feature/api: Designed to support multiple transports; stdio child process available; SSE/WS achievable via adapters.
  - gap?: Keep stdio now; plan SSE/WS behind a feature flag later.

- Schema/validation/versioning
  - current_impl: Private `mcp-types` (version `2025-06-18`).
  - sdk_feature/api: `rmcp` versions track spec; verify spec/date alignment and bump as needed.
  - gap?: Version mismatch risk — gate via integration tests.

- Safety/security controls
  - current_impl: Exec/patch approval elicitation; sandbox policy integration; secrets/env filter in server tools; audit via rollout logs.
  - sdk_feature/api: Neutral — must be preserved in adapters/handlers.
  - gap?: Re‑plumb elicitation + approvals into SDK handlers; preserve sandboxing.

## 3) Target Architecture & Design

Design goals

- Minimize invasive changes: preserve public surfaces (`mcp_servers` config, Codex events, FQ tool names), while swapping the internal transport/types to `rmcp`.
- Keep stdio as the transport initially; add SSE/WS later behind flags.
- Preserve safety gates and sandbox integration exactly.

High‑level

- Client side
  - Replace `codex-rs/mcp-client` with a thin wrapper over `rmcp` client primitives.
  - Use `rmcp::transport::TokioChildProcess` with `tokio::process::Command` to spawn servers.
  - Map `initialize` + `tools/list` + `tools/call` to SDK methods; propagate timeouts and cancellation.
  - Add notification dispatching for streaming progress (currently only logged).

- Server side
  - Re‑implement `codex-rs/mcp-server` using `rmcp` server handler traits.
  - Register two tools mirroring today’s behavior: `codex` and `codex-reply`.
  - Bridge Codex events to SDK notifications; keep `_meta.requestId` where applicable.
  - Implement resource endpoints (list/read/templates) to reach parity or explicitly mark unsupported with clear errors.

- Data contracts
  - Replace `mcp_types` with SDK’s types. Where SDK types differ, add local adapter structs and From/Into conversions.
  - Keep protocol/date in sync with `rmcp` version; pin exact crate version in `Cargo.toml` and audit on bump.

- Config
  - Keep `mcp_servers` (command/args/env/startup_timeout_ms). Since stdio remains, no config change required for initial cutover.

- Observability
  - Centralize JSON‑RPC logs and per‑tool call latencies via SDK hooks; retain redaction rules for secrets and env.

## 4) Security & Safety (no regressions)

Threat model preservation

- AuthN/AuthZ
  - Preserve AuthManager integration in the server (login/chatgpt/api‑key). Map Codex‑specific notifications (e.g., login complete) through SDK notifications.

- Sandboxing & isolation
  - Keep existing sandbox invocation (Seatbelt or Linux sandbox) as is. Do not weaken policies. The server’s exec and patch approval elicitation must remain unchanged in semantics.

- Secrets handling
  - Keep environment filtering for spawned MCP servers (client) and Codex tool subprocesses (server). Ensure no new env leakage; continue default excludes for KEY/SECRET/TOKEN patterns.

- Input validation & content safety
  - Maintain strict JSON size limits where applicable; validate tool arguments via JSON Schema where provided by SDK types. Preserve path/network egress guards.

- Rate limiting & quotas
  - Maintain existing per‑conversation throttles if present; add SDK‑level rate guards if needed.

- Audit logging
  - Retain rollout logs and event streams; ensure MCP notifications still carry Codex events for UX/test parity.

- Supply chain
  - Pin `rmcp` crate versions; run `cargo deny` and `cargo audit` in CI; keep Cargo.lock discipline.

- Privacy/PII
  - No change: preserve masking/redaction and retention policies around logs and transcripts.

Each control is mapped in the migration steps to verify parity.

## 5) Migration Plan (reversible)

Pre‑work

- Capture baseline behavior: golden responses for `tools/list` and representative `tools/call` across at least 2 external servers.
- Record perf snapshots: client startup time, list latency, call latency (p50/p95), memory footprint.
- Freeze must‑keep capabilities (see §2 and §4).

Cutover strategy (strangler, feature‑flagged)

- Add `feature = "rmcp_sdk"` across relevant crates.
- Introduce parallel adapters: keep old path live under default; enable new path with `rmcp_sdk` for canary.
- Route a small % of traffic (or local dev users) via `rmcp_sdk` and compare golden outputs.

Module‑by‑module order

1) Types: add `rmcp` crate and minimal adapters to replace `mcp_types` in non‑behavioral modules (protocol, TS export). Build only.
2) Client: re‑implement `McpClient` over `rmcp` with preserved API; keep stdio.
3) Core manager: switch `McpConnectionManager` to the new client wrapper; preserve FQ tool naming.
4) Server: port `codex-rs/mcp-server` handlers to SDK server; wire tools and notifications.
5) TUI and protocol: ensure types compile with adapters; adjust rendering if enum shapes changed.
6) Remove `mcp-types` codegen and CI hook once acceptance criteria are met.

Data/Config migration

- No config changes for stdio. Confirm env passthrough equivalence.
- If SDK uses different schema version strings, keep internal constants for conversion; document in release notes.

Rollback plan

- Feature flag `rmcp_sdk` off restores legacy path immediately.
- Keep dual code paths for one release; tag and branch for hotfix rollback.

Per‑step template (example for Client)

- Prerequisites: `rmcp` added to workspace; compile‑only adapters in place.
- Commands:
  - `cargo build -p codex-mcp-client`
  - `cargo test -p codex-core -- mcp` (targeted tests)
- Owner: Core team (MCP area).
- Expected output: Same `tools/list` aggregates; identical FQ names; same timeouts.
- Health checks: Compare golden outputs; monitor startup failures and call error rates.
- Rollback: Toggle feature flag; revert to legacy client.

## 6) Testing & Verification

Contract tests

- Round‑trip requests: `initialize`, `tools/list`, `tools/call`, notifications receipt. Include adversarial payloads (missing fields, wrong types, oversized args).
- Compare golden outputs between legacy and SDK paths (byte‑wise when feasible).

Property‑based tests

- Validate JSON value ↔ typed conversions for tool arguments and results; ensure fallible conversions fail with clear errors.

Fuzzing (cargo‑fuzz)

- Fuzz decoders for JSON‑RPC messages and server handlers that parse arguments.

Chaos/fault injection

- Simulate timeouts, partial reads, broken pipes, and reconnect storms for stdio child processes. Verify back‑pressure and graceful cancellation.

Soak tests

- High concurrency `tools/call` throughput with bounded channels; check for descriptor/memory leaks.

Security tests

- Attempt auth bypass and scope violations in server handlers; verify sandbox/approval gates block appropriately. Test path traversal on file inputs; validate egress is still blocked when expected.

Performance gates

- Regressions must be ≤ baseline + 5% for p95 latency and memory across representative scenarios.

Commands and CI

- Targeted:
  - `cargo test -p codex-mcp-client`
  - `cargo test -p codex-core -- mcp`
  - `cargo test -p codex-mcp-server`
- Workspace (post‑parity):
  - `cargo test --all-features`
  - `cargo clippy --workspace --all-features -D warnings`
  - `cargo fmt --all --check`
  - `cargo audit` and `cargo deny check`

## 7) Performance & Resource Targets

- Baseline collection
  - Startup: time from process start to `tools/list` aggregated result.
  - Calls: `tools/call` p50/p95 latency (local and remote servers), CPU/memory.

- Acceptance thresholds
  - No more than +5% p95 latency or RSS versus baseline under same load.

- Profiling plan
  - `tokio-console` for async hotspots; `pprof-rs` for CPU; heap snapshots for leaks; enable per‑call tracing spans in SDK adapters.

- Capacity headroom
  - Ensure bounded channels across client/server; document max in‑flight requests; test failure modes (child CPU spikes, slow consumers).

## 8) CI/CD & Ops Changes

- Replace `mcp-types` codegen verify step with:
  - `cargo audit`, `cargo deny` for `rmcp` supply chain.
  - Add SDK feature build matrix (on/off) until full removal.

- Build matrices
  - Linux/macOS/Windows; x86_64/aarch64; ensure stdio child process spawning works on all.

- Container hardening
  - Keep non‑root, readonly FS, minimal base image; preserve current sandbox integration.

- Deployment
  - Canary release with `rmcp_sdk` flag; staged rollout; automated rollback on error rate spikes.

## 9) Documentation & Developer Experience

- Update READMEs and architecture diagrams to reflect `rmcp` usage.
- Add “Getting Started with rmcp” section:
  - `rmcp = { version = "=0.2.0", features = ["server"] }` (pin exact)
  - Examples for client spawn and tool registration.
- Migration notes for contributors: adapters, feature flagging, and removal timeline for `mcp-types`.

## 10) Risks, Assumptions, Decisions

Risks

- Schema/version drift between `mcp_types` (2025‑06‑18) and `rmcp` release; mitigated by adapter layer and contract tests.
- Behavior differences in JSON‑RPC error mapping; mitigate with golden outputs.
- Hidden coupling in TUI/protocol to concrete enum shapes; mitigate with adapters and snapshot test updates.

Assumptions

- Stdio transport remains primary in near term; SSE/WS can follow.
- Safety controls remain in Codex core/server; SDK is transport/types only.

Decisions (initial)

- Feature‑flagged migration (`rmcp_sdk`) with strangler pattern.
- Preserve fully‑qualified tool naming and timeouts.

## 11) Deliverables

- This document: `MCP-CONVERSION.doc` (you are reading it).
- Inventory JSON: `docs/mcp-inventory.json` (complete coverage of MCP usage/references).
- Parity matrix CSV: `docs/mcp-parity.csv`.
- Design diagrams: included inline as text; optional links can be added as images later.
- Test plan: section 6 above; detailed cases to live under each crate’s `tests/` once implemented.

## 12) Immediate Questions

- Do we depend on private MCP extensions not present in `rmcp` (e.g., Codex‑specific notifications beyond what we already model)?
- Required transports in production: remain stdio only, or add SSE/WS during this migration?
- Compliance constraints (PII, retention) that might affect log content in SDK adapters?
- Target platforms that must be supported on day one (Windows stdio child process is already supported by current code; confirm SDK equivalence)?
- Any hard SLOs that must be met (latency/availability) to define acceptance gates?

Acceptance Criteria

- Zero capability, safety, or security regressions.
- All tests in §6 pass; perf meets thresholds in §7.
- Inventory and parity matrix complete and reviewed.
- Legacy MCP code removed or behind a minimal adapter with deprecation timeline.
- Rollback path verified in staging.

