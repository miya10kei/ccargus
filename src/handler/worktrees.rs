use crossterm::event::{KeyCode, KeyModifiers};

use crate::components::confirm_dialog::ConfirmAction;
use crate::context::{AppContext, UiContext};
use crate::domain;
use crate::layout::current_pty_sizes_with_config;

pub fn handle_worktrees_key(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    key: crossterm::event::KeyEvent,
) {
    let kb = &ctx.config.keybindings;
    match key.code {
        KeyCode::Char('q' | 'c')
            if key.code == KeyCode::Char('q') || key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            ui.confirm_dialog
                .open("Quit ccargus?", ConfirmAction::QuitApp);
        }
        KeyCode::Char(c) if c == kb.delete_worktree => {
            if let Some(wt) = ctx.worktree_pool.get(ctx.app.selected_worktree) {
                let message = format!("Delete worktree '{}/{}'?", wt.repo, wt.branch);
                ui.confirm_dialog
                    .open(message, ConfirmAction::DeleteWorktree);
            }
        }
        KeyCode::Char(c) if c == kb.open_editor => {
            open_editor(ctx);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            ctx.app.select_next_worktree(ctx.worktree_pool.len());
        }
        KeyCode::Char('k') | KeyCode::Up => {
            ctx.app.select_prev_worktree(ctx.worktree_pool.len());
        }
        KeyCode::Char(c) if c == kb.new_worktree => {
            ui.repo_selector.open();
        }
        KeyCode::Char(c) if c == kb.qa_worktree => {
            if let Some(wt) = ctx.worktree_pool.get(ctx.app.selected_worktree)
                && wt.is_running()
            {
                ui.qa_selector.open();
            }
        }
        KeyCode::Char('?') => {
            ui.help_overlay.toggle();
        }
        KeyCode::Char('x') => {
            if let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree) {
                ctx.status_cache.cleanup(&wt.working_dir());
                wt.stop();
            }
        }
        KeyCode::Enter => {
            if let Some(wt) = ctx.worktree_pool.get_mut(ctx.app.selected_worktree) {
                if wt.is_running() {
                    // Focus into running worktree
                    let has_qa = wt.has_qa();
                    ctx.app.toggle_focus(has_qa);
                } else {
                    // Start stopped worktree
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
                    ctx.app.focus = crate::app::Focus::Terminal;
                }
            }
        }
        KeyCode::Tab => {
            if !ctx.worktree_pool.is_empty() {
                let has_qa = ctx
                    .worktree_pool
                    .get(ctx.app.selected_worktree)
                    .is_some_and(domain::worktree::Worktree::has_qa);
                ctx.app.toggle_focus(has_qa);
            }
        }
        _ => {}
    }
}

fn open_editor(ctx: &mut AppContext) {
    if !domain::tmux::is_running() {
        ctx.notify(
            "エディタを開くにはtmux環境が必要です".to_owned(),
            crate::context::NotificationLevel::Error,
        );
        return;
    }
    let Some(wt) = ctx.worktree_pool.get(ctx.app.selected_worktree) else {
        return;
    };
    let session =
        domain::tmux::sanitize_session_name(&format!("ccargus-{}-{}", wt.repo, wt.branch));
    if let Err(e) = domain::tmux::open_editor_popup(
        &ctx.config.editor.popup.options,
        &wt.working_dir(),
        &ctx.config.editor.command,
        &session,
    ) {
        ctx.notify(
            format!("Failed to open editor: {e}"),
            crate::context::NotificationLevel::Error,
        );
    }
}
