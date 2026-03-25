use ratatui::layout::Rect;

use crate::ui::widgets;

use super::Counts;
use crate::monitor::{
    config,
    layout::{BoxRequest, axes_layout, box_layout, split_buttons_column},
    state::Focus,
    view_model::MonitorViewModel,
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

pub(super) fn plan_areas(
    content: Rect,
    counts: Counts,
    min_button_gap: u16,
    current_focus: Focus,
    view_model: &MonitorViewModel,
) -> AreaPlan {
    let (main_area, buttons_column) = split_buttons_column(
        content,
        view_model.buttons_available(),
        view_model.main_min_width(),
        config::BUTTONS_COLUMN_MIN_WIDTH,
        min_button_gap,
    );

    let axes_present = view_model.axes_available() && main_area.width >= config::AXIS_MIN_WIDTH;
    let touch_present = view_model.touch_enabled() && main_area.width >= config::TOUCHPAD_MIN_WIDTH;
    let button_width = main_area.width / config::BUTTONS_PER_ROW as u16;
    let buttons_present = view_model.buttons_available() && button_width > min_button_gap;

    let (layout, buttons_box) = if let Some(buttons_area) = buttons_column {
        let layout = box_layout(
            main_area,
            BoxRequest::new(
                view_model.joystick_present(),
                view_model.joystick_count(),
                view_model.hat_present(),
                touch_present,
                axes_present,
                false,
            ),
        );
        (layout, Some(buttons_area))
    } else {
        let layout = box_layout(
            main_area,
            BoxRequest::new(
                view_model.joystick_present(),
                view_model.joystick_count(),
                view_model.hat_present(),
                touch_present,
                axes_present,
                buttons_present,
            ),
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
    use crate::monitor::state::Focus;

    #[test]
    fn synced_focus_forces_the_remaining_visible_section() {
        assert_eq!(synced_focus(Focus::Axes, true, true), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, true, false), Focus::Axes);
        assert_eq!(synced_focus(Focus::Axes, false, true), Focus::Buttons);
        assert_eq!(synced_focus(Focus::Buttons, false, false), Focus::Buttons);
    }
}
