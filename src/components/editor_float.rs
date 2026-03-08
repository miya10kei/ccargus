use std::sync::{Arc, Mutex};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear};

use crate::components::Component;
use crate::domain::pty::PtySession;

pub struct EditorFloat {
    pub visible: bool,
    pty: Option<PtySession>,
    title: String,
}

impl EditorFloat {
    pub fn new() -> Self {
        Self {
            visible: false,
            pty: None,
            title: String::new(),
        }
    }

    pub fn clear_dirty(&self) {
        if let Some(pty) = &self.pty {
            pty.clear_dirty();
        }
    }

    pub fn close(&mut self) {
        if let Some(pty) = &mut self.pty {
            pty.kill();
        }
        self.pty = None;
        self.visible = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.pty.as_ref().is_some_and(PtySession::is_dirty)
    }

    pub fn is_process_alive(&mut self) -> bool {
        self.pty.as_mut().is_some_and(PtySession::is_alive)
    }

    pub fn open(
        &mut self,
        editor_command: &str,
        working_dir: &str,
        rows: u16,
        cols: u16,
    ) -> color_eyre::Result<()> {
        let inner_rows = (rows * 80 / 100).saturating_sub(2);
        let inner_cols = (cols * 80 / 100).saturating_sub(2);

        let pty = PtySession::spawn(editor_command, working_dir, inner_rows, inner_cols)?;
        self.pty = Some(pty);
        self.title = format!(" {editor_command} ({working_dir}) ");
        self.visible = true;
        Ok(())
    }

    pub fn screen(&self) -> Option<Arc<Mutex<vt100::Parser>>> {
        self.pty.as_ref().map(PtySession::screen)
    }

    pub fn write(&mut self, data: &[u8]) -> color_eyre::Result<()> {
        if let Some(pty) = &mut self.pty {
            pty.write(data)?;
        }
        Ok(())
    }
}

impl Component for EditorFloat {
    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if let Some(parser_arc) = self.screen()
            && let Ok(mut parser) = parser_arc.lock()
        {
            let vt_screen = parser.screen_mut();
            crate::components::terminal_pane::render_vt100_screen(
                vt_screen,
                inner,
                frame.buffer_mut(),
                0,
            );
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_is_not_visible() {
        let editor = EditorFloat::new();
        assert!(!editor.visible);
        assert!(editor.screen().is_none());
    }

    #[test]
    fn close_hides_editor() {
        let mut editor = EditorFloat::new();
        editor.visible = true;
        editor.close();
        assert!(!editor.visible);
        assert!(editor.screen().is_none());
    }

    #[test]
    fn open_with_valid_command_makes_visible() {
        let mut editor = EditorFloat::new();
        let result = editor.open("cat", "/tmp", 24, 80);
        assert!(result.is_ok());
        assert!(editor.visible);
        assert!(editor.screen().is_some());
        editor.close();
    }

    #[test]
    fn centered_rect_produces_valid_area() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = centered_rect(80, 80, area);
        assert!(popup.width > 0);
        assert!(popup.height > 0);
        assert!(popup.x > 0);
        assert!(popup.y > 0);
    }
}
