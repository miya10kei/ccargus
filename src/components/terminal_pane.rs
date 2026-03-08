use std::sync::{Arc, Mutex};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

use crate::components::Component;

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
    pub focused: bool,
    pub qa_focused: bool,
    pub qa_screen: Option<Arc<Mutex<vt100::Parser>>>,
    pub screen: Option<Arc<Mutex<vt100::Parser>>>,
}

impl TerminalPane {
    pub fn new() -> Self {
        Self {
            focused: false,
            qa_focused: false,
            qa_screen: None,
            screen: None,
        }
    }

    fn border_color_for(focused: bool) -> Color {
        if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }

    fn render_single_pane(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Terminal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::border_color_for(self.focused)));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if let Some(parser_arc) = &self.screen {
            if let Ok(parser) = parser_arc.lock() {
                let vt_screen = parser.screen();
                render_vt100_screen(vt_screen, inner, frame.buffer_mut());
            }
        } else {
            render_placeholder(inner, frame.buffer_mut());
        }
    }

    fn render_split_pane(&self, frame: &mut Frame, area: Rect) {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Main terminal (left)
        let main_block = Block::default()
            .title(" Terminal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::border_color_for(self.focused)));

        let main_inner = main_block.inner(horizontal[0]);
        frame.render_widget(main_block, horizontal[0]);

        if let Some(parser_arc) = &self.screen
            && let Ok(parser) = parser_arc.lock()
        {
            let vt_screen = parser.screen();
            render_vt100_screen(vt_screen, main_inner, frame.buffer_mut());
        }

        // Q&A terminal (right)
        let qa_block = Block::default()
            .title(" Q&A ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::border_color_for(self.qa_focused)));

        let qa_inner = qa_block.inner(horizontal[1]);
        frame.render_widget(qa_block, horizontal[1]);

        if let Some(parser_arc) = &self.qa_screen
            && let Ok(parser) = parser_arc.lock()
        {
            let vt_screen = parser.screen();
            render_vt100_screen(vt_screen, qa_inner, frame.buffer_mut());
        }
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

pub fn render_vt100_screen(vt_screen: &vt100::Screen, area: Rect, buf: &mut Buffer) {
    let rows = usize::from(area.height);
    let cols = usize::from(area.width);

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

                let buf_cell = &mut buf[(buf_x, buf_y)];
                buf_cell.set_symbol(symbol);
                buf_cell.set_fg(fg);
                buf_cell.set_bg(bg);

                if cell.bold() {
                    buf_cell.set_style(
                        Style::default()
                            .fg(fg)
                            .bg(bg)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    );
                }
            }
        }
    }
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
            focused: true,
            qa_focused: false,
            qa_screen: None,
            screen: Some(Arc::clone(&parser)),
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
}
