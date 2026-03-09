use crossterm::event::{KeyCode, KeyModifiers};

use crate::app;
use crate::components::terminal_pane::TerminalPane;
use crate::layout::terminal_half_page_size;

pub fn scrollback_max(app: &app::App, qa: bool) -> usize {
    app.worktree_pool
        .get(app.selected_worktree)
        .and_then(|wt| {
            let pty = if qa {
                wt.qa_pty.as_ref()
            } else {
                wt.pty.as_ref()
            };
            pty.and_then(|p| {
                p.screen().lock().ok().map(|mut parser| {
                    let screen = parser.screen_mut();
                    // set_scrollback clamps to the actual scrollback buffer size
                    screen.set_scrollback(usize::MAX);
                    let max = screen.scrollback();
                    screen.set_scrollback(0);
                    max
                })
            })
        })
        .unwrap_or(0)
}

/// Returns true if the key was handled as a scroll action.
pub fn handle_scroll_key(
    app: &app::App,
    terminal_pane: &mut TerminalPane,
    key: crossterm::event::KeyEvent,
    qa: bool,
) -> bool {
    // Ctrl+b: enter/continue scroll mode (half page up)
    if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
        let max = scrollback_max(app, qa);
        terminal_pane.scroll_up(qa, terminal_half_page_size(), max);
        return true;
    }

    // Ctrl+f: half page down
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        terminal_pane.scroll_down(qa, terminal_half_page_size());
        return true;
    }

    if !terminal_pane.is_scrolling(qa) {
        return false;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            let max = scrollback_max(app, qa);
            terminal_pane.scroll_up(qa, 1, max);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            terminal_pane.scroll_down(qa, 1);
        }
        KeyCode::PageUp => {
            let max = scrollback_max(app, qa);
            terminal_pane.scroll_up(qa, terminal_half_page_size() * 2, max);
        }
        KeyCode::PageDown => {
            terminal_pane.scroll_down(qa, terminal_half_page_size() * 2);
        }
        KeyCode::Char('v') => {
            let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            // Approximate viewport: terminal height minus borders and status line
            let viewport_rows = usize::from(rows).saturating_sub(4);
            let viewport_cols = 80; // Will be refined by actual render area
            terminal_pane.enter_copy_mode(qa, viewport_rows, viewport_cols);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            terminal_pane.exit_scroll(qa);
        }
        _ => {
            terminal_pane.exit_scroll(qa);
            return false;
        }
    }
    true
}
