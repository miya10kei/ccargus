mod copy_mode;
pub mod mouse;
pub mod scroll;
pub mod terminal;
pub mod worktrees;

use crossterm::event::KeyCode;

use crate::app::Focus;
use crate::components::Component;
use crate::components::confirm_dialog::ConfirmAction;
use crate::components::qa_selector::QaMode;
use crate::context::{AppContext, UiContext};
use crate::domain;
use crate::keys::key_to_bytes;
use crate::layout::current_pty_sizes;

pub fn handle_key_press(ctx: &mut AppContext, ui: &mut UiContext, key: crossterm::event::KeyEvent) {
    if ui.editor_float.visible {
        if key.code == KeyCode::Esc {
            ui.editor_float.close();
            return;
        }
        if !ui.editor_float.is_process_alive() {
            ui.editor_float.close();
            return;
        }
        let bytes = key_to_bytes(key);
        if !bytes.is_empty() {
            let _ = ui.editor_float.write(&bytes);
        }
        return;
    }

    if ui.confirm_dialog.visible {
        ui.confirm_dialog.handle_key_event(key);

        if let Some((true, action)) = ui.confirm_dialog.take_result() {
            match action {
                ConfirmAction::DeleteWorktree => {
                    if let Some(wt) = ctx.app.worktree_pool.get(ctx.app.selected_worktree) {
                        let entry = wt.to_entry();
                        ctx.status_cache.cleanup(&wt.working_dir());
                        let _ = ctx.worktree_manager.remove_worktree(&entry);
                        ctx.app.worktree_pool.remove(ctx.app.selected_worktree);
                        if ctx.app.selected_worktree >= ctx.app.worktree_pool.len()
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
        return;
    }

    if ui.repo_selector.visible {
        ui.repo_selector.handle_key_event(key);

        if let Some(result) = ui.repo_selector.take_result() {
            match ctx.worktree_manager.add_worktree(
                &result.repo,
                &result.branch,
                result.base_branch.as_deref(),
            ) {
                Ok(entry) => {
                    let mut wt = domain::worktree::Worktree::from_entry(&entry);
                    let sizes = current_pty_sizes();
                    let _ = wt.start(sizes.single_rows, sizes.single_cols, ctx.config.claude.plan);
                    ctx.app.selected_worktree = ctx.app.worktree_pool.add(wt);
                    ctx.app.focus = Focus::Terminal;
                }
                Err(_e) => {
                    // TODO: display error to user via status line
                }
            }
        }
    } else if ui.qa_selector.visible {
        ui.qa_selector.handle_key_event(key);

        if let Some(mode) = ui.qa_selector.take_result() {
            let sizes = current_pty_sizes();
            let fork = mode == QaMode::Fork;
            if let Some(wt) = ctx.app.worktree_pool.get_mut(ctx.app.selected_worktree) {
                let _ = wt.create_qa(fork, sizes.split_qa_rows, sizes.split_qa_cols);
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
    } else {
        match ctx.app.focus {
            Focus::Worktrees => worktrees::handle_worktrees_key(ctx, ui, key),
            Focus::Terminal => terminal::handle_terminal_key(ctx, ui, key),
            Focus::QaTerminal => terminal::handle_qa_terminal_key(ctx, ui, key),
        }
    }
}
