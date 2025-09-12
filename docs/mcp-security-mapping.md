# MCP Security & Safety Mapping — Legacy → rmcp

This mapping preserves all security/safety controls with explicit integration points in rmcp.

- AuthN/AuthZ
  - Integration: server notifications and request handlers; preserve AuthManager and scope checks.
  - rmcp hooks: server handler layer (request routers) and notification APIs.
  - Negative tests: auth bypass attempt → expect error code -32601 (method not found) or domain-specific denial plus audit event.

- Approvals / Elicitation (exec/patch)
  - Integration: keep elicitation flows (exec_approval.rs, patch_approval.rs) and map to rmcp request/response.
  - rmcp hooks: tool handlers can block pending approval; use structured replies.
  - Negative tests: denial path blocks command/patch; audit shows decision and parameters.

- Sandboxing & Isolation
  - Integration: unchanged Seatbelt/Linux sandbox invocation paths (no change allowed).
  - rmcp hooks: none needed; sandboxing occurs in Codex exec layers invoked by tools.
  - Negative tests: attempt network egress or filesystem escape under restricted policy → denied with audit evidence.

- Secrets / Environment Filtering
  - Integration: retain `create_env_for_mcp_server` and shell env policy; do not broaden.
  - rmcp hooks: child process transport uses curated env; ensure filters applied.
  - Negative tests: variables with KEY/SECRET/TOKEN patterns absent unless explicitly included.

- Input Validation & Size Limits
  - Integration: validate JSON args against schema where available; enforce size caps.
  - rmcp hooks: typed params; custom validation before handler execution.
  - Negative tests: oversized payload → error -32602 (invalid params) and redacted logs.

- Rate Limits & Quotas
  - Integration: per-conversation quotas enforced at Codex layer.
  - rmcp hooks: pre-handler guard.
  - Negative tests: exceed quota → rejection and audit counter increment.

- Audit Logging
  - Integration: continue to emit Codex events via notifications with `_meta.requestId`.
  - rmcp hooks: notification APIs.
  - Negative tests: verify audit trail for denied/errored operations.

- Supply Chain Security
  - Integration: pin rmcp `=0.6.4`; CI runs `cargo audit` and `cargo deny`.
  - rmcp hooks: n/a.
  - Negative tests: CI fails on vulnerable dependency or denied license.

- Privacy/PII
  - Integration: keep redaction rules for logs and transcripts; never emit secrets.
  - rmcp hooks: centralized logging/formatting.
  - Negative tests: PII-like strings are masked in outputs.
