use color_eyre::Result;

use super::pty::PtySession;

pub struct SessionInfo {
    pub id: usize,
    pub name: String,
    pub repo: String,
    pub branch: String,
    pub pty: PtySession,
    pub qa_pty: Option<PtySession>,
}

impl SessionInfo {
    pub fn close_qa_session(&mut self) {
        if let Some(qa) = &mut self.qa_pty {
            qa.kill();
        }
        self.qa_pty = None;
    }

    pub fn create_qa_session(&mut self, fork: bool, rows: u16, cols: u16) -> Result<()> {
        let working_dir = self.pty.working_dir().to_owned();
        let qa_pty = if fork {
            PtySession::spawn_with_args("claude", &["--continue"], &working_dir, rows, cols)?
        } else {
            PtySession::spawn("claude", &working_dir, rows, cols)?
        };
        self.qa_pty = Some(qa_pty);
        Ok(())
    }

    pub fn has_qa_session(&self) -> bool {
        self.qa_pty.is_some()
    }
}

pub struct SessionManager {
    next_id: usize,
    sessions: Vec<SessionInfo>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            sessions: Vec::new(),
        }
    }

    pub fn create_session(
        &mut self,
        name: &str,
        repo: &str,
        branch: &str,
        working_dir: &str,
        rows: u16,
        cols: u16,
    ) -> Result<usize> {
        let pty = PtySession::spawn("claude", working_dir, rows, cols)?;
        let id = self.next_id;
        self.next_id += 1;
        self.sessions.push(SessionInfo {
            id,
            name: name.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            pty,
            qa_pty: None,
        });
        Ok(id)
    }

    pub fn get(&self, index: usize) -> Option<&SessionInfo> {
        self.sessions.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut SessionInfo> {
        self.sessions.get_mut(index)
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn remove_session(&mut self, index: usize) {
        if index < self.sessions.len() {
            self.sessions[index].close_qa_session();
            self.sessions[index].pty.kill();
            self.sessions.remove(index);
        }
    }

    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    #[cfg(test)]
    pub fn create_test_session(
        &mut self,
        name: &str,
        repo: &str,
        branch: &str,
        working_dir: &str,
    ) -> Result<usize> {
        let pty = PtySession::spawn("cat", working_dir, 24, 80)?;
        let id = self.next_id;
        self.next_id += 1;
        self.sessions.push(SessionInfo {
            id,
            name: name.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            pty,
            qa_pty: None,
        });
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(manager: &mut SessionManager) -> usize {
        manager
            .create_test_session("test-session", "test/repo", "main", "/tmp")
            .unwrap()
    }

    #[test]
    fn new_manager_is_empty() {
        let manager = SessionManager::new();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }

    #[test]
    fn create_session_increases_len() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        assert_eq!(manager.len(), 1);
        assert!(!manager.is_empty());
    }

    #[test]
    fn create_multiple_sessions() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        create_test_session(&mut manager);
        create_test_session(&mut manager);
        assert_eq!(manager.len(), 3);
    }

    #[test]
    fn ids_are_sequential() {
        let mut manager = SessionManager::new();
        let id1 = create_test_session(&mut manager);
        let id2 = create_test_session(&mut manager);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn remove_session_decreases_len() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        create_test_session(&mut manager);
        manager.remove_session(0);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn remove_out_of_bounds_is_safe() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        manager.remove_session(99);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn get_returns_session() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        let session = manager.get(0);
        assert!(session.is_some());
        assert_eq!(session.unwrap().name, "test-session");
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let manager = SessionManager::new();
        assert!(manager.get(99).is_none());
    }
}
