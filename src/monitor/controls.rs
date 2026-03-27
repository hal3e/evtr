use crossterm::event::KeyEvent;

use crate::{
    config,
    monitor::{
        MonitorExit,
        model::InputCollection,
        plan::NavigationContext,
        state::{ActivePopup, MonitorState},
    },
};

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
    let keys = config::keys().monitor;
    match popup {
        ActivePopup::Info => {
            if matches_any(key_event, &keys.toggle_info) {
                Command::ToggleInfo
            } else if matches_any(key_event, &keys.exit) {
                Command::ExitApp
            } else {
                Command::None
            }
        }
        ActivePopup::Help => {
            if matches_any(key_event, &keys.toggle_help) {
                Command::ToggleHelp
            } else if matches_any(key_event, &keys.exit) {
                Command::ExitApp
            } else {
                Command::None
            }
        }
        ActivePopup::None => {
            if matches_any(key_event, &keys.back) {
                Command::BackToSelector
            } else if matches_any(key_event, &keys.exit) {
                Command::ExitApp
            } else if matches_any(key_event, &keys.reset) {
                Command::Reset
            } else if matches_any(key_event, &keys.home) {
                Command::Home
            } else if matches_any(key_event, &keys.end) {
                Command::End
            } else if matches_any(key_event, &keys.scroll_up) {
                Command::Scroll(-1)
            } else if matches_any(key_event, &keys.scroll_down) {
                Command::Scroll(1)
            } else if matches_any(key_event, &keys.toggle_info) {
                Command::ToggleInfo
            } else if matches_any(key_event, &keys.toggle_invert_y) {
                Command::ToggleInvertY
            } else if matches_any(key_event, &keys.toggle_help) {
                Command::ToggleHelp
            } else if matches_any(key_event, &keys.focus_next) {
                Command::FocusNext
            } else if matches_any(key_event, &keys.focus_prev) {
                Command::FocusPrev
            } else if matches_any(key_event, &keys.page_up) {
                Command::Page(-1)
            } else if matches_any(key_event, &keys.page_down) {
                Command::Page(1)
            } else {
                Command::None
            }
        }
    }
}

pub(super) fn help_lines() -> Vec<String> {
    let keys = config::keys().monitor;
    vec![
        format!("Scroll up: {}", bindings(&keys.scroll_up)),
        format!("Scroll down: {}", bindings(&keys.scroll_down)),
        format!("Page up: {}", bindings(&keys.page_up)),
        format!("Page down: {}", bindings(&keys.page_down)),
        format!("Home: {}", bindings(&keys.home)),
        format!("End: {}", bindings(&keys.end)),
        format!("Reset: {}", bindings(&keys.reset)),
        format!("Info: {}", bindings(&keys.toggle_info)),
        format!("Invert Y: {}", bindings(&keys.toggle_invert_y)),
        format!("Focus next: {}", bindings(&keys.focus_next)),
        format!("Focus previous: {}", bindings(&keys.focus_prev)),
        format!("Back: {}", bindings(&keys.back)),
        format!("Exit: {}", bindings(&keys.exit)),
        format!("Help: {}", bindings(&keys.toggle_help)),
    ]
}

fn matches_any(key_event: KeyEvent, bindings: &[config::KeyBinding]) -> bool {
    bindings.iter().any(|binding| binding.matches(key_event))
}

fn bindings(bindings: &[config::KeyBinding]) -> String {
    bindings
        .iter()
        .map(config::KeyBinding::display)
        .collect::<Vec<_>>()
        .join(", ")
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
            state.scroll_page(dir, navigation, config::monitor().page_scroll_steps);
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
    use crate::{
        config::StartupFocus,
        monitor::{
            MonitorExit,
            model::{DeviceInput, InputCollection, InputKind},
            plan::{Counts, NavigationContext, TestScrollBounds, TestScrollState},
            state::{ActivePopup, Focus, MonitorState},
        },
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
        MonitorState::new(
            Counts::new(2, 0, 6),
            vec!["info".to_string()],
            StartupFocus::Auto,
            true,
        )
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
            command_for(shifted_char('G'), ActivePopup::None),
            Command::End
        );
        assert_eq!(
            command_for(key(KeyCode::Char('k')), ActivePopup::None),
            Command::Scroll(-1)
        );
        assert_eq!(
            command_for(key(KeyCode::Char('j')), ActivePopup::None),
            Command::Scroll(1)
        );
    }

    #[test]
    fn apply_command_updates_scroll_focus_popup_and_invert_state() {
        let mut state = monitor_state();
        let mut inputs = InputCollection::from_entries_for_tests(
            Vec::new(),
            vec![(1, relative_input("rel_x", 3))],
            vec![],
        );
        let navigation = navigation(Focus::Axes, true);

        assert!(state.joystick_invert_y());
        apply_command(Command::ToggleInvertY, &mut state, &mut inputs, navigation);
        assert!(!state.joystick_invert_y());

        apply_command(Command::ToggleInfo, &mut state, &mut inputs, navigation);
        assert_eq!(state.active_popup(), ActivePopup::Info);

        apply_command(Command::ToggleHelp, &mut state, &mut inputs, navigation);
        assert_eq!(state.active_popup(), ActivePopup::Help);
    }

    #[test]
    fn apply_command_reset_clears_relative_axes() {
        let mut state = monitor_state();
        let mut inputs = InputCollection::from_entries_for_tests(
            Vec::new(),
            vec![
                (1, relative_input("rel_x", 3)),
                (2, relative_input("rel_y", -2)),
            ],
            Vec::new(),
        );

        apply_command(
            Command::Reset,
            &mut state,
            &mut inputs,
            navigation(Focus::Axes, false),
        );

        assert_eq!(
            inputs.relative_inputs()[0].input_type,
            InputKind::Relative(0)
        );
        assert_eq!(
            inputs.relative_inputs()[1].input_type,
            InputKind::Relative(0)
        );
    }

    #[test]
    fn help_lines_reflect_default_bindings() {
        let lines = help_lines();
        assert!(lines.iter().any(|line| line == "Exit: Ctrl-c"));
        assert!(lines.iter().any(|line| line == "Scroll up: Up, k"));
    }

    #[test]
    fn apply_command_returns_requested_exit_variant() {
        let mut state = monitor_state();
        let mut inputs = empty_inputs();

        assert_eq!(
            apply_command(
                Command::BackToSelector,
                &mut state,
                &mut inputs,
                navigation(Focus::Axes, false),
            ),
            Some(MonitorExit::BackToSelector)
        );
        assert_eq!(
            apply_command(
                Command::ExitApp,
                &mut state,
                &mut inputs,
                navigation(Focus::Axes, false),
            ),
            Some(MonitorExit::ExitApp)
        );
    }
}
