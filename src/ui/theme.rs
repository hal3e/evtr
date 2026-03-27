use ratatui::style::{Color, Style};

use crate::config;

pub(crate) fn text_color() -> Color {
    config::theme().palette.text
}

pub(crate) fn muted_color() -> Color {
    config::theme().palette.muted
}

pub(crate) fn accent_color() -> Color {
    config::theme().palette.accent
}

pub(crate) fn accent_strong_color() -> Color {
    config::theme().palette.accent_strong
}

pub(crate) fn danger_color() -> Color {
    config::theme().palette.danger
}

pub(crate) fn style_text() -> Style {
    Style::new().fg(text_color())
}

pub(crate) fn style_header() -> Style {
    style_text().bold()
}

pub(crate) fn style_panel_focused() -> Style {
    Style::new().fg(accent_color()).bold()
}

pub(crate) fn style_panel_unfocused() -> Style {
    Style::new().fg(muted_color())
}

pub(crate) fn style_gauge() -> Style {
    Style::new().fg(accent_strong_color())
}

pub(crate) fn style_error() -> Style {
    Style::new().fg(danger_color()).bold()
}
