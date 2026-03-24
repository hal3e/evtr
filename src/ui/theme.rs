use ratatui::style::{Color, Style, palette::tailwind};

pub(crate) const TEXT_COLOR: Color = tailwind::SLATE.c200;
pub(crate) const MUTED_COLOR: Color = tailwind::SLATE.c500;
pub(crate) const ACCENT_COLOR: Color = tailwind::BLUE.c300;
pub(crate) const ACCENT_STRONG_COLOR: Color = tailwind::BLUE.c400;
pub(crate) const DANGER_COLOR: Color = tailwind::RED.c400;

pub(crate) fn style_text() -> Style {
    Style::new().fg(TEXT_COLOR)
}

pub(crate) fn style_header() -> Style {
    style_text().bold()
}

pub(crate) fn style_panel_focused() -> Style {
    Style::new().fg(ACCENT_COLOR).bold()
}

pub(crate) fn style_panel_unfocused() -> Style {
    Style::new().fg(MUTED_COLOR)
}

pub(crate) fn style_gauge() -> Style {
    Style::new().fg(ACCENT_STRONG_COLOR)
}

pub(crate) fn style_error() -> Style {
    Style::new().fg(DANGER_COLOR).bold()
}
