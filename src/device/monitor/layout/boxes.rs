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
        TopRowRequest::new(joystick_present, hat_present),
    );
    let remaining_after_top = area.height.saturating_sub(top_row.height());
    let (touch_present, touch_height) =
        allocate_touch(remaining_after_top, touch_present, minimums);
    let remaining_height = area.height.saturating_sub(top_row.height() + touch_height);
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

    use super::box_layout;
    use crate::device::monitor::config;

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
