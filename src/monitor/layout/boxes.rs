mod split;
mod top_row;

use ratatui::layout::Rect;

pub(crate) use self::split::split_buttons_column;
use self::top_row::{TopRowRequest, place_top_row, plan_top_row};
use crate::monitor::config;

pub(crate) struct BoxLayout {
    pub(crate) joystick_box: Option<Rect>,
    pub(crate) hat_box: Option<Rect>,
    pub(crate) axes_box: Option<Rect>,
    pub(crate) touch_box: Option<Rect>,
    pub(crate) buttons_box: Option<Rect>,
}

#[derive(Clone, Copy)]
pub(crate) struct LayoutRequest {
    joystick: Option<JoystickPanel>,
    hat: Option<HatPanel>,
    touch: Option<TouchPanel>,
    axes: Option<AxesPanel>,
    buttons: Option<ButtonsPanel>,
}

#[derive(Clone, Copy)]
pub(crate) struct JoystickPanel {
    columns: usize,
}

impl JoystickPanel {
    pub(crate) fn new(columns: usize) -> Self {
        Self { columns }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct HatPanel;

impl HatPanel {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TouchPanel;

impl TouchPanel {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AxesPanel;

impl AxesPanel {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ButtonsPanel;

impl ButtonsPanel {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl LayoutRequest {
    pub(crate) fn new(
        joystick: Option<JoystickPanel>,
        hat: Option<HatPanel>,
        touch: Option<TouchPanel>,
        axes: Option<AxesPanel>,
        buttons: Option<ButtonsPanel>,
    ) -> Self {
        Self {
            joystick,
            hat,
            touch,
            axes,
            buttons,
        }
    }

    fn top_row_request(self) -> TopRowRequest {
        TopRowRequest::new(self.joystick, self.hat)
    }

    fn joystick_columns(self) -> usize {
        self.joystick.map_or(0, |panel| panel.columns)
    }

    fn has_joystick(self) -> bool {
        self.joystick.is_some()
    }

    fn has_hat(self) -> bool {
        self.hat.is_some()
    }

    fn has_touch(self) -> bool {
        self.touch.is_some()
    }

    fn has_axes(self) -> bool {
        self.axes.is_some()
    }

    fn has_buttons(self) -> bool {
        self.buttons.is_some()
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
    fn for_request(request: LayoutRequest) -> Self {
        Self {
            axes: min_box_height(request.has_axes(), 2),
            buttons: min_box_height(request.has_buttons(), 1),
            touch: min_box_height(request.has_touch(), config::TOUCHPAD_MIN_HEIGHT + 2),
            joystick: min_box_height(request.has_joystick(), config::JOYSTICK_MIN_SIZE),
            hat: min_box_height(request.has_hat(), config::HAT_MIN_SIZE),
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
    fn plan(
        remaining_height: u16,
        request: LayoutRequest,
        minimums: BoxMinimums,
        axes_box_percent: u16,
    ) -> Self {
        match (request.has_axes(), request.has_buttons()) {
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
                    let desired_axes = (remaining_height * axes_box_percent) / 100;
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

pub(crate) fn box_layout(area: Rect, request: LayoutRequest) -> BoxLayout {
    let minimums = BoxMinimums::for_request(request);
    let joystick_hat_joystick_percent = config::joystick_hat_joystick_percent();
    let top_row = plan_top_row(
        area,
        request.joystick_columns(),
        minimums,
        request.top_row_request(),
        joystick_hat_joystick_percent,
    );

    let mut budget = LayoutBudget::new(area.height);
    budget.reserve(top_row.height());

    let touch = plan_touch(budget, request, minimums);
    budget.reserve(touch.height());

    let lower = LowerSectionLayout::plan(
        budget.remaining(),
        request,
        minimums,
        config::axes_box_percent(),
    );

    let mut cursor_y = area.y;
    let (joystick_box, hat_box) =
        place_top_row(area, &mut cursor_y, top_row, joystick_hat_joystick_percent);
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

fn plan_touch(
    budget: LayoutBudget,
    request: LayoutRequest,
    minimums: BoxMinimums,
) -> TouchAllocation {
    if !request.has_touch() {
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

    use super::{
        AxesPanel, ButtonsPanel, HatPanel, JoystickPanel, LayoutBudget, LayoutRequest,
        LowerSectionLayout, TouchPanel, box_layout, plan_touch,
    };
    use crate::monitor::config;

    fn request(
        joystick: Option<JoystickPanel>,
        hat: Option<HatPanel>,
        touch: Option<TouchPanel>,
        axes: Option<AxesPanel>,
        buttons: Option<ButtonsPanel>,
    ) -> LayoutRequest {
        LayoutRequest::new(joystick, hat, touch, axes, buttons)
    }

    #[test]
    fn box_layout_gives_small_remaining_space_to_buttons_first() {
        let area = Rect::new(0, 0, 60, 1);

        let layout = box_layout(
            area,
            request(
                None,
                None,
                None,
                Some(AxesPanel::new()),
                Some(ButtonsPanel::new()),
            ),
        );

        assert!(layout.axes_box.is_none());
        assert_eq!(layout.buttons_box, Some(area));
    }

    #[test]
    fn box_layout_drops_touch_when_it_cannot_fit_with_axes_and_buttons() {
        let area = Rect::new(0, 0, 60, config::TOUCHPAD_MIN_HEIGHT + 2);

        let layout = box_layout(
            area,
            request(
                None,
                None,
                Some(TouchPanel::new()),
                Some(AxesPanel::new()),
                Some(ButtonsPanel::new()),
            ),
        );

        assert!(layout.touch_box.is_none());
        assert!(layout.axes_box.is_some());
        assert!(layout.buttons_box.is_some());
    }

    #[test]
    fn box_layout_keeps_joystick_and_hat_side_by_side_when_both_fit() {
        let area = Rect::new(0, 0, 60, 12);

        let layout = box_layout(
            area,
            request(
                Some(JoystickPanel::new(1)),
                Some(HatPanel::new()),
                None,
                None,
                None,
            ),
        );

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_some());
    }

    #[test]
    fn box_layout_falls_back_to_joystick_when_hat_cannot_fit() {
        let area = Rect::new(0, 0, 12, 6);

        let layout = box_layout(
            area,
            request(
                Some(JoystickPanel::new(1)),
                Some(HatPanel::new()),
                None,
                None,
                None,
            ),
        );

        assert!(layout.joystick_box.is_some());
        assert!(layout.hat_box.is_none());
    }

    #[test]
    fn touch_allocation_uses_full_remaining_height_when_it_is_the_only_section() {
        let request = request(None, None, Some(TouchPanel::new()), None, None);
        let minimums = super::BoxMinimums::for_request(request);

        let touch = plan_touch(LayoutBudget::new(12), request, minimums);

        assert_eq!(touch, super::TouchAllocation::shown(12));
    }

    #[test]
    fn lower_section_layout_prioritizes_buttons_when_budget_is_tight() {
        let request = request(
            None,
            None,
            None,
            Some(AxesPanel::new()),
            Some(ButtonsPanel::new()),
        );
        let minimums = super::BoxMinimums::for_request(request);

        let lower = LowerSectionLayout::plan(1, request, minimums, 75);

        assert_eq!(lower.axes_height(), 0);
        assert_eq!(lower.buttons_height(), 1);
    }

    #[test]
    fn lower_section_layout_respects_axes_box_percent() {
        let request = request(
            None,
            None,
            None,
            Some(AxesPanel::new()),
            Some(ButtonsPanel::new()),
        );
        let minimums = super::BoxMinimums::for_request(request);

        let lower = LowerSectionLayout::plan(20, request, minimums, 60);

        assert_eq!(lower.axes_height(), 12);
        assert_eq!(lower.buttons_height(), 8);
    }
}
