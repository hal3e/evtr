use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Stylize},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::device::monitor::{config, ui, model::{DeviceInput, InputSlice, InputKind}};

pub(crate) struct ButtonGrid;

pub(crate) struct GridMetrics {
    pub(crate) button_width: u16,
    pub(crate) max_rows: usize,
}

impl ButtonGrid {
    fn metrics(area: Rect) -> GridMetrics {
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
        if metrics.max_rows == 0 {
            return;
        }

        let start_button = scroll_row_offset * config::BUTTONS_PER_ROW;
        let max_visible_buttons = metrics.max_rows * config::BUTTONS_PER_ROW;
        let (start, count) = ui::visible_window(buttons.len(), start_button, max_visible_buttons);

        for i in 0..count {
            let idx = start + i;
            let (row, col) = Self::grid_position(i);
            let button_area = Self::calculate_button_area(area, row, col, metrics.button_width);
            Self::render_button(button_area, buttons[idx], buf);
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
