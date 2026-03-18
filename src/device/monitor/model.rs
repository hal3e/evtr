use std::collections::BTreeMap;

use evdev::{
    AbsoluteAxisCode, AttributeSetRef, Device, EventType, InputEvent, KeyCode, RelativeAxisCode,
};

use crate::device::monitor::{InitialStateLoad, config, math};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InputTypeId {
    Abs,
    Rel,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InputId(pub(crate) InputTypeId, pub(crate) u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AbsoluteState {
    Kernel { min: i32, max: i32, value: i32 },
    Fallback { min: i32, max: i32, value: i32 },
}

#[derive(Debug, Clone)]
pub(crate) enum InputKind {
    Absolute(AbsoluteState),
    Relative(i32),
    Button(bool),
}

impl InputKind {
    pub(crate) fn normalized(&self) -> f64 {
        match *self {
            Self::Absolute(AbsoluteState::Kernel { min, max, value })
            | Self::Absolute(AbsoluteState::Fallback { min, max, value }) => {
                math::normalize_range(value, min, max)
            }
            Self::Relative(value) => math::normalize_wrapped(value, config::RELATIVE_DISPLAY_RANGE),
            Self::Button(pressed) => (pressed as u8) as f64,
        }
    }

    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::Absolute(AbsoluteState::Kernel { value, .. })
            | Self::Absolute(AbsoluteState::Fallback { value, .. }) => value.to_string(),
            Self::Relative(value) => {
                math::wrapped_value(*value, config::RELATIVE_DISPLAY_RANGE).to_string()
            }
            Self::Button(pressed) => button_label(*pressed).to_string(),
        }
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        let value = event.value();
        match (self, event.event_type()) {
            (
                Self::Absolute(AbsoluteState::Kernel { value: current, .. })
                | Self::Absolute(AbsoluteState::Fallback { value: current, .. }),
                EventType::ABSOLUTE,
            ) => *current = value,
            (Self::Relative(v), EventType::RELATIVE) => {
                *v = v.saturating_add(value);
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct AbsoluteAxis {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) value: i32,
}

pub(crate) type InputsVec<'a> = Vec<&'a DeviceInput>;
pub(crate) type InputSlice<'a> = &'a [&'a DeviceInput];

pub(crate) struct InputCollection {
    inputs: BTreeMap<InputId, DeviceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AxisSnapshot {
    min: i32,
    max: i32,
    value: i32,
}

impl InputCollection {
    pub(crate) fn from_device(device: &Device) -> (Self, InitialStateLoad) {
        let mut inputs = BTreeMap::new();
        let mut initial_state_load = InitialStateLoad::Full;

        let abs_state = if device.supported_absolute_axes().is_some() {
            match device.get_abs_state() {
                Ok(state) => Some(state),
                Err(err) => {
                    initial_state_load.record_warning(format!(
                        "unable to load absolute axis state; using fallback defaults until events arrive: {err}"
                    ));
                    None
                }
            }
        } else {
            None
        };

        let key_state = if device.supported_keys().is_some() {
            match device.get_key_state() {
                Ok(state) => Some(state),
                Err(err) => {
                    initial_state_load.record_warning(format!(
                        "unable to load key/button state; buttons start released until events arrive: {err}"
                    ));
                    None
                }
            }
        } else {
            None
        };

        // Collect absolute axes
        if let Some(axes) = device.supported_absolute_axes() {
            for axis in axes.iter() {
                let code = axis.0;
                inputs.insert(
                    InputId(InputTypeId::Abs, code),
                    DeviceInput {
                        name: format!("{:?}", AbsoluteAxisCode(code)).to_lowercase(),
                        input_type: InputKind::Absolute(absolute_state_from_snapshot(
                            abs_state.as_ref().and_then(|state| {
                                state.get(code as usize).map(|info| AxisSnapshot {
                                    min: info.minimum,
                                    max: info.maximum,
                                    value: info.value,
                                })
                            }),
                        )),
                    },
                );
            }
        }

        // Collect relative axes
        if let Some(axes) = device.supported_relative_axes() {
            for axis in axes.iter() {
                let code = axis.0;
                inputs.insert(
                    InputId(InputTypeId::Rel, code),
                    DeviceInput {
                        name: format!("{:?}", RelativeAxisCode(code)).to_lowercase(),
                        input_type: InputKind::Relative(0),
                    },
                );
            }
        }

        // Collect buttons
        if let Some(keys) = device.supported_keys() {
            for key in keys.iter() {
                if is_touch_contact_button(key) {
                    continue;
                }
                let code = key.0;
                inputs.insert(
                    InputId(InputTypeId::Key, code),
                    DeviceInput {
                        name: strip_btn_prefix(&format!("{key:?}").to_lowercase()),
                        input_type: InputKind::Button(is_key_pressed(key, key_state.as_deref())),
                    },
                );
            }
        }

        (Self { inputs }, initial_state_load)
    }

    pub(crate) fn handle_event(&mut self, event: &InputEvent) {
        let kind = match event.event_type() {
            EventType::ABSOLUTE => Some(InputTypeId::Abs),
            EventType::RELATIVE => Some(InputTypeId::Rel),
            EventType::KEY => Some(InputTypeId::Key),
            _ => None,
        };
        if let Some(kind) = kind
            && let Some(input) = self.inputs.get_mut(&InputId(kind, event.code()))
        {
            input.input_type.update(event);
        }
    }

    pub(crate) fn reset_relative_axes(&mut self) {
        for input in self.inputs.values_mut() {
            if let InputKind::Relative(v) = &mut input.input_type {
                *v = 0;
            }
        }
    }

    pub(crate) fn absolute_axis(&self, code: AbsoluteAxisCode) -> Option<AbsoluteAxis> {
        self.inputs
            .get(&InputId(InputTypeId::Abs, code.0))
            .and_then(|input| match input.input_type {
                InputKind::Absolute(AbsoluteState::Kernel { min, max, value })
                | InputKind::Absolute(AbsoluteState::Fallback { min, max, value }) => {
                    Some(AbsoluteAxis { min, max, value })
                }
                _ => None,
            })
    }

    pub(crate) fn absolute_axis_pair(
        &self,
        x: AbsoluteAxisCode,
        y: AbsoluteAxisCode,
    ) -> Option<(AbsoluteAxis, AbsoluteAxis)> {
        Some((self.absolute_axis(x)?, self.absolute_axis(y)?))
    }

    pub(crate) fn iter_absolute(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Absolute(_)))
    }

    pub(crate) fn iter_relative(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Relative(_)))
    }

    pub(crate) fn iter_buttons(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Button(_)))
    }
}

