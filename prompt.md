# Agent Instructions — Verify Dual‑Path MCP (Legacy & Official rmcp)

You are in the `codex-rs` workspace. Your task is to **verify** that:
1) The **official** Rust MCP SDK path is implemented and works (initialize/list/call, approvals if wired),
2) The **legacy** custom implementation still works,
3) Users can select **either** implementation at startup on both **server and client**, and
4) Defaults remain **legacy** when no flags/env/config are set.

Use `todo.md` at the repo root as your checklist and update it as you go.

---

## What to Verify

### 1) Runtime selection (server & client)
- Flags:
  - Server: `--mcp-impl legacy|official`
  - Client: `--mcp-client-impl legacy|official`
- Env:
  - `CODEX_MCP_IMPL=legacy|official`
  - `CODEX_MCP_CLIENT_IMPL=legacy|official`
- Config (optional defaults): `~/.codex/config.toml`
  ```toml
  mcp_impl = "legacy"
  mcp_client_impl = "legacy"
  ```

- Precedence MUST be: **flag > env > config > legacy**.

**Actions**

- Boot each combination (legacy/official) and confirm logs show the chosen path.
- Verify client and server can be chosen independently.

### 2) Server (official) initialize/list/call

- `initialize` returns:
  - `protocolVersion = mcp_types::MCP_SCHEMA_VERSION`
  - `capabilities.tools.listChanged = true`
  - `serverInfo { name="codex-mcp-server", title="Codex", version=CARGO_PKG_VERSION }`
  - **Note:** `user_agent` is **not** present in rmcp models (legacy‑only field).
- `tools/list` exposes `codex` and `codex-reply` (schemas match legacy).
- `tools/call`:
  - `codex` runs full Codex session and returns final `CallToolResult`.
  - `codex-reply` replies in an existing conversation.
  - Forward notifications to rmcp `notifications/logging/message`.

**Actions**

```powershell
# Parity of tool names
RUST_LOG=codex_mcp_server=debug `
  cargo test -p codex-mcp-server test_tools_list_names_parity_between_impls -- --nocapture

# Official mocked call (should hit mock and return assistant message)
RUST_LOG=codex_core=debug,codex_mcp_server=debug,rmcp=info `
  cargo test -p codex-mcp-server test_official_codex_tool_mocked_call_returns_final_message -- --nocapture
```

### 3) Approvals (elicitation)

- Legacy approvals already covered by existing tests.
- Official path: if raw pass‑through is implemented, verify approval prompts preserve Codex‑specific fields (e.g., `codex_mcp_tool_call_id`).
- If not yet implemented, mark the gap in `todo.md` and confirm legacy remains fully functional.

**Actions**

```powershell
# Legacy approval trigger (should pass)
cargo test -p codex-mcp-server --test all -- suite::codex_tool::test_shell_command_approval_triggers_elicitation -- --nocapture

# Official approval tests (enable once implemented)
# cargo test -p codex-mcp-server test_official_..._elicitation -- --nocapture
```

### 4) Client wrapper (official)

- `mcp-client/src/rmcp_wrapper.rs` initialize/list/call work against both servers.
- Flag/env/config selection verified as in (1).

**Actions**

```powershell
$env:CODEX_MCP_CLIENT_IMPL="official"
cargo test -p codex-mcp-client -- --nocapture
```

### 5) Docs & Precedence

- README documents flags, env vars, config keys, defaults, and known differences (no `user_agent` in rmcp).
- Commands are accurate for PowerShell and Unix shells.

---

## Deliverables

- Updated checkboxes in `todo.md`.
- Test outputs (paste when asked) for:
  - Parity test
  - Official mocked call
  - Legacy approval trigger
- If official approvals are wired: logs proving Codex‑specific fields round‑trip.

## Guardrails

- Do **not** modify or add logic related to `CODEX_SANDBOX_*` env vars.
- Keep changes minimal and focused on verification.
- Prefer scoped `clippy` and targeted tests; no repo‑wide sweeping changes.