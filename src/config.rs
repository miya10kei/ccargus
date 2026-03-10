use std::fmt;
use std::path::PathBuf;

use color_eyre::Result;
use crossterm::event::{KeyEvent, KeyModifiers};
use serde::Deserialize;
use serde::de::{self, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: char,
}

impl Keybinding {
    fn plain(key: char) -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            key,
        }
    }

    fn ctrl(key: char) -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            key,
        }
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        let crossterm::event::KeyCode::Char(c) = event.code else {
            return false;
        };
        c == self.key
            && self.ctrl == event.modifiers.contains(KeyModifiers::CONTROL)
            && self.alt == event.modifiers.contains(KeyModifiers::ALT)
            && self.shift == event.modifiers.contains(KeyModifiers::SHIFT)
    }

    fn parse(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() {
            return Err("keybinding must not be empty".to_owned());
        }

        if s.starts_with('<') && s.ends_with('>') {
            let inner = &s[1..s.len() - 1];
            if inner.is_empty() {
                return Err("empty angle bracket notation".to_owned());
            }
            let parts: Vec<&str> = inner.split('-').collect();
            if parts.len() < 2 {
                return Err(format!("invalid keybinding notation: {s}"));
            }
            let key_part = parts.last().unwrap();
            if key_part.len() != 1 {
                return Err(format!("key must be a single character, got: {key_part}"));
            }
            let key = key_part.chars().next().unwrap();
            let mut ctrl = false;
            let mut alt = false;
            let mut shift = false;
            for &modifier in &parts[..parts.len() - 1] {
                match modifier {
                    "C" => ctrl = true,
                    "A" => alt = true,
                    "S" => shift = true,
                    _ => return Err(format!("unknown modifier: {modifier}")),
                }
            }
            Ok(Keybinding {
                ctrl,
                alt,
                shift,
                key,
            })
        } else if s.len() == 1 {
            Ok(Keybinding::plain(s.chars().next().unwrap()))
        } else {
            Err(format!("invalid keybinding: {s}"))
        }
    }
}

impl fmt::Display for Keybinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            write!(f, "Ctrl+")?;
        }
        if self.alt {
            write!(f, "Alt+")?;
        }
        if self.shift {
            write!(f, "Shift+")?;
        }
        write!(f, "{}", self.key)
    }
}

impl<'de> Deserialize<'de> for Keybinding {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Keybinding::parse(&s).map_err(de::Error::custom)
    }
}

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

fn default_true() -> bool {
    true
}
fn default_claude_command() -> String {
    "claude".to_owned()
}
fn default_delete_worktree() -> Keybinding {
    Keybinding::plain('d')
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
fn default_new_worktree() -> Keybinding {
    Keybinding::plain('n')
}
fn default_open_editor() -> Keybinding {
    Keybinding::plain('e')
}
fn default_open_shell() -> Keybinding {
    Keybinding::plain('t')
}
fn default_protected_branches() -> Vec<String> {
    vec!["main".to_owned(), "master".to_owned(), "develop".to_owned()]
}
fn default_qa_split_percent() -> u16 {
    50
}
fn default_qa_worktree() -> Keybinding {
    Keybinding::plain('s')
}
fn default_terminal_open_editor() -> Keybinding {
    Keybinding::ctrl('e')
}
fn default_terminal_open_shell() -> Keybinding {
    Keybinding::ctrl('t')
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
    #[serde(default = "default_true")]
    pub auto_continue: bool,
    #[serde(default = "default_claude_command")]
    pub command: String,
    #[serde(default)]
    pub plan: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            auto_continue: true,
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
    pub popup: PopupConfig,
    #[serde(default)]
    pub worktree: WorktreeConfig,
}

#[derive(Debug, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_editor_command")]
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct PopupConfig {
    #[serde(default = "default_popup_options")]
    pub options: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_delete_worktree")]
    pub delete_worktree: Keybinding,
    #[serde(default = "default_new_worktree")]
    pub new_worktree: Keybinding,
    #[serde(default = "default_open_editor")]
    pub open_editor: Keybinding,
    #[serde(default = "default_open_shell")]
    pub open_shell: Keybinding,
    #[serde(default = "default_qa_worktree")]
    pub qa_worktree: Keybinding,
    #[serde(default = "default_terminal_open_editor")]
    pub terminal_open_editor: Keybinding,
    #[serde(default = "default_terminal_open_shell")]
    pub terminal_open_shell: Keybinding,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            command: default_editor_command(),
        }
    }
}

