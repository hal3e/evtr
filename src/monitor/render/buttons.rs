mod layout;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    prelude::Stylize,
    style::Color,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::monitor::model::{DeviceInput, InputKind, InputSlice};
use crate::ui::text;

use self::layout::{ButtonLayout, GridMetrics};
use crate::monitor::config;

pub(crate) struct ButtonGrid;

impl ButtonGrid {
    pub(crate) fn metrics(area: Rect) -> GridMetrics {
        GridMetrics::for_area(area)
    }

    pub(crate) fn render_with_scroll(
        buttons: InputSlice,
        area: Rect,
        scroll_row_offset: usize,
        buf: &mut Buffer,
    ) {
        let Some(layout) = ButtonLayout::new(area, buttons.len(), scroll_row_offset) else {
            return;
        };

        for visible_index in 0..layout.visible_count() {
            let button_index = layout.button_index(visible_index);
            let Some(button) = buttons.get(button_index) else {
                break;
            };
            Self::render_button(
                layout.button_area(visible_index),
                button,
                layout.compact(),
                buf,
            );
        }

        if layout.compact() {
            Self::render_compact_separators(&layout, buf);
        }
    }

    fn render_button(area: Rect, input: &DeviceInput, compact: bool, buf: &mut Buffer) {
        let pressed = matches!(input.input_type, InputKind::Button(true));
        let available_width = if compact {
            area.width
        } else {
            area.width.saturating_sub(2)
        };
        let text = text::truncate_display_width(&input.name, available_width as usize);
        if compact {
            let mut label_style = config::style_label();
            if pressed {
                label_style = label_style.bg(config::color_button_pressed());
            }
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .style(label_style)
                .render(area, buf);
        } else {
            let bg = if pressed {
                config::color_button_pressed()
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

    fn render_compact_separators(layout: &ButtonLayout, buf: &mut Buffer) {
        let sep_style = config::style_label();
        for (x, y) in layout.separator_positions() {
            buf[(x, y)].set_symbol("|").set_style(sep_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect};

    use crate::monitor::{
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
        let inputs = build_buttons((config::buttons_per_row() * 2) + 1);
        let width = config::buttons_per_row() as u16 * 6;
        let height = config::BTN_SECTION_VERT_PADDING + (config::BUTTON_HEIGHT * 2);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&inputs, area, 1, &mut buf);

        assert!(buffer_contains(&buf, "b3"));
        assert!(buffer_contains(&buf, "b6"));
        assert!(!buffer_contains(&buf, "b0"));
    }

    #[test]
    fn render_with_scroll_offsets_by_button_row() {
        let inputs = build_buttons(config::buttons_per_row() * 3);
        let width = config::buttons_per_row() as u16 * 6;
        let height = config::BTN_SECTION_VERT_PADDING + config::BUTTON_HEIGHT;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&inputs, area, 2, &mut buf);

        assert!(buffer_contains(&buf, "b6"));
        assert!(!buffer_contains(&buf, "b3"));
        assert!(!buffer_contains(&buf, "b0"));
    }

    #[test]
    fn render_with_scroll_skips_when_too_narrow() {
        let inputs = build_buttons(config::buttons_per_row());
        let width = config::buttons_per_row() as u16;
        let height = config::BTN_SECTION_VERT_PADDING + config::BUTTON_HEIGHT;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&inputs, area, 0, &mut buf);

        assert!(buffer_is_blank(&buf));
    }

    #[test]
    fn render_compact_buttons_in_single_line() {
        let inputs = build_buttons(config::buttons_per_row());
        let width = config::buttons_per_row() as u16 * 3;
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);

        ButtonGrid::render_with_scroll(&inputs, area, 0, &mut buf);

        assert!(buffer_contains(&buf, "b0"));
    }
}
