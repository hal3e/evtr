#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Quit,
    Reset,
    Scroll(i32),
    Page(i32),
    Home,
    End,
    FocusNext,
    FocusPrev,
    None,
}
