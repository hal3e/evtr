use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectorMode {
    Browsing,
    Help,
}

impl SelectorMode {
    pub(super) fn is_browsing(self) -> bool {
        matches!(self, Self::Browsing)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectorCommand {
    Exit,
    Back,
    ToggleHelp,
    Refresh,
    Select,
    ClearSearch,
    DeleteChar,
    AddChar(char),
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    None,
}

pub(super) fn command_for(key: KeyEvent, mode: SelectorMode) -> SelectorCommand {
    let keys = config::keys().selector;
    match mode {
        SelectorMode::Help => {
            if matches_any(key, &keys.toggle_help) {
                SelectorCommand::ToggleHelp
            } else if matches_any(key, &keys.exit) {
                SelectorCommand::Exit
            } else {
                SelectorCommand::None
            }
        }
        SelectorMode::Browsing => {
            if matches_any(key, &keys.select) {
                SelectorCommand::Select
            } else if matches_any(key, &keys.back) {
                SelectorCommand::Back
            } else if matches_any(key, &keys.toggle_help) {
                SelectorCommand::ToggleHelp
            } else if matches_any(key, &keys.exit) {
                SelectorCommand::Exit
            } else if matches_any(key, &keys.move_up) {
                SelectorCommand::MoveUp
            } else if matches_any(key, &keys.move_down) {
                SelectorCommand::MoveDown
            } else if matches_any(key, &keys.page_up) {
                SelectorCommand::PageUp
            } else if matches_any(key, &keys.page_down) {
                SelectorCommand::PageDown
            } else if matches_any(key, &keys.home) {
                SelectorCommand::Home
            } else if matches_any(key, &keys.end) {
                SelectorCommand::End
            } else if matches_any(key, &keys.delete_char) {
                SelectorCommand::DeleteChar
            } else if matches_any(key, &keys.clear_search) {
                SelectorCommand::ClearSearch
            } else if matches_any(key, &keys.refresh) {
                SelectorCommand::Refresh
            } else {
                add_char_command(key)
            }
        }
    }
}

pub(super) fn help_lines() -> Vec<String> {
    let keys = config::keys().selector;
    vec![
        format!("Move up: {}", bindings(&keys.move_up)),
        format!("Move down: {}", bindings(&keys.move_down)),
        format!("Page up: {}", bindings(&keys.page_up)),
        format!("Page down: {}", bindings(&keys.page_down)),
        format!("Home: {}", bindings(&keys.home)),
        format!("End: {}", bindings(&keys.end)),
        format!("Select: {}", bindings(&keys.select)),
        format!("Back: {}", bindings(&keys.back)),
        format!("Exit: {}", bindings(&keys.exit)),
        format!("Search clear: {}", bindings(&keys.clear_search)),
        format!("Delete char: {}", bindings(&keys.delete_char)),
        format!("Refresh: {}", bindings(&keys.refresh)),
        format!("Help: {}", bindings(&keys.toggle_help)),
    ]
}

fn matches_any(key: KeyEvent, bindings: &[config::KeyBinding]) -> bool {
    bindings.iter().any(|binding| binding.matches(key))
}

fn bindings(bindings: &[config::KeyBinding]) -> String {
    bindings
        .iter()
        .map(config::KeyBinding::display)
        .collect::<Vec<_>>()
        .join(", ")
}

fn add_char_command(key: KeyEvent) -> SelectorCommand {
    match key.code {
        KeyCode::Char(c) if key.modifiers == KeyModifiers::SHIFT || key.modifiers.is_empty() => {
            SelectorCommand::AddChar(c)
        }
        _ => SelectorCommand::None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{SelectorCommand, SelectorMode, command_for, help_lines};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn shifted_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn command_for_ctrl_c_exits_from_any_mode() {
        for mode in [SelectorMode::Browsing, SelectorMode::Help] {
            assert_eq!(command_for(ctrl_char('c'), mode), SelectorCommand::Exit);
        }
    }

    #[test]
    fn command_for_escape_depends_on_mode() {
        assert_eq!(
            command_for(key(KeyCode::Esc), SelectorMode::Browsing),
            SelectorCommand::Back
        );
        assert_eq!(
            command_for(key(KeyCode::Char('?')), SelectorMode::Help),
            SelectorCommand::ToggleHelp
        );
    }

    #[test]
    fn command_for_maps_navigation_keys_to_explicit_variants() {
        assert_eq!(
            command_for(key(KeyCode::Up), SelectorMode::Browsing),
            SelectorCommand::MoveUp
        );
        assert_eq!(
            command_for(ctrl_char('p'), SelectorMode::Browsing),
            SelectorCommand::MoveUp
        );
        assert_eq!(
            command_for(key(KeyCode::Down), SelectorMode::Browsing),
            SelectorCommand::MoveDown
        );
        assert_eq!(
            command_for(ctrl_char('n'), SelectorMode::Browsing),
            SelectorCommand::MoveDown
        );
    }

    #[test]
    fn command_for_only_adds_plain_and_shifted_characters() {
        assert_eq!(
            command_for(key(KeyCode::Char('a')), SelectorMode::Browsing),
            SelectorCommand::AddChar('a')
        );
        assert_eq!(
            command_for(shifted_char('A'), SelectorMode::Browsing),
            SelectorCommand::AddChar('A')
        );
        assert_eq!(
            command_for(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
                SelectorMode::Browsing
            ),
            SelectorCommand::None
        );
    }

    #[test]
    fn help_lines_reflect_default_bindings() {
        let lines = help_lines();
        assert!(lines.iter().any(|line| line == "Move up: Up, Ctrl-p"));
        assert!(lines.iter().any(|line| line == "Refresh: Ctrl-r"));
    }
}
