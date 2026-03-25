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

    use super::{Command, apply_command, command_for, help_lines};
    use crate::monitor::{
        MonitorExit,
        model::{AbsoluteState, DeviceInput, InputCollection, InputKind},
        plan::{Counts, NavigationContext, TestScrollBounds, TestScrollState},
        state::{ActivePopup, Focus, MonitorState},
    };

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn shifted_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn navigation(focus: Focus, focusable: bool) -> NavigationContext {
        NavigationContext::new_for_tests(
            focus,
            TestScrollState {
                axis: 1,
                button_row: 1,
            },
            TestScrollBounds::new_for_tests(4, 3, true, true),
            focusable,
        )
    }

    fn monitor_state() -> MonitorState {
        MonitorState::new(Counts::new(2, 0, 6), vec!["info".to_string()])
    }

    fn relative_input(name: &str, value: i32) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Relative(value),
        }
    }

    fn empty_inputs() -> InputCollection {
        InputCollection::from_entries_for_tests(Vec::new(), Vec::new(), Vec::new())
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
    fn command_for_maps_normal_mode_bindings() {
        assert_eq!(
            command_for(key(KeyCode::Char('r')), ActivePopup::None),
            Command::Reset
        );
        assert_eq!(
            command_for(key(KeyCode::Home), ActivePopup::None),
            Command::Home
        );
        assert_eq!(
            command_for(key(KeyCode::Char('g')), ActivePopup::None),
            Command::Home
        );
        assert_eq!(
            command_for(key(KeyCode::End), ActivePopup::None),
            Command::End
        );
        assert_eq!(
            command_for(shifted_char('G'), ActivePopup::None),
            Command::End
        );
        assert_eq!(
            command_for(key(KeyCode::Up), ActivePopup::None),
            Command::Scroll(-1)
        );
        assert_eq!(
            command_for(key(KeyCode::Char('k')), ActivePopup::None),
            Command::Scroll(-1)
        );
        assert_eq!(
            command_for(key(KeyCode::Down), ActivePopup::None),
            Command::Scroll(1)
        );
        assert_eq!(
            command_for(key(KeyCode::Char('j')), ActivePopup::None),
            Command::Scroll(1)
        );
        assert_eq!(
            command_for(key(KeyCode::PageUp), ActivePopup::None),
            Command::Page(-1)
        );
        assert_eq!(
            command_for(key(KeyCode::PageDown), ActivePopup::None),
            Command::Page(1)
        );
        assert_eq!(
            command_for(key(KeyCode::Char('i')), ActivePopup::None),
            Command::ToggleInfo
        );
        assert_eq!(
            command_for(key(KeyCode::Char('y')), ActivePopup::None),
            Command::ToggleInvertY
        );
        assert_eq!(
            command_for(key(KeyCode::Char('?')), ActivePopup::None),
            Command::ToggleHelp
        );
        assert_eq!(
            command_for(shifted_char('J'), ActivePopup::None),
            Command::FocusNext
        );
        assert_eq!(
            command_for(shifted_char('K'), ActivePopup::None),
            Command::FocusPrev
        );
    }

    #[test]
    fn command_for_popup_modes_only_allows_close_or_exit() {
        assert_eq!(
            command_for(key(KeyCode::Char('i')), ActivePopup::Info),
            Command::ToggleInfo
        );
        assert_eq!(
            command_for(key(KeyCode::Char('?')), ActivePopup::Help),
            Command::ToggleHelp
        );
        assert_eq!(
            command_for(key(KeyCode::Enter), ActivePopup::Info),
            Command::None
        );
        assert_eq!(
            command_for(key(KeyCode::Char('j')), ActivePopup::Info),
            Command::None
        );
        assert_eq!(
            command_for(key(KeyCode::Char('i')), ActivePopup::Help),
            Command::None
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

    #[test]
    fn apply_command_returns_requested_exit_variant() {
        let navigation = navigation(Focus::Axes, true);
        let mut state = monitor_state();
        let mut inputs = empty_inputs();

        assert!(matches!(
            apply_command(Command::BackToSelector, &mut state, &mut inputs, navigation),
            Some(MonitorExit::BackToSelector)
        ));
        assert!(matches!(
            apply_command(Command::ExitApp, &mut state, &mut inputs, navigation),
            Some(MonitorExit::ExitApp)
        ));
    }

    #[test]
    fn apply_command_reset_clears_relative_axes() {
        let mut inputs = InputCollection::from_entries_for_tests(
            vec![(
                0,
                DeviceInput {
                    name: "abs_x".to_string(),
                    input_type: InputKind::Absolute(AbsoluteState::kernel(-10, 10, 4)),
                },
            )],
            vec![
                (1, relative_input("rel_x", 3)),
                (2, relative_input("rel_y", -2)),
            ],
            Vec::new(),
        );

        apply_command(
            Command::Reset,
            &mut monitor_state(),
            &mut inputs,
            navigation(Focus::Axes, true),
        );

        assert_eq!(
            inputs.relative_inputs()[0].input_type,
            InputKind::Relative(0)
        );
        assert_eq!(
            inputs.relative_inputs()[1].input_type,
            InputKind::Relative(0)
        );
        assert_eq!(
            inputs.absolute_inputs()[0].input_type,
            InputKind::Absolute(AbsoluteState::kernel(-10, 10, 4))
        );
    }

    #[test]
    fn apply_command_updates_scroll_focus_popup_and_invert_state() {
        let mut state = monitor_state();
        let mut inputs = empty_inputs();

        apply_command(
            Command::Scroll(1),
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.axis_scroll(), 1);
        assert_eq!(state.button_row_scroll(), 0);

        apply_command(
            Command::Page(1),
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.axis_scroll(), 4);

        apply_command(
            Command::Home,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.axis_scroll(), 0);

        apply_command(
            Command::End,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.axis_scroll(), 4);

        apply_command(
            Command::FocusNext,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.focus(), Focus::Buttons);

        apply_command(
            Command::FocusPrev,
            &mut state,
            &mut inputs,
            navigation(Focus::Buttons, true),
        );
        assert_eq!(state.focus(), Focus::Axes);

        assert!(state.joystick_invert_y());
        apply_command(
            Command::ToggleInvertY,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert!(!state.joystick_invert_y());

        apply_command(
            Command::ToggleInfo,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.active_popup(), ActivePopup::Info);

        apply_command(
            Command::ToggleHelp,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, true),
        );
        assert_eq!(state.active_popup(), ActivePopup::Help);
    }

    #[test]
    fn apply_command_none_is_a_no_op() {
        let mut state = monitor_state();
        let mut inputs = empty_inputs();

        assert!(
            apply_command(
                Command::None,
                &mut state,
                &mut inputs,
                navigation(Focus::Axes, true),
            )
            .is_none()
        );
        assert_eq!(state.axis_scroll(), 0);
        assert_eq!(state.button_row_scroll(), 0);
        assert_eq!(state.focus(), Focus::Axes);
        assert_eq!(state.active_popup(), ActivePopup::None);
        assert!(state.joystick_invert_y());
    }
}
