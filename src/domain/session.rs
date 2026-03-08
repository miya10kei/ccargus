use std::path::PathBuf;

use color_eyre::Result;

use super::pty::PtySession;
use super::worktree::WorktreeEntry;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Running,
    Stopped,
}

#[allow(dead_code)]
pub struct SessionInfo {
    pub branch: String,
    pub pty: Option<PtySession>,
    pub qa_pty: Option<PtySession>,
    pub repo: String,
    pub source_repo_path: String,
    pub worktree_path: PathBuf,
}

impl SessionInfo {
    pub fn close_qa_session(&mut self) {
        if let Some(qa) = &mut self.qa_pty {
            qa.kill();
        }
        self.qa_pty = None;
    }

    pub fn create_qa_session(&mut self, fork: bool, rows: u16, cols: u16) -> Result<()> {
        let working_dir = self.working_dir();
        let qa_pty = if fork {
            PtySession::spawn_with_args("claude", &["--continue"], &working_dir, rows, cols)?
        } else {
            PtySession::spawn("claude", &working_dir, rows, cols)?
        };
        self.qa_pty = Some(qa_pty);
        Ok(())
    }

    pub fn from_worktree_entry(entry: &WorktreeEntry) -> Self {
        Self {
            branch: entry.branch.clone(),
            pty: None,
            qa_pty: None,
            repo: entry.repo_name.clone(),
            source_repo_path: entry.source_repo_path.clone(),
            worktree_path: entry.worktree_path.clone(),
        }
    }

    pub fn to_worktree_entry(&self) -> WorktreeEntry {
        WorktreeEntry {
            branch: self.branch.clone(),
            repo_name: self.repo.clone(),
            source_repo_path: self.source_repo_path.clone(),
            worktree_path: self.worktree_path.clone(),
        }
    }

    pub fn has_qa_session(&self) -> bool {
        self.qa_pty.is_some()
    }

    pub fn is_running(&self) -> bool {
        self.pty.is_some()
    }

    pub fn start(&mut self, rows: u16, cols: u16) -> Result<()> {
        if self.pty.is_some() {
            return Ok(());
        }
        let working_dir = self.working_dir();
        let pty = PtySession::spawn("claude", &working_dir, rows, cols)?;
        self.pty = Some(pty);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn state(&self) -> SessionState {
        if self.is_running() {
            SessionState::Running
        } else {
            SessionState::Stopped
        }
    }

    pub fn stop(&mut self) {
        self.close_qa_session();
        if let Some(pty) = &mut self.pty {
            pty.kill();
        }
        self.pty = None;
    }

    pub fn working_dir(&self) -> String {
        self.worktree_path.to_string_lossy().to_string()
    }
}

pub struct SessionManager {
    sessions: Vec<SessionInfo>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn add_session(&mut self, session: SessionInfo) {
        self.sessions.push(session);
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
            self.sessions[index].stop();
            self.sessions.remove(index);
        }
    }

    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    #[allow(dead_code)]
    pub fn sync_with_worktrees(&mut self, entries: &[WorktreeEntry]) {
        // Keep existing sessions that still have a worktree, add new ones
        let mut new_sessions: Vec<SessionInfo> = Vec::new();

        for entry in entries {
            if let Some(pos) = self
                .sessions
                .iter()
                .position(|s| s.worktree_path == entry.worktree_path)
            {
                // Move the existing session (preserves running PTY)
                new_sessions.push(self.sessions.remove(pos));
            } else {
                new_sessions.push(SessionInfo::from_worktree_entry(entry));
            }
        }

        // Kill remaining sessions whose worktrees no longer exist
        for session in &mut self.sessions {
            session.stop();
        }

        self.sessions = new_sessions;
    }

    #[cfg(test)]
    pub fn add_stopped_session(&mut self, repo: &str, branch: &str, worktree_path: &str) {
        self.sessions.push(SessionInfo {
            branch: branch.to_string(),
            pty: None,
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        });
    }

    #[cfg(test)]
    pub fn add_test_session(
        &mut self,
        repo: &str,
        branch: &str,
        worktree_path: &str,
    ) -> Result<()> {
        let pty = PtySession::spawn("cat", worktree_path, 24, 80)?;
        self.sessions.push(SessionInfo {
            branch: branch.to_string(),
            pty: Some(pty),
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session(manager: &mut SessionManager) {
        manager
            .add_test_session("test/repo", "main", "/tmp")
            .unwrap();
    }

    #[test]
    fn new_manager_is_empty() {
        let manager = SessionManager::new();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
    }

    #[test]
    fn add_test_session_increases_len() {
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
        assert_eq!(session.unwrap().repo, "test/repo");
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let manager = SessionManager::new();
        assert!(manager.get(99).is_none());
    }

    #[test]
    fn session_state_stopped_by_default() {
        let mut manager = SessionManager::new();
        manager.add_stopped_session("test/repo", "main", "/tmp");
        let session = manager.get(0).unwrap();
        assert_eq!(session.state(), SessionState::Stopped);
        assert!(!session.is_running());
    }

    #[test]
    fn session_state_running_with_pty() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        let session = manager.get(0).unwrap();
        assert_eq!(session.state(), SessionState::Running);
        assert!(session.is_running());
    }

    #[test]
    fn stop_kills_pty() {
        let mut manager = SessionManager::new();
        create_test_session(&mut manager);
        let session = manager.get_mut(0).unwrap();
        assert!(session.is_running());
        session.stop();
        assert!(!session.is_running());
    }

    #[test]
    fn sync_with_worktrees_adds_new_entries() {
        let mut manager = SessionManager::new();
        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/wt1"),
        }];
        manager.sync_with_worktrees(&entries);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.get(0).unwrap().branch, "main");
    }

    #[test]
    fn sync_with_worktrees_preserves_existing() {
        let mut manager = SessionManager::new();
        manager
            .add_test_session("test/repo", "main", "/tmp")
            .unwrap();

        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp"),
        }];
        manager.sync_with_worktrees(&entries);
        assert_eq!(manager.len(), 1);
        assert!(manager.get(0).unwrap().is_running()); // PTY preserved
    }

    #[test]
    fn sync_with_worktrees_removes_stale() {
        let mut manager = SessionManager::new();
        manager.add_stopped_session("test/repo", "old-branch", "/tmp/old");

        let entries = vec![WorktreeEntry {
            branch: "new-branch".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/new"),
        }];
        manager.sync_with_worktrees(&entries);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.get(0).unwrap().branch, "new-branch");
    }
}
