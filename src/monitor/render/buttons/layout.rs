use ratatui::layout::Rect;

use crate::monitor::config;

#[derive(Clone, Copy)]
pub(crate) struct GridMetrics {
    pub(crate) button_width: u16,
    pub(crate) max_rows: usize,
    pub(crate) row_height: u16,
    pub(crate) top_padding: u16,
    pub(crate) col_gap: u16,
    pub(crate) compact: bool,
}

impl GridMetrics {
    pub(crate) fn for_area(area: Rect) -> Self {
        let compact = area.height < config::BUTTON_HEIGHT + config::BTN_SECTION_VERT_PADDING;
        let row_height = if compact { 1 } else { config::BUTTON_HEIGHT };
        let top_padding = if compact {
            0
        } else {
            config::BTN_SECTION_TOP_PADDING
        };
        let col_gap = if compact {
            config::COMPACT_BTN_COL_GAP
        } else {
            config::BTN_COL_GAP
        };
        let vert_padding = if compact {
            0
        } else {
            config::BTN_SECTION_VERT_PADDING
        };
        let button_width = area.width / config::buttons_per_row() as u16;
        let max_rows = (area.height.saturating_sub(vert_padding) / row_height) as usize;

        Self {
            button_width,
            max_rows,
            row_height,
            top_padding,
            col_gap,
            compact,
        }
    }

    pub(crate) fn renderable(&self) -> bool {
        self.max_rows > 0 && self.button_width > self.col_gap
    }
}

pub(super) struct ButtonLayout {
    area: Rect,
    metrics: GridMetrics,
    start_button: usize,
    visible_count: usize,
}

impl ButtonLayout {
    pub(super) fn new(area: Rect, total_buttons: usize, scroll_row_offset: usize) -> Option<Self> {
        if total_buttons == 0 {
            return None;
        }

        let metrics = GridMetrics::for_area(area);
        if !metrics.renderable() {
            return None;
        }

        let start_button = scroll_row_offset * config::buttons_per_row();
        let max_visible_buttons = metrics.max_rows * config::buttons_per_row();
        let remaining = total_buttons.saturating_sub(start_button);
        let visible_count = remaining.min(max_visible_buttons);
        if visible_count == 0 {
            return None;
        }

        Some(Self {
            area,
            metrics,
            start_button,
            visible_count,
        })
    }

    pub(super) fn visible_count(&self) -> usize {
        self.visible_count
    }

    pub(super) fn compact(&self) -> bool {
        self.metrics.compact
    }

    pub(super) fn button_index(&self, visible_index: usize) -> usize {
        self.start_button + visible_index
    }

    pub(super) fn button_area(&self, visible_index: usize) -> Rect {
        let (row, col) = grid_position(visible_index);
        Rect::new(
            self.area.x + (col as u16 * self.metrics.button_width),
            self.area.y + self.metrics.top_padding + (row as u16 * self.metrics.row_height),
            self.metrics
                .button_width
                .saturating_sub(self.metrics.col_gap),
            self.metrics.row_height,
        )
    }

    pub(super) fn separator_positions(&self) -> Vec<(u16, u16)> {
        if !self.metrics.compact || self.metrics.col_gap == 0 {
            return Vec::new();
        }

        let columns = config::buttons_per_row();
        let rows = self.visible_count.div_ceil(columns);
        let mut positions = Vec::new();

        for row in 0..rows {
            let row_start = row * columns;
            let buttons_in_row = (self.visible_count - row_start).min(columns);
            if buttons_in_row <= 1 {
                continue;
            }

            for col in 0..(buttons_in_row - 1) {
                let x = self.area.x
                    + (col as u16 * self.metrics.button_width)
                    + self
                        .metrics
                        .button_width
                        .saturating_sub(self.metrics.col_gap);
                let y =
                    self.area.y + self.metrics.top_padding + (row as u16 * self.metrics.row_height);
                if x < self.area.x + self.area.width && y < self.area.y + self.area.height {
                    positions.push((x, y));
                }
            }
        }

        positions
    }
}

fn grid_position(index: usize) -> (usize, usize) {
    (
        index / config::buttons_per_row(),
        index % config::buttons_per_row(),
    )
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{ButtonLayout, GridMetrics};
    use crate::monitor::config;

    #[test]
    fn button_layout_limits_visible_buttons_by_scroll_and_capacity() {
        let area = Rect::new(
            0,
            0,
            config::buttons_per_row() as u16 * 6,
            config::BUTTON_HEIGHT + 2,
        );
        let layout = ButtonLayout::new(area, config::buttons_per_row() * 3, 1)
            .expect("layout should be renderable");

        assert_eq!(layout.visible_count(), config::buttons_per_row());
        assert_eq!(layout.button_index(0), config::buttons_per_row());
    }

    #[test]
    fn separator_positions_skip_missing_last_column() {
        let area = Rect::new(0, 0, config::buttons_per_row() as u16 * 3, 1);
        let layout =
            ButtonLayout::new(area, config::buttons_per_row() - 1, 0).expect("compact layout");

        assert_eq!(
            layout.separator_positions().len(),
            config::buttons_per_row() - 2
        );
    }

    #[test]
    fn grid_metrics_detect_compact_single_line_layout() {
        let metrics =
            GridMetrics::for_area(Rect::new(0, 0, config::buttons_per_row() as u16 * 3, 1));

        assert!(metrics.compact);
        assert!(metrics.renderable());
        assert_eq!(metrics.max_rows, 1);
    }
}
