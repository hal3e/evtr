use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub(crate) struct Popup<'a> {
    pub(crate) title: &'a str,
    pub(crate) lines: &'a [String],
    pub(crate) min_width: u16,
    pub(crate) min_height: u16,
    pub(crate) max_width: Option<u16>,
    pub(crate) max_height: Option<u16>,
    pub(crate) text_style: Style,
    pub(crate) border_style: Style,
    pub(crate) text_alignment: Alignment,
    pub(crate) title_alignment: Alignment,
    pub(crate) wrap: Wrap,
}

pub(crate) fn render_popup(area: Rect, buf: &mut Buffer, popup: &Popup<'_>) {
    if area.width < popup.min_width || area.height < popup.min_height {
        return;
    }

    let max_width = popup.max_width.unwrap_or(area.width).min(area.width);
    let max_height = popup.max_height.unwrap_or(area.height).min(area.height);
    if max_width == 0 || max_height == 0 {
        return;
    }

    let max_line = popup
        .lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16;
    let desired_width = max_line.saturating_add(2).clamp(popup.min_width, max_width);
    let text_width = desired_width.saturating_sub(2).max(1) as usize;
    let wrapped_lines: usize = popup
        .lines
        .iter()
        .map(|line| {
            let len = line.chars().count();
            if len == 0 {
                1
            } else {
                len.div_ceil(text_width)
            }
        })
        .sum();
    let desired_height = (wrapped_lines as u16 + 2).clamp(popup.min_height, max_height);

    let x = area.x + (area.width.saturating_sub(desired_width)) / 2;
    let y = area.y + (area.height.saturating_sub(desired_height)) / 2;
    let popup_area = Rect::new(x, y, desired_width, desired_height);

    Clear.render(popup_area, buf);

    let text = popup.lines.join("\n");
    Paragraph::new(text)
        .style(popup.text_style)
        .alignment(popup.text_alignment)
        .wrap(popup.wrap)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(popup.title)
                .title_alignment(popup.title_alignment)
                .border_style(popup.border_style),
        )
        .render(popup_area, buf);
}
