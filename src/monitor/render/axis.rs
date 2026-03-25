use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Gauge, Paragraph, Widget},
};

use crate::monitor::{
    config,
    model::{DeviceInput, InputSlice},
    ui,
};
use crate::ui::text;

pub(crate) struct AxisRenderer;

impl AxisRenderer {
    pub(crate) fn bar_height_for(available_height: u16, num_items: usize) -> u16 {
        if num_items == 0 {
            return 1;
        }
        for &height in &config::BAR_HEIGHTS {
            let total_needed = (height + config::AXIS_GAP) * num_items as u16;
            if total_needed <= available_height {
                return height;
            }
        }
        1
    }

    pub(crate) fn capacity_for(area: Rect, count: usize) -> usize {
        if count == 0 || area.height == 0 || area.width < config::AXIS_MIN_WIDTH {
            return 0;
        }
        let bar_height = Self::bar_height_for(area.height, count);
        let item_height = bar_height + config::AXIS_GAP;
        ((area.height / item_height) as usize).min(count)
    }
    fn split_label_gauge(area: Rect) -> (Rect, Rect) {
        let left_pad = config::AXIS_LEFT_PADDING.min(area.width);
        let padded_width = area.width.saturating_sub(left_pad);
        let label_width = config::AXIS_LABEL_MAX.min(padded_width / 3);
        let gauge_width = padded_width.saturating_sub(label_width + config::LABEL_GAUGE_GAP);
        let padded_area = Rect::new(area.x + left_pad, area.y, padded_width, area.height);
        let [label_area, gauge_area] = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Length(gauge_width),
        ])
        .areas(padded_area);

        let label_y = if area.height > 1 {
            label_area.y + (area.height / 2)
        } else {
            label_area.y
        };

        let label_rect = Rect::new(label_area.x, label_y, label_area.width, 1);
        (label_rect, gauge_area)
    }

    pub(crate) fn render_axes_with_scroll(
        inputs: InputSlice,
        area: Rect,
        scroll_offset: usize,
        buf: &mut Buffer,
    ) {
        if inputs.is_empty() || area.height == 0 {
            return;
        }

        let num_items = inputs.len();
        let bar_height = Self::bar_height_for(area.height, num_items);
        let item_height = bar_height + config::AXIS_GAP;

        let max_visible = (area.height / item_height) as usize;
        let (start, count) = ui::visible_window(num_items, scroll_offset, max_visible);
        if let Some(window) = inputs.get(start..start + count) {
            for (i, input) in window.iter().enumerate() {
                let y = area.y + (i as u16 * item_height);
                if y + bar_height > area.y + area.height {
                    break;
                }

                let item_area = Rect::new(area.x, y, area.width, bar_height);
                Self::render_axis_item(input, item_area, buf);
            }
        }
    }

    fn render_axis_item(input: &DeviceInput, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < config::AXIS_MIN_WIDTH {
            return;
        }

        let (label_rect, gauge_area) = Self::split_label_gauge(area);
        if gauge_area.width == 0 {
            return;
        }
        let truncated_name = text::truncate_display_width(&input.name, label_rect.width as usize);

        Paragraph::new(truncated_name)
            .style(config::style_label())
            .alignment(Alignment::Left)
            .render(label_rect, buf);

        let value_str = input.input_type.display_label();
        let ratio = input.input_type.normalized();

        Gauge::default()
            .gauge_style(config::style_gauge())
            .ratio(ratio)
            .label(value_str)
            .render(gauge_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use super::AxisRenderer;
    use crate::monitor::{
        config,
        model::{AbsoluteState, DeviceInput, InputKind},
    };

    fn absolute_input(name: &str, value: i32) -> DeviceInput {
        DeviceInput {
            name: name.to_string(),
            input_type: InputKind::Absolute(AbsoluteState::kernel(-100, 100, value)),
        }
    }

    fn non_blank_cells(buf: &Buffer) -> usize {
        buf.content()
            .iter()
            .filter(|cell| !cell.symbol().trim().is_empty())
            .count()
    }

    #[test]
    fn bar_height_for_falls_back_to_one_when_rows_do_not_fit() {
        assert_eq!(AxisRenderer::bar_height_for(1, 10), 1);
    }

    #[test]
    fn capacity_for_returns_zero_for_too_narrow_or_empty_areas() {
        assert_eq!(AxisRenderer::capacity_for(Rect::new(0, 0, 0, 5), 3), 0);
        assert_eq!(
            AxisRenderer::capacity_for(Rect::new(0, 0, config::AXIS_MIN_WIDTH - 1, 5), 3),
            0
        );
        assert_eq!(
            AxisRenderer::capacity_for(Rect::new(0, 0, config::AXIS_MIN_WIDTH, 0), 3),
            0
        );
    }

    #[test]
    fn split_label_gauge_handles_very_narrow_area() {
        let (label, gauge) =
            AxisRenderer::split_label_gauge(Rect::new(0, 0, config::AXIS_MIN_WIDTH, 1));

        assert!(label.width <= config::AXIS_LABEL_MAX);
        assert!(gauge.width > 0);
        assert_eq!(label.height, 1);
    }

    #[test]
    fn render_axes_with_scroll_is_a_no_op_for_empty_inputs() {
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);

        AxisRenderer::render_axes_with_scroll(&[], area, 0, &mut buf);

        assert_eq!(non_blank_cells(&buf), 0);
    }

    #[test]
    fn render_axes_with_scroll_is_a_no_op_when_capacity_is_zero() {
        let area = Rect::new(0, 0, config::AXIS_MIN_WIDTH - 1, 3);
        let mut buf = Buffer::empty(area);
        let inputs = [absolute_input("abs_x", 25)];

        AxisRenderer::render_axes_with_scroll(&inputs, area, 0, &mut buf);

        assert_eq!(non_blank_cells(&buf), 0);
    }

    #[test]
    fn render_axis_item_is_a_no_op_when_gauge_width_would_be_zero() {
        let area = Rect::new(0, 0, config::AXIS_MIN_WIDTH - 1, 1);
        let mut buf = Buffer::empty(area);

        AxisRenderer::render_axis_item(&absolute_input("abs_x", 25), area, &mut buf);

        assert_eq!(non_blank_cells(&buf), 0);
    }
}
