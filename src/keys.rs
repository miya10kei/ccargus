use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let mods = key.modifiers;
    let has_alt = mods.contains(KeyModifiers::ALT);
    let has_ctrl = mods.contains(KeyModifiers::CONTROL);
    let has_shift = mods.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Char(c) => {
            let mut bytes = if has_ctrl {
                vec![c as u8 & 0x1f]
            } else if has_shift {
                let mut buf = [0u8; 4];
                let upper = c.to_ascii_uppercase();
                let s = upper.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            };
            if has_alt {
                bytes.insert(0, 0x1b);
            }
            bytes
        }
        KeyCode::Backspace => {
            let mut bytes = vec![0x7f];
            if has_alt {
                bytes.insert(0, 0x1b);
            }
            bytes
        }
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Delete => tilde_seq(3, mods),
        KeyCode::Down => csi_letter_seq(b'B', mods),
        KeyCode::End => csi_letter_seq(b'F', mods),
        KeyCode::Enter => {
            let mut bytes = vec![b'\r'];
            if has_alt {
                bytes.insert(0, 0x1b);
            }
            bytes
        }
        KeyCode::Esc => vec![0x1b],
        KeyCode::F(n) => f_key_seq(n, mods),
        KeyCode::Home => csi_letter_seq(b'H', mods),
        KeyCode::Insert => tilde_seq(2, mods),
        KeyCode::Left => csi_letter_seq(b'D', mods),
        KeyCode::PageDown => tilde_seq(6, mods),
        KeyCode::PageUp => tilde_seq(5, mods),
        KeyCode::Right => csi_letter_seq(b'C', mods),
        KeyCode::Tab => {
            if has_shift {
                b"\x1b[Z".to_vec()
            } else {
                vec![b'\t']
            }
        }
        KeyCode::Up => csi_letter_seq(b'A', mods),
        _ => vec![],
    }
}

/// xterm modifier parameter: 1 + (Shift=1) + (Alt=2) + (Ctrl=4)
fn modifier_param(mods: KeyModifiers) -> Option<u8> {
    let code = 1
        + u8::from(mods.contains(KeyModifiers::SHIFT))
        + (u8::from(mods.contains(KeyModifiers::ALT)) << 1)
        + (u8::from(mods.contains(KeyModifiers::CONTROL)) << 2);
    if code > 1 { Some(code) } else { None }
}

/// `\x1b[A` or `\x1b[1;{mod}A`
fn csi_letter_seq(suffix: u8, mods: KeyModifiers) -> Vec<u8> {
    match modifier_param(mods) {
        Some(m) => format!("\x1b[1;{m}{}", suffix as char).into_bytes(),
        None => vec![0x1b, b'[', suffix],
    }
}

/// F1-F4: `\x1bOP`..`\x1bOS` or `\x1b[1;{mod}P`..`\x1b[1;{mod}S`
/// F5-F12: tilde sequences with standard codes
fn f_key_seq(n: u8, mods: KeyModifiers) -> Vec<u8> {
    match n {
        1..=4 => {
            let suffix = b'P' + n - 1;
            match modifier_param(mods) {
                Some(m) => format!("\x1b[1;{m}{}", suffix as char).into_bytes(),
                None => vec![0x1b, b'O', suffix],
            }
        }
        5..=12 => {
            let code = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => unreachable!(),
            };
            tilde_seq(code, mods)
        }
        _ => vec![],
    }
}

/// `\x1b[{code}~` or `\x1b[{code};{mod}~`
fn tilde_seq(code: u8, mods: KeyModifiers) -> Vec<u8> {
    match modifier_param(mods) {
        Some(m) => format!("\x1b[{code};{m}~").into_bytes(),
        None => format!("\x1b[{code}~").into_bytes(),
    }
}

