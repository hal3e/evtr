mod layout;
mod presets;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Clear, Paragraph, Widget, Wrap},
};

use super::widgets;

pub(crate) use self::presets::{error_popup, help_popup, info_popup};

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
