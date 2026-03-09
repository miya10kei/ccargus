use crate::app::Focus;
use crate::context::{AppContext, UiContext};
use crate::handler::scroll::scrollback_max;
use crate::keys::mouse_to_bytes;

pub fn handle_mouse_event(
    ctx: &mut AppContext,
    ui: &mut UiContext,
    mouse: crossterm::event::MouseEvent,
) {
    use crossterm::event::MouseEventKind;

    let is_scroll_wheel = matches!(
        mouse.kind,
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
    );

    if is_scroll_wheel
        && !ui.editor_float.visible
        && matches!(ctx.app.focus, Focus::Terminal | Focus::QaTerminal)
    {
        let is_qa = ctx.app.focus == Focus::QaTerminal;
        let max = scrollback_max(&ctx.app, is_qa);
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

    if ui.editor_float.visible {
        let _ = ui.editor_float.write(&bytes);
        return;
    }

    let pty = ctx
        .app
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