fn absolute_state_from_snapshot(snapshot: Option<AxisSnapshot>) -> AbsoluteState {
    if let Some(snapshot) = snapshot {
        AbsoluteState::Kernel {
            min: snapshot.min,
            max: snapshot.max,
            value: snapshot.value,
        }
    } else {
        AbsoluteState::Fallback {
            min: config::DEFAULT_AXIS_RANGE.0,
            max: config::DEFAULT_AXIS_RANGE.1,
            value: 0,
        }
    }
}

fn is_key_pressed(code: KeyCode, key_state: Option<&AttributeSetRef<KeyCode>>) -> bool {
    key_state.is_some_and(|state| state.contains(code))
}

fn is_touch_contact_button(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::BTN_TOUCH
            | KeyCode::BTN_TOOL_FINGER
            | KeyCode::BTN_TOOL_DOUBLETAP
            | KeyCode::BTN_TOOL_TRIPLETAP
            | KeyCode::BTN_TOOL_QUADTAP
            | KeyCode::BTN_TOOL_QUINTTAP
    )
}

fn strip_btn_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("btn_") {
        rest.to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use evdev::{AttributeSet, KeyCode};

    use super::{
        AbsoluteState, AxisSnapshot, absolute_state_from_snapshot, is_key_pressed,
        is_touch_contact_button,
    };
    use crate::device::monitor::config;

    #[test]
    fn absolute_state_from_snapshot_uses_kernel_values() {
        assert_eq!(
            absolute_state_from_snapshot(Some(AxisSnapshot {
                min: -10,
                max: 20,
                value: 7,
            })),
            AbsoluteState::Kernel {
                min: -10,
                max: 20,
                value: 7,
            }
        );
    }

    #[test]
    fn absolute_state_from_snapshot_uses_explicit_fallback_defaults() {
        assert_eq!(
            absolute_state_from_snapshot(None),
            AbsoluteState::Fallback {
                min: config::DEFAULT_AXIS_RANGE.0,
                max: config::DEFAULT_AXIS_RANGE.1,
                value: 0,
            }
        );
    }

    #[test]
    fn is_key_pressed_reads_initial_button_state() {
        let mut keys = AttributeSet::new();
        keys.insert(KeyCode::BTN_SOUTH);

        assert!(is_key_pressed(KeyCode::BTN_SOUTH, Some(&keys)));
        assert!(!is_key_pressed(KeyCode::BTN_EAST, Some(&keys)));
        assert!(!is_key_pressed(KeyCode::BTN_SOUTH, None));
    }

    #[test]
    fn is_touch_contact_button_filters_touch_contact_keys() {
        assert!(is_touch_contact_button(KeyCode::BTN_TOUCH));
        assert!(is_touch_contact_button(KeyCode::BTN_TOOL_FINGER));
        assert!(is_touch_contact_button(KeyCode::BTN_TOOL_DOUBLETAP));
        assert!(!is_touch_contact_button(KeyCode::BTN_LEFT));
        assert!(!is_touch_contact_button(KeyCode::BTN_SOUTH));
    }
}