pub fn mouse_to_bytes(event: crossterm::event::MouseEvent) -> Vec<u8> {
    use crossterm::event::{MouseButton, MouseEventKind};

    fn button_code(button: MouseButton, offset: u8) -> u8 {
        let base = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
        };
        base + offset
    }

    let col = event.column + 1;
    let row = event.row + 1;

    match event.kind {
        MouseEventKind::Down(button) => {
            format!("\x1b[<{};{};{}M", button_code(button, 0), col, row).into_bytes()
        }
        MouseEventKind::Up(button) => {
            format!("\x1b[<{};{};{}m", button_code(button, 0), col, row).into_bytes()
        }
        MouseEventKind::Drag(button) => {
            format!("\x1b[<{};{};{}M", button_code(button, 32), col, row).into_bytes()
        }
        MouseEventKind::ScrollUp => format!("\x1b[<64;{col};{row}M").into_bytes(),
        MouseEventKind::ScrollDown => format!("\x1b[<65;{col};{row}M").into_bytes(),
        MouseEventKind::Moved | MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => vec![],
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

    // --- Char keys ---

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
    fn ctrl_bracket_returns_esc() {
        let result = key_to_bytes(make_key(KeyCode::Char('['), KeyModifiers::CONTROL));
        assert_eq!(result, vec![0x1b]);
    }

    #[test]
    fn alt_char_returns_esc_prefixed() {
        let result = key_to_bytes(make_key(KeyCode::Char('a'), KeyModifiers::ALT));
        assert_eq!(result, vec![0x1b, 0x61]);
    }

    #[test]
    fn ctrl_alt_char_returns_esc_plus_ctrl_code() {
        let result = key_to_bytes(make_key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ));
        assert_eq!(result, vec![0x1b, 0x03]);
    }

    #[test]
    fn shift_char_returns_uppercase() {
        let result = key_to_bytes(make_key(KeyCode::Char('a'), KeyModifiers::SHIFT));
        assert_eq!(result, vec![0x41]);
    }

    // --- Enter / Backspace / Esc / Tab ---

    #[test]
    fn enter_returns_0x0d() {
        let result = key_to_bytes(make_key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(result, vec![0x0d]);
    }

    #[test]
    fn alt_enter_returns_esc_cr() {
        let result = key_to_bytes(make_key(KeyCode::Enter, KeyModifiers::ALT));
        assert_eq!(result, vec![0x1b, 0x0d]);
    }

    #[test]
    fn backspace_returns_0x7f() {
        let result = key_to_bytes(make_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(result, vec![0x7f]);
    }

    #[test]
    fn alt_backspace_returns_esc_del() {
        let result = key_to_bytes(make_key(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(result, vec![0x1b, 0x7f]);
    }

    #[test]
    fn esc_returns_0x1b() {
        let result = key_to_bytes(make_key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(result, vec![0x1b]);
    }

    #[test]
    fn tab_returns_0x09() {
        let result = key_to_bytes(make_key(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(result, vec![0x09]);
    }

    #[test]
    fn shift_tab_returns_backtab() {
        let result = key_to_bytes(make_key(KeyCode::Tab, KeyModifiers::SHIFT));
        assert_eq!(result, b"\x1b[Z");
    }

    #[test]
    fn backtab_returns_backtab_sequence() {
        let result = key_to_bytes(make_key(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(result, b"\x1b[Z");
    }

    // --- Arrow keys ---

    #[test]
    fn up_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[A");
    }

    #[test]
    fn down_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[B");
    }

    #[test]
    fn right_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[C");
    }

    #[test]
    fn left_returns_escape_sequence() {
        let result = key_to_bytes(make_key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[D");
    }

    #[test]
    fn shift_up_returns_modified_csi() {
        let result = key_to_bytes(make_key(KeyCode::Up, KeyModifiers::SHIFT));
        assert_eq!(result, b"\x1b[1;2A");
    }

    #[test]
    fn alt_right_returns_modified_csi() {
        let result = key_to_bytes(make_key(KeyCode::Right, KeyModifiers::ALT));
        assert_eq!(result, b"\x1b[1;3C");
    }

    #[test]
    fn ctrl_left_returns_modified_csi() {
        let result = key_to_bytes(make_key(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(result, b"\x1b[1;5D");
    }

    #[test]
    fn ctrl_shift_down_returns_modified_csi() {
        let result = key_to_bytes(make_key(
            KeyCode::Down,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        assert_eq!(result, b"\x1b[1;6B");
    }

    // --- Navigation keys ---

    #[test]
    fn home_returns_csi_h() {
        let result = key_to_bytes(make_key(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[H");
    }

    #[test]
    fn end_returns_csi_f() {
        let result = key_to_bytes(make_key(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[F");
    }

    #[test]
    fn page_up_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[5~");
    }

    #[test]
    fn page_down_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[6~");
    }

    #[test]
    fn insert_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::Insert, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[2~");
    }

    #[test]
    fn delete_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[3~");
    }

    #[test]
    fn ctrl_delete_returns_modified_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::Delete, KeyModifiers::CONTROL));
        assert_eq!(result, b"\x1b[3;5~");
    }

    // --- Function keys ---

    #[test]
    fn f1_returns_ss3_p() {
        let result = key_to_bytes(make_key(KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(result, b"\x1bOP");
    }

    #[test]
    fn f4_returns_ss3_s() {
        let result = key_to_bytes(make_key(KeyCode::F(4), KeyModifiers::NONE));
        assert_eq!(result, b"\x1bOS");
    }

    #[test]
    fn f5_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::F(5), KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[15~");
    }

    #[test]
    fn f12_returns_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::F(12), KeyModifiers::NONE));
        assert_eq!(result, b"\x1b[24~");
    }

    #[test]
    fn shift_f1_returns_modified_csi() {
        let result = key_to_bytes(make_key(KeyCode::F(1), KeyModifiers::SHIFT));
        assert_eq!(result, b"\x1b[1;2P");
    }

    #[test]
    fn ctrl_f5_returns_modified_tilde_seq() {
        let result = key_to_bytes(make_key(KeyCode::F(5), KeyModifiers::CONTROL));
        assert_eq!(result, b"\x1b[15;5~");
    }

    // --- Unknown ---

    #[test]
    fn unknown_key_returns_empty() {
        let result = key_to_bytes(make_key(KeyCode::F(20), KeyModifiers::NONE));
        assert!(result.is_empty());
    }

    #[test]
    fn mouse_scroll_up_returns_sgr_sequence() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 9,
            row: 4,
            modifiers: KeyModifiers::NONE,
        };
        let result = mouse_to_bytes(event);
        assert_eq!(result, b"\x1b[<64;10;5M");
    }

    #[test]
    fn mouse_scroll_down_returns_sgr_sequence() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let result = mouse_to_bytes(event);
        assert_eq!(result, b"\x1b[<65;1;1M");
    }

    fn make_mouse(
        kind: crossterm::event::MouseEventKind,
        col: u16,
        row: u16,
    ) -> crossterm::event::MouseEvent {
        crossterm::event::MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn char_multibyte_utf8() {
        let result = key_to_bytes(make_key(KeyCode::Char('é'), KeyModifiers::NONE));
        assert_eq!(result, "é".as_bytes());
    }

    #[test]
    fn mouse_drag_returns_sgr_with_offset() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let event = make_mouse(MouseEventKind::Drag(MouseButton::Left), 5, 3);
        let result = mouse_to_bytes(event);
        // button_code(Left, 32) = 32, col+1=6, row+1=4
        assert_eq!(result, b"\x1b[<32;6;4M");
    }

    #[test]
    fn mouse_left_down_returns_sgr_press() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let event = make_mouse(MouseEventKind::Down(MouseButton::Left), 0, 0);
        let result = mouse_to_bytes(event);
        assert_eq!(result, b"\x1b[<0;1;1M");
    }

    #[test]
    fn mouse_left_up_returns_sgr_release() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let event = make_mouse(MouseEventKind::Up(MouseButton::Left), 0, 0);
        let result = mouse_to_bytes(event);
        assert_eq!(result, b"\x1b[<0;1;1m");
    }

    #[test]
    fn mouse_moved_returns_empty() {
        use crossterm::event::MouseEventKind;
        let event = make_mouse(MouseEventKind::Moved, 5, 5);
        let result = mouse_to_bytes(event);
        assert!(result.is_empty());
    }

    #[test]
    fn mouse_right_down_returns_sgr_press() {
        use crossterm::event::{MouseButton, MouseEventKind};
        let event = make_mouse(MouseEventKind::Down(MouseButton::Right), 10, 5);
        let result = mouse_to_bytes(event);
        // button_code(Right, 0) = 2, col+1=11, row+1=6
        assert_eq!(result, b"\x1b[<2;11;6M");
    }
}
