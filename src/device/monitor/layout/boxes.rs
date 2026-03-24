use ratatui::layout::Rect;

use crate::device::monitor::config;

pub(crate) struct BoxLayout {
    pub(crate) joystick_box: Option<Rect>,
    pub(crate) hat_box: Option<Rect>,
    pub(crate) axes_box: Option<Rect>,
    pub(crate) touch_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

#[derive(Clone, Copy)]
struct BoxMinimums {
    axes: u16,
    buttons: u16,
    touch: u16,
    joystick: u16,
    hat: u16,
}

impl BoxMinimums {
    fn new(
        joystick_present: bool,
        hat_present: bool,
        touch_present: bool,
        axes_present: bool,
        buttons_present: bool,
    ) -> Self {
        Self {
            axes: min_box_height(axes_present, 2),
            buttons: min_box_height(buttons_present, 1),
            touch: min_box_height(touch_present, config::TOUCHPAD_MIN_HEIGHT + 2),
            joystick: min_box_height(joystick_present, config::JOYSTICK_MIN_SIZE),
            hat: min_box_height(hat_present, config::HAT_MIN_SIZE),
        }
    }

    fn reserved_below_top(&self) -> u16 {
        self.axes + self.buttons + self.touch
    }

    fn reserved_below_touch(&self) -> u16 {
        self.axes + self.buttons
    }
}

#[derive(Clone, Copy, Default)]
struct TopRowPlan {
    joystick_present: bool,
    hat_present: bool,
    side_by_side: bool,
    height: u16,
    gap: u16,
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

    let gap = gap_if_room(area.width, config::MAIN_BUTTONS_GAP);
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
    let minimums = BoxMinimums::new(
        joystick_present,
        hat_present,
        touch_present,
        axes_present,
        buttons_present,
    );
    let top_row = plan_top_row(
        area,
        joystick_columns,
        minimums,
        joystick_present,
        hat_present,
    );
    let remaining_after_top = area.height.saturating_sub(top_row.height);
    let (touch_present, touch_height) =
        allocate_touch(remaining_after_top, touch_present, minimums);
    let remaining_height = area.height.saturating_sub(top_row.height + touch_height);
    let (axes_height, buttons_height) =
        allocate_lower_sections(remaining_height, axes_present, buttons_present, minimums);

    let mut cursor_y = area.y;
    let (joystick_box, hat_box) = place_top_row(area, &mut cursor_y, top_row);
    let touch_box = if touch_present {
        take_next_box(area, &mut cursor_y, touch_height, minimums.touch)
    } else {
        None
    };
    let axes_box = take_next_box(area, &mut cursor_y, axes_height, minimums.axes);
    let buttons_box = take_next_box(area, &mut cursor_y, buttons_height, minimums.buttons);

