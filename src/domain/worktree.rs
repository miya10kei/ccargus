use std::path::PathBuf;

use color_eyre::Result;

use super::pty::PtySession;
use super::worktree_entry::WorktreeEntry;

#[allow(clippy::struct_field_names)]
pub struct Worktree {
    pub branch: String,
    pub pty: Option<PtySession>,
    pub qa_pty: Option<PtySession>,
    pub repo: String,
    pub source_repo_path: String,
    pub worktree_path: PathBuf,
}

impl Worktree {
    pub fn any_pty_dirty(&self) -> bool {
        let main_dirty = self.pty.as_ref().is_some_and(PtySession::is_dirty);
        let qa_dirty = self.qa_pty.as_ref().is_some_and(PtySession::is_dirty);
        main_dirty || qa_dirty
    }

    pub fn clear_pty_dirty(&self) {
        if let Some(pty) = &self.pty {
            pty.clear_dirty();
        }
        if let Some(qa) = &self.qa_pty {
            qa.clear_dirty();
        }
    }

    pub fn close_qa(&mut self) {
        if let Some(qa) = &mut self.qa_pty {
            qa.kill();
        }
        self.qa_pty = None;
    }

    pub fn create_qa(
        &mut self,
        fork: bool,
        rows: u16,
        cols: u16,
        plan: bool,
        claude_command: &str,
    ) -> Result<()> {
        let working_dir = self.working_dir();
        let mut args: Vec<&str> = Vec::new();
        if fork {
            args.push("--continue");
        }
        if plan {
            args.extend(["--permission-mode", "plan"]);
        }
        let qa_pty = if args.is_empty() {
            PtySession::spawn(claude_command, &working_dir, rows, cols)?
        } else {
            PtySession::spawn_with_args(claude_command, &args, &working_dir, rows, cols)?
        };
        self.qa_pty = Some(qa_pty);
        Ok(())
    }

    pub fn display_name(&self) -> &str {
        self.repo.rsplit('/').next().unwrap_or(&self.repo)
    }

    pub fn from_entry(entry: &WorktreeEntry) -> Self {
        Self {
            branch: entry.branch.clone(),
            pty: None,
            qa_pty: None,
            repo: entry.repo_name.clone(),
            source_repo_path: entry.source_repo_path.clone(),
            worktree_path: entry.worktree_path.clone(),
        }
    }

    pub fn has_qa(&self) -> bool {
        self.qa_pty.is_some()
    }

    pub fn is_running(&self) -> bool {
        self.pty.is_some()
    }

    pub fn resize_pty(&self, main_rows: u16, main_cols: u16, qa_rows: u16, qa_cols: u16) {
        if let Some(pty) = &self.pty {
            let _ = pty.resize(main_rows, main_cols);
        }
        if let Some(qa) = &self.qa_pty {
            let _ = qa.resize(qa_rows, qa_cols);
        }
    }

    pub fn start(
        &mut self,
        rows: u16,
        cols: u16,
        auto_continue: bool,
        plan: bool,
        claude_command: &str,
    ) -> Result<()> {
        if self.pty.is_some() {
            return Ok(());
        }
        let working_dir = self.working_dir();
        let mut args: Vec<&str> = Vec::new();
        if auto_continue {
            args.push("--continue");
        }
        if plan {
            args.extend(["--permission-mode", "plan"]);
        }
        let pty = if args.is_empty() {
            PtySession::spawn(claude_command, &working_dir, rows, cols)?
        } else {
            PtySession::spawn_with_args(claude_command, &args, &working_dir, rows, cols)?
        };
        self.pty = Some(pty);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.close_qa();
        if let Some(pty) = &mut self.pty {
            pty.kill();
        }
        self.pty = None;
    }

    pub fn to_entry(&self) -> WorktreeEntry {
        WorktreeEntry {
            branch: self.branch.clone(),
            repo_name: self.repo.clone(),
            source_repo_path: self.source_repo_path.clone(),
            worktree_path: self.worktree_path.clone(),
        }
    }

