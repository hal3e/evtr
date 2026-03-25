use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Line, Points},
    },
};

use crate::monitor::{
    config,
    view_model::{JoystickState, StickState},
};

use super::geometry::{
    centered_pair, fit_centered_aspect_rect, normalize_ratio, signed_unit_point,
};

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

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use super::JoystickRenderer;
    use crate::monitor::{model::AbsoluteAxis, view_model::JoystickState};

    fn joystick_state(left: bool, right: bool) -> JoystickState {
        let stick = || {
            (
                AbsoluteAxis {
                    min: -100,
                    max: 100,
                    value: 25,
                },
                AbsoluteAxis {
                    min: -100,
                    max: 100,
                    value: -25,
                },
            )
        };

        JoystickState::from_axes(left.then(stick), right.then(stick))
    }

    fn non_blank_cells(buf: &Buffer) -> usize {
        buf.content()
            .iter()
            .filter(|cell| !cell.symbol().trim().is_empty())
            .count()
    }

    #[test]
    fn render_with_no_sticks_leaves_the_buffer_unchanged() {
        let area = Rect::new(0, 0, 16, 8);
        let mut buf = Buffer::empty(area);

        JoystickRenderer::render(area, &joystick_state(false, false), true, &mut buf);

        assert_eq!(non_blank_cells(&buf), 0);
    }

    #[test]
    fn render_with_one_stick_writes_to_the_buffer() {
        let area = Rect::new(0, 0, 16, 8);
        let mut buf = Buffer::empty(area);

        JoystickRenderer::render(area, &joystick_state(true, false), true, &mut buf);

        assert!(non_blank_cells(&buf) > 0);
    }

    #[test]
    fn render_with_two_sticks_writes_to_the_buffer() {
        let area = Rect::new(0, 0, 24, 8);
        let mut buf = Buffer::empty(area);

        JoystickRenderer::render(area, &joystick_state(true, true), true, &mut buf);

        assert!(non_blank_cells(&buf) > 0);
    }
}
