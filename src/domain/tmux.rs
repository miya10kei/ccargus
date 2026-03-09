use std::process::Command;

use color_eyre::Result;
use color_eyre::eyre::eyre;

const EDITOR_SOCKET: &str = "ccargus-editor";

pub fn is_running() -> bool {
    check_tmux_env(std::env::var("TMUX").ok())
}

pub fn open_editor_popup(
    options: &[String],
    working_dir: &str,
    editor_command: &str,
    session_name: &str,
) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_owned());
    let escaped_session = shell_escape(session_name);
    let escaped_editor = shell_escape(editor_command);
    let escaped_dir = shell_escape(working_dir);
    let inner_cmd = format!(
        "TMUX='' tmux -L {EDITOR_SOCKET} new-session -A -s {escaped_session} -c {escaped_dir} {escaped_editor} \\; set status off"
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

fn check_tmux_env(tmux_var: Option<String>) -> bool {
    tmux_var.is_some_and(|v| !v.is_empty())
}

fn shell_escape(s: &str) -> String {
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
