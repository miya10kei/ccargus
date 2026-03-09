use crossterm::event::MouseEventKind;

use crate::app::Focus;
use crate::context::{AppContext, UiContext};
use crate::handler::scroll::scrollback_max;
use crate::keys::mouse_to_bytes;

pub fn handle_mouse_event(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    mouse: crossterm::event::MouseEvent,
) {
    // Handle click-to-focus
    if matches!(
        mouse.kind,
        MouseEventKind::Down(crossterm::event::MouseButton::Left)
    ) && !ui.repo_selector.visible
        && !ui.qa_selector.visible
        && !ui.confirm_dialog.visible
        && !ui.help_overlay.visible
    {
        let col = mouse.column;
        let row = mouse.row;

        if let Some(wt_area) = ui.last_worktree_area
            && wt_area.contains(ratatui::layout::Position::new(col, row))
        {
            ctx.app.focus = Focus::Worktrees;
            return;
        }

        if let Some(term_area) = ui.last_terminal_area
            && term_area.contains(ratatui::layout::Position::new(col, row))
        {
            // If Q&A pane is active, determine which half was clicked
            if ui.terminal_pane.qa_screen.is_some() {
                let mid_x =
                    term_area.x + term_area.width * (100 - ui.terminal_pane.qa_split_percent) / 100;
                if col >= mid_x {
                    ctx.app.focus = Focus::QaTerminal;
                } else {
                    ctx.app.focus = Focus::Terminal;
                }
            } else {
                ctx.app.focus = Focus::Terminal;
            }
            return;
        }
    }

    let is_scroll_wheel = matches!(
        mouse.kind,
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
    );

    if is_scroll_wheel && matches!(ctx.app.focus, Focus::Terminal | Focus::QaTerminal) {
        let is_qa = ctx.app.focus == Focus::QaTerminal;
        let max = scrollback_max(&ctx.worktree_pool, ctx.app.selected_worktree, is_qa);
        match mouse.kind {
            MouseEventKind::ScrollUp => ui.terminal_pane.scroll_up(is_qa, 3, max),
            MouseEventKind::ScrollDown => ui.terminal_pane.scroll_down(is_qa, 3),
            _ => {}
        }
        return;
    }

    let bytes = mouse_to_bytes(mouse);
    if bytes.is_empty() {
        return;
    }

    let pty = ctx
        .worktree_pool
        .get_mut(ctx.app.selected_worktree)
        .and_then(|wt| match ctx.app.focus {
            Focus::Terminal => wt.pty.as_mut(),
            Focus::QaTerminal => wt.qa_pty.as_mut(),
            Focus::Worktrees => None,
        });
    if let Some(pty) = pty {
        let _ = pty.write(&bytes);
    }
}
