use std::sync::{Arc, Mutex};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

use crate::components::Component;

const PLACEHOLDER_TEXT: &str = "No session selected. Press 'n' to create a new session.";

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
    let x = area.x;
    let y = area.y;
    for (i, ch) in PLACEHOLDER_TEXT.chars().enumerate() {
        let col = x + u16::try_from(i).unwrap_or(0);
        if col >= area.right() {
            break;
        }
        buf[(col, y)].set_char(ch);
    }
}

fn render_vt100_screen(vt_screen: &vt100::Screen, area: Rect, buf: &mut Buffer) {
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
    fn renders_placeholder_when_no_session() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let pane = TerminalPane::new();

        terminal
            .draw(|frame| {
                pane.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..10 {
            for x in 0..80 {
                text.push_str(buffer[(x, y)].symbol());
            }
        }
        assert!(
            text.contains("No session selected"),
            "Should contain placeholder text, got: {text}"
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
