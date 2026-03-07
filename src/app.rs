#[derive(Debug, Default, PartialEq, Eq)]
pub enum AppState {
    #[default]
    Running,
    Quit,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum Focus {
    #[default]
    Sessions,
    Terminal,
}

#[derive(Debug)]
pub struct App {
    pub state: AppState,
    pub focus: Focus,
    pub selected_session: usize,
}

impl App {
    pub fn is_running(&self) -> bool {
        self.state == AppState::Running
    }

    pub fn new() -> Self {
        Self {
            state: AppState::default(),
            focus: Focus::default(),
            selected_session: 0,
        }
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quit;
    }

    pub fn select_next_session(&mut self, max: usize) {
        if max > 0 && self.selected_session < max - 1 {
            self.selected_session += 1;
        }
    }

    pub fn select_prev_session(&mut self) {
        self.selected_session = self.selected_session.saturating_sub(1);
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sessions => Focus::Terminal,
            Focus::Terminal => Focus::Sessions,
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
        assert_eq!(app.focus, Focus::Sessions);
        assert_eq!(app.selected_session, 0);
    }

    #[test]
    fn quit_stops_app() {
        let mut app = App::new();
        app.quit();
        assert_eq!(app.state, AppState::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn toggle_focus_switches() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Sessions);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Terminal);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Sessions);
    }

    #[test]
    fn select_next_within_bounds() {
        let mut app = App::new();
        app.select_next_session(3);
        assert_eq!(app.selected_session, 1);
        app.select_next_session(3);
        assert_eq!(app.selected_session, 2);
        app.select_next_session(3);
        assert_eq!(app.selected_session, 2);
    }

    #[test]
    fn select_next_noop_when_empty() {
        let mut app = App::new();
        app.select_next_session(0);
        assert_eq!(app.selected_session, 0);
    }

    #[test]
    fn select_prev_with_floor() {
        let mut app = App::new();
        app.selected_session = 2;
        app.select_prev_session();
        assert_eq!(app.selected_session, 1);
        app.select_prev_session();
        assert_eq!(app.selected_session, 0);
        app.select_prev_session();
        assert_eq!(app.selected_session, 0);
    }
}
