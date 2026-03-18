use ratatui::layout::{Constraint, Layout, Rect};

use crate::device::monitor::config;

pub(crate) fn main_layout(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}

pub(crate) struct BoxLayout {
    pub(crate) joystick_box: Option<Rect>,
    pub(crate) hat_box: Option<Rect>,
    pub(crate) axes_box: Option<Rect>,
    pub(crate) touch_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

pub(crate) struct AxesLayout {
    pub(crate) abs_area: Option<Rect>,
    pub(crate) rel_area: Option<Rect>,
}

pub(crate) fn split_buttons_column(
    area: Rect,
    buttons_present: bool,
    main_min_width: u16,
    buttons_min_width: u16,
    min_button_gap: u16,
) -> (Rect, Option<Rect>) {
    if !buttons_present {
        return (area, None);
    }

    let gap = if area.width > config::MAIN_BUTTONS_GAP * 2 {
        config::MAIN_BUTTONS_GAP
    } else {
        0
    };
    let (main_width, buttons_width) = ratio_widths(area.width, gap, config::MAIN_COLUMN_PERCENT);

    if main_width < main_min_width || buttons_width < buttons_min_width {
        return (area, None);
    }
    if !buttons_width_ok(buttons_width, min_button_gap) {
        return (area, None);
    }

    let main_area = Rect::new(area.x, area.y, main_width, area.height);
    let buttons_area = Rect::new(
        area.x + main_width + gap,
        area.y,
        buttons_width,
        area.height,
    );
    (main_area, Some(buttons_area))
}

pub(crate) fn box_layout(
    area: Rect,
    joystick_present: bool,
    joystick_columns: usize,
    hat_present: bool,
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
    let min_hat_box = if hat_present { config::HAT_MIN_SIZE } else { 0 };

    let mut axes_height = 0;
    let mut touch_height = 0;
    let mut buttons_height = 0;

    let min_other = min_axes_box + min_buttons_box + min_touch_box;
    let max_top = area.height.saturating_sub(min_other);

    let mut joystick_present = joystick_present;
    let mut hat_present = hat_present;
    let mut top_row_height = 0;
    let mut side_by_side = false;
    let mut top_row_gap = 0;

    if joystick_present && hat_present {
        let gap = if area.width > config::JOYSTICK_GAP * 2 {
            config::JOYSTICK_GAP
        } else {
            0
        };
        let (joystick_width, hat_width) =
            ratio_widths(area.width, gap, config::JOYSTICK_HAT_JOYSTICK_PERCENT);
        let joystick_fit =
            joystick_height_for_width(joystick_width, max_top, min_joystick_box, joystick_columns);
        let hat_fit = hat_height_for_width(hat_width, max_top, min_hat_box);
        if let (Some(jh), Some(hh)) = (joystick_fit, hat_fit) {
            top_row_height = jh.max(hh);
            side_by_side = true;
            top_row_gap = gap;
        } else if joystick_fit.is_some() {
            hat_present = false;
        } else if hat_fit.is_some() {
            joystick_present = false;
        }
    }

    if !side_by_side && joystick_present && hat_present {
        if let Some(jh) =
            joystick_height_for_width(area.width, max_top, min_joystick_box, joystick_columns)
        {
            top_row_height = jh;
            hat_present = false;
        } else if let Some(hh) = hat_height_for_width(area.width, max_top, min_hat_box) {
            top_row_height = hh;
            joystick_present = false;
        } else {
            joystick_present = false;
            hat_present = false;
        }
    }

    if joystick_present && !hat_present {
        if let Some(jh) =
            joystick_height_for_width(area.width, max_top, min_joystick_box, joystick_columns)
        {
            top_row_height = jh;
        } else {
            joystick_present = false;
        }
    } else if hat_present && !joystick_present {
        if let Some(hh) = hat_height_for_width(area.width, max_top, min_hat_box) {
            top_row_height = hh;
        } else {
            hat_present = false;
        }
    }

    let remaining_after_top = area.height.saturating_sub(top_row_height);

    let mut touch_present = touch_present;
    if touch_present {
        let min_other = min_axes_box + min_buttons_box;
        if min_other == 0 {
            if remaining_after_top < min_touch_box {
                touch_present = false;
            } else {
                touch_height = remaining_after_top;
            }
        } else if remaining_after_top < min_other + min_touch_box {
            touch_present = false;
        } else {
            let preferred_touch = config::TOUCHPAD_HEIGHT + 2;
            let max_touch = remaining_after_top.saturating_sub(min_other);
            touch_height = preferred_touch.clamp(min_touch_box, max_touch);
        }
    }

    let remaining_height = area.height.saturating_sub(top_row_height + touch_height);

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
    let mut joystick_box = None;
    let mut hat_box = None;
    if top_row_height > 0 {
        if side_by_side && joystick_present && hat_present {
            let (left, right) = split_row_ratio(
                area.x,
                cursor_y,
                area.width,
                top_row_height,
                top_row_gap,
                config::JOYSTICK_HAT_JOYSTICK_PERCENT,
            );
            joystick_box = Some(left);
            hat_box = Some(right);
        } else if joystick_present {
            joystick_box = Some(Rect::new(area.x, cursor_y, area.width, top_row_height));
        } else if hat_present {
            hat_box = Some(Rect::new(area.x, cursor_y, area.width, top_row_height));
        }
        cursor_y = cursor_y.saturating_add(top_row_height);
    }
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
        hat_box,
        axes_box,
        touch_box,
        buttons_box,
    }
}

