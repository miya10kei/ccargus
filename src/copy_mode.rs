use std::cmp::Ordering;
use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorPos {
    pub col: usize,
    pub row: usize,
}

impl CursorPos {
    fn before_or_eq(&self, other: &Self) -> bool {
        (self.row, self.col) <= (other.row, other.col)
    }
}

pub enum ScrollDirection {
    Down,
    Up,
}

pub struct CopyModeState {
    pub anchor: Option<CursorPos>,
    pub cursor: CursorPos,
    pub viewport_cols: usize,
    pub viewport_rows: usize,
}

impl CopyModeState {
    pub fn new(viewport_rows: usize, viewport_cols: usize) -> Self {
        Self {
            anchor: None,
            cursor: CursorPos { row: 0, col: 0 },
            viewport_cols,
            viewport_rows,
        }
    }

    pub fn copy_to_clipboard(text: &str) -> color_eyre::Result<()> {
        if let Ok(mut clipboard) = arboard::Clipboard::new()
            && clipboard.set_text(text).is_ok()
        {
            return Ok(());
        }

        // OSC 52 fallback
        let encoded = STANDARD.encode(text.as_bytes());
        let osc = format!("\x1b]52;c;{encoded}\x07");
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(osc.as_bytes())?;
        stdout.flush()?;
        Ok(())
    }

    pub fn extract_text(&self, screen: &vt100::Screen, scroll_offset: usize) -> String {
        let Some(anchor) = &self.anchor else {
            return String::new();
        };

        let (start, end) = if anchor.before_or_eq(&self.cursor) {
            (anchor, &self.cursor)
        } else {
            (&self.cursor, anchor)
        };

        let mut result = String::new();
        let mut cloned_screen = screen.clone();
        cloned_screen.set_scrollback(scroll_offset);

        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col } else { 0 };
            let col_end = if row == end.row {
                end.col
            } else {
                self.viewport_cols.saturating_sub(1)
            };

            let mut line = String::new();
            let mut col = col_start;
            while col <= col_end {
                if let Some(cell) = cloned_screen.cell(
                    u16::try_from(row).unwrap_or(0),
                    u16::try_from(col).unwrap_or(0),
                ) {
                    let contents = cell.contents();
                    if contents.is_empty() {
                        line.push(' ');
                        col += 1;
                    } else {
                        let char_width = unicode_display_width(contents);
                        line.push_str(contents);
                        col += char_width.max(1);
                    }
                } else {
                    line.push(' ');
                    col += 1;
                }
            }

            let trimmed = line.trim_end();
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }

        result
    }

    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        let Some(anchor) = &self.anchor else {
            return false;
        };

        let (start, end) = if anchor.before_or_eq(&self.cursor) {
            (anchor, &self.cursor)
        } else {
            (&self.cursor, anchor)
        };

        match row.cmp(&start.row) {
            Ordering::Less => false,
            Ordering::Equal if start.row == end.row => col >= start.col && col <= end.col,
            Ordering::Equal => col >= start.col,
            Ordering::Greater => match row.cmp(&end.row) {
                Ordering::Greater => false,
                Ordering::Equal => col <= end.col,
                Ordering::Less => true,
            },
        }
    }

    pub fn move_down(&mut self) -> Option<ScrollDirection> {
        if self.cursor.row + 1 < self.viewport_rows {
            self.cursor.row += 1;
            None
        } else {
            Some(ScrollDirection::Down)
        }
    }

    pub fn move_left(&mut self) {
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub fn move_line_end(&mut self) {
        self.cursor.col = self.viewport_cols.saturating_sub(1);
    }

    pub fn move_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_right(&mut self) {
        if self.cursor.col + 1 < self.viewport_cols {
            self.cursor.col += 1;
        }
    }

    pub fn move_top(&mut self) {
        self.cursor.row = 0;
    }

    pub fn move_bottom(&mut self) {
        self.cursor.row = self.viewport_rows.saturating_sub(1);
    }

    pub fn move_up(&mut self) -> Option<ScrollDirection> {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            None
        } else {
            Some(ScrollDirection::Up)
        }
    }

    pub fn move_word_backward(&mut self, screen: &vt100::Screen, scroll_offset: usize) {
        let mut cloned = screen.clone();
        cloned.set_scrollback(scroll_offset);

        // Move left past current word, then past spaces to find previous word start
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }

        // Skip spaces backward
        while self.cursor.col > 0 && Self::cell_is_space(&cloned, self.cursor.row, self.cursor.col)
        {
            self.cursor.col -= 1;
        }

        // Skip word chars backward
        while self.cursor.col > 0
            && !Self::cell_is_space(&cloned, self.cursor.row, self.cursor.col - 1)
        {
            self.cursor.col -= 1;
        }
    }

    pub fn move_word_forward(&mut self, screen: &vt100::Screen, scroll_offset: usize) {
        let mut cloned = screen.clone();
        cloned.set_scrollback(scroll_offset);

        let max_col = self.viewport_cols.saturating_sub(1);

        // Skip current word chars
        while self.cursor.col < max_col
            && !Self::cell_is_space(&cloned, self.cursor.row, self.cursor.col)
        {
            self.cursor.col += 1;
        }

        // Skip spaces
        while self.cursor.col < max_col
            && Self::cell_is_space(&cloned, self.cursor.row, self.cursor.col)
        {
            self.cursor.col += 1;
        }
    }

    pub fn toggle_selection(&mut self) {
        if self.anchor.is_some() {
            self.anchor = None;
        } else {
            self.anchor = Some(self.cursor.clone());
        }
    }

    fn cell_is_space(screen: &vt100::Screen, row: usize, col: usize) -> bool {
        screen
            .cell(
                u16::try_from(row).unwrap_or(0),
                u16::try_from(col).unwrap_or(0),
            )
            .is_none_or(|cell| {
                let c = cell.contents();
                c.is_empty() || c == " "
            })
    }
}

