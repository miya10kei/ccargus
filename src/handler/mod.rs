mod copy_mode;
pub mod mouse;
pub mod scroll;
pub mod terminal;
pub mod worktrees;

use crate::app::Focus;
use crate::components::Component;
use crate::components::confirm_dialog::ConfirmAction;
use crate::components::qa_selector::QaMode;
use crate::context::{AppContext, UiContext};
use crate::domain;
use crate::layout::current_pty_sizes_with_config;

type ModalHandler = fn(&mut AppContext, &mut UiContext, crossterm::event::KeyEvent) -> bool;

/// Modal handlers in priority order (highest first).
const MODAL_HANDLERS: &[ModalHandler] = &[
    handle_confirm_dialog_key,
    handle_repo_selector_key,
    handle_qa_selector_key,
    handle_help_overlay_key,
];

pub fn handle_key_press(ctx: &mut AppContext, ui: &mut UiContext, key: crossterm::event::KeyEvent) {
    for handler in MODAL_HANDLERS {
        if handler(ctx, ui, key) {
            return;
        }
    }

    match ctx.app.focus {
        Focus::Worktrees => worktrees::handle_worktrees_key(ctx, ui, key),
        Focus::Terminal => terminal::handle_terminal_key(ctx, ui, key),
        Focus::QaTerminal => terminal::handle_qa_terminal_key(ctx, ui, key),
    }
}

fn handle_confirm_dialog_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) -> bool {
    if !ui.confirm_dialog.visible {
        return false;
    }

    ui.confirm_dialog.handle_key_event(key);

    if let Some((true, action)) = ui.confirm_dialog.take_result() {
        match action {
            ConfirmAction::DeleteWorktree => {
                if let Some(wt) = ctx.worktree_pool.get(ctx.app.selected_worktree) {
                    let entry = wt.to_entry();
                    ctx.status_cache.cleanup(&wt.working_dir());
                    if let Err(e) = ctx.worktree_manager.remove_worktree(&entry) {
                        ctx.notify(
                            format!("Failed to remove worktree: {e}"),
                            crate::context::NotificationLevel::Error,
                        );
                    }
                    ctx.worktree_pool.remove(ctx.app.selected_worktree);
                    if ctx.app.selected_worktree >= ctx.worktree_pool.len()
                        && ctx.app.selected_worktree > 0
                    {
                        ctx.app.selected_worktree -= 1;
                    }
                }
            }
            ConfirmAction::QuitApp => {
                ctx.app.quit();
            }
        }
    }
    true
}

fn handle_help_overlay_key(
    _ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) -> bool {
    if !ui.help_overlay.visible {
        return false;
    }

    ui.help_overlay.handle_key_event(key);
    true
}

fn handle_qa_selector_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) -> bool {
    if !ui.qa_selector.visible {
        return false;
    }

    ui.qa_selector.handle_key_event(key);

    if let Some(mode) = ui.qa_selector.take_result() {
        let sizes = current_pty_sizes_with_config(
            ctx.config.layout.worktree_pane_percent,
            ctx.config.layout.qa_split_percent,
        );
        let fork = mode == QaMode::Fork;
        if let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree) {
            let _ = wt.create_qa(
                fork,
                sizes.split_qa_rows,
                sizes.split_qa_cols,
                ctx.config.claude.plan,
                &ctx.config.claude.command,
            );
            let (main_rows, main_cols) = sizes.main_size(true);
            wt.resize_pty(
                main_rows,
                main_cols,
                sizes.split_qa_rows,
                sizes.split_qa_cols,
            );
        }
        ctx.app.focus = Focus::QaTerminal;
    }
    true
}

