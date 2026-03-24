use ratatui::layout::Rect;

use super::{Counts, areas::PlannedAreas};
use crate::device::monitor::{
    config,
    render::{axis::AxisRenderer, buttons::ButtonGrid},
    state::MonitorState,
};

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
    pub(super) fn from_capacities(
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

    pub(super) fn axis_offsets(
        &self,
        effective_counts: Counts,
        axis_scroll: usize,
    ) -> (usize, usize) {
        axis_offsets_for(
            axis_scroll,
            effective_counts.abs,
            effective_counts.rel,
            self.abs_max_start,
            self.rel_max_start,
        )
    }
}

pub(super) struct VisibleCapacities {
    pub(super) abs: usize,
    pub(super) rel: usize,
    pub(super) button_rows: usize,
}

impl VisibleCapacities {
    pub(super) fn from_areas(counts: Counts, areas: &PlannedAreas) -> Self {
        Self {
            abs: areas
                .abs
                .map(|area| AxisRenderer::capacity_for(area, counts.abs))
                .unwrap_or(0),
            rel: areas
                .rel
                .map(|area| AxisRenderer::capacity_for(area, counts.rel))
                .unwrap_or(0),
            button_rows: areas.buttons.map(button_rows_capacity).unwrap_or(0),
        }
    }
}

pub(super) fn clamp_scroll_state(
    state: &MonitorState,
    scroll_bounds: &ScrollBounds,
    capacities: &VisibleCapacities,
) -> ScrollState {
    let axis = if capacities.abs + capacities.rel == 0 {
        0
    } else {
        state.axis_scroll().min(scroll_bounds.axes_max)
    };
    let button_row = if capacities.button_rows == 0 {
        0
    } else {
        state
            .button_row_scroll()
            .min(scroll_bounds.button_row_max_start)
    };

    ScrollState { axis, button_row }
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
    use super::{ScrollBounds, aligned_window_start, axis_offsets_for};
    use crate::device::monitor::plan::Counts;

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
    fn aligned_window_start_respects_alignment_step() {
        assert_eq!(aligned_window_start(10, 3, 1), 7);
        assert_eq!(aligned_window_start(10, 3, 2), 6);
        assert_eq!(aligned_window_start(2, 5, 3), 0);
    }

    #[test]
    fn scroll_bounds_compute_button_row_limits() {
        let bounds = ScrollBounds::from_capacities(Counts::new(0, 0, 17), 0, 0, 2);

        assert_eq!(bounds.button_row_max_start, 4);
        assert!(bounds.buttons_overflow);
    }
}
