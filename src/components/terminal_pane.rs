use std::sync::{Arc, Mutex};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

use crate::components::Component;
use crate::copy_mode::CopyModeState;

const BANNER: &[&str] = &[
    "  ██████╗ ██████╗  █████╗ ██████╗  ██████╗ ██╗   ██╗███████╗",
    " ██╔════╝██╔════╝ ██╔══██╗██╔══██╗██╔════╝ ██║   ██║██╔════╝",
    " ██║     ██║      ███████║██████╔╝██║  ███╗██║   ██║███████╗",
    " ██║     ██║      ██╔══██║██╔══██╗██║   ██║██║   ██║╚════██║",
    " ╚██████╗╚██████╗ ██║  ██║██║  ██║╚██████╔╝╚██████╔╝███████║",
    "  ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝ ╚══════╝",
];
const HINT_TEXT: &str = "Press 'n' to create a new worktree.";

pub struct TerminalPane {
    pub copy_mode: Option<CopyModeState>,
    pub focused: bool,
    pub qa_copy_mode: Option<CopyModeState>,
    pub qa_focused: bool,
    pub qa_screen: Option<Arc<Mutex<vt100::Parser>>>,
    pub qa_scroll_offset: usize,
    pub screen: Option<Arc<Mutex<vt100::Parser>>>,
    pub scroll_offset: usize,
}

impl TerminalPane {
    pub fn new() -> Self {
        Self {
            copy_mode: None,
            focused: false,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: None,
            scroll_offset: 0,
        }
    }

    fn border_color_for(focused: bool, scrolling: bool, copy_mode: bool) -> Color {
        if copy_mode {
            Color::Magenta
        } else if scrolling {
            Color::Yellow
        } else if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }

    pub fn copy_mode_for(&self, qa: bool) -> Option<&CopyModeState> {
        if qa {
            self.qa_copy_mode.as_ref()
        } else {
            self.copy_mode.as_ref()
        }
    }

    pub fn copy_mode_mut(&mut self, qa: bool) -> Option<&mut CopyModeState> {
        if qa {
            self.qa_copy_mode.as_mut()
        } else {
            self.copy_mode.as_mut()
        }
    }

    pub fn enter_copy_mode(&mut self, qa: bool, viewport_rows: usize, viewport_cols: usize) {
        let state = CopyModeState::new(viewport_rows, viewport_cols);
        if qa {
            self.qa_copy_mode = Some(state);
        } else {
            self.copy_mode = Some(state);
        }
    }

    pub fn exit_copy_mode(&mut self, qa: bool) {
        if qa {
            self.qa_copy_mode = None;
            self.qa_scroll_offset = 0;
        } else {
            self.copy_mode = None;
            self.scroll_offset = 0;
        }
    }

    pub fn exit_scroll(&mut self, qa: bool) {
        *self.scroll_offset_mut(qa) = 0;
    }

    /// Returns the vt100 cursor position mapped to screen coordinates for
    /// the focused pane. Called after `tui.draw()` to send `cursor::MoveTo`
    /// so the terminal emulator positions the (hidden) cursor for IME.
    pub fn cursor_position_for_ime(&self, terminal_area: Rect) -> Option<(u16, u16)> {
        let is_qa = self.qa_focused;
        let focused = if is_qa { self.qa_focused } else { self.focused };
        if !focused {
            return None;
        }
        let qa = is_qa;
        if self.scroll_offset_for(qa) > 0 || self.is_in_copy_mode(qa) {
            return None;
        }

        let screen = if qa {
            self.qa_screen.as_ref()
        } else {
            self.screen.as_ref()
        };

        let parser_arc = screen?;
        let parser = parser_arc.lock().ok()?;
        let vt_screen = parser.screen();
        let (cursor_row, cursor_col) = vt_screen.cursor_position();

        // Calculate inner area matching render_pane's block.inner(area)
        let pane_area = if self.qa_screen.is_some() {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(terminal_area);
            if qa { split[1] } else { split[0] }
        } else {
            terminal_area
        };
        let inner = Block::default().borders(Borders::ALL).inner(pane_area);

        let x = inner.x + cursor_col;
        let y = inner.y + cursor_row;
        if x < inner.right() && y < inner.bottom() {
            Some((x, y))
        } else {
            None
        }
    }

    pub fn is_in_copy_mode(&self, qa: bool) -> bool {
        self.copy_mode_for(qa).is_some()
    }

    pub fn is_scrolling(&self, qa: bool) -> bool {
        self.scroll_offset_for(qa) > 0
    }

    pub fn scroll_down(&mut self, qa: bool, lines: usize) {
        let offset = self.scroll_offset_mut(qa);
        *offset = offset.saturating_sub(lines);
    }

