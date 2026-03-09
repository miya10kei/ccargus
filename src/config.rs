use std::path::PathBuf;

use color_eyre::Result;
use serde::Deserialize;

fn expand_tilde(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        dirs::home_dir().unwrap_or(path)
    } else if let Some(rest) = s.strip_prefix("~/") {
        dirs::home_dir().map(|home| home.join(rest)).unwrap_or(path)
    } else {
        path
    }
}

fn default_claude_command() -> String {
    "claude".to_owned()
}
fn default_delete_worktree() -> char {
    'd'
}
fn default_editor_command() -> String {
    "vim".to_owned()
}
fn default_popup_options() -> Vec<String> {
    vec![
        "-E".to_owned(),
        "-w".to_owned(),
        "80%".to_owned(),
        "-h".to_owned(),
        "80%".to_owned(),
    ]
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
    20
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
    #[serde(default)]
    pub popup: EditorPopupConfig,
}

#[derive(Debug, Deserialize)]
pub struct EditorPopupConfig {
    #[serde(default = "default_popup_options")]
    pub options: Vec<String>,
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
            popup: EditorPopupConfig::default(),
        }
    }
}

impl Default for EditorPopupConfig {
    fn default() -> Self {
        Self {
            options: default_popup_options(),
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
        if self.claude.command.is_empty() {
            return Err(color_eyre::eyre::eyre!("claude.command must not be empty"));
        }
        if self.editor.command.is_empty() {
            return Err(color_eyre::eyre::eyre!("editor.command must not be empty"));
        }
        if !(1..=99).contains(&self.layout.worktree_pane_percent) {
            return Err(color_eyre::eyre::eyre!(
                "layout.worktree_pane_percent must be 1-99, got {}",
                self.layout.worktree_pane_percent
            ));
        }
        if !(1..=99).contains(&self.layout.qa_split_percent) {
            return Err(color_eyre::eyre::eyre!(
                "layout.qa_split_percent must be 1-99, got {}",
                self.layout.qa_split_percent
            ));
        }
        if self.worktree.base_dir.as_os_str().is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "worktree.base_dir must not be empty"
            ));
        }
        self.validate_keybindings()
    }

    fn validate_keybindings(&self) -> Result<()> {
        let kb = &self.keybindings;
        let bindings = [
            ("delete_worktree", kb.delete_worktree),
            ("new_worktree", kb.new_worktree),
            ("open_editor", kb.open_editor),
            ("qa_worktree", kb.qa_worktree),
        ];
        for i in 0..bindings.len() {
            for j in (i + 1)..bindings.len() {
                if bindings[i].1 == bindings[j].1 {
                    return Err(color_eyre::eyre::eyre!(
                        "Duplicate keybinding '{}': {} and {} use the same key",
                        bindings[i].1,
                        bindings[i].0,
                        bindings[j].0,
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn from_toml(content: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(content)?;
        config.worktree.base_dir = expand_tilde(config.worktree.base_dir);
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
        assert_eq!(
            config.editor.popup.options,
            vec!["-E", "-w", "80%", "-h", "80%"]
        );
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

[editor.popup]
options = ["-E", "-w", "90%", "-h", "90%"]

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
        assert_eq!(
            config.editor.popup.options,
            vec!["-E", "-w", "90%", "-h", "90%"]
        );
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
        assert_eq!(
            config.editor.popup.options,
            vec!["-E", "-w", "80%", "-h", "80%"]
        );
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
    fn expand_tilde_expands_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde(PathBuf::from("~")), home);
        assert_eq!(
            expand_tilde(PathBuf::from("~/some/path")),
            home.join("some/path")
        );
    }

    #[test]
    fn expand_tilde_preserves_absolute_path() {
        assert_eq!(
            expand_tilde(PathBuf::from("/absolute/path")),
            PathBuf::from("/absolute/path")
        );
    }

    #[test]
    fn from_toml_expands_tilde_in_base_dir() {
        let toml = r#"
[worktree]
base_dir = "~/my/worktrees"
"#;
        let config = Config::from_toml(toml).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(config.worktree.base_dir, home.join("my/worktrees"));
    }

    #[test]
    fn invalid_toml_returns_error() {
        let result = Config::from_toml("invalid = [[[");
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_claude_command() {
        let mut config = Config::default();
        config.claude.command = String::new();
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("claude.command must not be empty"));
    }

    #[test]
    fn validate_rejects_zero_worktree_pane_percent() {
        let mut config = Config::default();
        config.layout.worktree_pane_percent = 0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("worktree_pane_percent must be 1-99"));
    }

    #[test]
    fn validate_rejects_100_worktree_pane_percent() {
        let mut config = Config::default();
        config.layout.worktree_pane_percent = 100;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("worktree_pane_percent must be 1-99"));
    }

    #[test]
    fn validate_rejects_zero_qa_split_percent() {
        let mut config = Config::default();
        config.layout.qa_split_percent = 0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("qa_split_percent must be 1-99"));
    }

    #[test]
    fn validate_rejects_duplicate_keybindings() {
        let mut config = Config::default();
        config.keybindings.new_worktree = 'd';
        config.keybindings.delete_worktree = 'd';
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("Duplicate keybinding"));
    }
}
