#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    CreateWorktree,
    DeleteWorktree,
    FocusNext,
    None,
    Quit,
    Render,
    Resize(u16, u16),
    SelectNextWorktree,
    SelectPrevWorktree,
    SendBytes(Vec<u8>),
    StartWorktree,
    StopWorktree,
    Tick,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_same_variants() {
        assert_eq!(Action::Quit, Action::Quit);
        assert_eq!(Action::Tick, Action::Tick);
        assert_eq!(Action::Render, Action::Render);
        assert_eq!(Action::None, Action::None);
        assert_eq!(Action::FocusNext, Action::FocusNext);
        assert_eq!(Action::CreateWorktree, Action::CreateWorktree);
        assert_eq!(Action::DeleteWorktree, Action::DeleteWorktree);
        assert_eq!(Action::SelectNextWorktree, Action::SelectNextWorktree);
        assert_eq!(Action::SelectPrevWorktree, Action::SelectPrevWorktree);
    }

    #[test]
    fn equality_different_variants() {
        assert_ne!(Action::Quit, Action::Tick);
        assert_ne!(Action::Render, Action::None);
        assert_ne!(Action::FocusNext, Action::Quit);
    }

    #[test]
    fn equality_with_data() {
        assert_eq!(Action::Resize(80, 24), Action::Resize(80, 24));
        assert_ne!(Action::Resize(80, 24), Action::Resize(100, 50));

        assert_eq!(
            Action::SendBytes(vec![0x1b, 0x5b]),
            Action::SendBytes(vec![0x1b, 0x5b])
        );
        assert_ne!(
            Action::SendBytes(vec![0x1b]),
            Action::SendBytes(vec![0x1b, 0x5b])
        );
    }

    #[test]
    fn clone_produces_equal_value() {
        let actions = [
            Action::Tick,
            Action::Render,
            Action::Quit,
            Action::None,
            Action::Resize(120, 40),
            Action::SendBytes(vec![1, 2, 3]),
        ];
        for action in &actions {
            assert_eq!(action, &action.clone());
        }
    }
}