    pub fn scroll_up(&mut self, qa: bool, lines: usize, max_scrollback: usize) {
        let offset = self.scroll_offset_mut(qa);
        *offset = (*offset + lines).min(max_scrollback);
    }

    pub fn scroll_offset_for(&self, qa: bool) -> usize {
        if qa {
            self.qa_scroll_offset
        } else {
            self.scroll_offset
        }
    }

    fn scroll_offset_mut(&mut self, qa: bool) -> &mut usize {
        if qa {
            &mut self.qa_scroll_offset
        } else {
            &mut self.scroll_offset
        }
    }

    fn render_pane(
        frame: &mut Frame,
        area: Rect,
        label: &str,
        focused: bool,
        scroll_offset: usize,
        screen: Option<&Arc<Mutex<vt100::Parser>>>,
        copy_mode: Option<&CopyModeState>,
    ) {
        let scrolling = scroll_offset > 0;
        let in_copy_mode = copy_mode.is_some();
        let title = if in_copy_mode {
            format!(" {label} [COPY] ")
        } else if scrolling {
            format!(" {label} [SCROLL] ")
        } else {
            format!(" {label} ")
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::border_color_for(
                focused,
                scrolling,
                in_copy_mode,
            )));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(parser_arc) = screen
            && let Ok(mut parser) = parser_arc.lock()
        {
            let vt_screen = parser.screen_mut();
            render_vt100_screen(
                vt_screen,
                inner,
                frame.buffer_mut(),
                scroll_offset,
                copy_mode,
            );
        }
    }

    fn render_single_pane(&self, frame: &mut Frame, area: Rect) {
        if self.screen.is_some() {
            Self::render_pane(
                frame,
                area,
                "Terminal",
                self.focused,
                self.scroll_offset,
                self.screen.as_ref(),
                self.copy_mode.as_ref(),
            );
        } else {
            let block = Block::default()
                .title(" Terminal ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Self::border_color_for(
                    self.focused,
                    false,
                    false,
                )));
            let inner = block.inner(area);
            frame.render_widget(block, area);
            render_placeholder(inner, frame.buffer_mut());
        }
    }

    fn render_split_pane(&self, frame: &mut Frame, area: Rect) {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        Self::render_pane(
            frame,
            horizontal[0],
            "Terminal",
            self.focused,
            self.scroll_offset,
            self.screen.as_ref(),
            self.copy_mode.as_ref(),
        );
        Self::render_pane(
            frame,
            horizontal[1],
            "Q&A",
            self.qa_focused,
            self.qa_scroll_offset,
            self.qa_screen.as_ref(),
            self.qa_copy_mode.as_ref(),
        );
    }
}

impl Component for TerminalPane {
    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.qa_screen.is_some() {
            self.render_split_pane(frame, area);
        } else {
            self.render_single_pane(frame, area);
        }
    }
}

fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn render_placeholder(area: Rect, buf: &mut Buffer) {
    let banner_height = u16::try_from(BANNER.len()).unwrap_or(0);
    let banner_width = u16::try_from(
        BANNER
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0),
    )
    .unwrap_or(0);

    // Total content height: banner + 1 blank line + hint
    let content_height = banner_height + 2;
    let start_y = area.y + area.height.saturating_sub(content_height) / 2;

    let banner_style = Style::default().fg(Color::Cyan);

    for (i, line) in BANNER.iter().enumerate() {
        let row = start_y + u16::try_from(i).unwrap_or(0);
        if row >= area.bottom() {
            break;
        }
        let start_x = area.x + area.width.saturating_sub(banner_width) / 2;
        let mut col = start_x;
        for ch in line.chars() {
            if col >= area.right() {
                break;
            }
            buf[(col, row)].set_char(ch).set_style(banner_style);
            col += 1;
        }
    }

    // Hint text below the banner
    let hint_row = start_y + banner_height + 1;
    if hint_row < area.bottom() {
        let hint_width = u16::try_from(HINT_TEXT.len()).unwrap_or(0);
        let hint_x = area.x + area.width.saturating_sub(hint_width) / 2;
        let hint_style = Style::default().fg(Color::DarkGray);
        for (i, ch) in HINT_TEXT.chars().enumerate() {
            let col = hint_x + u16::try_from(i).unwrap_or(0);
            if col >= area.right() {
                break;
            }
            buf[(col, hint_row)].set_char(ch).set_style(hint_style);
        }
    }
}

