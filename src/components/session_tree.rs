use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::components::Component;

pub struct SessionEntry {
    pub branch: String,
    pub name: String,
    pub repo: String,
}

pub struct SessionTree {
    pub focused: bool,
    pub selected: usize,
    pub sessions: Vec<SessionEntry>,
}

impl SessionTree {
    pub fn new() -> Self {
        Self {
            focused: true,
            selected: 0,
            sessions: Vec::new(),
        }
    }

    fn border_color(&self) -> Color {
        if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }
    }

    fn build_tree_items(&self) -> Vec<ListItem<'static>> {
        if self.sessions.is_empty() {
            return vec![ListItem::new("  (no sessions)")];
        }

        let groups = group_by_repo(&self.sessions);
        let mut items = Vec::new();
        let mut flat_index = 0usize;

        for (repo, entries) in &groups {
            items.push(ListItem::new(format!("\u{25bc} {repo}")));
            for entry in entries {
                let marker = if flat_index == self.selected {
                    "\u{25b6}"
                } else {
                    " "
                };
                items.push(ListItem::new(format!("  {marker} {}", entry.branch)));
                flat_index += 1;
            }
        }

        items
    }

    fn selected_list_index(&self) -> Option<usize> {
        if self.sessions.is_empty() {
            return None;
        }

        let groups = group_by_repo(&self.sessions);
        let mut list_index = 0usize;
        let mut flat_index = 0usize;

        for (_repo, entries) in &groups {
            list_index += 1; // repo header row
            for _entry in entries {
                if flat_index == self.selected {
                    return Some(list_index);
                }
                list_index += 1;
                flat_index += 1;
            }
        }

        None
    }
}

pub fn group_by_repo(sessions: &[SessionEntry]) -> Vec<(String, Vec<&SessionEntry>)> {
    if sessions.is_empty() {
        return Vec::new();
    }

    let mut order: Vec<String> = Vec::new();
    let mut map: BTreeMap<&str, Vec<&SessionEntry>> = BTreeMap::new();

    for session in sessions {
        if !map.contains_key(session.repo.as_str()) {
            order.push(session.repo.clone());
        }
        map.entry(&session.repo).or_default().push(session);
    }

    order
        .into_iter()
        .filter_map(|repo| map.remove(repo.as_str()).map(|entries| (repo, entries)))
        .collect()
}

impl Component for SessionTree {
    fn render(&self, frame: &mut Frame, area: Rect) {
        let items = self.build_tree_items();

        let block = Block::default()
            .title(" Sessions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(self.selected_list_index());

        frame.render_stateful_widget(list, area, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;

    fn make_entry(name: &str, repo: &str, branch: &str) -> SessionEntry {
        SessionEntry {
            branch: branch.to_owned(),
            name: name.to_owned(),
            repo: repo.to_owned(),
        }
    }

    #[test]
    fn group_by_repo_empty_input_returns_empty() {
        let result = group_by_repo(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn group_by_repo_same_repo_returns_one_group() {
        let entries = vec![
            make_entry("s1", "myrepo", "main"),
            make_entry("s2", "myrepo", "dev"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "myrepo");
        assert_eq!(result[0].1.len(), 2);
        assert_eq!(result[0].1[0].branch, "main");
        assert_eq!(result[0].1[1].branch, "dev");
    }

    #[test]
    fn group_by_repo_different_repos_returns_separate_groups() {
        let entries = vec![
            make_entry("s1", "repo-a", "main"),
            make_entry("s2", "repo-b", "main"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "repo-a");
        assert_eq!(result[1].0, "repo-b");
    }

    #[test]
    fn group_by_repo_preserves_insertion_order() {
        let entries = vec![
            make_entry("s1", "zzz-repo", "main"),
            make_entry("s2", "aaa-repo", "main"),
            make_entry("s3", "zzz-repo", "dev"),
        ];
        let result = group_by_repo(&entries);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "zzz-repo");
        assert_eq!(result[0].1.len(), 2);
        assert_eq!(result[1].0, "aaa-repo");
        assert_eq!(result[1].1.len(), 1);
    }

    #[test]
    fn renders_with_border_color_when_focused() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = SessionTree {
            focused: true,
            selected: 0,
            sessions: Vec::new(),
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
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
            sessions: Vec::new(),
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

    #[test]
    fn renders_no_sessions_when_empty() {
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
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect::<String>();
        assert!(
            text.contains("(no sessions)"),
            "Should show '(no sessions)' when empty, got: {text}"
        );
    }

    #[test]
    fn renders_tree_with_sessions() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = SessionTree {
            focused: true,
            selected: 0,
            sessions: vec![
                make_entry("s1", "my/repo", "main"),
                make_entry("s2", "my/repo", "dev"),
            ],
        };

        terminal
            .draw(|frame| {
                tree.render(frame, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row1: String = (0..40)
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect::<String>();
        let row2: String = (0..40)
            .map(|x| buffer[(x, 2)].symbol().to_string())
            .collect::<String>();
        assert!(
            row1.contains("my/repo"),
            "Should show repo header, got: {row1}"
        );
        assert!(
            row2.contains("main"),
            "Should show branch name, got: {row2}"
        );
    }
}
