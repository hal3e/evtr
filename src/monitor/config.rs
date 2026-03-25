// Theme items are consolidated in theme.rs; re-exported for convenience
pub use crate::monitor::theme::{
    COLOR_BUTTON_PRESSED, COLOR_TOUCH_INACTIVE, COLOR_TOUCH_POINT, style_gauge, style_header,
    style_label,
};

pub const BUTTONS_PER_ROW: usize = 3;
pub const BUTTON_HEIGHT: u16 = 3;
pub const RELATIVE_DISPLAY_RANGE: i32 = 1000; // -500 to +500 range
pub const DEFAULT_AXIS_RANGE: (i32, i32) = (-32768, 32767);
pub const BAR_HEIGHTS: [u16; 3] = [5, 3, 1];
pub const AXIS_LABEL_MAX: u16 = 20; // max chars allocated to axis label
pub const AXIS_GAP: u16 = 1; // vertical gap between axis bars
pub const REL_SECTION_GAP: u16 = 1; // spacer before relative section
pub const AXIS_LEFT_PADDING: u16 = 1; // left padding inside axis box
pub const AXES_BOX_PERCENT: u16 = 75; // percent of height to allocate to axes when both present
pub const BTN_SECTION_TOP_PADDING: u16 = 1; // top padding inside button grid area
pub const BTN_SECTION_VERT_PADDING: u16 = 2; // total vertical padding used for button section sizing
pub const BTN_COL_GAP: u16 = 1; // column gap inside button grid
pub const PAGE_SCROLL_STEPS: usize = 10; // page up/down step count
pub const AXIS_MIN_WIDTH: u16 = 20; // minimum width to render axis/gauge
pub const LABEL_GAUGE_GAP: u16 = 1; // horizontal gap between label and gauge
pub const COMPACT_BTN_COL_GAP: u16 = 1;
pub const TOUCHPAD_MIN_WIDTH: u16 = 20; // minimum width to render touchpad widget
pub const TOUCHPAD_MIN_HEIGHT: u16 = 4; // minimum height to render touchpad widget
pub const TOUCHPAD_HEIGHT: u16 = 7; // preferred height for touchpad box
pub const JOYSTICK_MIN_SIZE: u16 = 6; // minimum size for joystick widget box
pub const JOYSTICK_MAX_SIZE: u16 = 12; // maximum size for joystick widget box
pub const JOYSTICK_ASPECT_RATIO: u16 = 2; // width to height ratio for joystick view
pub const JOYSTICK_GAP: u16 = 2; // horizontal gap between joysticks
pub const HAT_MIN_SIZE: u16 = 6; // minimum size for d-pad widget box
pub const HAT_MAX_SIZE: u16 = 10; // maximum size for d-pad widget box
pub const HAT_BLOCKS: usize = 4; // blocks per d-pad direction (length)
pub const HAT_THICKNESS: usize = 2; // blocks across (thickness)
pub const HAT_PADDING: u16 = 0; // inset inside d-pad canvas
pub const JOYSTICK_HAT_JOYSTICK_PERCENT: u16 = 70; // width share for joystick when d-pad is present
pub const MAIN_COLUMN_PERCENT: u16 = 70; // width share for main column when buttons on the right
pub const MAIN_BUTTONS_GAP: u16 = 2; // horizontal gap between main column and buttons column
pub const MAIN_COLUMN_MIN_WIDTH: u16 = 30; // minimum width for main column before splitting
pub const BUTTONS_COLUMN_MIN_WIDTH: u16 = 24; // minimum width for buttons column before splitting

// All style/color definitions live in theme.rs
