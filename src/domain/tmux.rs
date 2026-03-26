use std::process::Command;

use color_eyre::Result;
use color_eyre::eyre::eyre;

const POPUP_SOCKET: &str = "ccargus-popup";

pub fn has_session(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["-L", POPUP_SOCKET, "has-session", "-t", session_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn has_window(session_name: &str, window_name: &str) -> bool {
    let output = Command::new("tmux")
        .args([
            "-L",
            POPUP_SOCKET,
            "list-windows",
            "-t",
            session_name,
            "-F",
            "#{window_name}",
        ])
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line == window_name),
        Err(_) => false,
    }
}

pub fn is_running() -> bool {
    check_tmux_env(std::env::var("TMUX").ok())
}

pub fn open_popup(
    options: &[String],
    working_dir: &str,
    command: &str,
    session_name: &str,
    window_name: &str,
) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_owned());
    let escaped_session = shell_escape(session_name);
    let escaped_command = shell_escape(command);
    let escaped_dir = shell_escape(working_dir);
    let escaped_window = shell_escape(window_name);

    if has_session(session_name) {
        if has_window(session_name, window_name) {
            select_window(session_name, window_name)?;
        } else {
            create_window_in_session(session_name, working_dir, command, window_name)?;
        }
    }

    let inner_cmd = format!(
        "TMUX='' tmux -L {POPUP_SOCKET} new-session -A -s {escaped_session} -n {escaped_window} -c {escaped_dir} {escaped_command}"
    );

    let status = Command::new("tmux")
        .arg("display-popup")
        .args(options)
        .arg("-d")
        .arg(working_dir)
        .arg("--")
        .arg(&shell)
        .arg("-ic")
        .arg(&inner_cmd)
        .status()?;

    if !status.success() {
        return Err(eyre!("tmux display-popup exited with status: {status}"));
    }
    Ok(())
}

fn check_tmux_env(tmux_var: Option<String>) -> bool {
    tmux_var.is_some_and(|v| !v.is_empty())
}

fn create_window_in_session(
    session_name: &str,
    working_dir: &str,
    command: &str,
    window_name: &str,
) -> Result<()> {
    let escaped_session = shell_escape(session_name);
    let escaped_command = shell_escape(command);
    let escaped_dir = shell_escape(working_dir);
    let escaped_window = shell_escape(window_name);
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_owned());
    let inner_cmd = format!(
        "TMUX='' tmux -L {POPUP_SOCKET} new-window -t {escaped_session} -n {escaped_window} -c {escaped_dir} {escaped_command}"
    );

    let status = Command::new("tmux")
        .args(["-L", POPUP_SOCKET])
        .arg("run-shell")
        .arg(format!("{shell} -c {}", shell_escape(&inner_cmd)))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        return Err(eyre!("tmux new-window exited with status: {status}"));
    }
    Ok(())
}

pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn select_window(session_name: &str, window_name: &str) -> Result<()> {
    let target = format!("{session_name}:{window_name}");
    let status = Command::new("tmux")
        .args(["-L", POPUP_SOCKET, "select-window", "-t", &target])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        return Err(eyre!("tmux select-window exited with status: {status}"));
    }
    Ok(())
}

pub(crate) fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_tmux_env_returns_true_when_set() {
        assert!(check_tmux_env(Some(
            "/tmp/tmux-1000/default,12345,0".to_owned()
        )));
    }

    #[test]
    fn check_tmux_env_returns_false_when_empty() {
        assert!(!check_tmux_env(Some(String::new())));
    }

    #[test]
    fn check_tmux_env_returns_false_when_none() {
        assert!(!check_tmux_env(None));
    }

    #[test]
    fn sanitize_session_name_keeps_alphanumeric() {
        assert_eq!(sanitize_session_name("abc-123_def"), "abc-123_def");
    }

    #[test]
    fn sanitize_session_name_replaces_special_chars() {
        assert_eq!(
            sanitize_session_name("repo/branch.name"),
            "repo-branch-name"
        );
    }

    #[test]
    fn sanitize_session_name_replaces_dots_and_colons() {
        assert_eq!(sanitize_session_name("a.b:c"), "a-b-c");
    }

    #[test]
    fn shell_escape_simple_string() {
        assert_eq!(shell_escape("nvim"), "'nvim'");
    }

    #[test]
    fn shell_escape_string_with_single_quote() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_escape_string_with_spaces() {
        assert_eq!(shell_escape("/path/to my/dir"), "'/path/to my/dir'");
    }
}
