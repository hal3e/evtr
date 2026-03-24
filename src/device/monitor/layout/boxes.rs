mod split;
mod top_row;

use ratatui::layout::Rect;

use crate::device::monitor::config;

pub(crate) use self::split::split_buttons_column;

use self::top_row::{TopRowRequest, place_top_row, plan_top_row};

pub(crate) struct BoxLayout {
    pub(crate) joystick_box: Option<Rect>,
    pub(crate) hat_box: Option<Rect>,
    pub(crate) axes_box: Option<Rect>,
    pub(crate) touch_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

#[derive(Clone, Copy)]
pub(crate) struct BoxRequest {
    pub(crate) joystick_columns: usize,
    pub(crate) joystick_present: bool,
    pub(crate) hat_present: bool,
    pub(crate) touch_present: bool,
    pub(crate) axes_present: bool,
    pub(crate) buttons_present: bool,
}

impl BoxRequest {
    pub(crate) fn new(
        joystick_present: bool,
        joystick_columns: usize,
        hat_present: bool,
        touch_present: bool,
        axes_present: bool,
        buttons_present: bool,
    ) -> Self {
        Self {
            joystick_columns,
            joystick_present,
            hat_present,
            touch_present,
            axes_present,
            buttons_present,
        }
    }

    fn top_row_request(self) -> TopRowRequest {
        TopRowRequest::new(self.joystick_present, self.hat_present)
    }
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
    fn for_request(request: BoxRequest) -> Self {
        Self {
            axes: min_box_height(request.axes_present, 2),
            buttons: min_box_height(request.buttons_present, 1),
            touch: min_box_height(request.touch_present, config::TOUCHPAD_MIN_HEIGHT + 2),
            joystick: min_box_height(request.joystick_present, config::JOYSTICK_MIN_SIZE),
            hat: min_box_height(request.hat_present, config::HAT_MIN_SIZE),
        }
    }

    fn reserved_below_top(self) -> u16 {
        self.axes + self.buttons + self.touch
    }

    fn reserved_below_touch(self) -> u16 {
        self.axes + self.buttons
    }
}

#[derive(Clone, Copy)]
struct LayoutBudget {
    remaining_height: u16,
}

impl LayoutBudget {
    fn new(total_height: u16) -> Self {
        Self {
            remaining_height: total_height,
        }
    }

    fn reserve(&mut self, height: u16) {
        self.remaining_height = self.remaining_height.saturating_sub(height);
    }

