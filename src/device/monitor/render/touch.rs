use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Points},
    },
};

use crate::device::monitor::config;

pub(crate) struct TouchRenderer;

impl TouchRenderer {
    pub(crate) fn render(
        area: Rect,
        active_points: &[(i32, i32)],
        inactive_points: &[(i32, i32)],
        x_range: (i32, i32),
        y_range: (i32, i32),
        buf: &mut Buffer,
    ) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let (min_x, max_x) = normalize_range(x_range);
        let (min_y, max_y) = normalize_range(y_range);

        let inactive = normalize_points(inactive_points, min_x, max_x, min_y, max_y);
        let active = normalize_points(active_points, min_x, max_x, min_y, max_y);

        Canvas::default()
            .marker(Marker::HalfBlock)
            .x_bounds([f64::from(min_x), f64::from(max_x)])
            .y_bounds([f64::from(min_y), f64::from(max_y)])
            .paint(|ctx| {
                if !inactive.is_empty() {
                    ctx.draw(&Points::new(&inactive, config::COLOR_TOUCH_INACTIVE));
                }
                if !active.is_empty() {
                    ctx.draw(&Points::new(&active, config::COLOR_TOUCH_POINT));
                }
            })
            .render(area, buf);
    }
}

fn normalize_points(
    points: &[(i32, i32)],
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
) -> Vec<(f64, f64)> {
    points
        .iter()
        .map(|(x, y)| {
            let clamped_x = clamp(*x, min_x, max_x);
            let clamped_y = clamp(*y, min_y, max_y);
            let inverted_y = max_y - (clamped_y - min_y);
            (f64::from(clamped_x), f64::from(inverted_y))
        })
        .collect()
}

fn normalize_range(range: (i32, i32)) -> (i32, i32) {
    let (mut min, mut max) = range;
    if min > max {
        std::mem::swap(&mut min, &mut max);
    }
    if min == max {
        max = min.saturating_add(1);
    }
    (min, max)
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
