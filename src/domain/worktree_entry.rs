use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    pub branch: String,
    pub repo_name: String,
    pub source_repo_path: String,
    pub worktree_path: PathBuf,
}
