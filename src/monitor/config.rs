use ratatui::style::{Color, Style};

use crate::{config, ui::theme};

pub const BUTTON_HEIGHT: u16 = 3;
pub const DEFAULT_AXIS_RANGE: (i32, i32) = (-32768, 32767);
pub const BAR_HEIGHTS: [u16; 3] = [5, 3, 1];
pub const AXIS_LABEL_MAX: u16 = 20; // max chars allocated to axis label
pub const AXIS_GAP: u16 = 1; // vertical gap between axis bars
pub const REL_SECTION_GAP: u16 = 1; // spacer before relative section
pub const AXIS_LEFT_PADDING: u16 = 1; // left padding inside axis box
pub const BTN_SECTION_TOP_PADDING: u16 = 1; // top padding inside button grid area
pub const BTN_SECTION_VERT_PADDING: u16 = 2; // total vertical padding used for button section sizing
pub const BTN_COL_GAP: u16 = 1; // column gap inside button grid
pub const AXIS_MIN_WIDTH: u16 = 20; // minimum width to render axis/gauge
pub const LABEL_GAUGE_GAP: u16 = 1; // horizontal gap between label and gauge
pub const COMPACT_BTN_COL_GAP: u16 = 1;
pub const TOUCHPAD_MIN_WIDTH: u16 = 20; // minimum width to render touchpad widget
pub const TOUCHPAD_MIN_HEIGHT: u16 = 4; // minimum height to render touchpad widget
pub const TOUCHPAD_HEIGHT: u16 = 7; // preferred height for touchpad box
pub const JOYSTICK_MIN_SIZE: u16 = 6; // minimum size for joystick widget box
pub const JOYSTICK_MAX_SIZE: u16 = 12; // maximum size for joystick widget box
pub const JOYSTICK_ASPECT_RATIO: u16 = 2; // width to height ratio for joystick view
pub const HAT_MIN_SIZE: u16 = 6; // minimum size for d-pad widget box
pub const HAT_MAX_SIZE: u16 = 10; // maximum size for d-pad widget box
pub const HAT_BLOCKS: usize = 4; // blocks per d-pad direction (length)
pub const HAT_THICKNESS: usize = 2; // blocks across (thickness)
pub const HAT_PADDING: u16 = 0; // inset inside d-pad canvas
pub const MAIN_BUTTONS_GAP: u16 = 2; // horizontal gap between main column and buttons column
pub const MAIN_COLUMN_MIN_WIDTH: u16 = 30; // minimum width for main column before splitting
pub const BUTTONS_COLUMN_MIN_WIDTH: u16 = 24; // minimum width for buttons column before splitting

pub fn style_label() -> Style {
    theme::style_text()
}

pub fn style_header() -> Style {
    theme::style_header()
}

pub fn style_gauge() -> Style {
    theme::style_gauge()
}

pub fn buttons_per_row() -> usize {
    config::layout().monitor.buttons_per_row
}

pub fn axes_box_percent() -> u16 {
    config::layout().monitor.axes_box_percent
}

pub fn joystick_gap() -> u16 {
    config::layout().monitor.joystick_gap
}

pub fn joystick_hat_joystick_percent() -> u16 {
    config::layout().monitor.joystick_hat_joystick_percent
}

pub fn main_column_percent() -> u16 {
    config::layout().monitor.main_column_percent
}

pub fn color_button_pressed() -> Color {
    theme::danger_color()
}

pub fn color_touch_point() -> Color {
    theme::danger_color()
}

pub fn color_touch_inactive() -> Color {
    theme::muted_color()
}
