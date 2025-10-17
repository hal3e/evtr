use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Gauge, Paragraph, Widget},
};

use crate::device::monitor::{
    config,
    model::{DeviceInput, InputSlice},
    ui,
};

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
        let label_width = config::AXIS_LABEL_MAX.min(area.width / 3);
        let gauge_width = area
            .width
            .saturating_sub(label_width + config::LABEL_GAUGE_GAP);
        let [label_area, gauge_area] = Layout::horizontal([
            Constraint::Length(label_width),
            Constraint::Length(gauge_width),
        ])
        .areas(area);

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
        for (i, input) in inputs[start..start + count].iter().enumerate() {
            let y = area.y + (i as u16 * item_height);
            if y + bar_height > area.y + area.height {
                break;
            }

            let item_area = Rect::new(area.x, y, area.width, bar_height);
            Self::render_axis_item(input, item_area, buf);
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
        let truncated_name = ui::truncate_utf8(&input.name, label_rect.width as usize);

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
