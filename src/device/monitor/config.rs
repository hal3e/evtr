// Theme items are consolidated in theme.rs; re-exported for convenience
pub use crate::device::monitor::theme::{
    COLOR_BUTTON_PRESSED, style_gauge, style_header, style_label,
};

pub const BUTTONS_PER_ROW: usize = 6;
pub const BUTTON_HEIGHT: u16 = 3;
pub const RELATIVE_DISPLAY_RANGE: i32 = 1000; // -500 to +500 range
pub const DEFAULT_AXIS_RANGE: (i32, i32) = (-32768, 32767);
pub const BAR_HEIGHTS: [u16; 3] = [5, 3, 1];
pub const AXIS_LABEL_MAX: u16 = 20; // max chars allocated to axis label
pub const AXIS_GAP: u16 = 1; // vertical gap between axis bars
pub const REL_SECTION_GAP: u16 = 1; // spacer before relative section
pub const BTN_SECTION_TOP_PADDING: u16 = 1; // top padding inside button grid area
pub const BTN_SECTION_VERT_PADDING: u16 = 2; // total vertical padding used for button section sizing
pub const BTN_COL_GAP: u16 = 1; // column gap inside button grid
pub const PAGE_SCROLL_STEPS: usize = 10; // page up/down step count
pub const AXIS_MIN_WIDTH: u16 = 20; // minimum width to render axis/gauge
pub const LABEL_GAUGE_GAP: u16 = 1; // horizontal gap between label and gauge
pub const COMPACT_BTN_COL_GAP: u16 = 1;

// All style/color definitions live in theme.rs
