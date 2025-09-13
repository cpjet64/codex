## Advanced

## Non-interactive / CI mode

Run Codex head-less in pipelines. Example GitHub Action step:

```yaml
- name: Update changelog via Codex
  run: |
    npm install -g @openai/codex
    codex login --api-key "${{ secrets.OPENAI_KEY }}"
    codex exec --full-auto "update CHANGELOG for next release"
```

## Tracing / verbose logging

Because Codex is written in Rust, it honors the `RUST_LOG` environment variable to configure its logging behavior.

The TUI defaults to `RUST_LOG=codex_core=info,codex_tui=info` and log messages are written to `~/.codex/log/codex-tui.log`, so you can leave the following running in a separate terminal to monitor log messages as they are written:

```
tail -F ~/.codex/log/codex-tui.log
```

By comparison, the non-interactive mode (`codex exec`) defaults to `RUST_LOG=error`, but messages are printed inline, so there is no need to monitor a separate file.

See the Rust documentation on [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more information on the configuration options.

## Model Context Protocol (MCP)

Codex supports MCP both as a client (connecting to external servers) and as a server (exposing Codex via MCP). This section covers usage, flags, and examples.

### Use external MCP servers (client)

Configure servers with [`mcp_servers`](./config.md#mcp_servers) in `~/.codex/config.toml`. This mirrors `mcpServers` in other tools, but uses TOML:

```toml
# IMPORTANT: the top-level key is `mcp_servers` rather than `mcpServers`.
[mcp_servers.example]
command = "npx"
args = ["-y", "my-mcp-server"]
env = { API_KEY = "value" }
# Optional startup timeout for initialize + initial tools/list (default: 10_000 ms)
startup_timeout_ms = 15000
```

Run Codex and it will connect to your configured servers on demand. Tools will be qualified as `<server>__<tool>` in the UI.

Client implementation selection (optional):

- CLI: `codex --mcp-client-impl=official`
- Env: `CODEX_MCP_CLIENT_IMPL=official`
- Config: `mcp_client_impl = "official"`

Precedence: CLI > env > config > legacy.

Note: Only stdio servers are supported directly. For SSE transports, consider an adapter such as `mcp-proxy`.

### Run Codex as an MCP server

You can launch Codex as an MCP server and connect an MCP client to it:

```bash
# Launch with default (legacy) server implementation
codex mcp

# Or select the official SDK implementation
codex --mcp-impl=official mcp
```

Try it with the Inspector:

```bash
npx @modelcontextprotocol/inspector codex mcp
```

Server implementation selection (optional):

- CLI: `codex --mcp-impl=official mcp`
- Env: `CODEX_MCP_IMPL=official`
- Config: `mcp_impl = "official"`

Precedence: CLI > env > config > legacy.

See the [MCP implementation switches](./config.md#mcp-implementation-clientserver) section for full details.

### Legacy vs Official MCP

- Legacy implementation
  - Default path used by Codex today.
  - Stable and well‑tested in the CLI.
  - Recommended when you depend on current behavior or need maximal stability.

- Official implementation
  - Uses the Model Context Protocol’s official Rust SDK under the hood.
  - Opt‑in via flags, env, or config.
  - Recommended when evaluating SDK capabilities or if your external server expects newer protocol features supported by the SDK.

You can choose independently for the client and server roles. Use this to migrate safely (e.g., try the official server while keeping the client legacy).