impl Default for PopupConfig {
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
            open_shell: default_open_shell(),
            qa_worktree: default_qa_worktree(),
            terminal_open_editor: default_terminal_open_editor(),
            terminal_open_shell: default_terminal_open_shell(),
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
        let bindings: &[(&str, &Keybinding)] = &[
            ("delete_worktree", &kb.delete_worktree),
            ("new_worktree", &kb.new_worktree),
            ("open_editor", &kb.open_editor),
            ("open_shell", &kb.open_shell),
            ("qa_worktree", &kb.qa_worktree),
            ("terminal_open_editor", &kb.terminal_open_editor),
            ("terminal_open_shell", &kb.terminal_open_shell),
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

        let reserved_terminal = [
            Keybinding::ctrl('w'),
            Keybinding::ctrl('d'),
            Keybinding::ctrl('b'),
        ];
        for (name, binding) in [
            ("terminal_open_editor", &kb.terminal_open_editor),
            ("terminal_open_shell", &kb.terminal_open_shell),
        ] {
            if reserved_terminal.contains(binding) {
                return Err(color_eyre::eyre::eyre!(
                    "{name} '{}' conflicts with reserved terminal shortcut",
                    binding,
                ));
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
    use crossterm::event::KeyCode;

    use super::*;

    fn kb(key: char) -> Keybinding {
        Keybinding::plain(key)
    }

    fn kb_ctrl(key: char) -> Keybinding {
        Keybinding::ctrl(key)
    }

    #[test]
    fn keybinding_parse_plain_char() {
        assert_eq!(Keybinding::parse("e").unwrap(), kb('e'));
    }

    #[test]
    fn keybinding_parse_ctrl() {
        assert_eq!(Keybinding::parse("<C-e>").unwrap(), kb_ctrl('e'));
    }

    #[test]
    fn keybinding_parse_alt() {
        let expected = Keybinding {
            ctrl: false,
            alt: true,
            shift: false,
            key: 'e',
        };
        assert_eq!(Keybinding::parse("<A-e>").unwrap(), expected);
    }

    #[test]
    fn keybinding_parse_shift() {
        let expected = Keybinding {
            ctrl: false,
            alt: false,
            shift: true,
            key: 'e',
        };
        assert_eq!(Keybinding::parse("<S-e>").unwrap(), expected);
    }

    #[test]
    fn keybinding_parse_ctrl_alt() {
        let expected = Keybinding {
            ctrl: true,
            alt: true,
            shift: false,
            key: 'e',
        };
        assert_eq!(Keybinding::parse("<C-A-e>").unwrap(), expected);
    }

    #[test]
    fn keybinding_parse_ctrl_shift_alt() {
        let expected = Keybinding {
            ctrl: true,
            alt: true,
            shift: true,
            key: 'x',
        };
        assert_eq!(Keybinding::parse("<C-S-A-x>").unwrap(), expected);
    }

    #[test]
    fn keybinding_parse_invalid_modifier() {
        assert!(Keybinding::parse("<X-e>").is_err());
    }

    #[test]
    fn keybinding_parse_empty_angle_brackets() {
        assert!(Keybinding::parse("<>").is_err());
    }

    #[test]
    fn keybinding_parse_empty_key_after_modifier() {
        assert!(Keybinding::parse("<C->").is_err());
    }

    #[test]
    fn keybinding_parse_empty_string() {
        assert!(Keybinding::parse("").is_err());
    }

    #[test]
    fn keybinding_matches_plain_key() {
        let binding = kb('e');
        let event = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        assert!(binding.matches(&event));

        let event_wrong = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!binding.matches(&event_wrong));
    }

    #[test]
    fn keybinding_matches_ctrl_key() {
        let binding = kb_ctrl('e');
        let event = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert!(binding.matches(&event));

        let event_no_ctrl = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        assert!(!binding.matches(&event_no_ctrl));
    }

    #[test]
    fn keybinding_plain_does_not_match_ctrl() {
        let binding = kb('e');
        let event = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert!(!binding.matches(&event));
    }

    #[test]
    fn keybinding_display_plain() {
        assert_eq!(kb('e').to_string(), "e");
    }

    #[test]
    fn keybinding_display_ctrl() {
        assert_eq!(kb_ctrl('e').to_string(), "Ctrl+e");
    }

    #[test]
    fn keybinding_display_ctrl_alt() {
        let binding = Keybinding {
            ctrl: true,
            alt: true,
            shift: false,
            key: 'e',
        };
        assert_eq!(binding.to_string(), "Ctrl+Alt+e");
    }

    #[test]
    fn default_values_are_correct() {
        let config = Config::default();
        assert!(config.claude.auto_continue);
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.popup.options, vec!["-E", "-w", "80%", "-h", "80%"]);
        assert_eq!(config.keybindings.new_worktree, kb('n'));
        assert_eq!(config.keybindings.delete_worktree, kb('d'));
        assert_eq!(config.keybindings.open_editor, kb('e'));
        assert_eq!(config.keybindings.open_shell, kb('t'));
        assert_eq!(config.keybindings.qa_worktree, kb('s'));
        assert_eq!(config.keybindings.terminal_open_editor, kb_ctrl('e'));
        assert_eq!(config.keybindings.terminal_open_shell, kb_ctrl('t'));
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn full_toml_deserialization() {
        let toml = r#"
[claude]
auto_continue = false
plan = true

[editor]
command = "nvim"

[popup]
options = ["-E", "-w", "90%", "-h", "90%"]

[keybindings]
new_worktree = "a"
delete_worktree = "x"
open_editor = "o"
open_shell = "r"
qa_worktree = "q"
terminal_open_editor = "<C-o>"
terminal_open_shell = "<C-r>"

[worktree]
base_dir = "/custom/worktrees"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(!config.claude.auto_continue);
        assert!(config.claude.plan);
        assert_eq!(config.editor.command, "nvim");
        assert_eq!(config.popup.options, vec!["-E", "-w", "90%", "-h", "90%"]);
        assert_eq!(config.keybindings.new_worktree, kb('a'));
        assert_eq!(config.keybindings.delete_worktree, kb('x'));
        assert_eq!(config.keybindings.open_editor, kb('o'));
        assert_eq!(config.keybindings.open_shell, kb('r'));
        assert_eq!(config.keybindings.qa_worktree, kb('q'));
        assert_eq!(config.keybindings.terminal_open_editor, kb_ctrl('o'));
        assert_eq!(config.keybindings.terminal_open_shell, kb_ctrl('r'));
        assert_eq!(config.worktree.base_dir, PathBuf::from("/custom/worktrees"));
    }

    #[test]
    fn full_toml_with_vim_notation() {
        let toml = r#"
[keybindings]
open_editor = "<C-o>"
terminal_open_editor = "<C-A-e>"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.keybindings.open_editor, kb_ctrl('o'));
        assert_eq!(
            config.keybindings.terminal_open_editor,
            Keybinding {
                ctrl: true,
                alt: true,
                shift: false,
                key: 'e',
            }
        );
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let toml = r#"
[editor]
command = "emacs"
"#;
        let config = Config::from_toml(toml).unwrap();
        assert!(config.claude.auto_continue);
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "emacs");
        assert_eq!(config.popup.options, vec!["-E", "-w", "80%", "-h", "80%"]);
        assert_eq!(config.keybindings.new_worktree, kb('n'));
        assert_eq!(config.keybindings.delete_worktree, kb('d'));
        assert_eq!(config.keybindings.open_editor, kb('e'));
        assert_eq!(config.keybindings.open_shell, kb('t'));
        assert_eq!(config.keybindings.qa_worktree, kb('s'));
        assert_eq!(config.keybindings.terminal_open_editor, kb_ctrl('e'));
        assert_eq!(config.keybindings.terminal_open_shell, kb_ctrl('t'));
        assert!(config.worktree.base_dir.ends_with("ccargus/worktrees"));
    }

