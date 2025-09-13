use std::borrow::Cow;
use std::env;
use std::path::Path;

pub struct Cmd {
    pub prog: String,
    pub args: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    PowerShell,
    Cmd,
    GitBash,
    Sh,
}

pub fn detect_shell() -> ShellKind {
    if let Ok(forced) = env::var("CODEX_TEST_SHELL") {
        let v = forced.to_ascii_lowercase();
        return match v.as_str() {
            "powershell" | "pwsh" => ShellKind::PowerShell,
            "cmd" => ShellKind::Cmd,
            "gitbash" | "bash" => ShellKind::GitBash,
            "sh" => ShellKind::Sh,
            _ => default_shell(),
        };
    }
    default_shell()
}

fn default_shell() -> ShellKind {
    if cfg!(windows) {
        let msys = env::var("MSYSTEM").unwrap_or_default();
        let shell = env::var("SHELL").unwrap_or_default();
        let lower_shell = shell.to_ascii_lowercase();
        if !msys.is_empty() || lower_shell.contains("bash") {
            ShellKind::GitBash
        } else {
            ShellKind::PowerShell
        }
    } else {
        ShellKind::Sh
    }
}

pub fn write_file_cmd_auto<P: AsRef<Path>>(path: P, content: &str) -> Cmd {
    write_file_cmd_with(detect_shell(), path.as_ref(), content)
}

pub fn write_file_cmd_with(kind: ShellKind, path: &Path, content: &str) -> Cmd {
    match kind {
        ShellKind::PowerShell => write_file_cmd_powershell(path, content),
        ShellKind::Cmd => write_file_cmd_cmd(path, content),
        ShellKind::GitBash => write_file_cmd_gitbash(path, content),
        ShellKind::Sh => write_file_cmd_sh(path, content),
    }
}

#[cfg(windows)]
fn utf16le_b64(s: &str) -> String {
    use base64::prelude::*;
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
    BASE64_STANDARD.encode(bytes)
}

#[cfg(windows)]
fn write_file_cmd_powershell(path: &Path, content: &str) -> Cmd {
    let escaped_content = content.replace('\'', "''");
    let script = format!(
        "Set-Content -LiteralPath '{}' -Value '{}'",
        path.display(),
        escaped_content
    );
    let encoded = utf16le_b64(&script);
    Cmd {
        prog: "powershell".into(),
        args: vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-EncodedCommand".into(),
            encoded,
        ],
    }
}

#[cfg(windows)]
fn write_file_cmd_cmd(path: &Path, content: &str) -> Cmd {
    let sanitized = content.replace(['\r', '\n'], " ");
    let arg = format!("echo {} > \"{}\"", sanitized, path.display());
    Cmd {
        prog: "cmd".into(),
        args: vec!["/d".into(), "/s".into(), "/c".into(), arg],
    }
}

#[cfg(windows)]
fn write_file_cmd_gitbash(path: &Path, content: &str) -> Cmd {
    let cmd = format!(
        "printf %s {} > {}",
        shell_escape::escape(Cow::Borrowed(content)),
        shell_escape::escape(path.display().to_string().into()),
    );
    Cmd {
        prog: "bash".into(),
        args: vec!["-lc".into(), cmd],
    }
}

#[cfg(not(windows))]
fn write_file_cmd_powershell(_path: &Path, _content: &str) -> Cmd {
    unreachable!("powershell path only on windows")
}
#[cfg(not(windows))]
fn write_file_cmd_cmd(_path: &Path, _content: &str) -> Cmd {
    unreachable!("cmd path only on windows")
}
#[cfg(not(windows))]
fn write_file_cmd_gitbash(_path: &Path, _content: &str) -> Cmd {
    unreachable!("gitbash path only on windows")
}

#[cfg(unix)]
fn write_file_cmd_sh(path: &Path, content: &str) -> Cmd {
    let cmd = format!(
        "printf %s {} > {}",
        shell_escape::escape(Cow::Borrowed(content)),
        shell_escape::escape(path.display().to_string().into()),
    );
    Cmd {
        prog: "sh".into(),
        args: vec!["-lc".into(), cmd],
    }
}

#[cfg(windows)]
fn write_file_cmd_sh(path: &Path, content: &str) -> Cmd {
    write_file_cmd_gitbash(path, content)
}

pub fn write_file_cmd<P: AsRef<Path>>(path: P, content: &str) -> Cmd {
    write_file_cmd_auto(path, content)
}
