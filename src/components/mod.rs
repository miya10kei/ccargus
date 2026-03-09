pub mod confirm_dialog;
pub mod help_overlay;
pub mod qa_selector;
pub mod repo_selector;
pub mod status_line;
pub mod terminal_pane;
pub mod utils;
pub mod worktree_tree;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

pub trait Component {
    fn handle_key_event(&mut self, key: KeyEvent) {
        let _ = key;
    }

    fn render(&self, frame: &mut Frame, area: Rect);
}
