use std::collections::BTreeMap;

use evdev::{AbsoluteAxisCode, Device, EventType, InputEvent, RelativeAxisCode};

use crate::device::monitor::{config, math};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InputTypeId {
    Abs,
    Rel,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InputId(pub(crate) InputTypeId, pub(crate) u16);

#[derive(Debug, Clone)]
pub(crate) enum InputKind {
    Absolute { min: i32, max: i32, value: i32 },
    Relative(i32),
    Button(bool),
}

impl InputKind {
    pub(crate) fn normalized(&self) -> f64 {
        match *self {
            Self::Absolute { min, max, value } => math::normalize_range(value, min, max),
            Self::Relative(value) => math::normalize_wrapped(value, config::RELATIVE_DISPLAY_RANGE),
            Self::Button(pressed) => (pressed as u8) as f64,
        }
    }

    pub(crate) fn display_label(&self) -> String {
        match self {
            Self::Absolute { value, .. } => value.to_string(),
            Self::Relative(value) => {
                math::wrapped_value(*value, config::RELATIVE_DISPLAY_RANGE).to_string()
            }
            Self::Button(pressed) => button_label(*pressed).to_string(),
        }
    }

    pub(crate) fn update(&mut self, event: &InputEvent) {
        let value = event.value();
        match (self, event.event_type()) {
            (Self::Absolute { value: v, .. }, EventType::ABSOLUTE) => *v = value,
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

pub(crate) type InputsVec<'a> = Vec<&'a DeviceInput>;
pub(crate) type InputSlice<'a> = &'a [&'a DeviceInput];

pub(crate) struct InputCollection {
    inputs: BTreeMap<InputId, DeviceInput>,
}

impl InputCollection {
    pub(crate) fn from_device(device: &Device) -> Self {
        let mut inputs = BTreeMap::new();

        // Collect absolute axes
        if let Some(axes) = device.supported_absolute_axes() {
            let abs_state = device.get_abs_state().ok();
            for axis in axes.iter() {
                let code = axis.0;
                let (min, max, value) = abs_state
                    .as_ref()
                    .and_then(|s| s.get(code as usize))
                    .map(|info| (info.minimum, info.maximum, info.value))
                    .unwrap_or((
                        config::DEFAULT_AXIS_RANGE.0,
                        config::DEFAULT_AXIS_RANGE.1,
                        0,
                    ));

                inputs.insert(
                    InputId(InputTypeId::Abs, code),
                    DeviceInput {
                        name: format!("{:?}", AbsoluteAxisCode(code)).to_lowercase(),
                        input_type: InputKind::Absolute { min, max, value },
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
                let code = key.0;
                inputs.insert(
                    InputId(InputTypeId::Key, code),
                    DeviceInput {
                        name: strip_btn_prefix(&format!("{key:?}").to_lowercase()),
                        input_type: InputKind::Button(false),
                    },
                );
            }
        }

        Self { inputs }
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

    pub(crate) fn iter_absolute(&self) -> impl Iterator<Item = &DeviceInput> {
        self.inputs
            .values()
            .filter(|input| matches!(input.input_type, InputKind::Absolute { .. }))
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

fn strip_btn_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("btn_") {
        rest.to_string()
    } else {
        name.to_string()
    }
}
