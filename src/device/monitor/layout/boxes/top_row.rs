use ratatui::layout::Rect;

use crate::device::monitor::config;

use super::{
    BoxMinimums,
    split::{gap_if_room, ratio_widths, split_row_ratio},
};

#[derive(Clone, Copy, Default)]
pub(super) struct TopRowPlan {
    joystick_present: bool,
    hat_present: bool,
    side_by_side: bool,
    pub(super) height: u16,
    gap: u16,
}

pub(super) fn plan_top_row(
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

pub(super) fn place_top_row(
    area: Rect,
    cursor_y: &mut u16,
    plan: TopRowPlan,
) -> (Option<Rect>, Option<Rect>) {
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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::plan_top_row;
    use crate::device::monitor::layout::boxes::BoxMinimums;

    #[test]
    fn plan_top_row_prefers_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);
        let minimums = BoxMinimums::new(true, true, false, false, false);

        let plan = plan_top_row(area, 1, minimums, true, true);

        assert!(plan.joystick_present);
        assert!(plan.hat_present);
        assert!(plan.side_by_side);
        assert!(plan.height > 0);
    }
}
