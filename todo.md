# MCP Migration to Official Rust SDK (rmcp) — TODO

> Source of truth for scope, acceptance criteria, and verification.
> Default runtime stays **legacy** until the community fully migrates. A startup flag (and env/config) selects **official**.

---

## Status Summary

- [x] **A. Runtime selection (server & client)**
- [x] **B. Server (official) parity: initialize / tools.list / tools.call**
- [x] **C. Approval (elicitation) parity for official**
- [x] **D. Client wrapper parity & selection**
- [x] **E. Tests & parity suite**
- [x] **F. Config & precedence**
- [ ] **G. CI jobs**
- [x] **H. Documentation**
- [ ] **I. De-risk / removal plan**

---

## A. Runtime Selection (Server & Client)

**Goal:** Users can run legacy or official MCP paths interchangeably via flags/env/config. Defaults remain **legacy**.

- **Verification notes**
  - Server impl selection logs observed in tests: legacy in parity test; official in mocked call and approvals parity.
  - Client impl selection verified via env (`CODEX_MCP_CLIENT_IMPL=official`) in client tests.

- [ ] **Server flag**: `--mcp-impl legacy|official`
  - Code: `mcp-server/src/main.rs` (arg parsing + logging selection)
  - Env: `CODEX_MCP_IMPL=legacy|official`
  - Config: `~/.codex/config.toml` → `mcp_impl = "legacy" | "official"`
  - Precedence: `flag > env > config > legacy`
  - **Acceptance:** Selection logged at startup; effective path used.

- [ ] **Client flag**: `--mcp-client-impl legacy|official`
  - Code: `codex-cli`/`mcp-client` entrypoint (arg parsing)
  - Env: `CODEX_MCP_CLIENT_IMPL=legacy|official`
  - Config: `~/.codex/config.toml` → `mcp_client_impl = "legacy" | "official"`
  - Precedence: `flag > env > config > legacy`
  - **Acceptance:** Client wrapper chosen at startup; confirmed via logs.

**Quick checks**
```powershell
# Server (official)
$env:CODEX_MCP_IMPL="official"; cargo test -p codex-mcp-server --test all -- --nocapture

# Client (official)
$env:CODEX_MCP_CLIENT_IMPL="official"; cargo test -p codex-mcp-client -- --nocapture
```

---

## B. Server (Official Path) Parity

**Goal:** Official server implements `initialize`, `tools/list`, `tools/call` with behavior parity (known schema differences documented).

- [ ] **initialize**
  - File: `mcp-server/src/rmcp_handlers.rs`
  - Must return `protocolVersion = mcp_types::MCP_SCHEMA_VERSION`, `capabilities.tools.listChanged = true`,
    `serverInfo { name = "codex-mcp-server", title = "Codex", version = CARGO_PKG_VERSION }`
  - **Note:** `user_agent` is legacy‑only; **not present** in rmcp model. Update tests accordingly.
  - **Acceptance:** Official initialize test passes.

- [ ] **tools/list**
  - Convert legacy tool schemas (`codex`, `codex-reply`) to `rmcp::model::Tool` via serde.
  - **Acceptance:** Parity test compares **sorted** tool names across legacy/official and passes.

- [ ] **tools/call**
  - `codex` → run full Codex session via existing runner; return `CallToolResult`.
  - `codex-reply` → conversation lookup + reply via existing runner.
  - Forward streaming/log notifications to `notifications/logging/message`.
  - **Acceptance:** Official mocked call test hits mock and returns final assistant message.

**Verification notes**
  - Parity of tool names between legacy and official passed:
    - `cargo test -p codex-mcp-server suite::parity::test_tools_list_names_parity_between_impls -- --exact`
  - Official mocked call returned final assistant message:
    - `cargo test -p codex-mcp-server suite::codex_tool::test_official_codex_tool_mocked_call_returns_final_message -- --exact`

**Quick checks**

```powershell
RUST_LOG=codex_core=debug,codex_mcp_server=debug,rmcp=info `
  cargo test -p codex-mcp-server test_official_codex_tool_mocked_call_returns_final_message -- --nocapture
