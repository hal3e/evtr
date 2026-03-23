use evdev::AbsoluteAxisCode;
use ratatui::layout::Rect;

use super::{
    config,
    layout::{axes_layout, box_layout, main_layout, split_buttons_column},
    model::InputCollection,
    render::{axis::AxisRenderer, buttons::ButtonGrid, hat::HatState, joystick::JoystickState},
    state::{Focus, MonitorState},
    touch::TouchState,
};
use crate::device::widgets;

#[derive(Clone, Copy)]
pub(crate) struct Counts {
    abs: usize,
    rel: usize,
    btn: usize,
}

impl Counts {
    pub(crate) fn new(abs: usize, rel: usize, btn: usize) -> Self {
        Self { abs, rel, btn }
    }

    pub(crate) fn total_axes(&self) -> usize {
        self.abs + self.rel
    }

    fn filtered(&self, abs_visible: bool, rel_visible: bool, buttons_visible: bool) -> Self {
        Self {
            abs: if abs_visible { self.abs } else { 0 },
            rel: if rel_visible { self.rel } else { 0 },
            btn: if buttons_visible { self.btn } else { 0 },
        }
    }
}

pub(crate) struct ScrollState {
    pub(crate) axis: usize,
    pub(crate) button_row: usize,
}

pub(crate) struct ScrollBounds {
    pub(crate) axes_max: usize,
    abs_max_start: usize,
    rel_max_start: usize,
    pub(crate) button_row_max_start: usize,
    pub(crate) axes_overflow: bool,
    pub(crate) buttons_overflow: bool,
}

impl ScrollBounds {
    fn from_capacities(
        effective_counts: Counts,
        abs_visible_capacity: usize,
        rel_visible_capacity: usize,
        button_rows_capacity: usize,
    ) -> Self {
        let abs_max_start = aligned_window_start(effective_counts.abs, abs_visible_capacity, 1);
        let rel_max_start = aligned_window_start(effective_counts.rel, rel_visible_capacity, 1);
        let axes_max = abs_max_start + rel_max_start;
        let axes_overflow = (abs_visible_capacity + rel_visible_capacity) > 0
            && (effective_counts.abs > abs_visible_capacity
                || effective_counts.rel > rel_visible_capacity);

        let total_button_rows = effective_counts.btn.div_ceil(config::BUTTONS_PER_ROW);
        let button_row_max_start = if button_rows_capacity == 0 {
            0
        } else {
            total_button_rows.saturating_sub(button_rows_capacity)
        };
        let buttons_overflow = button_rows_capacity > 0 && total_button_rows > button_rows_capacity;

        Self {
            axes_max,
            abs_max_start,
            rel_max_start,
            button_row_max_start,
            axes_overflow,
            buttons_overflow,
        }
    }

    fn axis_offsets(&self, effective_counts: Counts, axis_scroll: usize) -> (usize, usize) {
        axis_offsets_for(
            axis_scroll,
            effective_counts.abs,
            effective_counts.rel,
            self.abs_max_start,
            self.rel_max_start,
        )
    }
}

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

pub(crate) struct RenderPlan {
    pub(crate) focus: Focus,
    pub(crate) scroll: ScrollState,
    pub(crate) effective_counts: Counts,
    pub(crate) scroll_bounds: ScrollBounds,
    pub(crate) boxes: PlannedBoxes,
    pub(crate) areas: PlannedAreas,
    pub(crate) joystick: JoystickState,
    pub(crate) hat_state: Option<HatState>,
}

impl RenderPlan {
    pub(crate) fn focusable(&self) -> bool {
        self.boxes.axes.is_some() && self.boxes.buttons.is_some()
    }

    pub(crate) fn axis_offsets(&self) -> (usize, usize) {
        self.scroll_bounds
            .axis_offsets(self.effective_counts, self.scroll.axis)
    }
}

