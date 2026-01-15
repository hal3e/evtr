use ratatui::style::{Color, Style, palette::tailwind};

pub fn style_label() -> Style {
    Style::new().fg(tailwind::SLATE.c200)
}

pub fn style_header() -> Style {
    Style::new().fg(tailwind::SLATE.c200).bold()
}

pub fn style_gauge() -> Style {
    Style::new().fg(tailwind::BLUE.c400)
}

pub const COLOR_BUTTON_PRESSED: Color = tailwind::RED.c400;
