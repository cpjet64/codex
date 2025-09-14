<!-- file: todo.md -->
Task 01: Membrane and traits
----------------------------
- [x] Create codex‑mcp‑types crate with neutral DTOs
- [x] Define ToolSpec, ToolCall, ToolResult DTOs
- [x] Add serde + schemars derives for DTOs
- [x] Create iface module with McpClient trait
- [x] Create iface module with McpServer trait
- [x] Add codex‑common crate for shared helpers
- [x] Move sanitizer into codex‑common
- [ ] Ensure no rmcp imports outside SDK adapter
- [x] Ensure no rmcp imports outside SDK adapter
- [ ] Ensure no legacy JSON types leak to core
- [ ] Build passes; no API changes to core

Task 02: CLI toggle and precedence
----------------------------------
- [x] Keep --mcp‑client‑impl and --mcp‑server‑impl
- [ ] Add --mcp‑client‑url for HTTP client
- [x] Add --mcp‑client‑url for HTTP client
- [ ] Add --mcp‑client‑sse for SSE client
- [x] Add --mcp‑client‑sse for SSE client
- [ ] Add --mcp‑server‑http and --mcp‑path
- [x] Compute effective impl once in CLI
- [ ] Compute effective transport once in CLI
- [x] Compute effective transport once in CLI
- [x] Remove env overrides in lower layers
- [ ] Add unit tests for precedence logic
- [ ] Update help text and README flags
- [ ] Verify default leaves legacy unchanged

Task 03: SDK client transports
------------------------------
- [ ] Add new_stdio_child(program, args, env)
- [ ] Add new_streamable_http_client(url)
- [ ] Add new_sse_client(url)
- [ ] Select transport from CLI inputs
- [ ] Map DTOs to rmcp::model types
- [ ] Implement initialize/list/call on client
- [ ] Handle timeouts and cancellations
- [ ] Add simple HTTP client smoke test
- [ ] Add SSE client smoke test (optional)
- [ ] Document client transport choices

Task 04: SDK server transports
------------------------------
- [ ] Implement stdio server via SDK io transport
- [ ] Implement HTTP server via SDK service
- [ ] Bind to 127.0.0.1 by default
- [ ] Add optional SSE server if needed
- [ ] Map rmcp requests to DTOs and core
- [ ] Advertise accurate capabilities
- [ ] Add HTTP roundtrip test (init/list/call)
- [ ] Verify session header behavior
- [ ] Add graceful shutdown path
- [ ] Document server flags and endpoints

Task 05: Tool macros (first tool)
---------------------------------
- [ ] Choose one simple tool to migrate
- [ ] Implement with #[tool] and router macros
- [ ] Map DTOs to rmcp tool args/results
- [ ] Replace JSON bridge for this tool in SDK path
- [ ] Add schema golden test for list output
- [ ] Add call roundtrip test for the tool
- [ ] Ensure sanitizer not duplicated
- [ ] Keep legacy tool path unchanged
- [ ] Document macro usage and limits
- [ ] Commit with clear scope and tests

Task 06: Capabilities and lifecycle
-----------------------------------
- [ ] Set ServerCapabilities from real features
- [ ] Implement progress notifications
- [ ] Implement cancel handling
- [ ] Implement logging notifications
- [ ] Add progress test over stdio
- [ ] Add progress test over HTTP
- [ ] Add cancel smoke test
- [ ] Verify logging flow once
- [ ] Document lifecycle behavior
- [ ] Update README capability table

Task 07: Sanitization unification
---------------------------------
- [x] Move sanitize_tool_name to codex‑common
- [ ] Keep single implementation and tests
- [x] Clamp and hash over 64 chars
- [ ] Apply in legacy path where needed
- [ ] Prefer macro names in SDK path
- [ ] Remove duplicate sanitizers
- [x] Add unit tests for sanitizer cases
- [ ] Note normalization in docs
- [ ] Review all call sites for drift
- [ ] Commit small, focused diff

Task 08: Error handling and logs
--------------------------------
- [ ] Replace expect/unwrap in new code
- [ ] Map SDK errors to anyhow with context
- [ ] Add a small error type if needed
- [ ] Add stderr capture flag for dev
- [ ] Add error path unit tests
- [ ] Add timeout path test
- [ ] Add cancellation path test
- [ ] Prefix logs with adapter tag
- [ ] Keep log lines short and useful
- [ ] Document debug flags in README

Task 09: Cargo features and versions
------------------------------------
- [ ] Use caret rmcp version unless pin justified
- [ ] Split client and server features per crate
- [ ] Remove unused feature flags
- [ ] Default features minimal (default = [])
- [ ] Add feature docs in Cargo.toml comments
- [ ] Add cargo tree feature check to CI
- [ ] Ensure no server features in client crate
- [ ] Ensure no client features in server crate
- [ ] Verify compile times stay reasonable
- [ ] Commit with config only changes

Task 10: Test matrix and parity
-------------------------------
- [ ] Add legacy‑stdio parity tests
- [ ] Add sdk‑stdio parity tests
- [ ] Add sdk‑http parity tests
- [ ] Add session header HTTP test
- [ ] Add progress tests for both transports
- [ ] Add cancel tests for both transports
- [ ] Keep tests within size limits
- [ ] Mark SSE tests optional or skipped
- [ ] Ensure tests do not create PRs
- [ ] Document how to run the tests

Task 11: Docs and examples
--------------------------
- [ ] Update README with new flags and flows
- [ ] Update CLI help text
- [ ] Add sdk‑http server example
- [ ] Add sdk‑http client example
- [ ] Add brief migration notes
- [ ] Add security note on bind address
- [ ] Keep examples under size limits
- [ ] Re‑wrap long doc lines to 75 chars
- [ ] Verify code fences compile
- [ ] Commit with docs only changes

Task 12: Security and cleanup
-----------------------------
- [ ] Default HTTP bind to loopback
- [ ] Warn on non‑loopback binds
- [ ] Delete stray backup files
- [ ] Remove dead_code allows in new code
- [ ] Confirm no secrets in repo
- [ ] Run semgrep_scan for quick pass
- [ ] Run security_check if available
- [ ] Note findings in todo "Notes"
- [ ] Final pass for size constraints
- [ ] Prepare follow‑up tasks list

Notes
-----
- Use only allowed MCP tools. No PRs or issues.
- Update checkboxes as you progress.