    BoxLayout {
        joystick_box,
        hat_box,
        axes_box,
        touch_box,
        buttons_box,
    }
}

fn min_box_height(present: bool, height: u16) -> u16 {
    if present { height } else { 0 }
}

fn gap_if_room(width: u16, preferred_gap: u16) -> u16 {
    if width > preferred_gap * 2 {
        preferred_gap
    } else {
        0
    }
}

fn plan_top_row(
    area: Rect,
    joystick_columns: usize,
    minimums: BoxMinimums,
    joystick_present: bool,
    hat_present: bool,
) -> TopRowPlan {
    let max_top = area.height.saturating_sub(minimums.reserved_below_top());
    let mut plan = TopRowPlan {
        joystick_present,
        hat_present,
        ..TopRowPlan::default()
    };

    if plan.joystick_present && plan.hat_present {
        plan = plan_dual_top_row(area.width, max_top, joystick_columns, minimums);
    }

    if !plan.side_by_side && plan.joystick_present && plan.hat_present {
        if let Some(height) =
            joystick_height_for_width(area.width, max_top, minimums.joystick, joystick_columns)
        {
            plan.height = height;
            plan.hat_present = false;
        } else if let Some(height) = hat_height_for_width(area.width, max_top, minimums.hat) {
            plan.height = height;
            plan.joystick_present = false;
        } else {
            plan.joystick_present = false;
            plan.hat_present = false;
        }
    }

    if plan.joystick_present && !plan.hat_present {
        if let Some(height) =
            joystick_height_for_width(area.width, max_top, minimums.joystick, joystick_columns)
        {
            plan.height = height;
        } else {
            plan.joystick_present = false;
        }
    } else if plan.hat_present && !plan.joystick_present {
        if let Some(height) = hat_height_for_width(area.width, max_top, minimums.hat) {
            plan.height = height;
        } else {
            plan.hat_present = false;
        }
    }

    plan
}

fn plan_dual_top_row(
    width: u16,
    max_top: u16,
    joystick_columns: usize,
    minimums: BoxMinimums,
) -> TopRowPlan {
    let gap = gap_if_room(width, config::JOYSTICK_GAP);
    let (joystick_width, hat_width) =
        ratio_widths(width, gap, config::JOYSTICK_HAT_JOYSTICK_PERCENT);
    let joystick_fit =
        joystick_height_for_width(joystick_width, max_top, minimums.joystick, joystick_columns);
    let hat_fit = hat_height_for_width(hat_width, max_top, minimums.hat);

    match (joystick_fit, hat_fit) {
        (Some(joystick_height), Some(hat_height)) => TopRowPlan {
            joystick_present: true,
            hat_present: true,
            side_by_side: true,
            height: joystick_height.max(hat_height),
            gap,
        },
        (Some(_), None) => TopRowPlan {
            joystick_present: true,
            hat_present: false,
            ..TopRowPlan::default()
        },
        (None, Some(_)) => TopRowPlan {
            joystick_present: false,
            hat_present: true,
            ..TopRowPlan::default()
        },
        (None, None) => TopRowPlan {
            joystick_present: true,
            hat_present: true,
            ..TopRowPlan::default()
        },
    }
}

fn allocate_touch(
    remaining_after_top: u16,
    touch_present: bool,
    minimums: BoxMinimums,
) -> (bool, u16) {
    if !touch_present {
        return (false, 0);
    }

    let min_other = minimums.reserved_below_touch();
    if min_other == 0 {
        return if remaining_after_top < minimums.touch {
            (false, 0)
        } else {
            (true, remaining_after_top)
        };
    }

    if remaining_after_top < min_other + minimums.touch {
        return (false, 0);
    }

    let preferred_touch = config::TOUCHPAD_HEIGHT + 2;
    let max_touch = remaining_after_top.saturating_sub(min_other);
    (true, preferred_touch.clamp(minimums.touch, max_touch))
}

fn allocate_lower_sections(
    remaining_height: u16,
    axes_present: bool,
    buttons_present: bool,
    minimums: BoxMinimums,
) -> (u16, u16) {
    match (axes_present, buttons_present) {
        (true, true) => {
            if remaining_height < minimums.axes + minimums.buttons {
                if remaining_height >= minimums.buttons {
                    (0, remaining_height)
                } else {
                    (0, 0)
                }
            } else {
                let desired_axes = (remaining_height * config::AXES_BOX_PERCENT) / 100;
                let max_axes = remaining_height.saturating_sub(minimums.buttons);
                let axes_height = desired_axes.clamp(minimums.axes, max_axes);
                (axes_height, remaining_height.saturating_sub(axes_height))
            }
        }
        (true, false) => {
            if remaining_height >= minimums.axes {
                (remaining_height, 0)
            } else {
                (0, 0)
            }
        }
        (false, true) => {
            if remaining_height >= minimums.buttons {
                (0, remaining_height)
            } else {
                (0, 0)
            }
        }
        (false, false) => (0, 0),
    }
}

fn place_top_row(area: Rect, cursor_y: &mut u16, plan: TopRowPlan) -> (Option<Rect>, Option<Rect>) {
    if plan.height == 0 {
        return (None, None);
    }

    let (joystick_box, hat_box) = if plan.side_by_side && plan.joystick_present && plan.hat_present
    {
        let (left, right) = split_row_ratio(
            area.x,
            *cursor_y,
            area.width,
            plan.height,
            plan.gap,
            config::JOYSTICK_HAT_JOYSTICK_PERCENT,
        );
        (Some(left), Some(right))
    } else if plan.joystick_present {
        (
            Some(Rect::new(area.x, *cursor_y, area.width, plan.height)),
            None,
        )
    } else if plan.hat_present {
        (
            None,
            Some(Rect::new(area.x, *cursor_y, area.width, plan.height)),
        )
    } else {
        (None, None)
    };

    *cursor_y = cursor_y.saturating_add(plan.height);
    (joystick_box, hat_box)
}

fn take_next_box(area: Rect, cursor_y: &mut u16, height: u16, min_height: u16) -> Option<Rect> {
    if height < min_height || height == 0 {
        return None;
    }

    let rect = Rect::new(area.x, *cursor_y, area.width, height);
    *cursor_y = cursor_y.saturating_add(height);
    Some(rect)
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
    bounded_square_height(width_per, max_height, min_height, config::JOYSTICK_MAX_SIZE)
}

fn hat_height_for_width(width: u16, max_height: u16, min_height: u16) -> Option<u16> {
    bounded_square_height(width, max_height, min_height, config::HAT_MAX_SIZE)
}

fn bounded_square_height(
    width: u16,
    max_height: u16,
    min_height: u16,
    max_size: u16,
) -> Option<u16> {
    if width == 0 || max_height < min_height {
        return None;
    }
    let ratio = config::JOYSTICK_ASPECT_RATIO.max(1);
    let height_for_width = width.saturating_div(ratio);
    let max_size = max_size.min(max_height);
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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{box_layout, split_buttons_column};
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
    fn box_layout_keeps_joystick_and_hat_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);

        let layout = box_layout(area, true, 1, true, false, false, false);

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_some());
    }

    #[test]
    fn box_layout_falls_back_to_joystick_when_hat_cannot_fit() {
        let area = Rect::new(0, 0, 12, 6);

        let layout = box_layout(area, true, 1, true, false, false, false);

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_none());
    }
}
