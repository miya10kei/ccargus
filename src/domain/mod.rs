pub mod claude_status;
pub mod git;
pub mod pty;
pub mod repo;
pub mod worktree;
pub mod worktree_entry;
pub mod worktree_manager;

pub use git::{branch_exists, list_local_branches};
pub use worktree_manager::WorktreeManager;
