use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Stylize},
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
}

impl GridMetrics {
    pub(crate) fn renderable(&self) -> bool {
        self.max_rows > 0 && self.button_width > config::BTN_COL_GAP
    }
}

impl ButtonGrid {
    pub(crate) fn metrics(area: Rect) -> GridMetrics {
        let button_width = area.width / config::BUTTONS_PER_ROW as u16;
        let max_rows = ((area.height.saturating_sub(config::BTN_SECTION_VERT_PADDING))
            / config::BUTTON_HEIGHT) as usize;
        GridMetrics {
            button_width,
            max_rows,
        }
    }

    pub(crate) fn render_with_scroll(
        buttons: InputSlice,
        area: Rect,
        scroll_row_offset: usize,
        buf: &mut Buffer,
    ) {
        if buttons.is_empty() || area.height < config::BUTTON_HEIGHT {
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
            let button_area = Self::calculate_button_area(area, row, col, metrics.button_width);
            Self::render_button(button_area, button, buf);
        }
    }

    fn grid_position(index: usize) -> (usize, usize) {
        (
            index / config::BUTTONS_PER_ROW,
            index % config::BUTTONS_PER_ROW,
        )
    }

    fn calculate_button_area(area: Rect, row: usize, col: usize, button_width: u16) -> Rect {
        Rect::new(
            area.x + (col as u16 * button_width),
            area.y + config::BTN_SECTION_TOP_PADDING + (row as u16 * config::BUTTON_HEIGHT),
            button_width.saturating_sub(config::BTN_COL_GAP),
            config::BUTTON_HEIGHT,
        )
    }

    fn render_button(area: Rect, input: &DeviceInput, buf: &mut Buffer) {
        let pressed = matches!(input.input_type, InputKind::Button(true));

        let block = Block::default().borders(Borders::ALL).bg(if pressed {
            config::COLOR_BUTTON_PRESSED
        } else {
            Color::default()
        });

        let text = ui::truncate_utf8(&input.name, area.width.saturating_sub(2) as usize);

        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .style(config::style_label())
            .render(area, buf);
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
        let inputs = build_buttons(config::BUTTONS_PER_ROW + 1);
        let refs: Vec<&DeviceInput> = inputs.iter().collect();
        let width = config::BUTTONS_PER_ROW as u16 * 6;
        let height = config::BTN_SECTION_VERT_PADDING + config::BUTTON_HEIGHT;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&refs, area, 1, &mut buf);

        assert!(buffer_contains(&buf, "b6"));
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
}
