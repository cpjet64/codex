## FAQ

### OpenAI released a model called Codex in 2021 - is this related?

In 2021, OpenAI released Codex, an AI system designed to generate code from natural language prompts. That original Codex model was deprecated as of March 2023 and is separate from the CLI tool.

### Which models are supported?

We recommend using Codex with GPT-5, our best coding model. The default reasoning level is medium, and you can upgrade to high for complex tasks with the `/model` command.

You can also use older models by using API-based auth and launching codex with the `--model` flag.

### Why does `o3` or `o4-mini` not work for me?

It's possible that your [API account needs to be verified](https://help.openai.com/en/articles/10910291-api-organization-verification) in order to start streaming responses and seeing chain of thought summaries from the API. If you're still running into issues, please let us know!

### How do I stop Codex from editing my files?

By default, Codex can modify files in your current working directory (Auto mode). To prevent edits, run `codex` in read-only mode with the CLI flag `--sandbox read-only`. Alternatively, you can change the approval level mid-conversation with `/approvals`.

### Does it work on Windows?

Running Codex directly on Windows may work, but is not officially supported. We recommend using [Windows Subsystem for Linux (WSL2)](https://learn.microsoft.com/en-us/windows/wsl/install). 

### How do I enable MCP servers in Codex?

Add an `mcp_servers` section to `~/.codex/config.toml`:

```toml
[mcp_servers.example]
command = "npx"
args = ["-y", "my-mcp-server"]
env = { API_KEY = "value" }
# Optional: extend startup timeout for initialize + initial tools/list (ms)
startup_timeout_ms = 15000
```

Then run `codex`. Tools appear qualified as `<server>__<tool>`.

### Can Codex act as an MCP server?

Yes. Launch: `codex mcp`. Try with the Inspector:

```bash
npx @modelcontextprotocol/inspector codex mcp
```

### What is the difference between “legacy” and “official” MCP implementations?

- Legacy: Codex’s original, built‑in MCP implementation (default).
- Official: Based on the Model Context Protocol’s official Rust SDK.

Switch independently for server and client:

- Client: `codex --mcp-client-impl=official` (or `CODEX_MCP_CLIENT_IMPL=official` or `mcp_client_impl = "official"` in config)
- Server: `codex --mcp-impl=official mcp` (or `CODEX_MCP_IMPL=official` or `mcp_impl = "official"` in config)

Precedence: CLI > env > config > legacy.

Guidance:

- Prefer legacy for stability if you rely on existing behavior.
- Try official to evaluate the SDK path or if a server advertises newer features.
- You can mix: e.g., official server + legacy client.

### My MCP server seems slow to start or times out

Increase `startup_timeout_ms` in the server’s entry. Default is 10,000 ms.

### Do you support non‑stdio transports (SSE/WebSocket)?

Codex directly supports stdio servers. For SSE servers, consider using an adapter such as `mcp-proxy`.
