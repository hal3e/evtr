use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::monitor::{
    MonitorExit, config,
    model::InputCollection,
    plan::NavigationContext,
    state::{ActivePopup, MonitorState},
};

const EXIT_KEY: char = 'c';
const RESET_KEY: char = 'r';
const HOME_KEY: char = 'g';
const END_KEY: char = 'G';
const SCROLL_UP_KEY: char = 'k';
const SCROLL_DOWN_KEY: char = 'j';
const INFO_KEY: char = 'i';
const INVERT_Y_KEY: char = 'y';
const HELP_KEY: char = '?';
const FOCUS_NEXT_KEY: char = 'J';
const FOCUS_PREV_KEY: char = 'K';

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Command {
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

pub(super) fn command_for(key_event: KeyEvent, popup: ActivePopup) -> Command {
    match popup {
        ActivePopup::Info => match key_event.code {
            KeyCode::Esc | KeyCode::Char(INFO_KEY) => Command::ToggleInfo,
            KeyCode::Char(EXIT_KEY) if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::Help => match key_event.code {
            KeyCode::Esc | KeyCode::Char(HELP_KEY) => Command::ToggleHelp,
            KeyCode::Char(EXIT_KEY) if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::None => match key_event.code {
            KeyCode::Esc => Command::BackToSelector,
            KeyCode::Char(EXIT_KEY) if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            KeyCode::Char(RESET_KEY) => Command::Reset,
            KeyCode::Home | KeyCode::Char(HOME_KEY) => Command::Home,
            KeyCode::End | KeyCode::Char(END_KEY) => Command::End,
            KeyCode::Up | KeyCode::Char(SCROLL_UP_KEY) => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char(SCROLL_DOWN_KEY) => Command::Scroll(1),
            KeyCode::Char(INFO_KEY) => Command::ToggleInfo,
            KeyCode::Char(INVERT_Y_KEY) => Command::ToggleInvertY,
            KeyCode::Char(HELP_KEY) => Command::ToggleHelp,
            KeyCode::Char(FOCUS_NEXT_KEY) => Command::FocusNext,
            KeyCode::Char(FOCUS_PREV_KEY) => Command::FocusPrev,
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        },
    }
}

pub(super) fn help_lines() -> Vec<String> {
    vec![
        format!("Scroll: Up/Down or {SCROLL_UP_KEY}/{SCROLL_DOWN_KEY}, PageUp/PageDown"),
        format!("Jump: Home/End or {HOME_KEY}/{END_KEY}"),
        format!("Reset: {RESET_KEY} (relative axes)"),
        format!("Info: {INFO_KEY} (press {INFO_KEY} or Esc to close)"),
        format!("Invert Y: {INVERT_Y_KEY}"),
        format!(
            "Focus: Shift+{} / Shift+{} (when axes and buttons show)",
            FOCUS_NEXT_KEY.to_ascii_uppercase(),
            FOCUS_PREV_KEY.to_ascii_uppercase(),
        ),
        "Back: Esc (when no popup is open)".to_string(),
        format!("Exit app: {}", ctrl_key(EXIT_KEY)),
        format!("Help: {HELP_KEY} (press {HELP_KEY} or Esc to close)"),
    ]
}

fn ctrl_key(key: char) -> String {
    format!("Ctrl-{}", key.to_ascii_uppercase())
}

pub(super) fn apply_command(
    command: Command,
    state: &mut MonitorState,
    inputs: &mut InputCollection,
    navigation: NavigationContext,
) -> Option<MonitorExit> {
    match command {
        Command::BackToSelector => Some(MonitorExit::BackToSelector),
        Command::ExitApp => Some(MonitorExit::ExitApp),
        Command::Reset => {
            inputs.reset_relative_axes();
            None
        }
        Command::Scroll(dir) => {
            state.scroll_by(dir, navigation);
            None
        }
        Command::Page(dir) => {
            state.scroll_page(dir, navigation, config::PAGE_SCROLL_STEPS);
            None
        }
        Command::Home => {
            state.scroll_home(navigation);
            None
        }
        Command::End => {
            state.scroll_end(navigation);
            None
        }
        Command::FocusNext => {
            state.focus_next(navigation);
            None
        }
        Command::FocusPrev => {
            state.focus_prev(navigation);
            None
        }
        Command::ToggleInvertY => {
            state.toggle_invert_y();
            None
        }
        Command::ToggleInfo => {
            state.toggle_info();
            None
        }
        Command::ToggleHelp => {
            state.toggle_help();
            None
        }
        Command::None => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Command, command_for, help_lines};
    use crate::monitor::state::ActivePopup;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn command_for_ctrl_c_exits_from_any_popup_state() {
        for popup in [ActivePopup::None, ActivePopup::Info, ActivePopup::Help] {
            assert_eq!(command_for(ctrl_char('c'), popup), Command::ExitApp);
        }
    }

    #[test]
    fn command_for_escape_backs_out_only_without_popup() {
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::None),
            Command::BackToSelector
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Info),
            Command::ToggleInfo
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Help),
            Command::ToggleHelp
        );
    }

    #[test]
    fn help_lines_match_the_documented_monitor_bindings() {
        assert_eq!(
            help_lines(),
            vec![
                "Scroll: Up/Down or k/j, PageUp/PageDown".to_string(),
                "Jump: Home/End or g/G".to_string(),
                "Reset: r (relative axes)".to_string(),
                "Info: i (press i or Esc to close)".to_string(),
                "Invert Y: y".to_string(),
                "Focus: Shift+J / Shift+K (when axes and buttons show)".to_string(),
                "Back: Esc (when no popup is open)".to_string(),
                "Exit app: Ctrl-C".to_string(),
                "Help: ? (press ? or Esc to close)".to_string(),
            ]
        );
    }
}
