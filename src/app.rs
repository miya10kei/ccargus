use crate::domain::worktree::WorktreePool;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum AppState {
    #[default]
    Running,
    Quit,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    #[default]
    Worktrees,
    Terminal,
    QaTerminal,
}

pub struct App {
    pub state: AppState,
    pub focus: Focus,
    pub selected_worktree: usize,
    pub worktree_pool: WorktreePool,
}

impl App {
    pub fn is_running(&self) -> bool {
        self.state == AppState::Running
    }

    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            focus: Focus::default(),
            selected_worktree: 0,
            worktree_pool: WorktreePool::new(),
        }
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quit;
    }

    pub fn select_next_worktree(&mut self, max: usize) {
        if max > 0 && self.selected_worktree < max - 1 {
            self.selected_worktree += 1;
        }
    }

    pub fn select_prev_worktree(&mut self) {
        self.selected_worktree = self.selected_worktree.saturating_sub(1);
    }

    pub fn toggle_focus(&mut self, has_qa: bool) {
        self.focus = match self.focus {
            Focus::Worktrees => Focus::Terminal,
            Focus::Terminal => {
                if has_qa {
                    Focus::QaTerminal
                } else {
                    Focus::Worktrees
                }
            }
            Focus::QaTerminal => Focus::Worktrees,
        };
    }

    pub fn toggle_terminal_qa_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Terminal => Focus::QaTerminal,
            Focus::QaTerminal => Focus::Terminal,
            Focus::Worktrees => Focus::Worktrees,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_is_running() {
        let app = App::new();
        assert_eq!(app.state, AppState::Running);
        assert_eq!(app.focus, Focus::Worktrees);
        assert_eq!(app.selected_worktree, 0);
    }

    #[test]
    fn quit_stops_app() {
        let mut app = App::new();
        app.quit();
        assert_eq!(app.state, AppState::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn toggle_focus_without_qa() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Worktrees);
        app.toggle_focus(false);
        assert_eq!(app.focus, Focus::Terminal);
        app.toggle_focus(false);
        assert_eq!(app.focus, Focus::Worktrees);
    }

    #[test]
    fn toggle_focus_with_qa() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Worktrees);
        app.toggle_focus(true);
        assert_eq!(app.focus, Focus::Terminal);
        app.toggle_focus(true);
        assert_eq!(app.focus, Focus::QaTerminal);
        app.toggle_focus(true);
        assert_eq!(app.focus, Focus::Worktrees);
    }

    #[test]
    fn toggle_terminal_qa_focus_switches() {
        let mut app = App::new();
        app.focus = Focus::Terminal;
        app.toggle_terminal_qa_focus();
        assert_eq!(app.focus, Focus::QaTerminal);
        app.toggle_terminal_qa_focus();
        assert_eq!(app.focus, Focus::Terminal);
    }

    #[test]
    fn toggle_terminal_qa_focus_noop_from_worktrees() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Worktrees);
        app.toggle_terminal_qa_focus();
        assert_eq!(app.focus, Focus::Worktrees);
    }

    #[test]
    fn select_next_within_bounds() {
        let mut app = App::new();
        app.select_next_worktree(3);
        assert_eq!(app.selected_worktree, 1);
        app.select_next_worktree(3);
        assert_eq!(app.selected_worktree, 2);
        app.select_next_worktree(3);
        assert_eq!(app.selected_worktree, 2);
    }

    #[test]
    fn select_next_noop_when_empty() {
        let mut app = App::new();
        app.select_next_worktree(0);
        assert_eq!(app.selected_worktree, 0);
    }

    #[test]
    fn select_prev_with_floor() {
        let mut app = App::new();
        app.selected_worktree = 2;
        app.select_prev_worktree();
        assert_eq!(app.selected_worktree, 1);
        app.select_prev_worktree();
        assert_eq!(app.selected_worktree, 0);
        app.select_prev_worktree();
        assert_eq!(app.selected_worktree, 0);
    }
}
