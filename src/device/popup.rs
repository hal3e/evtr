use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use super::text;

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
    let Some(popup_area) = popup_area(area, popup) else {
        return;
    };

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

fn popup_area(area: Rect, popup: &Popup<'_>) -> Option<Rect> {
    if area.width < popup.min_width || area.height < popup.min_height {
        return None;
    }

    let max_width = popup.max_width.unwrap_or(area.width).min(area.width);
    let max_height = popup.max_height.unwrap_or(area.height).min(area.height);
    if max_width == 0 || max_height == 0 {
        return None;
    }

    let desired_width = popup_width(popup, max_width);
    let desired_height = popup_height(popup, desired_width, max_height);
    let x = area.x + (area.width.saturating_sub(desired_width)) / 2;
    let y = area.y + (area.height.saturating_sub(desired_height)) / 2;
    Some(Rect::new(x, y, desired_width, desired_height))
}

fn popup_width(popup: &Popup<'_>, max_width: u16) -> u16 {
    let max_line = popup
        .lines
        .iter()
        .map(|line| text::display_width(line))
        .max()
        .unwrap_or(0) as u16;
    max_line.saturating_add(2).clamp(popup.min_width, max_width)
}

fn popup_height(popup: &Popup<'_>, width: u16, max_height: u16) -> u16 {
    let text_width = width.saturating_sub(2).max(1) as usize;
    let wrapped_lines: usize = popup
        .lines
        .iter()
        .map(|line| {
            let line_width = text::display_width(line);
            line_width.max(1).div_ceil(text_width)
        })
        .sum();
    (wrapped_lines as u16 + 2).clamp(popup.min_height, max_height)
}

#[cfg(test)]
mod tests {
    use ratatui::{
        layout::{Alignment, Rect},
        style::Style,
        widgets::Wrap,
    };

    use super::{Popup, popup_area, popup_height, popup_width};

    fn popup<'a>(lines: &'a [String]) -> Popup<'a> {
        Popup {
            title: "Info",
            lines,
            min_width: 3,
            min_height: 3,
            max_width: None,
            max_height: None,
            text_style: Style::default(),
            border_style: Style::default(),
            text_alignment: Alignment::Left,
            title_alignment: Alignment::Left,
            wrap: Wrap { trim: false },
        }
    }

    #[test]
    fn popup_width_uses_display_width() {
        let lines = [String::from("界界界")];
        let popup = popup(&lines);

        assert_eq!(popup_width(&popup, 20), 8);
    }

    #[test]
    fn popup_height_wraps_by_display_width() {
        let lines = [String::from("界界界")];
        let popup = popup(&lines);

        assert_eq!(popup_height(&popup, 6, 20), 4);
    }

    #[test]
    fn popup_area_centers_measured_size() {
        let lines = [String::from("abc")];
        let popup = popup(&lines);

        assert_eq!(
            popup_area(Rect::new(0, 0, 9, 7), &popup),
            Some(Rect::new(2, 2, 5, 3))
        );
    }
}
