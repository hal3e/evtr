use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::Marker,
    widgets::{
        Widget,
        canvas::{Canvas, Points},
    },
};

use crate::monitor::config;

use super::geometry::{canvas_range, clamp_i32, invert_in_range};

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

        let (min_x, max_x) = canvas_range(x_range);
        let (min_y, max_y) = canvas_range(y_range);

        let inactive = normalize_points(inactive_points, min_x, max_x, min_y, max_y);
        let active = normalize_points(active_points, min_x, max_x, min_y, max_y);

        Canvas::default()
            .marker(Marker::HalfBlock)
            .x_bounds([f64::from(min_x), f64::from(max_x)])
            .y_bounds([f64::from(min_y), f64::from(max_y)])
            .paint(|ctx| {
                if !inactive.is_empty() {
                    ctx.draw(&Points::new(&inactive, config::color_touch_inactive()));
                }
                if !active.is_empty() {
                    ctx.draw(&Points::new(&active, config::color_touch_point()));
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
            let clamped_x = clamp_i32(*x, min_x, max_x);
            let inverted_y = invert_in_range(*y, min_y, max_y);
            (f64::from(clamped_x), f64::from(inverted_y))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_points;
    use crate::monitor::render::geometry::canvas_range;

    #[test]
    fn normalize_range_swaps_reversed_bounds() {
        assert_eq!(canvas_range((10, -5)), (-5, 10));
    }

    #[test]
    fn normalize_range_expands_zero_span() {
        assert_eq!(canvas_range((7, 7)), (7, 8));
    }

    #[test]
    fn normalize_points_clamps_and_inverts_y_coordinates() {
        let normalized = normalize_points(&[(-5, -10), (5, 10), (15, 30)], 0, 10, 0, 20);

        assert_eq!(normalized, vec![(0.0, 20.0), (5.0, 10.0), (10.0, 0.0)]);
    }
}
