use std::path::PathBuf;

use color_eyre::Result;
use serde::Deserialize;

fn default_delete_session() -> char {
    'd'
}
fn default_editor_command() -> String {
    "vim".to_owned()
}
fn default_new_session() -> char {
    'n'
}
fn default_open_editor() -> char {
    'e'
}
fn default_qa_session() -> char {
    's'
}
fn default_worktree_base_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("ccargus")
        .join("worktrees")
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub worktree: WorktreeConfig,
}

#[derive(Debug, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_editor_command")]
    pub command: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_delete_session")]
    pub delete_session: char,
    #[serde(default = "default_new_session")]
    pub new_session: char,
    #[serde(default = "default_open_editor")]
    pub open_editor: char,
    #[serde(default = "default_qa_session")]
    pub qa_session: char,
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
            delete_session: default_delete_session(),
            new_session: default_new_session(),
            open_editor: default_open_editor(),
            qa_session: default_qa_session(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default = "default_worktree_base_dir")]
    pub base_dir: PathBuf,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            base_dir: default_worktree_base_dir(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("ccargus")
            .join("config.toml")
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
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.keybindings.new_session, 'n');
        assert_eq!(config.keybindings.delete_session, 'd');
        assert_eq!(config.keybindings.open_editor, 'e');
        assert_eq!(config.keybindings.qa_session, 's');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn full_toml_deserialization() {
        let toml = r#"
[editor]
command = "nvim"

[keybindings]
new_session = "a"
delete_session = "x"
open_editor = "o"
qa_session = "q"

[worktree]
base_dir = "/custom/worktrees"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.editor.command, "nvim");
        assert_eq!(config.keybindings.new_session, 'a');
        assert_eq!(config.keybindings.delete_session, 'x');
        assert_eq!(config.keybindings.open_editor, 'o');
        assert_eq!(config.keybindings.qa_session, 'q');
        assert_eq!(config.worktree.base_dir, PathBuf::from("/custom/worktrees"));
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let toml = r#"
[editor]
command = "emacs"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.editor.command, "emacs");
        assert_eq!(config.keybindings.new_session, 'n');
        assert_eq!(config.keybindings.delete_session, 'd');
        assert_eq!(config.keybindings.open_editor, 'e');
        assert_eq!(config.keybindings.qa_session, 's');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn empty_toml_returns_defaults() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.keybindings.new_session, 'n');
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = Config::from_toml("invalid = [[[");
        assert!(result.is_err());
    }
}
