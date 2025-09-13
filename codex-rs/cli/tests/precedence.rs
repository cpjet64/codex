use std::io::Read;
use std::process::Stdio;

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

fn spawn_and_capture(mut cmd: std::process::Command) -> String {
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn codex");
    // Give time for startup logs to be emitted
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let _ = child.kill();
    let _ = child.wait();
    let mut buf = String::new();
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_string(&mut buf);
    }
    buf
}

#[test]
fn server_precedence_config_official() {
    // Config-only: mcp_impl = "official" -> server should start as official
    let home = TempDir::new().expect("tmp home");
    std::fs::write(home.path().join("config.toml"), "mcp_impl = \"official\"\n")
        .expect("write config");

    let bin = cargo_bin("codex");
    let mut cmd = std::process::Command::new(bin);
    cmd.env_remove("CODEX_MCP_IMPL");
    cmd.env("RUST_LOG", "info");
    cmd.env("CODEX_HOME", home.path());
    cmd.env("RUST_LOG", "info");
    cmd.arg("mcp");
    let err = spawn_and_capture(cmd);
    assert!(
        err.contains("codex-mcp-server starting with impl: official")
            || err.contains("rmcp server error:"),
        "stderr did not indicate official path: {}",
        err
    );
}

#[test]
fn server_precedence_env_legacy_over_config_official() {
    // Config official + env legacy -> env wins; expect legacy
    let home = TempDir::new().expect("tmp home");
    std::fs::write(home.path().join("config.toml"), "mcp_impl = \"official\"\n")
        .expect("write config");

    let bin = cargo_bin("codex");
    let mut cmd = std::process::Command::new(bin);
    cmd.env("CODEX_HOME", home.path());
    cmd.env("CODEX_MCP_IMPL", "legacy");
    cmd.env("RUST_LOG", "info");
    cmd.arg("mcp");
    let err = spawn_and_capture(cmd);
    assert!(
        err.contains("codex-mcp-server starting with impl: legacy"),
        "stderr did not show legacy start: {}",
        err
    );
}

#[test]
fn server_precedence_cli_official_over_env_legacy() {
    // CLI official + env legacy -> flag wins; expect official
    let home = TempDir::new().expect("tmp home");
    std::fs::write(home.path().join("config.toml"), "mcp_impl = \"legacy\"\n")
        .expect("write config");

    let bin = cargo_bin("codex");
    let mut cmd = std::process::Command::new(bin);
    cmd.env("CODEX_HOME", home.path());
    cmd.env("CODEX_MCP_IMPL", "legacy");
    cmd.args(["--mcp-impl", "official", "mcp"]);
    let err = spawn_and_capture(cmd);
    assert!(
        err.contains("codex-mcp-server starting with impl: official")
            || err.contains("rmcp server error:"),
        "stderr did not indicate official path: {}",
        err
    );
}