fn unicode_display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if ('\u{1100}'..='\u{115F}').contains(&c)
                || ('\u{2E80}'..='\u{A4CF}').contains(&c)
                || ('\u{AC00}'..='\u{D7A3}').contains(&c)
                || ('\u{F900}'..='\u{FAFF}').contains(&c)
                || ('\u{FE10}'..='\u{FE6F}').contains(&c)
                || ('\u{FF01}'..='\u{FF60}').contains(&c)
                || ('\u{FFE0}'..='\u{FFE6}').contains(&c)
                || ('\u{20000}'..='\u{2FFFD}').contains(&c)
                || ('\u{30000}'..='\u{3FFFD}').contains(&c)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(rows: usize, cols: usize) -> CopyModeState {
        CopyModeState::new(rows, cols)
    }

    #[test]
    fn cursor_starts_at_origin() {
        let state = make_state(24, 80);
        assert_eq!(state.cursor, CursorPos { row: 0, col: 0 });
        assert!(state.anchor.is_none());
    }

    #[test]
    fn move_right_clamps_at_viewport() {
        let mut state = make_state(24, 3);
        state.move_right();
        assert_eq!(state.cursor.col, 1);
        state.move_right();
        assert_eq!(state.cursor.col, 2);
        state.move_right();
        assert_eq!(state.cursor.col, 2); // clamped
    }

    #[test]
    fn move_left_saturates_at_zero() {
        let mut state = make_state(24, 80);
        state.move_left();
        assert_eq!(state.cursor.col, 0);
    }

    #[test]
    fn move_down_returns_scroll_at_bottom() {
        let mut state = make_state(3, 80);
        assert!(state.move_down().is_none());
        assert_eq!(state.cursor.row, 1);
        assert!(state.move_down().is_none());
        assert_eq!(state.cursor.row, 2);
        assert!(state.move_down().is_some()); // need scroll
        assert_eq!(state.cursor.row, 2); // stays at bottom
    }

    #[test]
    fn move_up_returns_scroll_at_top() {
        let mut state = make_state(3, 80);
        assert!(state.move_up().is_some()); // need scroll
        assert_eq!(state.cursor.row, 0);
    }

    #[test]
    fn move_line_start_end() {
        let mut state = make_state(24, 80);
        state.cursor.col = 40;
        state.move_line_start();
        assert_eq!(state.cursor.col, 0);
        state.move_line_end();
        assert_eq!(state.cursor.col, 79);
    }

    #[test]
    fn move_top_bottom() {
        let mut state = make_state(24, 80);
        state.cursor.row = 12;
        state.move_top();
        assert_eq!(state.cursor.row, 0);
        state.move_bottom();
        assert_eq!(state.cursor.row, 23);
    }

    #[test]
    fn toggle_selection() {
        let mut state = make_state(24, 80);
        state.cursor = CursorPos { row: 5, col: 10 };
        state.toggle_selection();
        assert_eq!(state.anchor, Some(CursorPos { row: 5, col: 10 }));
        state.toggle_selection();
        assert!(state.anchor.is_none());
    }

    #[test]
    fn is_selected_single_line() {
        let mut state = make_state(24, 80);
        state.anchor = Some(CursorPos { row: 2, col: 5 });
        state.cursor = CursorPos { row: 2, col: 10 };

        assert!(!state.is_selected(2, 4));
        assert!(state.is_selected(2, 5));
        assert!(state.is_selected(2, 7));
        assert!(state.is_selected(2, 10));
        assert!(!state.is_selected(2, 11));
        assert!(!state.is_selected(1, 7));
        assert!(!state.is_selected(3, 7));
    }

    #[test]
    fn is_selected_multi_line() {
        let mut state = make_state(24, 80);
        state.anchor = Some(CursorPos { row: 1, col: 5 });
        state.cursor = CursorPos { row: 3, col: 10 };

        // Row 0: not selected
        assert!(!state.is_selected(0, 5));
        // Row 1: from col 5 onward
        assert!(!state.is_selected(1, 4));
        assert!(state.is_selected(1, 5));
        assert!(state.is_selected(1, 79));
        // Row 2: entire row
        assert!(state.is_selected(2, 0));
        assert!(state.is_selected(2, 79));
        // Row 3: up to col 10
        assert!(state.is_selected(3, 0));
        assert!(state.is_selected(3, 10));
        assert!(!state.is_selected(3, 11));
        // Row 4: not selected
        assert!(!state.is_selected(4, 0));
    }

    #[test]
    fn is_selected_reversed_anchor_cursor() {
        let mut state = make_state(24, 80);
        state.anchor = Some(CursorPos { row: 3, col: 10 });
        state.cursor = CursorPos { row: 1, col: 5 };

        // Same selection as forward direction
        assert!(state.is_selected(1, 5));
        assert!(state.is_selected(2, 40));
        assert!(state.is_selected(3, 10));
        assert!(!state.is_selected(3, 11));
    }

    #[test]
    fn extract_text_single_line() {
        let parser = vt100::Parser::new(24, 80, 0);
        let screen = parser.screen().clone();

        let mut state = make_state(24, 80);
        // No anchor -> empty
        assert_eq!(state.extract_text(&screen, 0), "");

        // With anchor
        state.anchor = Some(CursorPos { row: 0, col: 0 });
        state.cursor = CursorPos { row: 0, col: 4 };
        // All spaces, trimmed
        assert_eq!(state.extract_text(&screen, 0), "");
    }

    #[test]
    fn extract_text_with_content() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Hello World");
        let screen = parser.screen().clone();

        let mut state = make_state(24, 80);
        state.anchor = Some(CursorPos { row: 0, col: 0 });
        state.cursor = CursorPos { row: 0, col: 10 };
        assert_eq!(state.extract_text(&screen, 0), "Hello World");
    }

    #[test]
    fn extract_text_multi_line() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(b"Line 1\r\nLine 2\r\nLine 3");
        let screen = parser.screen().clone();

        let mut state = make_state(24, 80);
        state.anchor = Some(CursorPos { row: 0, col: 0 });
        state.cursor = CursorPos { row: 2, col: 5 };
        let text = state.extract_text(&screen, 0);
        assert!(text.contains("Line 1"));
        assert!(text.contains("Line 2"));
        assert!(text.contains("Line 3"));
    }

    #[test]
    fn is_selected_no_anchor_returns_false() {
        let state = make_state(24, 80);
        assert!(!state.is_selected(0, 0));
        assert!(!state.is_selected(5, 10));
    }

    #[test]
    fn move_word_backward_finds_previous_word() {
        let mut parser = vt100::Parser::new(1, 20, 0);
        parser.process(b"hello world");
        let screen = parser.screen().clone();

        let mut state = make_state(1, 20);
        state.cursor.col = 10; // end of "world"
        state.move_word_backward(&screen, 0);
        assert_eq!(state.cursor.col, 6); // start of "world"
    }

    #[test]
    fn move_word_forward_skips_to_next_word() {
        let mut parser = vt100::Parser::new(1, 20, 0);
        parser.process(b"hello world");
        let screen = parser.screen().clone();

        let mut state = make_state(1, 20);
        state.cursor.col = 0;
        state.move_word_forward(&screen, 0);
        assert_eq!(state.cursor.col, 6); // start of "world"
    }

    #[test]
    fn unicode_display_width_ascii() {
        assert_eq!(unicode_display_width("hello"), 5);
    }

    #[test]
    fn unicode_display_width_cjk() {
        assert_eq!(unicode_display_width("世界"), 4);
    }

    #[test]
    fn unicode_display_width_empty() {
        assert_eq!(unicode_display_width(""), 0);
    }

    #[test]
    fn unicode_display_width_mixed() {
        assert_eq!(unicode_display_width("hi世"), 4);
    }
}