pub fn render_vt100_screen(
    vt_screen: &mut vt100::Screen,
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    copy_mode: Option<&CopyModeState>,
) {
    let rows = usize::from(area.height);
    let cols = usize::from(area.width);

    vt_screen.set_scrollback(scroll_offset);

    for row in 0..rows {
        for col in 0..cols {
            let cell = vt_screen.cell(
                u16::try_from(row).unwrap_or(0),
                u16::try_from(col).unwrap_or(0),
            );

            let buf_x = area.x + u16::try_from(col).unwrap_or(0);
            let buf_y = area.y + u16::try_from(row).unwrap_or(0);

            if let Some(cell) = cell {
                let contents = cell.contents();
                let symbol = if contents.is_empty() { " " } else { contents };

                let fg = convert_color(cell.fgcolor());
                let bg = convert_color(cell.bgcolor());
                let mut modifier = ratatui::style::Modifier::empty();

                if cell.bold() {
                    modifier |= ratatui::style::Modifier::BOLD;
                }

                if let Some(cm) = copy_mode {
                    let is_cursor = cm.cursor.row == row && cm.cursor.col == col;
                    let is_selected = cm.is_selected(row, col);
                    if is_cursor || is_selected {
                        modifier |= ratatui::style::Modifier::REVERSED;
                    }
                }

                let buf_cell = &mut buf[(buf_x, buf_y)];
                buf_cell.set_symbol(symbol);
                buf_cell.set_style(Style::default().fg(fg).bg(bg).add_modifier(modifier));
            }
        }
    }

    vt_screen.set_scrollback(0);
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn renders_banner_when_no_worktree() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let pane = TerminalPane::new();

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..20 {
            for x in 0..80 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("██████"),
            "Should contain banner art, got: {text}"
        );
        assert!(
            text.contains("Press 'n' to create a new worktree."),
            "Should contain hint text, got: {text}"
        );
    }

    #[test]
    fn renders_vt100_screen_content() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let parser = Arc::new(Mutex::new(vt100::Parser::new(8, 38, 0)));
        {
            let mut p = parser.lock().unwrap();
            p.process(b"Hello, terminal!");
        }

        let pane = TerminalPane {
            copy_mode: None,
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 0,
        };

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..10 {
            for x in 0..40 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("Hello, terminal!"),
            "Should contain vt100 screen content, got: {text}"
        );
    }

    #[test]
    fn convert_color_default_returns_reset() {
        assert_eq!(convert_color(vt100::Color::Default), Color::Reset);
    }

    #[test]
    fn convert_color_idx_returns_indexed() {
        assert_eq!(convert_color(vt100::Color::Idx(1)), Color::Indexed(1));
    }

    #[test]
    fn convert_color_rgb_returns_rgb() {
        assert_eq!(
            convert_color(vt100::Color::Rgb(255, 0, 0)),
            Color::Rgb(255, 0, 0)
        );
    }

    #[test]
    fn exit_scroll_resets_offset() {
        let mut pane = TerminalPane::new();
        pane.scroll_offset = 10;
        pane.exit_scroll(false);
        assert_eq!(pane.scroll_offset, 0);
    }

    #[test]
    fn exit_qa_scroll_resets_offset() {
        let mut pane = TerminalPane::new();
        pane.qa_scroll_offset = 5;
        pane.exit_scroll(true);
        assert_eq!(pane.qa_scroll_offset, 0);
    }

    #[test]
    fn is_scrolling_reflects_offset() {
        let mut pane = TerminalPane::new();
        assert!(!pane.is_scrolling(false));
        pane.scroll_offset = 1;
        assert!(pane.is_scrolling(false));
    }

    #[test]
    fn is_qa_scrolling_reflects_offset() {
        let mut pane = TerminalPane::new();
        assert!(!pane.is_scrolling(true));
        pane.qa_scroll_offset = 1;
        assert!(pane.is_scrolling(true));
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut pane = TerminalPane::new();
        pane.scroll_offset = 2;
        pane.scroll_down(false, 5);
        assert_eq!(pane.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_clamps_to_max() {
        let mut pane = TerminalPane::new();
        pane.scroll_up(false, 100, 50);
        assert_eq!(pane.scroll_offset, 50);
    }

    #[test]
    fn scroll_up_increments_offset() {
        let mut pane = TerminalPane::new();
        pane.scroll_up(false, 3, 100);
        assert_eq!(pane.scroll_offset, 3);
        pane.scroll_up(false, 5, 100);
        assert_eq!(pane.scroll_offset, 8);
    }

    #[test]
    fn scroll_down_decrements_offset() {
        let mut pane = TerminalPane::new();
        pane.scroll_offset = 10;
        pane.scroll_down(false, 3);
        assert_eq!(pane.scroll_offset, 7);
    }

    #[test]
    fn qa_scroll_up_clamps_to_max() {
        let mut pane = TerminalPane::new();
        pane.scroll_up(true, 100, 30);
        assert_eq!(pane.qa_scroll_offset, 30);
    }

    #[test]
    fn qa_scroll_down_saturates_at_zero() {
        let mut pane = TerminalPane::new();
        pane.qa_scroll_offset = 1;
        pane.scroll_down(true, 5);
        assert_eq!(pane.qa_scroll_offset, 0);
    }

    #[test]
    fn renders_scroll_indicator_in_title() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let parser = Arc::new(Mutex::new(vt100::Parser::new(8, 38, 0)));

        let pane = TerminalPane {
            copy_mode: None,
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 5,
        };

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for x in 0..40 {
            text.push_str(buffer[(x, 0)].symbol());
        }
        assert!(
            text.contains("[SCROLL]"),
            "Should contain [SCROLL] indicator, got: {text}"
        );
    }

    #[test]
    fn renders_yellow_border_when_scrolling() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let parser = Arc::new(Mutex::new(vt100::Parser::new(8, 38, 0)));

        let pane = TerminalPane {
            copy_mode: None,
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 1,
        };

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Top-left corner border cell should be yellow
        let border_cell = &buffer[(0, 0)];
        assert_eq!(
            border_cell.fg,
            Color::Yellow,
            "Border should be yellow when scrolling"
        );
    }

    #[test]
    fn renders_scrollback_content() {
        let backend = TestBackend::new(42, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        // Create a parser with scrollback buffer (4 visible rows, 40 cols, 100 scrollback)
        let parser = Arc::new(Mutex::new(vt100::Parser::new(4, 40, 100)));
        {
            let mut p = parser.lock().unwrap();
            // Write enough lines to push content into scrollback
            for i in 0..10 {
                p.process(format!("line {i}\r\n").as_bytes());
            }
            p.process(b"current");
        }

        // With scroll_offset=0, should see the current visible screen
        let pane = TerminalPane {
            copy_mode: None,
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 0,
        };

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..6 {
            for x in 0..42 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("current"),
            "Should show current screen content, got: {text}"
        );

        // With scroll_offset > 0, should see scrollback content
        let pane_scrolled = TerminalPane {
            copy_mode: None,
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 6,
        };

        terminal
            .draw(|frame| {
                pane_scrolled.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..6 {
            for x in 0..42 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("line 4") || text.contains("line 5"),
            "Should show scrollback content, got: {text}"
        );
    }

    #[test]
    fn border_color_copy_mode_returns_magenta() {
        assert_eq!(
            TerminalPane::border_color_for(true, true, true),
            Color::Magenta
        );
    }

    #[test]
    fn border_color_focused_returns_cyan() {
        assert_eq!(
            TerminalPane::border_color_for(true, false, false),
            Color::Cyan
        );
    }

    #[test]
    fn border_color_scrolling_returns_yellow() {
        assert_eq!(
            TerminalPane::border_color_for(true, true, false),
            Color::Yellow
        );
    }

    #[test]
    fn enter_copy_mode_creates_state() {
        let mut pane = TerminalPane::new();
        pane.enter_copy_mode(false, 24, 80);
        assert!(pane.copy_mode.is_some());
    }

    #[test]
    fn enter_qa_copy_mode_creates_state() {
        let mut pane = TerminalPane::new();
        pane.enter_copy_mode(true, 24, 80);
        assert!(pane.qa_copy_mode.is_some());
    }

    #[test]
    fn exit_copy_mode_clears_state_and_scroll() {
        let mut pane = TerminalPane::new();
        pane.enter_copy_mode(false, 24, 80);
        pane.scroll_offset = 5;
        pane.exit_copy_mode(false);
        assert!(pane.copy_mode.is_none());
        assert_eq!(pane.scroll_offset, 0);
    }

    #[test]
    fn exit_qa_copy_mode_clears_state_and_scroll() {
        let mut pane = TerminalPane::new();
        pane.enter_copy_mode(true, 24, 80);
        pane.qa_scroll_offset = 5;
        pane.exit_copy_mode(true);
        assert!(pane.qa_copy_mode.is_none());
        assert_eq!(pane.qa_scroll_offset, 0);
    }

    #[test]
    fn is_in_copy_mode_false_by_default() {
        let pane = TerminalPane::new();
        assert!(!pane.is_in_copy_mode(false));
        assert!(!pane.is_in_copy_mode(true));
    }

    #[test]
    fn is_in_copy_mode_true_when_set() {
        let mut pane = TerminalPane::new();
        pane.enter_copy_mode(false, 24, 80);
        assert!(pane.is_in_copy_mode(false));
    }

    #[test]
    fn renders_copy_indicator_in_title() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let parser = Arc::new(Mutex::new(vt100::Parser::new(8, 38, 0)));
        let copy_state = CopyModeState::new(8, 38);

        let pane = TerminalPane {
            copy_mode: Some(copy_state),
            focused: true,
            qa_copy_mode: None,
            qa_focused: false,
            qa_screen: None,
            qa_scroll_offset: 0,
            screen: Some(Arc::clone(&parser)),
            scroll_offset: 0,
        };

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect();
        assert!(
            text.contains("[COPY]"),
            "Should contain [COPY] indicator, got: {text}"
        );
    }
}
