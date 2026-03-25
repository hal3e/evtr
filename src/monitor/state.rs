mod popup;
mod scroll;

use crate::monitor::plan::Counts;

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
    pub(super) fn new(counts: Counts, info_lines: Vec<String>) -> Self {
        let focus = if counts.total_axes() > 0 {
            Focus::Axes
        } else {
            Focus::Buttons
        };

        Self {
            counts,
            info_lines,
            active_popup: ActivePopup::None,
            focus,
            axis_scroll: 0,
            button_row_scroll: 0,
            joystick_invert_y: true,
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
