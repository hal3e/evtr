use ratatui::layout::Rect;

pub(super) fn fit_centered_aspect_rect(area: Rect, aspect_ratio: u16) -> Option<Rect> {
    if area.width < 2 || area.height < 2 {
        return None;
    }

    let aspect_ratio = aspect_ratio.max(1);
    let height = area.height.min(area.width.saturating_div(aspect_ratio));
    let width = height.saturating_mul(aspect_ratio);
    if width < 2 || height < 2 {
        return None;
    }

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Some(Rect::new(x, y, width, height))
}

pub(super) fn centered_pair(area: Rect, preferred_gap: u16) -> (Rect, Rect) {
    let gap = gap_if_room(area.width, preferred_gap);
    let width_each = area.width.saturating_sub(gap) / 2;
    let total = width_each * 2 + gap;
    let start_x = area.x + (area.width.saturating_sub(total)) / 2;
    let left = Rect::new(start_x, area.y, width_each, area.height);
    let right = Rect::new(start_x + width_each + gap, area.y, width_each, area.height);
    (left, right)
}

pub(super) fn inset_rect(rect: Rect, padding: u16) -> Rect {
    if rect.width <= padding * 2 || rect.height <= padding * 2 {
        return rect;
    }

    Rect::new(
        rect.x + padding,
        rect.y + padding,
        rect.width.saturating_sub(padding * 2),
        rect.height.saturating_sub(padding * 2),
    )
}

pub(super) fn normalize_ratio(value: i32, min: i32, max: i32) -> f64 {
    let (min, max) = ordered_range((min, max));
    let range = (max - min) as f64;
    if range == 0.0 {
        0.5
    } else {
        ((clamp_i32(value, min, max) - min) as f64 / range).clamp(0.0, 1.0)
    }
}

pub(super) fn signed_unit_point(x_ratio: f64, y_ratio: f64, invert_y: bool) -> (f64, f64) {
    let y = if invert_y {
        1.0 - (y_ratio * 2.0)
    } else {
        y_ratio * 2.0 - 1.0
    };
    (x_ratio * 2.0 - 1.0, y)
}

pub(super) fn canvas_range(range: (i32, i32)) -> (i32, i32) {
    let (min, mut max) = ordered_range(range);
    if min == max {
        max = min.saturating_add(1);
    }
    (min, max)
}

pub(super) fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub(super) fn invert_in_range(value: i32, min: i32, max: i32) -> i32 {
    max - (clamp_i32(value, min, max) - min)
}

pub(super) fn coord_from_index(index: usize, total: usize) -> f64 {
    if total <= 1 {
        0.0
    } else {
        -1.0 + (index as f64) * 2.0 / (total as f64 - 1.0)
    }
}

fn gap_if_room(total_width: u16, preferred_gap: u16) -> u16 {
    if total_width > preferred_gap.saturating_mul(2) {
        preferred_gap
    } else {
        0
    }
}

fn ordered_range(range: (i32, i32)) -> (i32, i32) {
    let (mut min, mut max) = range;
    if min > max {
        std::mem::swap(&mut min, &mut max);
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{
        canvas_range, centered_pair, coord_from_index, fit_centered_aspect_rect, inset_rect,
        invert_in_range, normalize_ratio, signed_unit_point,
    };

    #[test]
    fn fit_centered_aspect_rect_centers_the_fitted_shape() {
        let rect = fit_centered_aspect_rect(Rect::new(2, 3, 11, 12), 2).unwrap();

        assert_eq!(rect, Rect::new(2, 6, 10, 5));
    }

    #[test]
    fn centered_pair_drops_the_gap_when_width_is_tight() {
        let (left, right) = centered_pair(Rect::new(0, 0, 4, 6), 3);

        assert_eq!(left, Rect::new(0, 0, 2, 6));
        assert_eq!(right, Rect::new(2, 0, 2, 6));
    }

    #[test]
    fn inset_rect_preserves_rect_when_padding_would_collapse_it() {
        let rect = inset_rect(Rect::new(1, 2, 3, 3), 2);

        assert_eq!(rect, Rect::new(1, 2, 3, 3));
    }

    #[test]
    fn normalize_ratio_clamps_and_handles_zero_span() {
        assert_eq!(normalize_ratio(7, 7, 7), 0.5);
        assert_eq!(normalize_ratio(-5, 0, 10), 0.0);
        assert_eq!(normalize_ratio(15, 0, 10), 1.0);
    }

    #[test]
    fn signed_unit_point_flips_y_when_requested() {
        assert_eq!(signed_unit_point(0.75, 0.25, false), (0.5, -0.5));
        assert_eq!(signed_unit_point(0.75, 0.25, true), (0.5, 0.5));
    }

    #[test]
    fn canvas_range_and_invert_in_range_normalize_touch_coordinates() {
        let (min, max) = canvas_range((20, 20));

        assert_eq!((min, max), (20, 21));
        assert_eq!(invert_in_range(-5, 0, 20), 20);
        assert_eq!(invert_in_range(10, 0, 20), 10);
        assert_eq!(invert_in_range(30, 0, 20), 0);
    }

    #[test]
    fn coord_from_index_spans_canvas_extents() {
        assert_eq!(coord_from_index(0, 5), -1.0);
        assert_eq!(coord_from_index(2, 5), 0.0);
        assert_eq!(coord_from_index(4, 5), 1.0);
    }
}
