use ratatui::layout::Rect;

use super::{
    BoxMinimums, HatPanel, JoystickPanel,
    split::{gap_if_room, ratio_widths, split_row_ratio},
};
use crate::monitor::config;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TopRowRequest {
    #[default]
    None,
    Joystick,
    Hat,
    Both,
}

impl TopRowRequest {
    pub(super) fn new(joystick: Option<JoystickPanel>, hat: Option<HatPanel>) -> Self {
        match (joystick, hat) {
            (Some(_), Some(_)) => Self::Both,
            (Some(_), None) => Self::Joystick,
            (None, Some(_)) => Self::Hat,
            (None, None) => Self::None,
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
    joystick_hat_joystick_percent: u16,
) -> TopRowLayout {
    let max_top = area.height.saturating_sub(minimums.reserved_below_top());
    match request {
        TopRowRequest::None => TopRowLayout::None,
        TopRowRequest::Joystick => {
            joystick_layout(area.width, max_top, minimums.joystick, joystick_columns)
                .unwrap_or_default()
        }
        TopRowRequest::Hat => hat_layout(area.width, max_top, minimums.hat).unwrap_or_default(),
        TopRowRequest::Both => plan_dual_top_row(
            area.width,
            max_top,
            joystick_columns,
            minimums,
            joystick_hat_joystick_percent,
        )
        .unwrap_or_default(),
    }
}

pub(super) fn place_top_row(
    area: Rect,
    cursor_y: &mut u16,
    layout: TopRowLayout,
    joystick_hat_joystick_percent: u16,
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
                joystick_hat_joystick_percent,
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
    joystick_hat_joystick_percent: u16,
) -> Option<TopRowLayout> {
    let gap = gap_if_room(width, config::joystick_gap());
    let (joystick_width, hat_width) = ratio_widths(width, gap, joystick_hat_joystick_percent);
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
    let gap = if columns > 1 {
        config::joystick_gap()
    } else {
        0
    };
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

    use super::{TopRowLayout, TopRowRequest, place_top_row, plan_top_row};
    use crate::monitor::layout::boxes::{BoxMinimums, HatPanel, JoystickPanel, LayoutRequest};

    fn dual_top_row_request() -> LayoutRequest {
        LayoutRequest::new(
            Some(JoystickPanel::new(1)),
            Some(HatPanel::new()),
            None,
            None,
            None,
        )
    }

    #[test]
    fn plan_top_row_prefers_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);
        let minimums = BoxMinimums::for_request(dual_top_row_request());

        let plan = plan_top_row(area, 1, minimums, TopRowRequest::Both, 70);

        assert!(matches!(plan, TopRowLayout::Split { height, .. } if height > 0));
    }

    #[test]
    fn plan_top_row_prefers_hat_when_only_hat_fits_split_layout() {
        let area = Rect::new(0, 0, 20, 12);
        let minimums = BoxMinimums::for_request(dual_top_row_request());

        let plan = plan_top_row(area, 2, minimums, TopRowRequest::Both, 70);

        assert!(matches!(plan, TopRowLayout::Hat { height } if height > 0));
    }

    #[test]
    fn plan_top_row_falls_back_to_joystick_when_neither_split_widget_fits() {
        let area = Rect::new(0, 0, 12, 6);
        let minimums = BoxMinimums::for_request(dual_top_row_request());

        let plan = plan_top_row(area, 1, minimums, TopRowRequest::Both, 70);

        assert!(matches!(plan, TopRowLayout::Joystick { height } if height > 0));
    }

    #[test]
    fn place_top_row_respects_joystick_hat_split_percent() {
        let area = Rect::new(0, 0, 40, 8);
        let layout = TopRowLayout::Split { height: 6, gap: 0 };
        let mut wide_cursor_y = area.y;
        let mut narrow_cursor_y = area.y;

        let (wide_joystick, wide_hat) = place_top_row(area, &mut wide_cursor_y, layout, 80);
        let (narrow_joystick, narrow_hat) = place_top_row(area, &mut narrow_cursor_y, layout, 60);

        assert!(wide_joystick.unwrap().width > narrow_joystick.unwrap().width);
        assert!(wide_hat.unwrap().width < narrow_hat.unwrap().width);
    }
}
