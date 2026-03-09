use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders};

pub struct PtySizes {
    /// Single pane dimensions (no Q&A active)
    pub single_cols: u16,
    pub single_rows: u16,
    /// Split pane dimensions for main terminal (Q&A active)
    pub split_main_cols: u16,
    pub split_main_rows: u16,
    /// Split pane dimensions for Q&A terminal
    pub split_qa_cols: u16,
    pub split_qa_rows: u16,
}

impl PtySizes {
    pub fn main_size(&self, has_qa: bool) -> (u16, u16) {
        if has_qa {
            (self.split_main_rows, self.split_main_cols)
        } else {
            (self.single_rows, self.single_cols)
        }
    }
}

pub fn calculate_pty_sizes(term_cols: u16, term_rows: u16) -> PtySizes {
    let full = Rect::new(0, 0, term_cols, term_rows);

    // Vertical: content area + status line
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(full);

    // Horizontal: worktree tree (25%) + terminal pane (75%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(vertical[0]);

    let terminal_area = horizontal[1];

    // Single pane (no Q&A): terminal area with border
    let single_inner = Block::default().borders(Borders::ALL).inner(terminal_area);

    // Split pane (Q&A): 50/50 horizontal split, each with border
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(terminal_area);
    let split_main_inner = Block::default().borders(Borders::ALL).inner(split[0]);
    let split_qa_inner = Block::default().borders(Borders::ALL).inner(split[1]);

    PtySizes {
        single_cols: single_inner.width,
        single_rows: single_inner.height,
        split_main_cols: split_main_inner.width,
        split_main_rows: split_main_inner.height,
        split_qa_cols: split_qa_inner.width,
        split_qa_rows: split_qa_inner.height,
    }
}

pub fn current_pty_sizes() -> PtySizes {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    calculate_pty_sizes(cols, rows)
}

pub fn terminal_half_page_size() -> usize {
    let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    usize::from(rows) / 2
}
