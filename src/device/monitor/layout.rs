use ratatui::layout::{Constraint, Layout, Rect};

use crate::device::monitor::config;

pub(crate) fn main_layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}

pub(crate) struct BoxLayout {
    pub(crate) joystick_box: Option<Rect>,
    pub(crate) axes_box: Option<Rect>,
    pub(crate) touch_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

pub(crate) struct AxesLayout {
    pub(crate) abs_area: Option<Rect>,
    pub(crate) rel_area: Option<Rect>,
}

pub(crate) fn box_layout(
    area: Rect,
    joystick_present: bool,
    joystick_columns: usize,
    touch_present: bool,
    axes_present: bool,
    buttons_present: bool,
) -> BoxLayout {
    let min_axes_box = if axes_present { 2 } else { 0 };
    let min_buttons_box = if buttons_present { 1 } else { 0 };
    let min_touch_box = if touch_present {
        config::TOUCHPAD_MIN_HEIGHT + 2
    } else {
        0
    };
    let min_joystick_box = if joystick_present {
        config::JOYSTICK_MIN_SIZE
    } else {
        0
    };

    let mut axes_height = 0;
    let mut touch_height = 0;
    let mut joystick_height = 0;
    let mut buttons_height = 0;

    let mut joystick_present = joystick_present;
    if joystick_present {
        let min_other = min_axes_box + min_buttons_box + min_touch_box;
        if area.height < min_other + min_joystick_box {
            joystick_present = false;
        } else {
            let max_joystick = area.height.saturating_sub(min_other);
            let columns = joystick_columns.max(1) as u16;
            let gap = if joystick_columns > 1 {
                config::JOYSTICK_GAP
            } else {
                0
            };
            let width_per = area.width.saturating_sub(gap) / columns;
            if width_per == 0 {
                joystick_present = false;
            } else {
                let max_size = config::JOYSTICK_MAX_SIZE.min(max_joystick);
                let height_for_width =
                    width_per.saturating_div(config::JOYSTICK_ASPECT_RATIO.max(1));
                let preferred = height_for_width.min(max_size);
                joystick_height = preferred.clamp(min_joystick_box, max_size);
            }
        }
    }

    let remaining_after_joystick = area.height.saturating_sub(joystick_height);

    let mut touch_present = touch_present;
    if touch_present {
        let min_other = min_axes_box + min_buttons_box;
        if min_other == 0 {
            if remaining_after_joystick < min_touch_box {
                touch_present = false;
            } else {
                touch_height = remaining_after_joystick;
            }
        } else if remaining_after_joystick < min_other + min_touch_box {
            touch_present = false;
        } else {
            let preferred_touch = config::TOUCHPAD_HEIGHT + 2;
            let max_touch = remaining_after_joystick.saturating_sub(min_other);
            touch_height = preferred_touch.clamp(min_touch_box, max_touch);
        }
    }

    let remaining_height = area.height.saturating_sub(joystick_height + touch_height);

    match (axes_present, buttons_present) {
        (true, true) => {
            if remaining_height < min_axes_box + min_buttons_box {
                if remaining_height >= min_buttons_box {
                    buttons_height = remaining_height;
                }
            } else {
                let desired_axes = (remaining_height * config::AXES_BOX_PERCENT) / 100;
                let max_axes = remaining_height.saturating_sub(min_buttons_box);
                axes_height = desired_axes.clamp(min_axes_box, max_axes);
                buttons_height = remaining_height.saturating_sub(axes_height);
            }
        }
        (true, false) => {
            if remaining_height >= min_axes_box {
                axes_height = remaining_height;
            }
        }
        (false, true) => {
            if remaining_height >= min_buttons_box {
                buttons_height = remaining_height;
            }
        }
        (false, false) => {}
    }

    let mut cursor_y = area.y;
    let joystick_box =
        if joystick_present && joystick_height >= min_joystick_box && joystick_height > 0 {
            let rect = Rect::new(area.x, cursor_y, area.width, joystick_height);
            cursor_y = cursor_y.saturating_add(joystick_height);
            Some(rect)
        } else {
            None
        };
    let touch_box = if touch_present && touch_height >= min_touch_box && touch_height > 0 {
        let rect = Rect::new(area.x, cursor_y, area.width, touch_height);
        cursor_y = cursor_y.saturating_add(touch_height);
        Some(rect)
    } else {
        None
    };
    let axes_box = if axes_height >= min_axes_box && axes_height > 0 {
        let rect = Rect::new(area.x, cursor_y, area.width, axes_height);
        cursor_y = cursor_y.saturating_add(axes_height);
        Some(rect)
    } else {
        None
    };
    let buttons_box = if buttons_height >= min_buttons_box && buttons_height > 0 {
        Some(Rect::new(area.x, cursor_y, area.width, buttons_height))
    } else {
        None
    };

    BoxLayout {
        joystick_box,
        axes_box,
        touch_box,
        buttons_box,
    }
}

pub(crate) fn axes_layout(area: Rect, abs_count: usize, rel_count: usize) -> AxesLayout {
    let total_axes = abs_count + rel_count;
    if total_axes == 0 || area.height == 0 {
        return AxesLayout {
            abs_area: None,
            rel_area: None,
        };
    }

    if abs_count > 0 && rel_count > 0 {
        let gap = config::REL_SECTION_GAP;
        let available_for_content = area.height.saturating_sub(gap);
        let abs_portion = (available_for_content * abs_count as u16) / total_axes as u16;
        let rel_portion = available_for_content.saturating_sub(abs_portion);

        let abs_area = Rect::new(area.x, area.y, area.width, abs_portion);
        let rel_area = Rect::new(area.x, area.y + abs_portion + gap, area.width, rel_portion);

        AxesLayout {
            abs_area: Some(abs_area),
            rel_area: Some(rel_area),
        }
    } else if abs_count > 0 {
        AxesLayout {
            abs_area: Some(area),
            rel_area: None,
        }
    } else {
        AxesLayout {
            abs_area: None,
            rel_area: Some(area),
        }
    }
}
