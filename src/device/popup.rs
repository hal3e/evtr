use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Clear, Paragraph, Widget, Wrap},
};

use super::{text, widgets};

pub(crate) struct Popup<'a> {
    pub(crate) title: &'a str,
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

impl<'a> Popup<'a> {
    pub(crate) fn new(title: &'a str) -> Self {
        Self {
            title,
            min_width: 0,
            min_height: 0,
            max_width: None,
            max_height: None,
            text_style: Style::default(),
            border_style: Style::default(),
            text_alignment: Alignment::Left,
            title_alignment: Alignment::Left,
            wrap: Wrap { trim: false },
        }
    }

    pub(crate) fn min_size(mut self, width: u16, height: u16) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    pub(crate) fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }

    pub(crate) fn text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    pub(crate) fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub(crate) fn text_alignment(mut self, alignment: Alignment) -> Self {
        self.text_alignment = alignment;
        self
    }

    pub(crate) fn title_alignment(mut self, alignment: Alignment) -> Self {
        self.title_alignment = alignment;
        self
    }

    pub(crate) fn wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
        self
    }
}

pub(crate) fn render_popup<S: AsRef<str>>(
    area: Rect,
    buf: &mut Buffer,
    popup: &Popup<'_>,
    lines: &[S],
) {
    let Some(popup_area) = popup_area(area, popup, lines) else {
        return;
    };

    Clear.render(popup_area, buf);

    let text = lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("\n");
    Paragraph::new(text)
        .style(popup.text_style)
        .alignment(popup.text_alignment)
        .wrap(popup.wrap)
        .block(widgets::bordered_titled_block(
            popup.title,
            popup.border_style,
            popup.title_alignment,
        ))
        .render(popup_area, buf);
}

fn popup_area<S: AsRef<str>>(area: Rect, popup: &Popup<'_>, lines: &[S]) -> Option<Rect> {
    if area.width < popup.min_width || area.height < popup.min_height {
        return None;
    }

    let max_width = popup.max_width.unwrap_or(area.width).min(area.width);
    let max_height = popup.max_height.unwrap_or(area.height).min(area.height);
    if max_width == 0 || max_height == 0 {
        return None;
    }

    let desired_width = popup_width(popup, lines, max_width);
    let desired_height = popup_height(popup, lines, desired_width, max_height);
    let x = area.x + (area.width.saturating_sub(desired_width)) / 2;
    let y = area.y + (area.height.saturating_sub(desired_height)) / 2;
    Some(Rect::new(x, y, desired_width, desired_height))
}

fn popup_width<S: AsRef<str>>(popup: &Popup<'_>, lines: &[S], max_width: u16) -> u16 {
    let max_line = lines
        .iter()
        .map(|line| text::display_width(line.as_ref()))
        .max()
        .unwrap_or(0) as u16;
    max_line.saturating_add(2).clamp(popup.min_width, max_width)
}

fn popup_height<S: AsRef<str>>(popup: &Popup<'_>, lines: &[S], width: u16, max_height: u16) -> u16 {
    let text_width = width.saturating_sub(2).max(1) as usize;
    let wrapped_lines: usize = lines
        .iter()
        .map(|line| {
            let line_width = text::display_width(line.as_ref());
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

    fn popup() -> Popup<'static> {
        Popup::new("Info")
            .min_size(3, 3)
            .text_style(Style::default())
            .border_style(Style::default())
            .text_alignment(Alignment::Left)
            .title_alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
    }

    #[test]
    fn popup_width_uses_display_width() {
        let lines = ["界界界"];
        let popup = popup();

        assert_eq!(popup_width(&popup, &lines, 20), 8);
    }

    #[test]
    fn popup_height_wraps_by_display_width() {
        let lines = ["界界界"];
        let popup = popup();

        assert_eq!(popup_height(&popup, &lines, 6, 20), 4);
    }

    #[test]
    fn popup_area_centers_measured_size() {
        let lines = ["abc"];
        let popup = popup();

        assert_eq!(
            popup_area(Rect::new(0, 0, 9, 7), &popup, &lines),
            Some(Rect::new(2, 2, 5, 3))
        );
    }
}