    pub fn working_dir(&self) -> String {
        self.worktree_path.to_string_lossy().to_string()
    }
}

pub struct WorktreePool {
    worktrees: Vec<Worktree>,
}

impl WorktreePool {
    pub fn new() -> Self {
        Self {
            worktrees: Vec::new(),
        }
    }

    pub fn add(&mut self, wt: Worktree) -> usize {
        let name = wt.display_name().to_string();
        if let Some(last_pos) = self
            .worktrees
            .iter()
            .rposition(|w| w.display_name() == name)
        {
            let insert_pos = last_pos + 1;
            self.worktrees.insert(insert_pos, wt);
            insert_pos
        } else {
            self.worktrees.push(wt);
            self.worktrees.len() - 1
        }
    }

    pub fn all(&self) -> &[Worktree] {
        &self.worktrees
    }

    pub fn all_mut(&mut self) -> &mut [Worktree] {
        &mut self.worktrees
    }

    pub fn get(&self, index: usize) -> Option<&Worktree> {
        self.worktrees.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Worktree> {
        self.worktrees.get_mut(index)
    }

    pub fn is_empty(&self) -> bool {
        self.worktrees.is_empty()
    }

    pub fn len(&self) -> usize {
        self.worktrees.len()
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.worktrees.len() {
            self.worktrees[index].stop();
            self.worktrees.remove(index);
        }
    }

    pub fn sync_with_worktrees(&mut self, entries: &[WorktreeEntry]) {
        // Keep existing worktrees that still have an entry, add new ones
        let mut new_worktrees: Vec<Worktree> = Vec::new();

        for entry in entries {
            if let Some(pos) = self
                .worktrees
                .iter()
                .position(|wt| wt.worktree_path == entry.worktree_path)
            {
                // Move the existing worktree (preserves running PTY)
                new_worktrees.push(self.worktrees.remove(pos));
            } else {
                new_worktrees.push(Worktree::from_entry(entry));
            }
        }

        // Kill remaining worktrees whose entries no longer exist
        for wt in &mut self.worktrees {
            wt.stop();
        }

        self.worktrees = new_worktrees;
    }

    #[cfg(test)]
    pub fn add_stopped(&mut self, repo: &str, branch: &str, worktree_path: &str) -> usize {
        self.add(Worktree {
            branch: branch.to_string(),
            pty: None,
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        })
    }

    #[cfg(test)]
    pub fn add_test(&mut self, repo: &str, branch: &str, worktree_path: &str) -> Result<usize> {
        let pty = PtySession::spawn("cat", worktree_path, 24, 80)?;
        Ok(self.add(Worktree {
            branch: branch.to_string(),
            pty: Some(pty),
            qa_pty: None,
            repo: repo.to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from(worktree_path),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_worktree(pool: &mut WorktreePool) {
        pool.add_test("test/repo", "main", "/tmp").unwrap();
    }

    #[test]
    fn new_pool_is_empty() {
        let pool = WorktreePool::new();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
    }

    #[test]
    fn add_test_increases_len() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        assert_eq!(pool.len(), 1);
        assert!(!pool.is_empty());
    }

    #[test]
    fn create_multiple_worktrees() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        assert_eq!(pool.len(), 3);
    }

    #[test]
    fn remove_decreases_len() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        create_test_worktree(&mut pool);
        pool.remove(0);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn remove_out_of_bounds_is_safe() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        pool.remove(99);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn get_returns_worktree() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get(0);
        assert!(wt.is_some());
        assert_eq!(wt.unwrap().repo, "test/repo");
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let pool = WorktreePool::new();
        assert!(pool.get(99).is_none());
    }

    #[test]
    fn display_name_returns_last_component() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("github.com/owner/myrepo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.display_name(), "myrepo");
    }

    #[test]
    fn display_name_without_slash() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("myrepo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.display_name(), "myrepo");
    }

    #[test]
    fn stopped_by_default() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert!(!wt.is_running());
    }

    #[test]
    fn running_with_pty() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get(0).unwrap();
        assert!(wt.is_running());
    }

