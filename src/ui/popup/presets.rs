use ratatui::{layout::Alignment, widgets::Wrap};

use super::Popup;
use crate::ui::theme;

pub(crate) fn help_popup(
    title: &str,
    min_width: u16,
    min_height: u16,
    max_width: u16,
) -> Popup<'_> {
    Popup::new(title)
        .min_size(min_width, min_height)
        .max_width(max_width)
        .text_style(theme::style_text())
        .border_style(theme::style_panel_focused())
        .text_alignment(Alignment::Left)
        .title_alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
}

pub(crate) fn error_popup(
    title: &str,
    min_width: u16,
    min_height: u16,
    max_width: u16,
) -> Popup<'_> {
    Popup::new(title)
        .min_size(min_width, min_height)
        .max_width(max_width)
        .text_style(theme::style_error())
        .border_style(theme::style_error())
        .text_alignment(Alignment::Center)
        .title_alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
}

pub(crate) fn info_popup(title: &str, min_width: u16, min_height: u16) -> Popup<'_> {
    Popup::new(title)
        .min_size(min_width, min_height)
        .text_style(theme::style_text())
        .border_style(theme::style_panel_focused())
        .text_alignment(Alignment::Left)
        .title_alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Alignment;

    use super::{error_popup, help_popup, info_popup};

    #[test]
    fn help_popup_uses_shared_styles() {
        let popup = help_popup(" Help ", 10, 3, 40);

        assert_eq!(popup.title_alignment, Alignment::Center);
        assert_eq!(popup.text_alignment, Alignment::Left);
        assert!(!popup.wrap.trim);
    }

    #[test]
    fn error_popup_is_centered_and_trimmed() {
        let popup = error_popup(" Error ", 10, 3, 40);

        assert_eq!(popup.title_alignment, Alignment::Center);
        assert_eq!(popup.text_alignment, Alignment::Center);
        assert!(popup.wrap.trim);
    }

    #[test]
    fn info_popup_keeps_multiline_text_untrimmed() {
        let popup = info_popup(" Info ", 10, 3);

        assert_eq!(popup.title_alignment, Alignment::Center);
        assert_eq!(popup.text_alignment, Alignment::Left);
        assert!(!popup.wrap.trim);
    }
}
