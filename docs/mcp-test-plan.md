# MCP Migration — Test Plan

Scope

- Verify parity and regressions for MCP client/server using `rmcp` vs legacy.
- Cover functional, negative/adversarial, performance, and security tests.

Targets

- Client: initialization, tool discovery, tool calls, notifications.
- Server: initialization response, tool list, `codex` and `codex-reply` tools, elicitation flows.

Contract tests (examples)

- Initialize
  - `cargo test -p codex-mcp-client -- --nocapture`
  - Assert protocol version, capabilities.tools.listChanged=true.
- Tools list
  - `cargo test -p codex-core -- mcp list-tools`
  - Aggregate across servers; ensure FQ names and length rules.
- Tools call
  - Happy path: simple echo tool and `codex` tool with minimal prompt.
  - Adversarial: missing args, wrong types, oversized payloads (size limits).

Test IDs (for cross-referencing)

- CORE_MCP_LIST: aggregate list parity
- CORE_MCP_CALL: basic tools/call parity
- SRV_RES_LIST: server resources/list parity
- SRV_RES_READ: server resources/read parity
- CLI_NOTIF_ROUTE: notification routing on client
- CONC_SOAK_01: concurrency soak test
- CANCEL_01: cancellation timeout mapped
- RETRY_01: retry helper behavior
- ERRMAP_01: legacy→rmcp error mapping
- JSONRPC_OK: JSON-RPC compliance basic
- TRANS_STDIO: stdio transport behavior
- SCHEMA_COMP_01: schema conversion roundtrip
- VER_PIN: rmcp version pin check
- FF_RMCP: feature flag toggling
- AUTH_FLOW: login/auth flow parity
- SBX_EXEC: sandbox execution guard
- SECRETS_ENV: env filtering retained
- ARG_VALID: argument validation failure
- RLIM_01: rate limit rejection
- AUDIT_EVT: audit events present
- CI_SUPPLY: supply-chain CI gates
- PRIV_RED: privacy/PII redaction

Property-based tests

- Arbitrary serde_json::Value ↔ typed tool args/results round-trip.

Fuzzing targets (cargo-fuzz)

- JSONRPCMessage decoder.
- Server handler for `tools/call` (argument parsing).

Chaos/fault injection

- Child stdio: partial writes/reads, abrupt exit, slow server.
- Timeouts: startup/list/call; validate cleanup of pending map.

Soak tests

- Sustained concurrent `tools/call` with bounded channels; monitor RSS/FDs.

Security tests

- Auth flows (login/status) where applicable.
- Exec/patch approvals; ensure denial blocks server action.
- Path traversal inputs; ensure rejection.
- Network egress blocked under sandbox when expected.

Goldens

- Capture legacy JSON for `tools/list` and representative `tools/call`.
- Compare byte-wise or normalized JSON against SDK path.

Performance gates

- Collect p50/p95 for startup/list/call; RSS; CPU.
- Fail build if > +5% p95 vs baseline.

CI steps

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-features -D warnings`
- `cargo test --all-features`
- `cargo audit` and `cargo deny check`

Notes

- For snapshot tests in `codex-rs/tui`, update with `cargo insta accept -p codex-tui` only after intentional UI deltas.
