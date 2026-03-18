use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    prelude::Stylize,
    style::Color,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::device::monitor::{
    config,
    model::{DeviceInput, InputKind, InputSlice},
    ui,
};

pub(crate) struct ButtonGrid;

pub(crate) struct GridMetrics {
    pub(crate) button_width: u16,
    pub(crate) max_rows: usize,
    pub(crate) row_height: u16,
    pub(crate) top_padding: u16,
    pub(crate) col_gap: u16,
    pub(crate) compact: bool,
}

impl GridMetrics {
    pub(crate) fn renderable(&self) -> bool {
        self.max_rows > 0 && self.button_width > self.col_gap
    }
}

impl ButtonGrid {
    pub(crate) fn metrics(area: Rect) -> GridMetrics {
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
        let button_width = area.width / config::BUTTONS_PER_ROW as u16;
        let max_rows = (area.height.saturating_sub(vert_padding) / row_height) as usize;
        GridMetrics {
            button_width,
            max_rows,
            row_height,
            top_padding,
            col_gap,
            compact,
        }
    }

    pub(crate) fn render_with_scroll(
        buttons: InputSlice,
        area: Rect,
        scroll_row_offset: usize,
        buf: &mut Buffer,
    ) {
        if buttons.is_empty() {
            return;
        }

        let metrics = Self::metrics(area);
        if !metrics.renderable() {
            return;
        }

        let start_button = scroll_row_offset * config::BUTTONS_PER_ROW;
        let max_visible_buttons = metrics.max_rows * config::BUTTONS_PER_ROW;
        let remaining = buttons.len().saturating_sub(start_button);
        let count = remaining.min(max_visible_buttons);
        if count == 0 {
            return;
        }

        for i in 0..count {
            let idx = start_button + i;
            let Some(button) = buttons.get(idx) else {
                break;
            };
            let (row, col) = Self::grid_position(i);
            let button_area = Self::calculate_button_area(area, row, col, &metrics);
            Self::render_button(button_area, button, metrics.compact, buf);
        }

        if metrics.compact && metrics.col_gap > 0 {
            Self::render_compact_separators(area, &metrics, count, buf);
        }
    }

    fn grid_position(index: usize) -> (usize, usize) {
        (
            index / config::BUTTONS_PER_ROW,
            index % config::BUTTONS_PER_ROW,
        )
    }

    fn calculate_button_area(area: Rect, row: usize, col: usize, metrics: &GridMetrics) -> Rect {
        Rect::new(
            area.x + (col as u16 * metrics.button_width),
            area.y + metrics.top_padding + (row as u16 * metrics.row_height),
            metrics.button_width.saturating_sub(metrics.col_gap),
            metrics.row_height,
        )
    }

    fn render_button(area: Rect, input: &DeviceInput, compact: bool, buf: &mut Buffer) {
        let pressed = matches!(input.input_type, InputKind::Button(true));
        let available_width = if compact {
            area.width
        } else {
            area.width.saturating_sub(2)
        };
        let text = ui::truncate_utf8(&input.name, available_width as usize);
        if compact {
            let mut label_style = config::style_label();
            if pressed {
                label_style = label_style.bg(config::COLOR_BUTTON_PRESSED);
            }
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(label_style)
                .render(area, buf);
        } else {
            let bg = if pressed {
                config::COLOR_BUTTON_PRESSED
            } else {
                Color::default()
            };
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(config::style_label())
                .block(Block::default().borders(Borders::ALL).bg(bg))
                .render(area, buf);
        }
    }

    fn render_compact_separators(
        area: Rect,
        metrics: &GridMetrics,
        count: usize,
        buf: &mut Buffer,
    ) {
        if metrics.col_gap == 0 {
            return;
        }
        let columns = config::BUTTONS_PER_ROW;
        let rows = count.div_ceil(columns);
        let sep_style = config::style_label();
        for row in 0..rows {
            let row_start = row * columns;
            let buttons_in_row = (count - row_start).min(columns);
            if buttons_in_row <= 1 {
                continue;
            }
            for col in 0..(buttons_in_row - 1) {
                let x = area.x
                    + (col as u16 * metrics.button_width)
                    + metrics.button_width.saturating_sub(metrics.col_gap);
                let y = area.y + metrics.top_padding + (row as u16 * metrics.row_height);
                if x >= area.x + area.width || y >= area.y + area.height {
                    continue;
                }
                buf[(x, y)].set_symbol("|").set_style(sep_style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use crate::device::monitor::{
        config,
        model::{DeviceInput, InputKind},
        render::buttons::ButtonGrid,
    };

    fn build_buttons(count: usize) -> Vec<DeviceInput> {
        (0..count)
            .map(|idx| DeviceInput {
                name: format!("b{idx}"),
                input_type: InputKind::Button(false),
            })
            .collect()
    }

    fn buffer_contains(buf: &Buffer, text: &str) -> bool {
        let area = buf.area;
        (0..area.height).any(|y| {
            let mut line = String::new();
            for x in 0..area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            line.contains(text)
        })
    }

    fn buffer_is_blank(buf: &Buffer) -> bool {
        buf.content().iter().all(|cell| cell.symbol() == " ")
    }

    #[test]
    fn render_with_scroll_shows_partial_last_row() {
        let inputs = build_buttons((config::BUTTONS_PER_ROW * 2) + 1);
        let refs: Vec<&DeviceInput> = inputs.iter().collect();
        let width = config::BUTTONS_PER_ROW as u16 * 6;
        let height = config::BTN_SECTION_VERT_PADDING + (config::BUTTON_HEIGHT * 2);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&refs, area, 1, &mut buf);

        assert!(buffer_contains(&buf, "b3"));
        assert!(buffer_contains(&buf, "b6"));
        assert!(!buffer_contains(&buf, "b0"));
    }

    #[test]
    fn render_with_scroll_offsets_by_button_row() {
        let inputs = build_buttons(config::BUTTONS_PER_ROW * 3);
        let refs: Vec<&DeviceInput> = inputs.iter().collect();
        let width = config::BUTTONS_PER_ROW as u16 * 6;
        let height = config::BTN_SECTION_VERT_PADDING + config::BUTTON_HEIGHT;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&refs, area, 2, &mut buf);

        assert!(buffer_contains(&buf, "b6"));
        assert!(!buffer_contains(&buf, "b3"));
        assert!(!buffer_contains(&buf, "b0"));
    }

    #[test]
    fn render_with_scroll_skips_when_too_narrow() {
        let inputs = build_buttons(config::BUTTONS_PER_ROW);
        let refs: Vec<&DeviceInput> = inputs.iter().collect();
        let width = config::BUTTONS_PER_ROW as u16;
        let height = config::BTN_SECTION_VERT_PADDING + config::BUTTON_HEIGHT;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&refs, area, 0, &mut buf);

        assert!(buffer_is_blank(&buf));
    }

    #[test]
    fn render_compact_buttons_in_single_line() {
        let inputs = build_buttons(config::BUTTONS_PER_ROW);
        let refs: Vec<&DeviceInput> = inputs.iter().collect();
        let width = config::BUTTONS_PER_ROW as u16 * 3;
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&refs, area, 0, &mut buf);

        assert!(buffer_contains(&buf, "b0"));
    }
}
