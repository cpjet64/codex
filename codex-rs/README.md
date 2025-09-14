# Codex CLI (Rust Implementation)

We provide Codex CLI as a standalone, native executable to ensure a
zero-dependency install.

## Installing Codex

Today, the easiest way to install Codex is via `npm`, though we plan to
publish Codex to other package managers soon.

```shell
npm i -g @openai/codex@native
codex
```

You can also download a platform-specific release directly from our
GitHub Releases.

## What's new in the Rust CLI

While we are working to close the gap between the TypeScript and Rust
implementations of Codex CLI, note that the Rust CLI has a number of
features that the TypeScript CLI does not.

### Config

Codex supports a rich set of configuration options. Note that the Rust
CLI uses `config.toml` instead of `config.json`. See
`docs/config.md` for details.

### Model Context Protocol Support

Codex CLI functions as an MCP client that can connect to MCP servers on
startup. See the `mcp_servers` section in the configuration docs.

You can also launch Codex as an MCP server by running `codex mcp`. Use
`@modelcontextprotocol/inspector` to try it out.

```shell
npx @modelcontextprotocol/inspector codex mcp
```

When running as a server, Codex honors the same sandbox and approval
settings as the interactive CLI.

#### MCP lifecycle (official)

- Progress: emits `progress/update` at start and end of a `codex`
  tool call. Over HTTP, forwarded as `logging/message` with
  `logger = "progress"`.
- Cancel: SDK client exposes `cancel()` which forwards to server.
- HTTP: adds `Mcp-Session-Id` header on successful responses.

Capability summary

- Tools: `tools.listChanged = true`
- Progress: `start/end` markers
- Cancel: supported via SDK
- HTTP: `Mcp-Session-Id` header

Server HTTP bind

- Bind address is configured by `CODEX_MCP_SERVER_HTTP_BIND`, e.g.:
  `CODEX_MCP_SERVER_HTTP_BIND=127.0.0.1:18123`
- Default is loopback for safety. Non-loopback binds are warned.
- Stop the server with `Ctrl+C`.

#### Run SDK tests locally

- SDK client: `cargo test -p codex-mcp-client --features sdk`
- SDK adapter: `cargo test -p codex-mcp-sdk --features rmcp`
- Server (official): `CODEX_MCP_IMPL=official cargo test -p codex-mcp-server`

Tip: HTTP tests use a loopback port; set `RUST_TEST_THREADS=1` to avoid
conflicts.

## Code Organization

This folder is the root of a Cargo workspace. Here are the key crates:

- `core/` contains core logic for Codex.
- `exec/` headless CLI for automation.
- `tui/` fullscreen TUI built with Ratatui.
- `cli/` multitool that provides the CLIs via subcommands.
