mod layout;
mod presets;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Clear, Paragraph, Widget, Wrap},
};

pub(crate) use self::presets::{error_popup, help_popup, info_popup};
use super::widgets;

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

    pub(crate) fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
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
    let Some(popup_area) = layout::popup_area(area, popup, lines) else {
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

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::{Alignment, Rect},
        style::{Modifier, Style},
        widgets::Wrap,
    };

    use super::{Popup, render_popup};

    fn buffer_text(buf: &Buffer) -> String {
        buf.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn popup_builder_setters_update_the_expected_fields() {
        let popup = Popup::new("Help")
            .min_size(10, 4)
            .max_width(30)
            .max_height(12)
            .text_style(Style::default().add_modifier(Modifier::BOLD))
            .border_style(Style::default().add_modifier(Modifier::ITALIC))
            .text_alignment(Alignment::Center)
            .title_alignment(Alignment::Right)
            .wrap(Wrap { trim: true });

        assert_eq!(popup.title, "Help");
        assert_eq!(popup.min_width, 10);
        assert_eq!(popup.min_height, 4);
        assert_eq!(popup.max_width, Some(30));
        assert_eq!(popup.max_height, Some(12));
        assert_eq!(popup.text_alignment, Alignment::Center);
        assert_eq!(popup.title_alignment, Alignment::Right);
        assert!(popup.text_style.add_modifier.contains(Modifier::BOLD));
        assert!(popup.border_style.add_modifier.contains(Modifier::ITALIC));
        assert!(popup.wrap.trim);
    }

    #[test]
    fn render_popup_is_a_no_op_when_the_area_cannot_fit_the_popup() {
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        let popup = Popup::new("Help").min_size(10, 4);

        render_popup(area, &mut buf, &popup, &["line 1"]);

        assert_eq!(buffer_text(&buf).trim(), "");
    }

    #[test]
    fn render_popup_writes_title_and_joined_body_lines() {
        let area = Rect::new(0, 0, 24, 8);
        let mut buf = Buffer::empty(area);
        let popup = Popup::new("Info").min_size(10, 4).max_width(20);

        render_popup(area, &mut buf, &popup, &["first line", "second line"]);

        let text = buffer_text(&buf);
        assert!(text.contains("Info"));
        assert!(text.contains("first line"));
        assert!(text.contains("second line"));
    }
}
