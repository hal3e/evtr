use ratatui::style::{Color, Style};

use crate::ui::theme;

pub fn style_label() -> Style {
    theme::style_text()
}

pub fn style_header() -> Style {
    theme::style_header()
}

pub fn style_gauge() -> Style {
    theme::style_gauge()
}

pub const COLOR_BUTTON_PRESSED: Color = theme::DANGER_COLOR;
pub const COLOR_TOUCH_POINT: Color = theme::DANGER_COLOR;
pub const COLOR_TOUCH_INACTIVE: Color = theme::MUTED_COLOR;
