use evdev::AbsoluteAxisCode;

use crate::monitor::{
    config,
    model::{AbsoluteAxis, InputCollection},
    plan::Counts,
    touch::TouchState,
};

#[derive(Clone, Copy)]
pub(super) struct StickState {
    pub(super) x: AbsoluteAxis,
    pub(super) y: AbsoluteAxis,
}

#[derive(Default)]
pub(super) struct JoystickState {
    pub(super) left: Option<StickState>,
    pub(super) right: Option<StickState>,
}

impl JoystickState {
    pub(super) fn from_axes(
        left: Option<(AbsoluteAxis, AbsoluteAxis)>,
        right: Option<(AbsoluteAxis, AbsoluteAxis)>,
    ) -> Self {
        Self {
            left: left.map(|(x, y)| StickState { x, y }),
            right: right.map(|(x, y)| StickState { x, y }),
        }
    }

    pub(super) fn count(&self) -> usize {
        self.left.is_some() as usize + self.right.is_some() as usize
    }
}

#[derive(Clone, Copy)]
pub(super) struct HatState {
    pub(super) x: i32,
    pub(super) y: i32,
}

impl HatState {
    pub(super) fn from_axes(x: AbsoluteAxis, y: AbsoluteAxis, invert_y: bool) -> Self {
        let y = if invert_y {
            -sign(y.value)
        } else {
            sign(y.value)
        };

        Self {
            x: sign(x.value),
            y,
        }
    }
}

pub(super) struct MonitorViewModel {
    joystick: JoystickState,
    hat_state: Option<HatState>,
    axes_available: bool,
    touch_enabled: bool,
    buttons_available: bool,
}

impl MonitorViewModel {
    pub(super) fn from_inputs(
        counts: Counts,
        inputs: &InputCollection,
        touch: &TouchState,
        invert_y: bool,
    ) -> Self {
        let joystick = if touch.is_touch_device() {
            JoystickState::default()
        } else {
            JoystickState::from_axes(
                inputs.absolute_axis_pair(AbsoluteAxisCode::ABS_X, AbsoluteAxisCode::ABS_Y),
                inputs.absolute_axis_pair(AbsoluteAxisCode::ABS_RX, AbsoluteAxisCode::ABS_RY),
            )
        };
        let hat_state = if touch.is_touch_device() {
            None
        } else {
            inputs
                .absolute_axis_pair(AbsoluteAxisCode::ABS_HAT0X, AbsoluteAxisCode::ABS_HAT0Y)
                .map(|(x, y)| HatState::from_axes(x, y, invert_y))
        };

        Self {
            joystick,
            hat_state,
            axes_available: counts.total_axes() > 0,
            touch_enabled: touch.enabled(),
            buttons_available: counts.has_buttons(),
        }
    }

    pub(super) fn joystick(&self) -> &JoystickState {
        &self.joystick
    }

    pub(super) fn hat_state(&self) -> Option<HatState> {
        self.hat_state
    }

    pub(super) fn joystick_count(&self) -> usize {
        self.joystick.count()
    }

    pub(super) fn joystick_present(&self) -> bool {
        self.joystick_count() > 0
    }

    pub(super) fn hat_present(&self) -> bool {
        self.hat_state.is_some()
    }

    pub(super) fn axes_available(&self) -> bool {
        self.axes_available
    }

    pub(super) fn touch_enabled(&self) -> bool {
        self.touch_enabled
    }

    pub(super) fn buttons_available(&self) -> bool {
        self.buttons_available
    }

    pub(super) fn main_min_width(&self) -> u16 {
        let mut width = config::MAIN_COLUMN_MIN_WIDTH;
        if self.axes_available {
            width = width.max(config::AXIS_MIN_WIDTH);
        }
        if self.touch_enabled {
            width = width.max(config::TOUCHPAD_MIN_WIDTH);
        }
        if self.joystick_present() {
            width = width.max(config::JOYSTICK_MIN_SIZE);
        }
        if self.hat_present() {
            width = width.max(config::HAT_MIN_SIZE);
        }
        width
    }
}

fn sign(value: i32) -> i32 {
    if value > 0 {
        1
    } else if value < 0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{HatState, JoystickState};
    use crate::monitor::model::AbsoluteAxis;

    #[test]
    fn joystick_count_tracks_visible_sticks() {
        let state = JoystickState::from_axes(
            Some((
                AbsoluteAxis {
                    min: -1,
                    max: 1,
                    value: 0,
                },
                AbsoluteAxis {
                    min: -1,
                    max: 1,
                    value: 0,
                },
            )),
            None,
        );

        assert_eq!(state.count(), 1);
    }

    #[test]
    fn hat_state_respects_invert_y() {
        let state = HatState::from_axes(
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: -1,
            },
            AbsoluteAxis {
                min: -1,
                max: 1,
                value: 1,
            },
            true,
        );

        assert_eq!(state.x, -1);
        assert_eq!(state.y, -1);
    }
}