fn joystick_height_for_width(
    width: u16,
    max_height: u16,
    min_height: u16,
    columns: usize,
) -> Option<u16> {
    if width == 0 || max_height < min_height {
        return None;
    }
    let columns = columns.max(1) as u16;
    let gap = if columns > 1 { config::JOYSTICK_GAP } else { 0 };
    let width_per = width.saturating_sub(gap) / columns;
    if width_per == 0 {
        return None;
    }
    let ratio = config::JOYSTICK_ASPECT_RATIO.max(1);
    let height_for_width = width_per.saturating_div(ratio);
    let max_size = config::JOYSTICK_MAX_SIZE.min(max_height);
    if max_size < min_height {
        return None;
    }
    let preferred = height_for_width.min(max_size);
    if preferred < min_height || preferred == 0 {
        return None;
    }
    Some(preferred.clamp(min_height, max_size))
}

fn hat_height_for_width(width: u16, max_height: u16, min_height: u16) -> Option<u16> {
    if width == 0 || max_height < min_height {
        return None;
    }
    let ratio = config::JOYSTICK_ASPECT_RATIO.max(1);
    let height_for_width = width.saturating_div(ratio);
    let max_size = config::HAT_MAX_SIZE.min(max_height);
    if max_size < min_height {
        return None;
    }
    let preferred = height_for_width.min(max_size);
    if preferred < min_height || preferred == 0 {
        return None;
    }
    Some(preferred.clamp(min_height, max_size))
}

fn buttons_width_ok(width: u16, min_gap: u16) -> bool {
    if width == 0 {
        return false;
    }
    let button_width = width / config::BUTTONS_PER_ROW as u16;
    button_width > min_gap
}

fn ratio_widths(width: u16, gap: u16, left_percent: u16) -> (u16, u16) {
    let available = width.saturating_sub(gap);
    if available < 2 {
        return (0, 0);
    }
    let left_percent = left_percent.clamp(1, 99);
    let mut left = ((available as u32).saturating_mul(left_percent as u32) / 100) as u16;
    left = left.max(1).min(available.saturating_sub(1));
    let right = available.saturating_sub(left);
    (left, right)
}

fn split_row_ratio(
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    gap: u16,
    left_percent: u16,
) -> (Rect, Rect) {
    let (left_width, right_width) = ratio_widths(width, gap, left_percent);
    let left = Rect::new(x, y, left_width, height);
    let right = Rect::new(x + left_width + gap, y, right_width, height);
    (left, right)
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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{axes_layout, box_layout, split_buttons_column};
    use crate::device::monitor::config;

    #[test]
    fn split_buttons_column_returns_sidebar_when_width_allows() {
        let area = Rect::new(0, 0, 100, 20);

        let (main, buttons) = split_buttons_column(
            area,
            true,
            config::MAIN_COLUMN_MIN_WIDTH,
            config::BUTTONS_COLUMN_MIN_WIDTH,
            config::BTN_COL_GAP,
        );

        let buttons = buttons.expect("expected a buttons column");
        assert_eq!(
            main.width + buttons.width + config::MAIN_BUTTONS_GAP,
            area.width
        );
        assert!(main.width >= config::MAIN_COLUMN_MIN_WIDTH);
        assert!(buttons.width >= config::BUTTONS_COLUMN_MIN_WIDTH);
    }

    #[test]
    fn split_buttons_column_stays_single_column_when_sidebar_is_too_narrow() {
        let area = Rect::new(0, 0, 50, 20);

        let (main, buttons) = split_buttons_column(
            area,
            true,
            config::MAIN_COLUMN_MIN_WIDTH,
            config::BUTTONS_COLUMN_MIN_WIDTH,
            config::BTN_COL_GAP,
        );

        assert_eq!(main, area);
        assert!(buttons.is_none());
    }

    #[test]
    fn box_layout_gives_small_remaining_space_to_buttons_first() {
        let area = Rect::new(0, 0, 60, 1);

        let layout = box_layout(area, false, 0, false, false, true, true);

        assert!(layout.axes_box.is_none());
        assert_eq!(layout.buttons_box, Some(area));
    }

    #[test]
    fn box_layout_drops_touch_when_it_cannot_fit_with_axes_and_buttons() {
        let area = Rect::new(0, 0, 60, config::TOUCHPAD_MIN_HEIGHT + 2);

        let layout = box_layout(area, false, 0, false, true, true, true);

        assert!(layout.touch_box.is_none());
        assert!(layout.axes_box.is_some());
        assert!(layout.buttons_box.is_some());
    }

    #[test]
    fn axes_layout_splits_absolute_and_relative_sections_with_gap() {
        let layout = axes_layout(Rect::new(0, 0, 40, 10), 2, 2);

        assert_eq!(layout.abs_area, Some(Rect::new(0, 0, 40, 4)));
        assert_eq!(layout.rel_area, Some(Rect::new(0, 5, 40, 5)));
    }
}
