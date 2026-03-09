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
