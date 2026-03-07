use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                vec![(c as u8).wrapping_sub(b'a').wrapping_add(1)]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Backspace => vec![127],
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Esc => vec![27],
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Up => b"\x1b[A".to_vec(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn char_a_returns_0x61() {
        let result = key_to_bytes(make_key(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(result, vec![0x61]);
    }

    #[test]
    fn ctrl_c_returns_0x03() {
        let result = key_to_bytes(make_key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(result, vec![0x03]);
    }

    #[test]
    fn enter_returns_0x0d() {
        let result = key_to_bytes(make_key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(result, vec![0x0d]);
    }

    #[test]
    fn backspace_returns_0x7f() {
        let result = key_to_bytes(make_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(result, vec![0x7f]);
    }

    #[test]
    fn esc_returns_0x1b() {
        let result = key_to_bytes(make_key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b]);
    }

    #[test]
    fn up_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b, 0x5b, 0x41]);
    }

    #[test]
    fn tab_returns_0x09() {
        let result = key_to_bytes(make_key(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(result, vec![0x09]);
    }

    #[test]
    fn down_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b, 0x5b, 0x42]);
    }

    #[test]
    fn right_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b, 0x5b, 0x43]);
    }

    #[test]
    fn left_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b, 0x5b, 0x44]);
    }

    #[test]
    fn unknown_key_returns_empty() {
        let result = key_to_bytes(make_key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(result.is_empty());
    }
}
