Multi-shell helpers for deterministic test commands

This folder contains small helpers to build shell commands in a cross-platform, deterministic way for tests.

- write_file_cmd(path, content): returns a Cmd { prog, args } that writes the given content to the path using the appropriate shell:
  - Windows PowerShell or Cmd
  - Git Bash on Windows
  - sh on Unix

Shell selection
- By default, detection picks Git Bash on Windows if MSYS/Bash is present; otherwise PowerShell. On Unix it uses sh.
- Override for tests with the CODEX_TEST_SHELL environment variable: powershell | cmd | gitbash | sh.

Examples
- PowerShell (Windows): set the override before running tests
  - PowerShell: $env:CODEX_TEST_SHELL = "powershell"
- Bash: export CODEX_TEST_SHELL=powershell

Typical usage in a test
- Import the helper (path shown from tests/suite):
  - #[path = "../common/shell.rs"] mod shell;
  - use shell::{write_file_cmd, Cmd};
- Build a deterministic command and embed it in backticks in the prompt:
  - let cmd = write_file_cmd(&probe_path, "ok");
  - let prompt = format!("run `{}`", format!("{} {}", cmd.prog, cmd.args.join(" ")));
- When constructing the expected shell invocation (for mocks), pass Vec<String> built from cmd.prog + cmd.args.

Notes
- shell_escape expects a Cow<'_, str>; helpers take care of that.
