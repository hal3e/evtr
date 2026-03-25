use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const EXIT_KEY: char = 'c';
const HELP_KEY: char = '?';
const MOVE_UP_KEY: char = 'p';
const MOVE_DOWN_KEY: char = 'n';
const CLEAR_SEARCH_KEY: char = 'u';
const REFRESH_KEY: char = 'r';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorMode {
    Browsing,
    Help,
}

impl SelectorMode {
    pub(crate) fn is_browsing(self) -> bool {
        matches!(self, Self::Browsing)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorCommand {
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

pub(crate) fn command_for(key: KeyEvent, mode: SelectorMode) -> SelectorCommand {
    match mode {
        SelectorMode::Help => match key.code {
            KeyCode::Esc | KeyCode::Char(HELP_KEY) => SelectorCommand::ToggleHelp,
            KeyCode::Char(EXIT_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            _ => SelectorCommand::None,
        },
        SelectorMode::Browsing => match key.code {
            KeyCode::Enter => SelectorCommand::Select,
            KeyCode::Esc => SelectorCommand::Back,
            KeyCode::Char(HELP_KEY) => SelectorCommand::ToggleHelp,
            KeyCode::Char(EXIT_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            KeyCode::Up => SelectorCommand::MoveUp,
            KeyCode::Char(MOVE_UP_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveUp
            }
            KeyCode::Down => SelectorCommand::MoveDown,
            KeyCode::Char(MOVE_DOWN_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveDown
            }
            KeyCode::PageUp => SelectorCommand::PageUp,
            KeyCode::PageDown => SelectorCommand::PageDown,
            KeyCode::Home => SelectorCommand::Home,
            KeyCode::End => SelectorCommand::End,
            KeyCode::Backspace => SelectorCommand::DeleteChar,
            KeyCode::Char(CLEAR_SEARCH_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::ClearSearch
            }
            KeyCode::Char(REFRESH_KEY) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Refresh
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::SHIFT || key.modifiers.is_empty() =>
            {
                SelectorCommand::AddChar(c)
            }
            _ => SelectorCommand::None,
        },
    }
}

pub(crate) fn help_lines() -> Vec<String> {
    vec![
        format!(
            "Move: Up/Down, {}, {}, PageUp/PageDown, Home/End",
            ctrl_key(MOVE_UP_KEY),
            ctrl_key(MOVE_DOWN_KEY),
        ),
        "Select: Enter".to_string(),
        format!("Exit: Esc or {}", ctrl_key(EXIT_KEY)),
        format!(
            "Search: type to filter, Backspace, {} clear",
            ctrl_key(CLEAR_SEARCH_KEY),
        ),
        format!("Refresh: {}", ctrl_key(REFRESH_KEY)),
        format!("Help: {HELP_KEY} (press {HELP_KEY} or Esc to close)"),
    ]
}

fn ctrl_key(key: char) -> String {
    format!("Ctrl-{}", key.to_ascii_uppercase())
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
            command_for(key(KeyCode::Esc), SelectorMode::Help),
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
        assert_eq!(
            command_for(key(KeyCode::PageUp), SelectorMode::Browsing),
            SelectorCommand::PageUp
        );
        assert_eq!(
            command_for(key(KeyCode::PageDown), SelectorMode::Browsing),
            SelectorCommand::PageDown
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
    fn help_lines_match_the_documented_selector_bindings() {
        assert_eq!(
            help_lines(),
            vec![
                "Move: Up/Down, Ctrl-P, Ctrl-N, PageUp/PageDown, Home/End".to_string(),
                "Select: Enter".to_string(),
                "Exit: Esc or Ctrl-C".to_string(),
                "Search: type to filter, Backspace, Ctrl-U clear".to_string(),
                "Refresh: Ctrl-R".to_string(),
                "Help: ? (press ? or Esc to close)".to_string(),
            ]
        );
    }
}
