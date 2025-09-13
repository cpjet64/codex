# MCP Gates — CI Commands and Thresholds

Perf and resource gates (fail if exceeded vs baseline):

- p95 latency: +5% max for startup→tools/list and representative tools/call cases.
- RSS (resident set size): +5% max under representative load.

Commands (examples; adapt to CI matrix):

- Format/lint/tests:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-features -D warnings`
  - `cargo test --all-features`

- Supply chain:
  - `cargo audit --deny warnings`
  - `cargo deny check`

- Baselines and comparisons:
- Record baseline (legacy path):
  - `CODEX_MCP_IMPL=legacy cargo run -p codex-cli -- measure-mcp` → writes `ci/baseline.json`
  - Measure SDK path (official implementation):
  - `CODEX_MCP_IMPL=official cargo run -p codex-cli -- measure-mcp` → writes `ci/sdk.json`
  - Compare (fail on regression):
    - `scripts/ci/compare-mcp-metrics.py --baseline ci/baseline.json --candidate ci/sdk.json --max-delta 0.05`

Matrix:

- OS: Ubuntu-latest, macOS-latest, Windows-latest
- Arch: x86_64 (required); aarch64 (optional nightly)

Notes:

- Feature flag `rmcp_sdk` controls SDK path.
- Store and version baselines per tag; update only on intentional perf work with justification.
