<!-- file: prompt.md -->
Goal
----
Fully utilize the official Rust MCP SDK side by side with legacy,
with a hard adapter membrane, one runtime toggle, and zero behavior
change for legacy users.

Scope
-----
Implement SDK client transports (stdio, Streamable HTTP, SSE),
SDK server transports (stdio, Streamable HTTP; SSE optional),
typed tool routers via SDK macros, precise capabilities, lifecycle
(progress, cancel, logging), unified sanitization, robust error
mapping, minimal Cargo features, test matrix, docs, and security.

Non‑goals
---------
Do not remove legacy in this pass. Do not create PRs or issues.
Do not alter default legacy behavior unless the CLI toggle is used.

Branch and repo
---------------
Work only on branch:
feat/mcp-rust-sdk-conversion

Key constraints
---------------
- Each file ≤200 lines; each line ≤75 chars.
- If a function is too long, split into smaller functions.
- Keep cyclomatic complexity low; prefer early returns.
- Do not import rmcp::* outside the SDK adapter crate.
- Respect precedence: CLI > ENV > config > default.
- Do not re‑read env in lower layers once CLI has decided.
- No PRs or issues. Commit directly to the existing branch.

MCP tools to use
----------------
- github‑official:
  - list_branches, get_file_contents
  - create_or_update_file, push_files
  - search_code, request_copilot_review (optional)
  - Do not use PR or issue tools.
- semgrep_scan, security_check (static analysis)
- docker (optional for local checks), curl (for HTTP smoke tests)

Commit policy
-------------
- Conventional commits with short subject ≤72 chars.
- One atomic concern per commit. Include Why and How in body.
- Do not squash unrelated changes. Keep diffs readable.

Work plan
---------
- Follow todo.md in order. Do not skip tasks.
- For each task, update todo.md checkboxes as you complete steps.
- If a step fails, stop and note the failure in a short comment
  block at the end of todo.md under a "Notes" section.

Definition of done
------------------
- SDK path supports stdio and Streamable HTTP end to end.
- Optional SSE path smokes (if enabled).
- At least one tool uses SDK macros end to end.
- Capabilities reflect real features.
- Progress, cancel, logging flow is validated.
- Precedence bug fixed; tests enforce it.
- One sanitizer shared; no duplicates.
- Cargo features minimal and correct per crate.
- CI matrix covers legacy‑stdio, sdk‑stdio, sdk‑http.
- README and help updated; examples compile.

Execution notes
---------------
- Keep every new or edited source file within limits.
- Prefer small modules over long files.
- Use adapters to map neutral DTOs to rmcp::model types.
- Keep legacy adapter unmodified except for shared utilities.
- Bind HTTP server to 127.0.0.1 by default. Warn on non‑loopback.
- Never commit secrets. Use placeholders in examples.

Reporting
---------
- After each major task, add a brief "Progress:" line to todo.md.
- Do not post comments to external systems.
- All status lives in the branch through the files you edit.
