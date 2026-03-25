use evdev::{EventType, InputEvent};

use crate::monitor::{config, math};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum InputTypeId {
    Abs,
    Rel,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct InputId {
    pub(crate) kind: InputTypeId,
    pub(crate) code: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AxisOrigin {
    Kernel,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbsoluteState {
    pub(crate) origin: AxisOrigin,
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) value: i32,
}

impl AbsoluteState {
    pub(crate) fn kernel(min: i32, max: i32, value: i32) -> Self {
        Self {
            origin: AxisOrigin::Kernel,
            min,
            max,
            value,
        }
    }

    pub(crate) fn fallback(min: i32, max: i32, value: i32) -> Self {
        Self {
            origin: AxisOrigin::Fallback,
            min,
            max,
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputKind {
    Absolute(AbsoluteState),
    Relative(i32),
    Button(bool),
}

impl InputKind {
    pub(crate) fn normalized(&self) -> f64 {
        match *self {
            Self::Absolute(state) => math::normalize_range(state.value, state.min, state.max),
            Self::Relative(value) => math::normalize_wrapped(value, config::RELATIVE_DISPLAY_RANGE),
            Self::Button(pressed) => (pressed as u8) as f64,
        }
    }

    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::Absolute(state) => state.value.to_string(),
            Self::Relative(value) => {
                math::wrapped_value(*value, config::RELATIVE_DISPLAY_RANGE).to_string()
            }
            Self::Button(pressed) => button_label(*pressed).to_string(),
        }
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        let value = event.value();
        match (self, event.event_type()) {
            (Self::Absolute(state), EventType::ABSOLUTE) => state.value = value,
            (Self::Relative(current), EventType::RELATIVE) => {
                *current = current.saturating_add(value);
            }
            (Self::Button(pressed), EventType::KEY) => *pressed = value != 0,
            _ => {}
        }
    }
}

fn button_label(pressed: bool) -> &'static str {
    if pressed { "ON" } else { "OFF" }
}

#[derive(Debug, Clone)]
pub(crate) struct DeviceInput {
    pub(crate) name: String,
    pub(crate) input_type: InputKind,
}

impl DeviceInput {
    pub(super) fn absolute(name: String, state: AbsoluteState) -> Self {
        Self {
            name,
            input_type: InputKind::Absolute(state),
        }
    }

    pub(super) fn relative(name: String) -> Self {
        Self {
            name,
            input_type: InputKind::Relative(0),
        }
    }

    pub(super) fn button(name: String, pressed: bool) -> Self {
        Self {
            name,
            input_type: InputKind::Button(pressed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbsoluteAxis {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) value: i32,
}

pub(crate) type InputSlice<'a> = &'a [DeviceInput];

impl InputId {
    pub(crate) fn new(kind: InputTypeId, code: u16) -> Self {
        Self { kind, code }
    }

    pub(crate) fn absolute(code: u16) -> Self {
        Self::new(InputTypeId::Abs, code)
    }

    pub(crate) fn relative(code: u16) -> Self {
        Self::new(InputTypeId::Rel, code)
    }

    pub(crate) fn key(code: u16) -> Self {
        Self::new(InputTypeId::Key, code)
    }

    pub(crate) fn from_event(event: &InputEvent) -> Option<Self> {
        let kind = match event.event_type() {
            EventType::ABSOLUTE => InputTypeId::Abs,
            EventType::RELATIVE => InputTypeId::Rel,
            EventType::KEY => InputTypeId::Key,
            _ => return None,
        };

        Some(Self::new(kind, event.code()))
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AbsoluteAxisCode, EventType, InputEvent};

    use super::{AbsoluteState, AxisOrigin, InputId, InputKind, InputTypeId};

    #[test]
    fn absolute_state_preserves_origin() {
        assert_eq!(AbsoluteState::kernel(-1, 1, 0).origin, AxisOrigin::Kernel);
        assert_eq!(
            AbsoluteState::fallback(-1, 1, 0).origin,
            AxisOrigin::Fallback
        );
    }

    #[test]
    fn input_id_helpers_use_named_fields() {
        assert_eq!(
            InputId::absolute(1),
            InputId {
                kind: InputTypeId::Abs,
                code: 1,
            }
        );
        assert_eq!(
            InputId::relative(2),
            InputId {
                kind: InputTypeId::Rel,
                code: 2,
            }
        );
        assert_eq!(
            InputId::key(3),
            InputId {
                kind: InputTypeId::Key,
                code: 3,
            }
        );
    }

    #[test]
    fn input_kind_update_routes_by_event_type() {
        let mut absolute = InputKind::Absolute(AbsoluteState::kernel(-1, 1, 0));
        let mut relative = InputKind::Relative(0);
        let mut button = InputKind::Button(false);

        absolute.update(&InputEvent::new(
            EventType::ABSOLUTE.0,
            AbsoluteAxisCode::ABS_X.0,
            1,
        ));
        relative.update(&InputEvent::new(EventType::RELATIVE.0, 0, 3));
        button.update(&InputEvent::new(EventType::KEY.0, 0, 1));

        assert_eq!(
            absolute,
            InputKind::Absolute(AbsoluteState::kernel(-1, 1, 1))
        );
        assert_eq!(relative, InputKind::Relative(3));
        assert_eq!(button, InputKind::Button(true));
    }
}
