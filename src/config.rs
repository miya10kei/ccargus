use std::path::PathBuf;

use color_eyre::Result;
use serde::Deserialize;

fn default_claude_command() -> String {
    "claude".to_owned()
}
fn default_delete_worktree() -> char {
    'd'
}
fn default_editor_command() -> String {
    "vim".to_owned()
}
fn default_new_worktree() -> char {
    'n'
}
fn default_open_editor() -> char {
    'e'
}
fn default_protected_branches() -> Vec<String> {
    vec!["main".to_owned(), "master".to_owned(), "develop".to_owned()]
}
fn default_qa_split_percent() -> u16 {
    50
}
fn default_qa_worktree() -> char {
    's'
}
fn default_worktree_pane_percent() -> u16 {
    25
}
fn default_worktree_base_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".local/share")
        })
        .join("ccargus")
        .join("worktrees")
}

#[derive(Debug, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default = "default_claude_command")]
    pub command: String,
    #[serde(default)]
    pub plan: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            command: default_claude_command(),
            plan: false,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub worktree: WorktreeConfig,
}

#[derive(Debug, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_editor_command")]
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_delete_worktree")]
    pub delete_worktree: char,
    #[serde(default = "default_new_worktree")]
    pub new_worktree: char,
    #[serde(default = "default_open_editor")]
    pub open_editor: char,
    #[serde(default = "default_qa_worktree")]
    pub qa_worktree: char,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            command: default_editor_command(),
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            delete_worktree: default_delete_worktree(),
            new_worktree: default_new_worktree(),
            open_editor: default_open_editor(),
            qa_worktree: default_qa_worktree(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LayoutConfig {
    #[serde(default = "default_qa_split_percent")]
    pub qa_split_percent: u16,
    #[serde(default = "default_worktree_pane_percent")]
    pub worktree_pane_percent: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            qa_split_percent: default_qa_split_percent(),
            worktree_pane_percent: default_worktree_pane_percent(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default = "default_worktree_base_dir")]
    pub base_dir: PathBuf,
    #[serde(default = "default_protected_branches")]
    pub protected_branches: Vec<String>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            base_dir: default_worktree_base_dir(),
            protected_branches: default_protected_branches(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("CCARGUS_CONFIG") {
            return PathBuf::from(path);
        }
        dirs::config_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/"))
                    .join(".config")
            })
            .join("ccargus")
            .join("config.toml")
    }

    pub fn validate(&self) -> Result<()> {
        if self.editor.command.is_empty() {
            return Err(color_eyre::eyre::eyre!("editor.command must not be empty"));
        }
        if self.worktree.base_dir.as_os_str().is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "worktree.base_dir must not be empty"
            ));
        }
        Ok(())
    }

    pub fn from_toml(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content)?;
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Self::from_toml(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_correct() {
        let config = Config::default();
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.keybindings.new_worktree, 'n');
        assert_eq!(config.keybindings.delete_worktree, 'd');
        assert_eq!(config.keybindings.open_editor, 'e');
        assert_eq!(config.keybindings.qa_worktree, 's');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn full_toml_deserialization() {
        let toml = r#"
[claude]
plan = true

[editor]
command = "nvim"

[keybindings]
new_worktree = "a"
delete_worktree = "x"
open_editor = "o"
qa_worktree = "q"

[worktree]
base_dir = "/custom/worktrees"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.claude.plan);
        assert_eq!(config.editor.command, "nvim");
        assert_eq!(config.keybindings.new_worktree, 'a');
        assert_eq!(config.keybindings.delete_worktree, 'x');
        assert_eq!(config.keybindings.open_editor, 'o');
        assert_eq!(config.keybindings.qa_worktree, 'q');
        assert_eq!(config.worktree.base_dir, PathBuf::from("/custom/worktrees"));
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let toml = r#"
[editor]
command = "emacs"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "emacs");
        assert_eq!(config.keybindings.new_worktree, 'n');
        assert_eq!(config.keybindings.delete_worktree, 'd');
        assert_eq!(config.keybindings.open_editor, 'e');
        assert_eq!(config.keybindings.qa_worktree, 's');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn empty_toml_returns_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.keybindings.new_worktree, 'n');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = Config::from_toml("invalid = [[[");
        assert!(result.is_err());
    }
}