pub(crate) fn build_render_plan(
    area: Rect,
    state: &MonitorState,
    inputs: &InputCollection,
    touch: &TouchState,
) -> RenderPlan {
    let counts = state.counts();
    let [_, content] = main_layout(area);
    let min_button_gap = config::BTN_COL_GAP.max(config::COMPACT_BTN_COL_GAP);
    let buttons_available = counts.btn > 0;
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
    let joystick_count = joystick.count();
    let joystick_present = joystick_count > 0;
    let hat_present = hat_state.is_some();

    let axes_available = counts.total_axes() > 0;
    let touch_enabled = touch.enabled();

    let mut main_min_width = config::MAIN_COLUMN_MIN_WIDTH;
    if axes_available {
        main_min_width = main_min_width.max(config::AXIS_MIN_WIDTH);
    }
    if touch_enabled {
        main_min_width = main_min_width.max(config::TOUCHPAD_MIN_WIDTH);
    }
    if joystick_present {
        main_min_width = main_min_width.max(config::JOYSTICK_MIN_SIZE);
    }
    if hat_present {
        main_min_width = main_min_width.max(config::HAT_MIN_SIZE);
    }

    let (main_area, buttons_column) = split_buttons_column(
        content,
        buttons_available,
        main_min_width,
        config::BUTTONS_COLUMN_MIN_WIDTH,
        min_button_gap,
    );

    let axes_present = axes_available && main_area.width >= config::AXIS_MIN_WIDTH;
    let touch_present = touch_enabled && main_area.width >= config::TOUCHPAD_MIN_WIDTH;
    let button_width = main_area.width / config::BUTTONS_PER_ROW as u16;
    let buttons_present = buttons_available && button_width > min_button_gap;

    let (layout, buttons_box) = if let Some(buttons_area) = buttons_column {
        let layout = box_layout(
            main_area,
            joystick_present,
            joystick_count,
            hat_present,
            touch_present,
            axes_present,
            false,
        );
        (layout, Some(buttons_area))
    } else {
        let layout = box_layout(
            main_area,
            joystick_present,
            joystick_count,
            hat_present,
            touch_present,
            axes_present,
            buttons_present,
        );
        let buttons_box = layout.buttons_box;
        (layout, buttons_box)
    };
    let joystick_box = layout.joystick_box;
    let hat_box = layout.hat_box;
    let axes_box = layout.axes_box;
    let touch_box = layout.touch_box;
    let focus = synced_focus(state.focus(), axes_box.is_some(), buttons_box.is_some());
    let axes_inner = axes_box.map(widgets::bordered_box_inner);
    let joystick_area = joystick_box.map(widgets::bordered_box_inner);
    let hat_area = hat_box.map(widgets::bordered_box_inner);
    let buttons_area = buttons_box.map(widgets::bordered_box_inner);
    let touch_area = touch_box.map(widgets::bordered_box_inner);
    let axes_sections = axes_inner.map(|inner| axes_layout(inner, counts.abs, counts.rel));
    let (abs_area, rel_area) = if let Some(sections) = axes_sections {
        (sections.abs_area, sections.rel_area)
    } else {
        (None, None)
    };

    let abs_visible_capacity = abs_area
        .map(|a| AxisRenderer::capacity_for(a, counts.abs))
        .unwrap_or(0);
    let rel_visible_capacity = rel_area
        .map(|a| AxisRenderer::capacity_for(a, counts.rel))
        .unwrap_or(0);

    let abs_visible = abs_visible_capacity > 0;
    let rel_visible = rel_visible_capacity > 0;

    let button_rows_capacity = buttons_area.map(button_rows_capacity).unwrap_or(0);
    let buttons_visible = button_rows_capacity > 0;

    let effective_counts = counts.filtered(abs_visible, rel_visible, buttons_visible);
    let scroll_bounds = ScrollBounds::from_capacities(
        effective_counts,
        abs_visible_capacity,
        rel_visible_capacity,
        button_rows_capacity,
    );
    let axis_scroll = if abs_visible_capacity + rel_visible_capacity == 0 {
        0
    } else {
        state.axis_scroll().min(scroll_bounds.axes_max)
    };
    let button_row_scroll = if button_rows_capacity == 0 {
        0
    } else {
        state
            .button_row_scroll()
            .min(scroll_bounds.button_row_max_start)
    };

    RenderPlan {
        focus,
        scroll: ScrollState {
            axis: axis_scroll,
            button_row: button_row_scroll,
        },
        effective_counts,
        scroll_bounds,
        boxes: PlannedBoxes {
            joystick: joystick_box,
            hat: hat_box,
            axes: axes_box,
            touch: touch_box,
            buttons: buttons_box,
        },
        areas: PlannedAreas {
            joystick: joystick_area,
            hat: hat_area,
            abs: abs_area,
            rel: rel_area,
            touch: touch_area,
            buttons: buttons_area,
        },
        joystick,
        hat_state,
    }
}

