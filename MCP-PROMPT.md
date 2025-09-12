ACTION PROMPT — Close Gaps on MCP Migration (Align to Spec + rmcp 0.6.x)

You are to correct, complete, and harden the MCP migration deliverables so they match the spec and are production‑reviewable with a reversible rollout. No hand‑waving. Show diffs, commands, and numbers. If you hit an Immediate Question below, ask me at once; otherwise do not pause.

Starting points (existing files)

Write‑up: /docs/MCP-CONVERSION.doc (currently at repo root — move into docs/ and fix links). It claims deliverables sit under docs/; make that true. 

MCP-CONVERSION

Design: docs/mcp-design.md (ASCII diagrams, module boundaries). 

mcp-design

Test plan: docs/mcp-test-plan.md (contract, fuzzing, chaos, soak, perf). 

mcp-test-plan

Rollback plan: docs/mcp-rollback-plan.md (feature‑flag rmcp_sdk, canary → staged). 

mcp-rollback-plan

Inventory: docs/mcp-inventory.json (good start, not exhaustive; expand to include tests/fixtures/CI refs). 

mcp-inventory

Deliverables (must exist at the end)

docs/MCP-CONVERSION.doc — corrected write‑up (sections §3–§8 updated), internal links and paths fixed. 

MCP-CONVERSION

docs/mcp-inventory.json — exhaustive inventory including code, tests, fixtures, snapshots, docs, configs, CI jobs. Plus docs/mcp-inventory.csv (tabular). 

mcp-inventory

docs/mcp-callgraph.csv — per‑symbol fan‑in/fan‑out for all MCP touch‑points (headers: symbol,path,fan_in,fan_out,entrypoints).

docs/mcp-parity.csv — hardened parity matrix: adapter trait/fn signatures, timeout/cancel semantics, legacy→rmcp error mapping, schema diffs, test IDs, and status (prototype|ready|blocked) with owner/date.

docs/mcp-security-mapping.md — one‑to‑one mapping from each security/safety control (authn/z, approvals/elicitation, sandboxing, secrets/env filtering, input size limits, egress guards, rate‑limits, audit, supply chain, privacy) to the exact rmcp hooks/handlers, with at least one negative test per control.

ci/mcp-gates.md — exact CI gates (commands + perf thresholds) to fail on regression (p95 latency & RSS).

Minimal dual‑path code guarded by rmcp_sdk feature (compile‑time or runtime flag): a thin client wrapper over rmcp TokioChildProcess and server handlers stubs sufficient to run the test harness and golden comparisons locally.

Tasks (do all, in order)
1) Fix names/paths + links (truth over convenience)

Move MCP-CONVERSION.doc into docs/ and update all internal links to docs/*. The current doc says everything is under docs/ — make it reality. 

MCP-CONVERSION

2) Align to the official spec and current SDK

Transports: Update wording to match the spec (Protocol Revision 2025‑03‑26): standard transports = stdio + Streamable HTTP; HTTP+SSE is deprecated (superseded by Streamable HTTP), and SSE is optional inside Streamable HTTP. WebSocket is custom/non‑standard, behind a non‑default feature. Update §3/§5/§8 accordingly. 
Model Context Protocol

Version pin: In §8 and “Getting Started,” pin rmcp to =0.6.4 (or the latest minor you certify), list features explicitly, and document upgrade policy (cargo audit, cargo deny, release checklist). 
Docs.rs

Replace any stale snippet like =0.2.0 in the doc.

3) Make the inventory truly exhaustive

Extend docs/mcp-inventory.json to include:

Tests: unit/integration/contract names/paths; fixtures; golden snapshots; snapshot updaters (e.g., insta).

Docs/config/CI refs already present — keep and expand.

Add fields: test_refs, ci_refs, fan_in, fan_out, risk.

Generate docs/mcp-callgraph.csv from ctags/LSP references (symbol → fan‑in/fan‑out). See current key artifacts (client, server, core, TUI) and ensure they’re covered. 

mcp-inventory

4) Harden the parity matrix

For every line marked gap/partial/“adapter needed”:

Provide adapter interfaces (Rust trait/fn signatures).

Map legacy errors → rmcp::ErrorData; define codes and messages.

Show schema diffs (old mcp_types vs rmcp types) and how adapters convert.

Reference test IDs (from docs/mcp-test-plan.md) that prove parity. 

mcp-test-plan

5) Security mapping (no regression allowed)

For each control, point to specific rmcp integration points: e.g., server handler traits, #[tool]/#[tool_router]/elicit_safe macros for structured output & elicitation, request filters, and redaction hooks.

Add negative tests (auth bypass, path traversal, oversized payload, blocked egress) and expected logs/audit evidence. Output goes in docs/mcp-security-mapping.md and is cross‑linked from §4 of MCP-CONVERSION.doc. 

MCP-CONVERSION

6) Baselines, goldens, and CI gates

Capture baselines: startup→tools/list, representative tools/call p50/p95, RSS/CPU.

Create goldens for:

tools/list aggregate

at least 3 tools/call cases (one streams progress, one errors)

Wire CI gates: fail if p95 latency or RSS regresses by > +5% vs baseline. Document exact commands in ci/mcp-gates.md and link from §8. 

mcp-test-plan

7) Wire the feature‑flagged dual path (enough for tests)

Add rmcp_sdk flag.

Implement a thin client wrapper over rmcp TokioChildProcess preserving FQ tool naming and timeout semantics, and server stubs for codex/codex-reply.

Keep the legacy path intact; enable side‑by‑side test selection to compare goldens.

Return unified diffs for added files and changes.

8) Update the write‑up and diagrams

In MCP-CONVERSION.doc §3–§8, reflect the transport correction, version pin, adapters, tests, and CI gates.

Ensure all links/paths resolve under docs/.

Keep the ASCII diagrams in docs/mcp-design.md aligned with the new wording. 

mcp-design

Acceptance criteria (hard line)

Zero capability/safety/security regressions.

Inventory and parity matrix are complete, with concrete adapter signatures and mapped tests.

Transport language matches the spec (stdio + Streamable HTTP standard; WS behind a flag as custom). 
Model Context Protocol

rmcp is pinned (>= 0.6.x) with supply‑chain checks in CI. 
Docs.rs

Goldens for tools/list and 3× tools/call match between legacy and SDK paths.

Rollback path (rmcp_sdk off) verified per docs/mcp-rollback-plan.md. 

mcp-rollback-plan

Immediate questions (blockers → ask me immediately)

Any private MCP extensions we rely on that rmcp doesn’t cover?

Do we need remote transport in this migration (Streamable HTTP now) or is stdio‑only sufficient for parity? 
Model Context Protocol

Compliance constraints (PII/residency/retention) affecting logs or adapters?

Target platforms (Windows/macOS/Linux; x86_64/aarch64) that must be green day one?

SLOs (latency/availability) that double as acceptance gates?

Return format (don’t deviate)

Bulleted list of created/updated files & paths.

One paragraph summarizing changes in §3–§8 of docs/MCP-CONVERSION.doc. 

MCP-CONVERSION

The CI run result (pass/fail) and which gate failed if any.

A patch/diff block (unified) for the code and docs you touched.