fn handle_repo_selector_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) -> bool {
    if !ui.repo_selector.visible {
        return false;
    }

    ui.repo_selector.handle_key_event(key);

    if let Some(result) = ui.repo_selector.take_result() {
        match ctx.worktree_manager.add_worktree(
            &result.repo,
            &result.branch,
            result.base_branch.as_deref(),
        ) {
            Ok(entry) => {
                let mut wt = domain::worktree::Worktree::from_entry(&entry);
                let sizes = current_pty_sizes_with_config(
                    ctx.config.layout.worktree_pane_percent,
                    ctx.config.layout.qa_split_percent,
                );
                if let Err(e) = wt.start(
                    sizes.single_rows,
                    sizes.single_cols,
                    ctx.config.claude.auto_continue,
                    ctx.config.claude.plan,
                    &ctx.config.claude.command,
                ) {
                    ctx.notify(
                        format!("Failed to start worktree: {e}"),
                        crate::context::NotificationLevel::Error,
                    );
                }
                ctx.app.selected_worktree = ctx.worktree_pool.add(wt);
                ctx.app.focus = Focus::Terminal;
            }
            Err(e) => {
                ctx.notify(
                    format!("Failed to create worktree: {e}"),
                    crate::context::NotificationLevel::Error,
                );
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::components::confirm_dialog::ConfirmDialog;
    use crate::components::help_overlay::HelpOverlay;
    use crate::components::qa_selector::QaSelector;
    use crate::components::repo_selector::RepoSelector;
    use crate::components::terminal_pane::TerminalPane;
    use crate::components::worktree_tree::WorktreeTree;
    use crate::config::{Config, KeybindingsConfig};
    use crate::domain::worktree::WorktreePool;

    struct TestEnv {
        ctx: AppContext,
        ui: UiContext,
        _tmp: tempfile::TempDir,
    }

    fn setup() -> TestEnv {
        let tmp = tempfile::tempdir().unwrap();
        let config = Config::default();
        let worktree_manager =
            crate::domain::WorktreeManager::new(tmp.path().to_path_buf(), vec!["main".into()])
                .unwrap();
        TestEnv {
            ctx: AppContext {
                app: crate::app::App::new(),
                config,
                notification: None,
                status_cache: crate::domain::claude_status::StatusCache::new(),
                worktree_manager,
                worktree_pool: WorktreePool::new(),
            },
            ui: UiContext {
                confirm_dialog: ConfirmDialog::new(),
                help_overlay: HelpOverlay::new(KeybindingsConfig::default()),
                last_worktree_area: None,
                last_terminal_area: None,
                qa_selector: QaSelector::new(),
                repo_selector: RepoSelector::new(),
                terminal_pane: TerminalPane::new('n', 50),
                worktree_tree: WorktreeTree::new(),
            },
            _tmp: tmp,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn confirm_dialog_consumes_key_when_visible() {
        let mut env = setup();
        env.ui.confirm_dialog.open("Test?", ConfirmAction::QuitApp);
        let result = handle_confirm_dialog_key(&mut env.ctx, &mut env.ui, key(KeyCode::Char('n')));
        assert!(result);
    }

    #[test]
    fn confirm_dialog_passes_through_when_hidden() {
        let mut env = setup();
        let result = handle_confirm_dialog_key(&mut env.ctx, &mut env.ui, key(KeyCode::Char('y')));
        assert!(!result);
    }

    #[test]
    fn confirm_quit_sets_quit_state() {
        let mut env = setup();
        env.ui.confirm_dialog.open("Quit?", ConfirmAction::QuitApp);
        handle_confirm_dialog_key(&mut env.ctx, &mut env.ui, key(KeyCode::Char('y')));
        assert!(!env.ctx.app.is_running());
    }

    #[test]
    fn confirm_delete_removes_from_pool() {
        let mut env = setup();
        let wt = crate::domain::worktree::Worktree {
            branch: "test-branch".into(),
            pty: None,
            qa_pty: None,
            repo: "test-repo".into(),
            source_repo_path: "/tmp".into(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
        };
        env.ctx.worktree_pool.add(wt);
        assert_eq!(env.ctx.worktree_pool.len(), 1);

        env.ui
            .confirm_dialog
            .open("Delete?", ConfirmAction::DeleteWorktree);
        handle_confirm_dialog_key(&mut env.ctx, &mut env.ui, key(KeyCode::Char('y')));
        assert_eq!(env.ctx.worktree_pool.len(), 0);
    }

    #[test]
    fn confirm_delete_notifies_on_error() {
        let mut env = setup();
        let wt = crate::domain::worktree::Worktree {
            branch: "nonexistent-branch".into(),
            pty: None,
            qa_pty: None,
            repo: "nonexistent-repo".into(),
            source_repo_path: "/nonexistent/path".into(),
            worktree_path: std::path::PathBuf::from("/nonexistent/worktree"),
        };
        env.ctx.worktree_pool.add(wt);

        env.ui
            .confirm_dialog
            .open("Delete?", ConfirmAction::DeleteWorktree);
        handle_confirm_dialog_key(&mut env.ctx, &mut env.ui, key(KeyCode::Char('y')));

        assert!(env.ctx.notification.is_some());
        let notification = env.ctx.notification.as_ref().unwrap();
        assert_eq!(notification.level, crate::context::NotificationLevel::Error);
        assert!(notification.message.contains("Failed to remove worktree"));
    }

    #[test]
    fn help_overlay_consumes_key_when_visible() {
        let mut env = setup();
        env.ui.help_overlay.toggle();
        assert!(env.ui.help_overlay.visible);
        let result = handle_help_overlay_key(&mut env.ctx, &mut env.ui, key(KeyCode::Esc));
        assert!(result);
    }

    #[test]
    fn help_overlay_passes_through_when_hidden() {
        let mut env = setup();
        let result = handle_help_overlay_key(&mut env.ctx, &mut env.ui, key(KeyCode::Esc));
        assert!(!result);
    }
}