```

---

## C. Approval (Elicitation) Parity for Official

**Goal:** Approval prompts (exec/patch) on the **official** path behave like legacy, preserving Codex‑specific fields.

- [ ] **Raw pass‑through of elicitation** from server→client in official mode
  - Where: `mcp-server/src/rmcp_handlers.rs` in the loop reading `out_rx.recv()` from the runner.
  - Convert `OutgoingMessage::Request(req)` into a raw JSON‑RPC request over rmcp peer **without** losing extra Codex fields
    (e.g., `codex_mcp_tool_call_id`, `codex_event_id`).
  - Await response and `notify_client_response(req.id, result)` back to the runner.
  - **Acceptance:** Official approval test passes offline with mocks; fields preserved.

- [ ] **Tests**
  - Add/enable official approval tests mirroring legacy: exec approval & patch approval.
  - **Acceptance:** Both approval flows pass on official path.

**Verification notes**
  - Official approvals parity passed:
    - `cargo test -p codex-mcp-server suite::codex_tool::test_official_shell_and_patch_approvals_parity -- --exact`

*If raw pass-through isn't possible in rmcp*, branch tests and document the limitation; re-attempt when SDK supports raw forwarding.

---

## D. MCP Client Wrapper Parity

**Goal:** `mcp-client/src/rmcp_wrapper.rs` maps `mcp_types` to rmcp client correctly; selection via startup flag.

- [ ] rmcp client initialize/list/call matches legacy semantics (with `user_agent` caveat).
- [ ] Tests ensure client can talk to both server impls.
- **Acceptance:** Client-side list/call tests pass with `--mcp-client-impl official`.

**Verification notes**
  - Official client list test passed with feature enabled:
    - `cargo test -p codex-mcp-client --features rmcp_sdk -- --nocapture`

---

## E. Tests & Parity Suite

- [ ] **Parity: tool names**
  - `suite::parity::test_tools_list_names_parity_between_impls`
- [ ] **Official list (client)**
  - `mcp-client/tests/official_list.rs`
- [ ] **Official mocked call**
  - `suite::codex_tool::test_official_codex_tool_mocked_call_returns_final_message`
- [ ] **Legacy approval trigger**
  - `suite::codex_tool::test_shell_command_approval_triggers_elicitation`
- [ ] **Official approvals (after C)**
  - Add mirrored approval tests.
- **Acceptance:** All above pass on Windows & CI.

**Verification notes**
  - Windows runs passed:
    - `suite::parity::test_tools_list_names_parity_between_impls`
    - `suite::codex_tool::test_official_codex_tool_mocked_call_returns_final_message`
    - `suite::codex_tool::test_shell_command_approval_triggers_elicitation`
    - `mcp-client/tests/official_list.rs` with `--features rmcp_sdk`

---

## F. Config & Precedence

- [ ] `~/.codex/config.toml` keys:
  ```toml
  mcp_impl = "legacy"   # or "official"
  mcp_client_impl = "legacy"  # or "official"
  ```
- [ ] Precedence: **flag > env > config > legacy** (documented & tested)
- **Acceptance:** Unit/integration checks verify effective choice.

**Verification notes**

- Server precedence verified via CLI integration tests (see `codex-rs/cli/tests/precedence.rs`):
  - Config only (official) selects official.
  - Env legacy over config official selects legacy.
  - CLI official over env legacy selects official.
- Client precedence verified via unit test in `mcp-client` crate:
  - `client_precedence_override_beats_env` confirms process override takes precedence over env as documented.

---

## G. CI

- [ ] Jobs:
  - `fmt` (stable); `clippy -D warnings`
  - Server tests: legacy & official (via env/flag)
  - Client tests: legacy & official
- **Acceptance:** CI green with both impls.

---

## H. Docs

- [ ] README: how to pick impl (server/client), known differences (no `user_agent` in rmcp).
- [ ] Example commands for Windows (PowerShell) & Unix shells.
- **Acceptance:** Docs render correctly; commands tested.

---

## I. De‑Risk & Removal Plan

- [ ] Keep legacy default **until** community migration is complete.
- [ ] Track SDK raw‑forwarding capability; switch official approvals when viable.
- [ ] When removing legacy:
  - Archive tests & docs; flip default; remove flags; provide migration notes.

---

## Acceptance Gate (All Up)

- Every test in §E passes on Windows/CI.
- Flags/env/config precedence validated (§A, §F).
- Official approvals function or are explicitly documented & gated (§C).
- README/docs updated (§H).
