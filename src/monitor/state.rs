mod popup;
mod scroll;

use crate::{config::StartupFocus, monitor::plan::Counts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Focus {
    Axes,
    Buttons,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActivePopup {
    None,
    Info,
    Help,
}

pub(super) struct MonitorState {
    counts: Counts,
    info_lines: Vec<String>,
    active_popup: ActivePopup,
    focus: Focus,
    axis_scroll: usize,
    button_row_scroll: usize,
    joystick_invert_y: bool,
}

impl MonitorState {
    pub(super) fn new(
        counts: Counts,
        info_lines: Vec<String>,
        startup_focus: StartupFocus,
        joystick_invert_y: bool,
    ) -> Self {
        let focus = initial_focus(counts, startup_focus);

        Self {
            counts,
            info_lines,
            active_popup: ActivePopup::None,
            focus,
            axis_scroll: 0,
            button_row_scroll: 0,
            joystick_invert_y,
        }
    }

    pub(super) fn counts(&self) -> Counts {
        self.counts
    }

    pub(super) fn info_lines(&self) -> &[String] {
        &self.info_lines
    }

    pub(super) fn joystick_invert_y(&self) -> bool {
        self.joystick_invert_y
    }

    pub(super) fn toggle_invert_y(&mut self) {
        self.joystick_invert_y = !self.joystick_invert_y;
    }
}

fn initial_focus(counts: Counts, startup_focus: StartupFocus) -> Focus {
    match startup_focus {
        StartupFocus::Auto => {
            if counts.total_axes() > 0 {
                Focus::Axes
            } else {
                Focus::Buttons
            }
        }
        StartupFocus::Axes => Focus::Axes,
        StartupFocus::Buttons => Focus::Buttons,
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivePopup, Focus, MonitorState};
    use crate::{config::StartupFocus, monitor::plan::Counts};

    #[test]
    fn new_prefers_axes_focus_when_any_axes_exist() {
        let state = MonitorState::new(
            Counts::new(1, 0, 3),
            vec!["info".to_string()],
            StartupFocus::Auto,
            true,
        );

        assert_eq!(state.focus, Focus::Axes);
    }

    #[test]
    fn new_falls_back_to_buttons_focus_when_no_axes_exist() {
        let state = MonitorState::new(
            Counts::new(0, 0, 3),
            vec!["info".to_string()],
            StartupFocus::Auto,
            true,
        );

        assert_eq!(state.focus, Focus::Buttons);
    }

    #[test]
    fn new_preserves_info_lines_and_defaults() {
        let state = MonitorState::new(
            Counts::new(0, 1, 2),
            vec![
                "name: pad".to_string(),
                "path: /dev/input/event3".to_string(),
            ],
            StartupFocus::Auto,
            true,
        );

        assert_eq!(
            state.info_lines(),
            &["name: pad", "path: /dev/input/event3"]
        );
        assert_eq!(state.active_popup, ActivePopup::None);
        assert_eq!(state.axis_scroll, 0);
        assert_eq!(state.button_row_scroll, 0);
        assert!(state.joystick_invert_y());
    }

    #[test]
    fn new_honors_explicit_button_focus() {
        let state = MonitorState::new(
            Counts::new(2, 0, 2),
            vec!["info".to_string()],
            StartupFocus::Buttons,
            false,
        );

        assert_eq!(state.focus, Focus::Buttons);
        assert!(!state.joystick_invert_y());
    }
}
