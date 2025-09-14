<!-- file: todo.md -->
Task 01: Membrane and traits
----------------------------
- [x] Create codex-mcp-types crate with neutral DTOs
- [x] Define ToolSpec, ToolCall, ToolResult DTOs
- [x] Add serde + schemars derives for DTOs
- [x] Create iface module with McpClient trait
- [x] Create iface module with McpServer trait
- [x] Add codex-common crate for shared helpers
- [x] Move sanitizer into codex-common
- [ ] Ensure no rmcp imports outside SDK adapter
- [ ] Ensure no legacy JSON types leak to core
- [ ] Build passes; no API changes to core

Task 02: CLI toggle and precedence
----------------------------------
- [x] Keep --mcp-client-impl and --mcp-server-impl
- [x] Add --mcp-client-url for HTTP client
- [x] Add --mcp-client-sse for SSE client
- [x] Add --mcp-server-http and --mcp-path
- [x] Compute effective impl once in CLI
- [x] Compute effective transport once in CLI
- [x] Remove env overrides in lower layers
- [x] Add unit tests for precedence logic
- [x] Update help text and README flags
- [ ] Verify default leaves legacy unchanged

Task 03: SDK client transports
------------------------------
- [x] Add new_stdio_child(program, args, env)
- [x] Add new_streamable_http_client(url)
- [x] Add new_sse_client(url)
- [x] Select transport from CLI inputs
- [ ] Map DTOs to rmcp::model types
- [x] Implement initialize/list/call on client
- [ ] Handle timeouts and cancellations
- [x] Add simple HTTP client smoke test
- [ ] Add SSE client smoke test (optional)
- [x] Document client transport choices

Task 04: SDK server transports
------------------------------
- [x] Implement stdio server via SDK io transport
- [x] Implement HTTP server via SDK service
- [x] Bind to 127.0.0.1 by default
- [ ] Add optional SSE server if needed
- [ ] Map rmcp requests to DTOs and core
- [x] Advertise accurate capabilities
- [x] Add HTTP roundtrip test (init/list/call)
- [x] Verify session header behavior
- [ ] Add graceful shutdown path
- [x] Document server flags and endpoints

Task 05: Tool macros (first tool)
---------------------------------
- [x] Choose one simple tool to migrate
- [x] Implement with #[tool] and router macros
- [x] Map DTOs to rmcp tool args/results
- [x] Replace JSON bridge for this tool in SDK path
- [x] Add schema golden test for list output
- [x] Add call roundtrip test for the tool
- [x] Ensure sanitizer not duplicated
- [ ] Keep legacy tool path unchanged
- [ ] Document macro usage and limits
- [ ] Commit with clear scope and tests

Task 06: Capabilities and lifecycle
-----------------------------------
- [ ] Set ServerCapabilities from real features
- [x] Implement progress notifications
- [x] Implement cancel handling
- [x] Implement logging notifications
- [x] Add progress test over stdio
- [ ] Add progress test over HTTP
- [x] Add cancel smoke test
- [x] Verify logging flow once
- [ ] Document lifecycle behavior
- [ ] Update README capability table

Task 07: Sanitization unification
---------------------------------
- [x] Move sanitize_tool_name to codex-common
- [x] Keep single implementation and tests
- [x] Clamp and hash over 64 chars
- [x] Apply in legacy path where needed
- [ ] Prefer macro names in SDK path
- [x] Remove duplicate sanitizers
- [x] Add unit tests for sanitizer cases
- [x] Note normalization in docs
- [ ] Review all call sites for drift
- [ ] Commit small, focused diff

Task 08: Error handling and logs
--------------------------------
- [ ] Replace expect/unwrap in new code
- [x] Map SDK errors to anyhow with context
- [ ] Add a small error type if needed
- [x] Add stderr capture flag for dev
- [x] Add error path unit tests
- [x] Add timeout path test
- [ ] Add cancellation path test
- [ ] Prefix logs with adapter tag
- [ ] Keep log lines short and useful
- [ ] Document debug flags in README

Task 09: Cargo features and versions
------------------------------------
- [x] Use caret rmcp version unless pin justified
- [ ] Split client and server features per crate
- [x] Remove unused feature flags (HTTP/SSE enabled in SDK)
- [x] Add feature docs in Cargo.toml comments
- [ ] Default features minimal (default = [])
- [ ] Add cargo tree feature check to CI
- [ ] Ensure no server features in client crate
- [ ] Ensure no client features in server crate
- [ ] Verify compile times stay reasonable
- [ ] Commit with config only changes

Task 10: Test matrix and parity
-------------------------------
- [ ] Add legacy-stdio parity tests
- [ ] Add sdk-stdio parity tests
- [x] Add sdk-http parity tests
- [x] Add session header HTTP test
- [x] Add progress tests for both transports (stdio only)
- [x] Add cancel tests for both transports
- [ ] Keep tests within size limits
- [ ] Mark SSE tests optional or skipped
- [ ] Ensure tests do not create PRs
- [ ] Document how to run the tests

Task 11: Docs and examples
--------------------------
- [x] Update README with new flags and flows
- [x] Update CLI help text
- [x] Add sdk-http server example
- [x] Add sdk-http client example
- [x] Add brief migration notes
- [x] Add security note on bind address
- [ ] Keep examples under size limits
- [ ] Re-wrap long doc lines to 75 chars
- [ ] Verify code fences compile
- [ ] Commit with docs only changes

Task 12: Security and cleanup
-----------------------------
- [x] Default HTTP bind to loopback
- [x] Warn on non-loopback binds
- [ ] Delete stray backup files
- [ ] Remove dead_code allows in new code
- [ ] Confirm no secrets in repo
- [ ] Run semgrep_scan for quick pass
- [ ] Run security_check if available
- [ ] Note findings in todo "Notes"
- [ ] Final pass for size constraints
- [ ] Prepare follow-up tasks list

Notes
-----
- Use only allowed MCP tools. No PRs or issues.
- Update checkboxes as you progress.
- Progress: added HTTP cancel + timeout tests; added stdio progress test;
  mapped progress/update to logger=progress; emitted start/end markers.
- Progress: documented cargo features (SDK, server) and switched rmcp
  versions to caret.
- Note: semgrep_findings blocked by missing token; will run when
  SEMGREP_APP_TOKEN is configured.
