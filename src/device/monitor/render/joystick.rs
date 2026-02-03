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
        if area.width < 2 || area.height < 2 {
            return;
        }

        match (state.left.as_ref(), state.right.as_ref()) {
            (None, None) => {}
            (Some(stick), None) | (None, Some(stick)) => {
                Self::render_stick(area, stick, invert_y, buf);
            }
            (Some(left), Some(right)) => {
                let (left_area, right_area) = split_two(area);
                Self::render_stick(left_area, left, invert_y, buf);
                Self::render_stick(right_area, right, invert_y, buf);
            }
        }
    }

    fn render_stick(area: Rect, stick: &StickState, invert_y: bool, buf: &mut Buffer) {
        let ratio = config::JOYSTICK_ASPECT_RATIO.max(1);
        let max_width = area.width;
        let max_height = area.height;
        let height = max_height.min(max_width.saturating_div(ratio));
        let width = height.saturating_mul(ratio);
        if width < 2 || height < 2 {
            return;
        }

        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let square = Rect::new(x, y, width, height);

        let x_norm = normalize_axis(&stick.x);
        let y_norm = normalize_axis(&stick.y);
        let y_pos = if invert_y {
            1.0 - (y_norm * 2.0)
        } else {
            y_norm * 2.0 - 1.0
        };
        let point = (x_norm * 2.0 - 1.0, y_pos);

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

fn split_two(area: Rect) -> (Rect, Rect) {
    let gap = if area.width > config::JOYSTICK_GAP * 2 {
        config::JOYSTICK_GAP
    } else {
        0
    };
    let width_each = area.width.saturating_sub(gap) / 2;
    let total = width_each * 2 + gap;
    let start_x = area.x + (area.width.saturating_sub(total)) / 2;
    let left = Rect::new(start_x, area.y, width_each, area.height);
    let right = Rect::new(start_x + width_each + gap, area.y, width_each, area.height);
    (left, right)
}

fn normalize_axis(axis: &AbsoluteAxis) -> f64 {
    let (min, max) = if axis.min <= axis.max {
        (axis.min, axis.max)
    } else {
        (axis.max, axis.min)
    };
    let range = (max - min) as f64;
    if range == 0.0 {
        0.5
    } else {
        ((axis.value - min) as f64 / range).clamp(0.0, 1.0)
    }
}
