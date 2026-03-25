use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::monitor::{
    MonitorExit, config,
    model::InputCollection,
    plan::RenderPlan,
    state::{ActivePopup, MonitorState},
};

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

pub(crate) fn command_for(key_event: KeyEvent, popup: ActivePopup) -> Command {
    match popup {
        ActivePopup::Info => match key_event.code {
            KeyCode::Esc | KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::Help => match key_event.code {
            KeyCode::Esc | KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::None => match key_event.code {
            KeyCode::Esc => Command::BackToSelector,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Home | KeyCode::Char('g') => Command::Home,
            KeyCode::End | KeyCode::Char('G') => Command::End,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('y') => Command::ToggleInvertY,
            KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('J') => Command::FocusNext,
            KeyCode::Char('K') => Command::FocusPrev,
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        },
    }
}

pub(crate) fn apply_command(
    command: Command,
    state: &mut MonitorState,
    inputs: &mut InputCollection,
    plan: &RenderPlan,
) -> Option<MonitorExit> {
    match command {
        Command::BackToSelector => Some(MonitorExit::BackToSelector),
        Command::ExitApp => Some(MonitorExit::ExitApp),
        Command::Reset => {
            inputs.reset_relative_axes();
            None
        }
        Command::Scroll(dir) => {
            state.scroll_by(dir, plan);
            None
        }
        Command::Page(dir) => {
            state.scroll_page(dir, plan, config::PAGE_SCROLL_STEPS);
            None
        }
        Command::Home => {
            state.scroll_home(plan);
            None
        }
        Command::End => {
            state.scroll_end(plan);
            None
        }
        Command::FocusNext => {
            state.focus_next(plan);
            None
        }
        Command::FocusPrev => {
            state.focus_prev(plan);
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

    use super::{Command, command_for};
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
}
