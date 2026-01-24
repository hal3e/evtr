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

pub fn style_box_focused() -> Style {
    Style::new().fg(tailwind::BLUE.c300).bold()
}

pub fn style_box_unfocused() -> Style {
    Style::new().fg(tailwind::SLATE.c500)
}

pub const COLOR_BUTTON_PRESSED: Color = tailwind::RED.c400;
