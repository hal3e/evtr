use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    PAGE_SCROLL_SIZE,
    state::{BackAction, SelectionAction, SelectorState},
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorEffect {
    Exit,
    RefreshDevices,
    OpenSelection,
}

pub(crate) fn command_for(key: KeyEvent, mode: SelectorMode) -> SelectorCommand {
    match mode {
        SelectorMode::Help => match key.code {
            KeyCode::Esc | KeyCode::Char('?') => SelectorCommand::ToggleHelp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            _ => SelectorCommand::None,
        },
        SelectorMode::Browsing => match key.code {
            KeyCode::Enter => SelectorCommand::Select,
            KeyCode::Esc => SelectorCommand::Back,
            KeyCode::Char('?') => SelectorCommand::ToggleHelp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::Exit
            }
            KeyCode::Up => SelectorCommand::MoveUp,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveUp
            }
            KeyCode::Down => SelectorCommand::MoveDown,
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::MoveDown
            }
            KeyCode::PageUp => SelectorCommand::PageUp,
            KeyCode::PageDown => SelectorCommand::PageDown,
            KeyCode::Home => SelectorCommand::Home,
            KeyCode::End => SelectorCommand::End,
            KeyCode::Backspace => SelectorCommand::DeleteChar,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SelectorCommand::ClearSearch
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
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

pub(crate) fn apply_command(
    state: &mut SelectorState,
    command: SelectorCommand,
) -> Option<SelectorEffect> {
    match command {
        SelectorCommand::Exit => Some(SelectorEffect::Exit),
        SelectorCommand::Back => match state.back_action() {
            BackAction::Exit => Some(SelectorEffect::Exit),
            BackAction::ClearSearch => {
                state.clear_search();
                None
            }
        },
        SelectorCommand::ToggleHelp => {
            state.toggle_help();
            None
        }
        SelectorCommand::Refresh => Some(SelectorEffect::RefreshDevices),
        SelectorCommand::Select => match state.selection_action() {
            Some(SelectionAction::Refresh) => Some(SelectorEffect::RefreshDevices),
            Some(SelectionAction::OpenSelected) => Some(SelectorEffect::OpenSelection),
            None => None,
        },
        SelectorCommand::ClearSearch => {
            state.clear_search();
            None
        }
        SelectorCommand::DeleteChar => {
            state.remove_char();
            None
        }
        SelectorCommand::AddChar(c) => {
            state.add_char(c);
            None
        }
        SelectorCommand::MoveUp => {
            state.move_selection_by(-1);
            None
        }
        SelectorCommand::MoveDown => {
            state.move_selection_by(1);
            None
        }
        SelectorCommand::PageUp => {
            state.move_selection_by(-(PAGE_SCROLL_SIZE as i32));
            None
        }
        SelectorCommand::PageDown => {
            state.move_selection_by(PAGE_SCROLL_SIZE as i32);
            None
        }
        SelectorCommand::Home => {
            state.select_index(0);
            None
        }
        SelectorCommand::End => {
            if let Some(last_index) = state.filtered_indexes().len().checked_sub(1) {
                state.select_index(last_index);
            }
            None
        }
        SelectorCommand::None => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{SelectorCommand, SelectorMode, command_for};

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
}
