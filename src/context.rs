use std::time::Instant;

use crate::app::App;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::editor_float::EditorFloat;
use crate::components::help_overlay::HelpOverlay;
use crate::components::qa_selector::QaSelector;
use crate::components::repo_selector::RepoSelector;
use crate::components::terminal_pane::TerminalPane;
use crate::components::worktree_tree::WorktreeTree;
use crate::config::Config;
use crate::domain::WorktreeManager;
use crate::domain::claude_status::StatusCache;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub created_at: Instant,
    pub level: NotificationLevel,
    pub message: String,
}

const NOTIFICATION_TTL_SECS: u64 = 5;

pub struct AppContext {
    pub app: App,
    pub config: Config,
    pub notification: Option<Notification>,
    pub status_cache: StatusCache,
    pub worktree_manager: WorktreeManager,
}

impl AppContext {
    pub fn active_notification(&self) -> Option<&Notification> {
        self.notification
            .as_ref()
            .filter(|n| n.created_at.elapsed().as_secs() < NOTIFICATION_TTL_SECS)
    }

    pub fn notify(&mut self, message: impl Into<String>, level: NotificationLevel) {
        self.notification = Some(Notification {
            created_at: Instant::now(),
            level,
            message: message.into(),
        });
    }
}

pub struct UiContext {
    pub confirm_dialog: ConfirmDialog,
    pub editor_float: EditorFloat,
    pub help_overlay: HelpOverlay,
    pub last_worktree_area: Option<ratatui::layout::Rect>,
    pub last_terminal_area: Option<ratatui::layout::Rect>,
    pub qa_selector: QaSelector,
    pub repo_selector: RepoSelector,
    pub terminal_pane: TerminalPane,
    pub worktree_tree: WorktreeTree,
}
