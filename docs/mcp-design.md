# MCP Migration — Target Design (using rmcp)

Client‑side (Codex as MCP client)

```
+-------------------------+        +---------------------------+
| codex_core              |        | External MCP Server(s)    |
|  McpConnectionManager   |        |  (stdio child processes)  |
|    ┌─────────────────┐  |  JSON  |                           |
|    | McpClient (new) |<==========> rmcp server impl          |
|    |  (rmcp wrapper) |  |  RPC   |   (various vendors)       |
|    └─────────────────┘  |        +---------------------------+
|   qualify tools + timeouts        ^
|   call_tool() w/ events           |
+-------------------------+        spawn via tokio::process::Command
```

- Replace legacy `mcp-client` with a thin wrapper around `rmcp` client primitives.
- Keep stdio transport via `TokioChildProcess`. Streamable HTTP may be added later behind a non-default feature; WebSocket remains custom and off by default.
- Preserve FQ tool naming and timeouts; add notification dispatch hooks.

Server‑side (Codex as MCP server)

```
 stdin/stdout                         Codex internals
     |                                       |
     v                                       v
+------------+    JSON RPC    +---------------------------+
| rmcp       |<==============>| MessageProcessor          |
| server     |                |  - initialize/tools/call  |
| handlers   |                |  - bridge to Codex        |
+------------+                |  - approvals/elicitation  |
     ^                        +---------------------------+
     |                                 |
     |                                 v
     +------------------------> OutgoingMessageSender -> notifications (codex/event)
```

- Implement tools `codex` and `codex-reply` via rmcp server handler traits.
- Stream Codex events back as notifications (preserving `_meta.requestId`).
- Maintain sandbox/approval flows and AuthManager wiring.

Data contracts

- Replace `mcp_types` usage with `rmcp` types; where shapes differ, provide small adapters and From/Into conversions.
- Pin SDK version; assert protocol date compatibility at initialization.

Observability

- Add tracing spans for `initialize`, `tools/list`, `tools/call`; record latency and outcome.
- Redact secrets; keep current logging destinations and levels (configurable via `RUST_LOG`).

Feature flag and rollout

- Feature `rmcp_sdk` gates the new path for canary; dual‑path comparison against goldens; fast rollback by disabling the feature.