pub(crate) fn axis_offsets_for(
    axis_scroll: usize,
    abs_count: usize,
    rel_count: usize,
    abs_max_start: usize,
    rel_max_start: usize,
) -> (usize, usize) {
    let axes_scroll_max = abs_max_start + rel_max_start;
    let axis_scroll = axis_scroll.min(axes_scroll_max);

    match (abs_count > 0, rel_count > 0) {
        (true, true) => {
            if axis_scroll <= abs_max_start {
                (axis_scroll, 0)
            } else {
                (
                    abs_max_start,
                    (axis_scroll - abs_max_start).min(rel_max_start),
                )
            }
        }
        (true, false) => (axis_scroll.min(abs_max_start), 0),
        (false, true) => (0, axis_scroll.min(rel_max_start)),
        (false, false) => (0, 0),
    }
}

pub(crate) fn aligned_window_start(count: usize, capacity: usize, align: usize) -> usize {
    if capacity == 0 || count == 0 {
        return 0;
    }
    let max_start = count.saturating_sub(capacity);
    if align <= 1 {
        max_start
    } else {
        (max_start / align) * align
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

fn button_rows_capacity(btn_area: Rect) -> usize {
    let metrics = ButtonGrid::metrics(btn_area);
    if metrics.renderable() {
        metrics.max_rows
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{Counts, aligned_window_start, axis_offsets_for, synced_focus};
    use crate::device::monitor::state::Focus;

    #[test]
    fn axis_offsets_scroll_rel_after_abs() {
        assert_eq!(axis_offsets_for(6, 10, 10, 6, 6), (6, 0));
        assert_eq!(axis_offsets_for(7, 10, 10, 6, 6), (6, 1));
        assert_eq!(axis_offsets_for(12, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_clamp_in_buttons_region() {
        assert_eq!(axis_offsets_for(25, 10, 10, 6, 6), (6, 6));
    }

    #[test]
    fn axis_offsets_rel_only() {
        assert_eq!(axis_offsets_for(1, 0, 5, 0, 2), (0, 1));
        assert_eq!(axis_offsets_for(4, 0, 5, 0, 2), (0, 2));
    }

    #[test]
    fn axis_offsets_abs_present_no_scroll() {
        assert_eq!(axis_offsets_for(1, 2, 6, 0, 3), (0, 1));
        assert_eq!(axis_offsets_for(3, 2, 6, 0, 3), (0, 3));
    }

    #[test]
    fn synced_focus_forces_the_remaining_visible_section() {
        assert_eq!(synced_focus(Focus::Axes, true, true), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, true, false), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, false, true), Focus::Buttons);
        assert_eq!(synced_focus(Focus::Buttons, false, false), Focus::Buttons);
    }

    #[test]
    fn aligned_window_start_respects_alignment_step() {
        assert_eq!(aligned_window_start(10, 3, 1), 7);
        assert_eq!(aligned_window_start(10, 3, 2), 6);
        assert_eq!(aligned_window_start(2, 5, 3), 0);
    }

    #[test]
    fn counts_total_axes_tracks_absolute_and_relative_inputs() {
        assert_eq!(Counts::new(2, 3, 4).total_axes(), 5);
    }
}
