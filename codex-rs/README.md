# Codex CLI (Rust Implementation)

We provide Codex CLI as a standalone, native executable to ensure a zero-dependency install.

## Installing Codex

Today, the easiest way to install Codex is via `npm`, though we plan to publish Codex to other package managers soon.

```shell
npm i -g @openai/codex@native
codex
```

You can also download a platform-specific release directly from our [GitHub Releases](https://github.com/openai/codex/releases).

## What's new in the Rust CLI

While we are [working to close the gap between the TypeScript and Rust implementations of Codex CLI](https://github.com/openai/codex/issues/1262), note that the Rust CLI has a number of features that the TypeScript CLI does not!

### Config

Codex supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`docs/config.md`](../docs/config.md) for details.

### Model Context Protocol Support

Codex CLI functions as an MCP client that can connect to MCP servers on startup. See the [`mcp_servers`](../docs/config.md#mcp_servers) section in the configuration documentation for details.

You can also launch Codex as an MCP _server_ by running `codex mcp`. Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector codex mcp
```

When running as a server, Codex honors the same sandbox and approval settings as the interactive CLI. For example:

```bash
# Allow workspace writes in the sandbox and ask for approval on failure
codex --sandbox=workspace-write --ask-for-approval=on-failure mcp
```

#### MCP Implementation Switches (Server and Client)

Codex supports both the original “legacy” MCP implementation and the official Rust MCP SDK (RMCP). You can choose the implementation for both the MCP server (`codex mcp`) and the embedded MCP client (used when Codex connects to external MCP servers configured in `config.toml`). Defaults remain “legacy”.

- Server (codex mcp): selects how the Codex MCP server is implemented.
- Client (MCP connections): selects which client is used to talk to configured MCP servers.

Config defaults (recommended)

Add either or both to `~/.codex/config.toml` (Windows: `C:\Users\<you>\.codex\config.toml`). These act as defaults when no flag/env is provided.

```toml
# Server implementation default for `codex mcp`
#   values: "legacy" (default), "official"
mcp_impl = "legacy"

# Client implementation default when Codex connects to external MCP servers
#   values: "legacy" (default), "official"
mcp_client_impl = "legacy"
```

CLI flags (highest precedence)

- Server (when running `codex mcp`):
  - `--mcp-impl legacy|official`
  - Example: `codex --mcp-impl=official mcp`

- Client (when connecting to configured servers):
  - `--mcp-client-impl legacy|official`
  - Example: `codex --mcp-client-impl=official`

Environment variables

- Server: `CODEX_MCP_IMPL=legacy|official`
- Client: `CODEX_MCP_CLIENT_IMPL=legacy|official`

Precedence

- Server: CLI flag > `CODEX_MCP_IMPL` > `mcp_impl` in config > legacy
- Client: CLI flag > `CODEX_MCP_CLIENT_IMPL` (or process override) > `mcp_client_impl` in config > legacy

Notes

- The server and client switches are independent. You can run the Codex MCP server with the official SDK while still using the legacy client to connect to other servers, or vice versa.
- The flags act as a safe rollback switch; defaults stay on legacy until you explicitly opt into the official SDK.

For more examples and precedence rules, see [docs/advanced.md](../docs/advanced.md#model-context-protocol-mcp) and [docs/config.md](../docs/config.md#mcp-implementation-clientserver).

##### Migration note: Official MCP approvals

When using the official RMCP implementation for the Codex MCP server (selected via `--mcp-impl=official` or `mcp_impl = "official"`), approval requests are forwarded using the SDK’s typed “elicitation” API:

- Requests use method `elicitation/create` and include the standard fields (`message`, `requestedSchema`). Any Codex‑specific correlators are preserved under `params._meta` (for example: `codex_elicitation`, `codex_mcp_tool_call_id`, `codex_event_id`, `codex_call_id`, etc.).
- Clients must respond with the typed elicitation result shape, e.g. `{ "action": "accept" }` (or `"decline"`, `"cancel"`). The server maps this to Codex’s legacy decision internally so existing flows continue to work.

Legacy implementation differences for reference:

- Requests previously placed Codex fields at the top level of `params` and tests often replied with a legacy response like `{ "decision": "approved" }`.

During the transition:

- Defaults remain on the legacy path; use the flags to opt into the official SDK for server and/or client independently.
- CI should run both implementations to ensure feature parity.
- Once parity is stable, we will flip the default to the official SDK and keep the rollback flag for 1–2 releases before removing the legacy code.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS.

### `codex exec` to run Codex programmatically/non-interactively

To run Codex non-interactively, run `codex exec PROMPT` (you can also pass the prompt via `stdin`) and Codex will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Use `@` for file search

Typing `@` triggers a fuzzy-filename search over the workspace root. Use up/down to select among the results and Tab or Enter to replace the `@` with the selected path. You can use Esc to cancel the search.

### Esc–Esc to edit a previous message

When the chat composer is empty, press Esc to prime “backtrack” mode. Press Esc again to open a transcript preview highlighting the last user message; press Esc repeatedly to step to older user messages. Press Enter to confirm and Codex will fork the conversation from that point, trim the visible transcript accordingly, and pre‑fill the composer with the selected user message so you can edit and resubmit it.

In the transcript preview, the footer shows an `Esc edit prev` hint while editing is active.

### `--cd`/`-C` flag

Sometimes it is not convenient to `cd` to the directory you want Codex to use as the "working root" before running Codex. Fortunately, `codex` supports a `--cd` option so you can specify whatever folder you want. You can confirm that Codex is honoring `--cd` by double-checking the **workdir** it reports in the TUI at the start of a new session.

### Shell completions

Generate shell completion scripts via:

```shell
codex completion bash
codex completion zsh
codex completion fish
```

### Experimenting with the Codex Sandbox

To test to see what happens when a command is run under the sandbox provided by Codex, we provide the following subcommands in Codex CLI:

```
# macOS
codex debug seatbelt [--full-auto] [COMMAND]...

# Linux
codex debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run Codex with the default, read-only sandbox
codex --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
codex --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
codex --sandbox danger-full-access
```

The same setting can be persisted in `~/.codex/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for Codex. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use Codex.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
