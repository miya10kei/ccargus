use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::components::Component;

const TREE_DATA: [&str; 4] = [
    "\u{25bc} miya10kei/ccargus",
    "  \u{25b6} main",
    "\u{25bc} miya10kei/api-server",
    "    main",
];

pub struct SessionTree {
    pub focused: bool,
    pub selected: usize,
}

impl SessionTree {
    pub fn new() -> Self {
        Self {
            focused: true,
            selected: 0,
        }
    }

    fn border_color(&self) -> Color {
        if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }
}

impl Component for SessionTree {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = TREE_DATA.iter().map(|&s| ListItem::new(s)).collect();

        let block = Block::default()
            .title(" Sessions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(Some(self.selected));

        frame.render_stateful_widget(list, area, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn renders_with_border_color_when_focused() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = SessionTree {
            focused: true,
            selected: 0,
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Top-left corner border cell should be Cyan
        let corner = &buffer[(0, 0)];
        assert_eq!(corner.fg, Color::Cyan);
    }

    #[test]
    fn renders_with_border_color_when_unfocused() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = SessionTree {
            focused: false,
            selected: 0,
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let corner = &buffer[(0, 0)];
        assert_eq!(corner.fg, Color::DarkGray);
    }

    #[test]
    fn renders_with_sessions_title() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = SessionTree::new();

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect::<String>();
        assert!(
            text.contains(" Sessions "),
            "Title should contain ' Sessions ', got: {text}"
        );
    }
}