    #[test]
    fn stop_kills_pty() {
        let mut pool = WorktreePool::new();
        create_test_worktree(&mut pool);
        let wt = pool.get_mut(0).unwrap();
        assert!(wt.is_running());
        wt.stop();
        assert!(!wt.is_running());
    }

    #[test]
    fn add_groups_by_repo() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("github.com/owner/repo-a", "b1", "/tmp/a1");
        pool.add_stopped("github.com/owner/repo-b", "b2", "/tmp/b1");
        pool.add_stopped("github.com/owner/repo-a", "b3", "/tmp/a2");

        assert_eq!(pool.get(0).unwrap().branch, "b1");
        assert_eq!(pool.get(1).unwrap().branch, "b3");
        assert_eq!(pool.get(2).unwrap().branch, "b2");
    }

    #[test]
    fn add_returns_correct_index() {
        let mut pool = WorktreePool::new();
        let idx0 = pool.add_stopped("github.com/owner/repo-a", "b1", "/tmp/a1");
        let idx1 = pool.add_stopped("github.com/owner/repo-b", "b2", "/tmp/b1");
        let idx2 = pool.add_stopped("github.com/owner/repo-a", "b3", "/tmp/a2");

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 1);
    }

    #[test]
    fn sync_with_worktrees_adds_new_entries() {
        let mut pool = WorktreePool::new();
        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/wt1"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(0).unwrap().branch, "main");
    }

    #[test]
    fn sync_with_worktrees_preserves_existing() {
        let mut pool = WorktreePool::new();
        pool.add_test("test/repo", "main", "/tmp").unwrap();

        let entries = vec![WorktreeEntry {
            branch: "main".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert!(pool.get(0).unwrap().is_running()); // PTY preserved
    }

    #[test]
    fn sync_with_worktrees_removes_stale() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "old-branch", "/tmp/old");

        let entries = vec![WorktreeEntry {
            branch: "new-branch".to_string(),
            repo_name: "test/repo".to_string(),
            source_repo_path: String::new(),
            worktree_path: PathBuf::from("/tmp/new"),
        }];
        pool.sync_with_worktrees(&entries);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(0).unwrap().branch, "new-branch");
    }

    #[test]
    fn any_pty_dirty_false_when_no_ptys() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert!(!wt.any_pty_dirty());
    }

    #[test]
    fn close_qa_when_no_qa_is_noop() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp");
        let wt = pool.get_mut(0).unwrap();
        wt.close_qa(); // should not panic
        assert!(!wt.has_qa());
    }

    #[test]
    fn has_qa_false_by_default() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp");
        let wt = pool.get(0).unwrap();
        assert!(!wt.has_qa());
    }

    #[test]
    fn to_entry_and_from_entry_roundtrip() {
        let entry = WorktreeEntry {
            branch: "feat".to_string(),
            repo_name: "github.com/owner/repo".to_string(),
            source_repo_path: "/src".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
        };
        let wt = Worktree::from_entry(&entry);
        let roundtrip = wt.to_entry();
        assert_eq!(roundtrip.branch, entry.branch);
        assert_eq!(roundtrip.repo_name, entry.repo_name);
        assert_eq!(roundtrip.source_repo_path, entry.source_repo_path);
        assert_eq!(roundtrip.worktree_path, entry.worktree_path);
    }

    #[test]
    fn working_dir_returns_path_string() {
        let mut pool = WorktreePool::new();
        pool.add_stopped("test/repo", "main", "/tmp/worktree");
        let wt = pool.get(0).unwrap();
        assert_eq!(wt.working_dir(), "/tmp/worktree");
    }
}
