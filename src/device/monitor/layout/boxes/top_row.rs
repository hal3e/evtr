use ratatui::layout::Rect;

use crate::device::monitor::config;

use super::{
    BoxMinimums,
    split::{gap_if_room, ratio_widths, split_row_ratio},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TopRowRequest {
    #[default]
    None,
    Joystick,
    Hat,
    Both,
}

impl TopRowRequest {
    pub(super) fn new(joystick_present: bool, hat_present: bool) -> Self {
        match (joystick_present, hat_present) {
            (true, true) => Self::Both,
            (true, false) => Self::Joystick,
            (false, true) => Self::Hat,
            (false, false) => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TopRowLayout {
    #[default]
    None,
    Joystick {
        height: u16,
    },
    Hat {
        height: u16,
    },
    Split {
        height: u16,
        gap: u16,
    },
}

impl TopRowLayout {
    pub(super) fn height(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Joystick { height } | Self::Hat { height } | Self::Split { height, .. } => height,
        }
    }
}

pub(super) fn plan_top_row(
    area: Rect,
    joystick_columns: usize,
    minimums: BoxMinimums,
    request: TopRowRequest,
) -> TopRowLayout {
    let max_top = area.height.saturating_sub(minimums.reserved_below_top());
    match request {
        TopRowRequest::None => TopRowLayout::None,
        TopRowRequest::Joystick => {
            joystick_layout(area.width, max_top, minimums.joystick, joystick_columns)
                .unwrap_or_default()
        }
        TopRowRequest::Hat => hat_layout(area.width, max_top, minimums.hat).unwrap_or_default(),
        TopRowRequest::Both => {
            plan_dual_top_row(area.width, max_top, joystick_columns, minimums).unwrap_or_default()
        }
    }
}

pub(super) fn place_top_row(
    area: Rect,
    cursor_y: &mut u16,
    layout: TopRowLayout,
) -> (Option<Rect>, Option<Rect>) {
    let height = layout.height();
    if height == 0 {
        return (None, None);
    }

    let (joystick_box, hat_box) = match layout {
        TopRowLayout::None => (None, None),
        TopRowLayout::Joystick { height } => {
            (Some(Rect::new(area.x, *cursor_y, area.width, height)), None)
        }
        TopRowLayout::Hat { height } => {
            (None, Some(Rect::new(area.x, *cursor_y, area.width, height)))
        }
        TopRowLayout::Split { height, gap } => {
            let (left, right) = split_row_ratio(
                area.x,
                *cursor_y,
                area.width,
                height,
                gap,
                config::JOYSTICK_HAT_JOYSTICK_PERCENT,
            );
            (Some(left), Some(right))
        }
    };

    *cursor_y = cursor_y.saturating_add(height);
    (joystick_box, hat_box)
}

fn plan_dual_top_row(
    width: u16,
    max_top: u16,
    joystick_columns: usize,
    minimums: BoxMinimums,
) -> Option<TopRowLayout> {
    let gap = gap_if_room(width, config::JOYSTICK_GAP);
    let (joystick_width, hat_width) =
        ratio_widths(width, gap, config::JOYSTICK_HAT_JOYSTICK_PERCENT);
    let joystick_fit =
        joystick_height_for_width(joystick_width, max_top, minimums.joystick, joystick_columns);
    let hat_fit = hat_height_for_width(hat_width, max_top, minimums.hat);

    match (joystick_fit, hat_fit) {
        (Some(joystick_height), Some(hat_height)) => Some(TopRowLayout::Split {
            height: joystick_height.max(hat_height),
            gap,
        }),
        (Some(_), None) => joystick_layout(width, max_top, minimums.joystick, joystick_columns),
        (None, Some(_)) => hat_layout(width, max_top, minimums.hat),
        (None, None) => joystick_layout(width, max_top, minimums.joystick, joystick_columns)
            .or_else(|| hat_layout(width, max_top, minimums.hat)),
    }
}

fn joystick_layout(
    width: u16,
    max_height: u16,
    min_height: u16,
    columns: usize,
) -> Option<TopRowLayout> {
    joystick_height_for_width(width, max_height, min_height, columns)
        .map(|height| TopRowLayout::Joystick { height })
}

fn hat_layout(width: u16, max_height: u16, min_height: u16) -> Option<TopRowLayout> {
    hat_height_for_width(width, max_height, min_height).map(|height| TopRowLayout::Hat { height })
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

    use super::{TopRowLayout, TopRowRequest, plan_top_row};
    use crate::device::monitor::layout::boxes::BoxMinimums;

    #[test]
    fn plan_top_row_prefers_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);
        let minimums = BoxMinimums::new(true, true, false, false, false);

        let plan = plan_top_row(area, 1, minimums, TopRowRequest::Both);

        assert!(matches!(plan, TopRowLayout::Split { height, .. } if height > 0));
    }

    #[test]
    fn plan_top_row_prefers_hat_when_only_hat_fits_split_layout() {
        let area = Rect::new(0, 0, 20, 12);
        let minimums = BoxMinimums::new(true, true, false, false, false);

        let plan = plan_top_row(area, 2, minimums, TopRowRequest::Both);

        assert!(matches!(plan, TopRowLayout::Hat { height } if height > 0));
    }

    #[test]
    fn plan_top_row_falls_back_to_joystick_when_neither_split_widget_fits() {
        let area = Rect::new(0, 0, 12, 6);
        let minimums = BoxMinimums::new(true, true, false, false, false);

        let plan = plan_top_row(area, 1, minimums, TopRowRequest::Both);

        assert!(matches!(plan, TopRowLayout::Joystick { height } if height > 0));
    }
}
