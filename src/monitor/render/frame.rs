use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Paragraph, Widget},
};

use crate::ui::{
    popup::{help_popup, info_popup, render_popup},
    widgets,
};

use super::{
    axis::AxisRenderer, buttons::ButtonGrid, hat::HatRenderer, joystick::JoystickRenderer,
    touch::TouchRenderer,
};
use crate::monitor::{
    config,
    controls::help_lines,
    layout::main_layout,
    model::InputCollection,
    plan::RenderPlan,
    state::{ActivePopup, MonitorState},
    touch::TouchState,
    view_model::MonitorViewModel,
};

const HELP_POPUP_MIN_WIDTH: u16 = 30;
const HELP_POPUP_MIN_HEIGHT: u16 = 6;
const HELP_POPUP_MAX_WIDTH: u16 = 80;
pub(crate) struct FrameData<'a> {
    identifier: &'a str,
    state: &'a MonitorState,
    inputs: &'a InputCollection,
    touch: &'a TouchState,
    view_model: &'a MonitorViewModel,
}

impl<'a> FrameData<'a> {
    pub(crate) fn new(
        identifier: &'a str,
        state: &'a MonitorState,
        inputs: &'a InputCollection,
        touch: &'a TouchState,
        view_model: &'a MonitorViewModel,
    ) -> Self {
        Self {
            identifier,
            state,
            inputs,
            touch,
            view_model,
        }
    }
}

pub(crate) fn render_frame(area: Rect, buf: &mut Buffer, data: &FrameData<'_>, plan: &RenderPlan) {
    let [header, _] = main_layout(area);

    Paragraph::new(data.identifier)
        .style(config::style_header())
        .alignment(Alignment::Center)
        .render(header, buf);

    render_content(buf, data, plan);

    match data.state.active_popup() {
        ActivePopup::None => {}
        ActivePopup::Info => render_info_popup(area, buf, data.state),
        ActivePopup::Help => render_help_popup(area, buf),
    }
}

fn render_content(buf: &mut Buffer, data: &FrameData<'_>, plan: &RenderPlan) {
    if let Some(box_area) = plan.boxes.axes {
        widgets::render_panel_box(
            box_area,
            " Axes ",
            matches!(plan.focus, crate::monitor::state::Focus::Axes),
            buf,
        );
    }
    if let Some(box_area) = plan.boxes.joystick {
        widgets::render_unfocused_panel_box(
            box_area,
            joystick_title(data.view_model.joystick_count()),
            buf,
        );
    }
    if let Some(box_area) = plan.boxes.hat {
        widgets::render_unfocused_panel_box(box_area, " D-pad ", buf);
    }
    if let Some(box_area) = plan.boxes.buttons {
        widgets::render_panel_box(
            box_area,
            " Buttons ",
            matches!(plan.focus, crate::monitor::state::Focus::Buttons),
            buf,
        );
    }
    if let Some(box_area) = plan.boxes.touch {
        widgets::render_unfocused_panel_box(box_area, " Touchpad ", buf);
    }

    let (abs_off, rel_off) = plan.axis_offsets();
    if let Some(abs_area) = plan.areas.abs {
        AxisRenderer::render_axes_with_scroll(
            data.inputs.absolute_inputs(),
            abs_area,
            abs_off,
            buf,
        );
    }

    if let Some(rel_area) = plan.areas.rel {
        AxisRenderer::render_axes_with_scroll(
            data.inputs.relative_inputs(),
            rel_area,
            rel_off,
            buf,
        );
    }

    if let (Some(touch_area), Some((x_range, y_range))) = (plan.areas.touch, data.touch.ranges()) {
        let active_points = data.touch.active_points();
        let inactive_points = data.touch.inactive_points();
        TouchRenderer::render(
            touch_area,
            &active_points,
            &inactive_points,
            x_range,
            y_range,
            buf,
        );
    }

    if let Some(joystick_area) = plan.areas.joystick {
        JoystickRenderer::render(
            joystick_area,
            data.view_model.joystick(),
            data.state.joystick_invert_y(),
            buf,
        );
    }

    if let (Some(hat_area), Some(hat_state)) = (plan.areas.hat, data.view_model.hat_state()) {
        HatRenderer::render(hat_area, hat_state, buf);
    }

    if let Some(btn_area) = plan.areas.buttons {
        ButtonGrid::render_with_scroll(
            data.inputs.button_inputs(),
            btn_area,
            plan.scroll.button_row,
            buf,
        );
    }
}

fn render_info_popup(area: Rect, buf: &mut Buffer, state: &MonitorState) {
    let popup = info_popup(" Device Info ", 20, 5);
    render_popup(area, buf, &popup, state.info_lines());
}

fn render_help_popup(area: Rect, buf: &mut Buffer) {
    let popup = help_popup(
        " Help ",
        HELP_POPUP_MIN_WIDTH,
        HELP_POPUP_MIN_HEIGHT,
        HELP_POPUP_MAX_WIDTH,
    );
    let lines = help_lines();
    render_popup(area, buf, &popup, &lines);
}

fn joystick_title(count: usize) -> &'static str {
    if count > 1 {
        " Joysticks "
    } else {
        " Joystick "
    }
}
