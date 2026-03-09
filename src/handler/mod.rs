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
use crate::keys::key_to_bytes;
use crate::layout::current_pty_sizes_with_config;

pub fn handle_key_press(ctx: &mut AppContext, ui: &mut UiContext, key: crossterm::event::KeyEvent) {
    if handle_editor_float_key(ui, key) {
        return;
    }
    if handle_confirm_dialog_key(ctx, ui, key) {
        return;
    }
    if handle_repo_selector_key(ctx, ui, key) {
        return;
    }
    if handle_qa_selector_key(ctx, ui, key) {
        return;
    }
    if handle_help_overlay_key(ui, key) {
        return;
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
    true
}

fn handle_help_overlay_key(ui: &mut UiContext, key: crossterm::event::KeyEvent) -> bool {
    if !ui.help_overlay.visible {
        return false;
    }

    ui.help_overlay.handle_key_event(key);
    true
}

fn handle_editor_float_key(ui: &mut UiContext, key: crossterm::event::KeyEvent) -> bool {
    if !ui.editor_float.visible {
        return false;
    }

    if !ui.editor_float.is_process_alive() {
        ui.editor_float.close();
        return true;
    }
    let bytes = key_to_bytes(key);
    if !bytes.is_empty() {
        let _ = ui.editor_float.write(&bytes);
    }
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
        if let Some(wt) = ctx.app.worktree_pool.get_mut(ctx.app.selected_worktree) {
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
                    ctx.config.claude.plan,
                    &ctx.config.claude.command,
                ) {
                    ctx.notify(
                        format!("Failed to start worktree: {e}"),
                        crate::context::NotificationLevel::Error,
                    );
                }
                ctx.app.selected_worktree = ctx.app.worktree_pool.add(wt);
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