    #[test]
    fn empty_toml_returns_defaults() {
        let config = Config::from_toml("").unwrap();
        assert!(config.claude.auto_continue);
        assert!(!config.claude.plan);
        assert_eq!(config.editor.command, "vim");
        assert_eq!(config.keybindings.new_worktree, kb('n'));
        assert_eq!(config.keybindings.open_shell, kb('t'));
        assert_eq!(config.keybindings.terminal_open_editor, kb_ctrl('e'));
        assert_eq!(config.keybindings.terminal_open_shell, kb_ctrl('t'));
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
        config.keybindings.new_worktree = kb('d');
        config.keybindings.delete_worktree = kb('d');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("Duplicate keybinding"));
    }

    #[test]
    fn validate_rejects_terminal_open_editor_ctrl_w() {
        let mut config = Config::default();
        config.keybindings.terminal_open_editor = kb_ctrl('w');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("conflicts with reserved terminal shortcut"));
    }

    #[test]
    fn validate_rejects_terminal_open_editor_ctrl_d() {
        let mut config = Config::default();
        config.keybindings.terminal_open_editor = kb_ctrl('d');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("conflicts with reserved terminal shortcut"));
    }

    #[test]
    fn validate_rejects_terminal_open_editor_ctrl_b() {
        let mut config = Config::default();
        config.keybindings.terminal_open_editor = kb_ctrl('b');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("conflicts with reserved terminal shortcut"));
    }

    #[test]
    fn validate_rejects_terminal_open_shell_ctrl_w() {
        let mut config = Config::default();
        config.keybindings.terminal_open_shell = kb_ctrl('w');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("terminal_open_shell"));
        assert!(err.contains("conflicts with reserved terminal shortcut"));
    }

    #[test]
    fn validate_rejects_terminal_open_shell_ctrl_d() {
        let mut config = Config::default();
        config.keybindings.terminal_open_shell = kb_ctrl('d');
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("terminal_open_shell"));
        assert!(err.contains("conflicts with reserved terminal shortcut"));
    }
}
