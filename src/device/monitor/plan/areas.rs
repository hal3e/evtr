use evdev::AbsoluteAxisCode;
use ratatui::layout::Rect;

use crate::device::widgets;

use super::Counts;
use crate::device::monitor::{
    config,
    layout::{axes_layout, box_layout, split_buttons_column},
    model::InputCollection,
    render::{hat::HatState, joystick::JoystickState},
    state::{Focus, MonitorState},
    touch::TouchState,
};

pub(crate) struct PlannedBoxes {
    pub(crate) joystick: Option<Rect>,
    pub(crate) hat: Option<Rect>,
    pub(crate) axes: Option<Rect>,
    pub(crate) touch: Option<Rect>,
    pub(crate) buttons: Option<Rect>,
}

pub(crate) struct PlannedAreas {
    pub(crate) joystick: Option<Rect>,
    pub(crate) hat: Option<Rect>,
    pub(crate) abs: Option<Rect>,
    pub(crate) rel: Option<Rect>,
    pub(crate) touch: Option<Rect>,
    pub(crate) buttons: Option<Rect>,
}

pub(super) struct AreaPlan {
    pub(super) focus: Focus,
    pub(super) boxes: PlannedBoxes,
    pub(super) areas: PlannedAreas,
}

pub(super) struct WidgetState {
    pub(super) joystick: JoystickState,
    pub(super) hat_state: Option<HatState>,
    joystick_count: usize,
    axes_available: bool,
    touch_enabled: bool,
    buttons_available: bool,
}

impl WidgetState {
    pub(super) fn from_inputs(
        state: &MonitorState,
        counts: Counts,
        inputs: &InputCollection,
        touch: &TouchState,
    ) -> Self {
        let joystick = if touch.is_touch_device() {
            JoystickState::default()
        } else {
            JoystickState::from_axes(
                inputs.absolute_axis_pair(AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
                inputs.absolute_axis_pair(AbsoluteAxisCode::ABS_RX, AbsoluteAxisCode::ABS_RY),
            )
        };
        let hat_state = if touch.is_touch_device() {
            None
        } else {
            inputs
                .absolute_axis_pair(AbsoluteAxisCode::ABS_HAT0X, AbsoluteAxisCode::ABS_HAT0Y)
                .map(|(x, y)| HatState::from_axes(x, y, state.joystick_invert_y()))
        };

        Self {
            joystick_count: joystick.count(),
            joystick,
            hat_state,
            axes_available: counts.total_axes() > 0,
            touch_enabled: touch.enabled(),
            buttons_available: counts.btn > 0,
        }
    }

    fn joystick_present(&self) -> bool {
        self.joystick_count > 0
    }

    fn hat_present(&self) -> bool {
        self.hat_state.is_some()
    }

    fn main_min_width(&self) -> u16 {
        let mut width = config::MAIN_COLUMN_MIN_WIDTH;
        if self.axes_available {
            width = width.max(config::AXIS_MIN_WIDTH);
        }
        if self.touch_enabled {
            width = width.max(config::TOUCHPAD_MIN_WIDTH);
        }
        if self.joystick_present() {
            width = width.max(config::JOYSTICK_MIN_SIZE);
        }
        if self.hat_present() {
            width = width.max(config::HAT_MIN_SIZE);
        }
        width
    }
}

pub(super) fn plan_areas(
    content: Rect,
    counts: Counts,
    min_button_gap: u16,
    current_focus: Focus,
    widget_state: &WidgetState,
) -> AreaPlan {
    let (main_area, buttons_column) = split_buttons_column(
        content,
        widget_state.buttons_available,
        widget_state.main_min_width(),
        config::BUTTONS_COLUMN_MIN_WIDTH,
        min_button_gap,
    );

    let axes_present = widget_state.axes_available && main_area.width >= config::AXIS_MIN_WIDTH;
    let touch_present = widget_state.touch_enabled && main_area.width >= config::TOUCHPAD_MIN_WIDTH;
    let button_width = main_area.width / config::BUTTONS_PER_ROW as u16;
    let buttons_present = widget_state.buttons_available && button_width > min_button_gap;

    let (layout, buttons_box) = if let Some(buttons_area) = buttons_column {
        let layout = box_layout(
            main_area,
            widget_state.joystick_present(),
            widget_state.joystick_count,
            widget_state.hat_present(),
            touch_present,
            axes_present,
            false,
        );
        (layout, Some(buttons_area))
    } else {
        let layout = box_layout(
            main_area,
            widget_state.joystick_present(),
            widget_state.joystick_count,
            widget_state.hat_present(),
            touch_present,
            axes_present,
            buttons_present,
        );
        let buttons_box = layout.buttons_box;
        (layout, buttons_box)
    };

    let boxes = PlannedBoxes {
        joystick: layout.joystick_box,
        hat: layout.hat_box,
        axes: layout.axes_box,
        touch: layout.touch_box,
        buttons: buttons_box,
    };
    let axes_inner = boxes.axes.map(widgets::bordered_box_inner);
    let axes_sections = axes_inner.map(|inner| axes_layout(inner, counts.abs, counts.rel));
    let (abs_area, rel_area) = if let Some(sections) = axes_sections {
        (sections.abs_area, sections.rel_area)
    } else {
        (None, None)
    };
    let areas = PlannedAreas {
        joystick: boxes.joystick.map(widgets::bordered_box_inner),
        hat: boxes.hat.map(widgets::bordered_box_inner),
        abs: abs_area,
        rel: rel_area,
        touch: boxes.touch.map(widgets::bordered_box_inner),
        buttons: boxes.buttons.map(widgets::bordered_box_inner),
    };

    AreaPlan {
        focus: synced_focus(current_focus, boxes.axes.is_some(), boxes.buttons.is_some()),
        boxes,
        areas,
    }
}

fn synced_focus(current: Focus, axes_box_present: bool, buttons_box_present: bool) -> Focus {
    match (axes_box_present, buttons_box_present) {
        (true, true) => current,
        (true, false) => Focus::Axes,
        (false, true) => Focus::Buttons,
        (false, false) => current,
    }
}

#[cfg(test)]
mod tests {
    use super::synced_focus;
    use crate::device::monitor::state::Focus;

    #[test]
    fn synced_focus_forces_the_remaining_visible_section() {
        assert_eq!(synced_focus(Focus::Axes, true, true), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, true, false), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, false, true), Focus::Buttons);
        assert_eq!(synced_focus(Focus::Buttons, false, false), Focus::Buttons);
    }
}
