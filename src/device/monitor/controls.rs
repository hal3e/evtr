#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    BackToSelector,
    ExitApp,
    Reset,
    Scroll(i32),
    Page(i32),
    Home,
    End,
    FocusNext,
    FocusPrev,
    ToggleInvertY,
    ToggleInfo,
    ToggleHelp,
    None,
}
