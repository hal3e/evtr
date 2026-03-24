use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Line, Points},
    },
};

use crate::device::monitor::{config, model::AbsoluteAxis};

use super::geometry::{
    centered_pair, fit_centered_aspect_rect, normalize_ratio, signed_unit_point,
};

#[derive(Clone, Copy)]
pub(crate) struct StickState {
    pub(crate) x: AbsoluteAxis,
    pub(crate) y: AbsoluteAxis,
}

#[derive(Default)]
pub(crate) struct JoystickState {
    pub(crate) left: Option<StickState>,
    pub(crate) right: Option<StickState>,
}

impl JoystickState {
    pub(crate) fn from_axes(
        left: Option<(AbsoluteAxis, AbsoluteAxis)>,
        right: Option<(AbsoluteAxis, AbsoluteAxis)>,
    ) -> Self {
        Self {
            left: left.map(|(x, y)| StickState { x, y }),
            right: right.map(|(x, y)| StickState { x, y }),
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.left.is_some() as usize + self.right.is_some() as usize
    }
}

pub(crate) struct JoystickRenderer;

impl JoystickRenderer {
    pub(crate) fn render(area: Rect, state: &JoystickState, invert_y: bool, buf: &mut Buffer) {
        match (state.left.as_ref(), state.right.as_ref()) {
            (None, None) => {}
            (Some(stick), None) | (None, Some(stick)) => {
                Self::render_stick(area, stick, invert_y, buf);
            }
            (Some(left), Some(right)) => {
                let (left_area, right_area) = centered_pair(area, config::JOYSTICK_GAP);
                Self::render_stick(left_area, left, invert_y, buf);
                Self::render_stick(right_area, right, invert_y, buf);
            }
        }
    }

    fn render_stick(area: Rect, stick: &StickState, invert_y: bool, buf: &mut Buffer) {
        let Some(square) = fit_centered_aspect_rect(area, config::JOYSTICK_ASPECT_RATIO) else {
            return;
        };

        let x_norm = normalize_ratio(stick.x.value, stick.x.min, stick.x.max);
        let y_norm = normalize_ratio(stick.y.value, stick.y.min, stick.y.max);
        let point = signed_unit_point(x_norm, y_norm, invert_y);

        Canvas::default()
            .marker(Marker::HalfBlock)
            .x_bounds([-1.0, 1.0])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                let axis_color = config::COLOR_TOUCH_INACTIVE;
                ctx.draw(&Line::new(-1.0, 0.0, 1.0, 0.0, axis_color));
                ctx.draw(&Line::new(0.0, -1.0, 0.0, 1.0, axis_color));
                ctx.draw(&Points::new(&[point], config::COLOR_TOUCH_POINT));
            })
            .render(square, buf);
    }
}
