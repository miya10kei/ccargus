use crate::app::App;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::editor_float::EditorFloat;
use crate::components::qa_selector::QaSelector;
use crate::components::repo_selector::RepoSelector;
use crate::components::terminal_pane::TerminalPane;
use crate::components::worktree_tree::WorktreeTree;
use crate::config::Config;
use crate::domain::WorktreeManager;
use crate::domain::claude_status::StatusCache;

pub struct AppContext {
    pub app: App,
    pub config: Config,
    pub status_cache: StatusCache,
    pub worktree_manager: WorktreeManager,
}

pub struct UiContext {
    pub confirm_dialog: ConfirmDialog,
    pub editor_float: EditorFloat,
    pub qa_selector: QaSelector,
    pub repo_selector: RepoSelector,
    pub terminal_pane: TerminalPane,
    pub worktree_tree: WorktreeTree,
}