    fn remaining(self) -> u16 {
        self.remaining_height
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TouchAllocation {
    visible: bool,
    height: u16,
}

impl TouchAllocation {
    fn hidden() -> Self {
        Self::default()
    }

    fn shown(height: u16) -> Self {
        Self {
            visible: true,
            height,
        }
    }

    fn is_visible(self) -> bool {
        self.visible
    }

    fn height(self) -> u16 {
        self.height
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct LowerSectionLayout {
    axes_height: u16,
    buttons_height: u16,
}

impl LowerSectionLayout {
    fn plan(remaining_height: u16, request: BoxRequest, minimums: BoxMinimums) -> Self {
        match (request.axes_present, request.buttons_present) {
            (true, true) => {
                if remaining_height < minimums.axes + minimums.buttons {
                    if remaining_height >= minimums.buttons {
                        Self {
                            axes_height: 0,
                            buttons_height: remaining_height,
                        }
                    } else {
                        Self::default()
                    }
                } else {
                    let desired_axes = (remaining_height * config::AXES_BOX_PERCENT) / 100;
                    let max_axes = remaining_height.saturating_sub(minimums.buttons);
                    let axes_height = desired_axes.clamp(minimums.axes, max_axes);
                    Self {
                        axes_height,
                        buttons_height: remaining_height.saturating_sub(axes_height),
                    }
                }
            }
            (true, false) => {
                if remaining_height >= minimums.axes {
                    Self {
                        axes_height: remaining_height,
                        buttons_height: 0,
                    }
                } else {
                    Self::default()
                }
            }
            (false, true) => {
                if remaining_height >= minimums.buttons {
                    Self {
                        axes_height: 0,
                        buttons_height: remaining_height,
                    }
                } else {
                    Self::default()
                }
            }
            (false, false) => Self::default(),
        }
    }

    fn axes_height(self) -> u16 {
        self.axes_height
    }

    fn buttons_height(self) -> u16 {
        self.buttons_height
    }
}

pub(crate) fn box_layout(area: Rect, request: BoxRequest) -> BoxLayout {
    let minimums = BoxMinimums::for_request(request);
    let top_row = plan_top_row(
        area,
        request.joystick_columns,
        minimums,
        request.top_row_request(),
    );

    let mut budget = LayoutBudget::new(area.height);
    budget.reserve(top_row.height());

    let touch = plan_touch(budget, request, minimums);
    budget.reserve(touch.height());

    let lower = LowerSectionLayout::plan(budget.remaining(), request, minimums);

    let mut cursor_y = area.y;
    let (joystick_box, hat_box) = place_top_row(area, &mut cursor_y, top_row);
    let touch_box = if touch.is_visible() {
        take_next_box(area, &mut cursor_y, touch.height(), minimums.touch)
    } else {
        None
    };
    let axes_box = take_next_box(area, &mut cursor_y, lower.axes_height(), minimums.axes);
    let buttons_box = take_next_box(
        area,
        &mut cursor_y,
        lower.buttons_height(),
        minimums.buttons,
    );

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

fn plan_touch(budget: LayoutBudget, request: BoxRequest, minimums: BoxMinimums) -> TouchAllocation {
    if !request.touch_present {
        return TouchAllocation::hidden();
    }

    let remaining_after_top = budget.remaining();
    let min_other = minimums.reserved_below_touch();
    if min_other == 0 {
        return if remaining_after_top < minimums.touch {
            TouchAllocation::hidden()
        } else {
            TouchAllocation::shown(remaining_after_top)
        };
    }

    if remaining_after_top < min_other + minimums.touch {
        return TouchAllocation::hidden();
    }

    let preferred_touch = config::TOUCHPAD_HEIGHT + 2;
    let max_touch = remaining_after_top.saturating_sub(min_other);
    TouchAllocation::shown(preferred_touch.clamp(minimums.touch, max_touch))
}

fn take_next_box(area: Rect, cursor_y: &mut u16, height: u16, min_height: u16) -> Option<Rect> {
    if height < min_height || height == 0 {
        return None;
    }

    let rect = Rect::new(area.x, *cursor_y, area.width, height);
    *cursor_y = cursor_y.saturating_add(height);
    Some(rect)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{BoxRequest, LayoutBudget, LowerSectionLayout, box_layout, plan_touch};
    use crate::device::monitor::config;

    #[test]
    fn box_layout_gives_small_remaining_space_to_buttons_first() {
        let area = Rect::new(0, 0, 60, 1);

        let layout = box_layout(area, BoxRequest::new(false, 0, false, false, true, true));

        assert!(layout.axes_box.is_none());
        assert_eq!(layout.buttons_box, Some(area));
    }

    #[test]
    fn box_layout_drops_touch_when_it_cannot_fit_with_axes_and_buttons() {
        let area = Rect::new(0, 0, 60, config::TOUCHPAD_MIN_HEIGHT + 2);

        let layout = box_layout(area, BoxRequest::new(false, 0, false, true, true, true));

        assert!(layout.touch_box.is_none());
        assert!(layout.axes_box.is_some());
        assert!(layout.buttons_box.is_some());
    }

    #[test]
    fn box_layout_keeps_joystick_and_hat_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);

        let layout = box_layout(area, BoxRequest::new(true, 1, true, false, false, false));

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_some());
    }

    #[test]
    fn box_layout_falls_back_to_joystick_when_hat_cannot_fit() {
        let area = Rect::new(0, 0, 12, 6);

        let layout = box_layout(area, BoxRequest::new(true, 1, true, false, false, false));

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_none());
    }

    #[test]
    fn touch_allocation_uses_full_remaining_height_when_it_is_the_only_section() {
        let request = BoxRequest::new(false, 0, false, true, false, false);
        let minimums = super::BoxMinimums::for_request(request);

        let touch = plan_touch(LayoutBudget::new(12), request, minimums);

        assert_eq!(touch, super::TouchAllocation::shown(12));
    }

    #[test]
    fn lower_section_layout_prioritizes_buttons_when_budget_is_tight() {
        let request = BoxRequest::new(false, 0, false, false, true, true);
        let minimums = super::BoxMinimums::for_request(request);

        let lower = LowerSectionLayout::plan(1, request, minimums);

        assert_eq!(lower.axes_height(), 0);
        assert_eq!(lower.buttons_height(), 1);
    }
}
