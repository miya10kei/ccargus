use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::style::Color;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeStatus {
    Processing,
    Stopped,
    WaitingForInput,
    WaitingForPermission,
}

impl ClaudeStatus {
    pub fn color(self) -> Color {
        match self {
            Self::Processing => Color::Yellow,
            Self::Stopped => Color::DarkGray,
            Self::WaitingForInput => Color::Green,
            Self::WaitingForPermission => Color::Red,
        }
    }

    pub fn from_status_str(s: &str) -> Option<Self> {
        match s {
            "processing" => Some(Self::Processing),
            "permission" => Some(Self::WaitingForPermission),
            "waiting_for_input" => Some(Self::WaitingForInput),
            _ => None,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Processing => "\u{f0525}",           // 󰔥 nf-md-timer_sand
            Self::Stopped => "\u{f0425}",              // 󰐥 nf-md-power
            Self::WaitingForInput => "\u{f030c}",      // 󰌌 nf-md-keyboard
            Self::WaitingForPermission => "\u{f0306}", // 󰌆 nf-md-lock
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Processing => "processing",
            Self::Stopped => "stopped",
            Self::WaitingForInput => "waiting for input",
            Self::WaitingForPermission => "waiting for permission",
        }
    }
}

pub struct StatusCache {
    cache: HashMap<String, ClaudeStatus>,
    socket_path: PathBuf,
}

impl StatusCache {
    pub fn new() -> Self {
        let runtime_dir =
            std::env::var("XDG_RUNTIME_DIR").map_or_else(|_| PathBuf::from("/tmp"), PathBuf::from);
        let socket_name = format!("notify-{}.sock", std::process::id());
        Self {
            cache: HashMap::new(),
            socket_path: runtime_dir.join("ccargus").join(socket_name),
        }
    }

    pub fn cleanup(&mut self, cwd: &str) {
        self.cache.remove(cwd);
    }

    pub fn read_status(&self, cwd: &str, has_pty: bool) -> ClaudeStatus {
        if !has_pty {
            return ClaudeStatus::Stopped;
        }
        self.cache
            .get(cwd)
            .copied()
            .unwrap_or(ClaudeStatus::Processing)
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    pub fn update(&mut self, cwd: &str, status: &str) {
        if let Some(s) = ClaudeStatus::from_status_str(status) {
            self.cache.insert(cwd.to_owned(), s);
        }
    }
}

pub fn start_socket_listener(socket_path: &PathBuf) -> mpsc::UnboundedReceiver<(String, String)> {
    let _ = std::fs::remove_file(socket_path);
    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = UnixListener::bind(socket_path).expect("Failed to bind Unix socket");
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };

            let mut buf = Vec::with_capacity(1024);
            if stream.read_to_end(&mut buf).await.is_err() {
                continue;
            }

            let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&buf) else {
                continue;
            };

            let cwd = parsed
                .get("cwd")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();
            let status = parsed
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();

            if !cwd.is_empty() && !status.is_empty() {
                let _ = tx.send((cwd, status));
            }
        }
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_status_str_permission() {
        assert_eq!(
            ClaudeStatus::from_status_str("permission"),
            Some(ClaudeStatus::WaitingForPermission)
        );
    }

    #[test]
    fn from_status_str_processing() {
        assert_eq!(
            ClaudeStatus::from_status_str("processing"),
            Some(ClaudeStatus::Processing)
        );
    }

    #[test]
    fn from_status_str_unknown() {
        assert_eq!(ClaudeStatus::from_status_str("unknown"), None);
    }

    #[test]
    fn from_status_str_waiting_for_input() {
        assert_eq!(
            ClaudeStatus::from_status_str("waiting_for_input"),
            Some(ClaudeStatus::WaitingForInput)
        );
    }

    #[test]
    fn stopped_when_no_pty() {
        let cache = StatusCache::new();
        assert_eq!(
            cache.read_status("/some/path", false),
            ClaudeStatus::Stopped
        );
    }

    #[test]
    fn processing_when_pty_but_no_cache_entry() {
        let cache = StatusCache::new();
        assert_eq!(
            cache.read_status("/some/path", true),
            ClaudeStatus::Processing
        );
    }

    #[test]
    fn update_and_read() {
        let mut cache = StatusCache::new();
        cache.update("/some/path", "processing");
        assert_eq!(
            cache.read_status("/some/path", true),
            ClaudeStatus::Processing
        );
    }

    #[test]
    fn update_permission_and_read() {
        let mut cache = StatusCache::new();
        cache.update("/some/path", "permission");
        assert_eq!(
            cache.read_status("/some/path", true),
            ClaudeStatus::WaitingForPermission
        );
    }

    #[test]
    fn cleanup_removes_entry() {
        let mut cache = StatusCache::new();
        cache.update("/some/path", "processing");
        cache.cleanup("/some/path");
        assert_eq!(
            cache.read_status("/some/path", true),
            ClaudeStatus::Processing
        );
    }

    #[test]
    fn update_ignores_unknown_status() {
        let mut cache = StatusCache::new();
        cache.update("/some/path", "unknown");
        assert_eq!(
            cache.read_status("/some/path", true),
            ClaudeStatus::Processing
        );
    }

    #[test]
    fn icon_returns_correct_symbols() {
        assert_eq!(ClaudeStatus::Stopped.icon(), "\u{f0425}");
        assert_eq!(ClaudeStatus::Processing.icon(), "\u{f0525}");
        assert_eq!(ClaudeStatus::WaitingForInput.icon(), "\u{f030c}");
        assert_eq!(ClaudeStatus::WaitingForPermission.icon(), "\u{f0306}");
    }

    #[test]
    fn label_returns_correct_strings() {
        assert_eq!(ClaudeStatus::Stopped.label(), "stopped");
        assert_eq!(ClaudeStatus::Processing.label(), "processing");
        assert_eq!(ClaudeStatus::WaitingForInput.label(), "waiting for input");
        assert_eq!(
            ClaudeStatus::WaitingForPermission.label(),
            "waiting for permission"
        );
    }

    #[test]
    fn color_returns_correct_values() {
        assert_eq!(ClaudeStatus::Processing.color(), Color::Yellow);
        assert_eq!(ClaudeStatus::Stopped.color(), Color::DarkGray);
        assert_eq!(ClaudeStatus::WaitingForInput.color(), Color::Green);
        assert_eq!(ClaudeStatus::WaitingForPermission.color(), Color::Red);
    }
}
