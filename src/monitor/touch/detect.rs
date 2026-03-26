use evdev::{Device, PropType};

use super::types::{MultiTouchSlots, TouchMode, TouchRange, preferred_touch_contact_key};

pub(super) struct TouchBootstrap {
    pub(super) mode: TouchMode,
    pub(super) slot_limit: Option<usize>,
    pub(super) x_range: TouchRange,
    pub(super) y_range: TouchRange,
    pub(super) startup_warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum TouchAxes {
    None,
    SingleTouch,
    MultiTouch { slots: MultiTouchSlots },
}

impl TouchAxes {
    fn from_device(device: &Device) -> Option<Self> {
        let axes = device.supported_absolute_axes()?;

        if axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_X)
            && axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_Y)
        {
            let slots = if axes.contains(evdev::AbsoluteAxisCode::ABS_MT_SLOT) {
                MultiTouchSlots::Explicit
            } else {
                MultiTouchSlots::ImplicitSingle
            };
            return Some(Self::MultiTouch { slots });
        }

        if axes.contains(evdev::AbsoluteAxisCode::ABS_X)
            && axes.contains(evdev::AbsoluteAxisCode::ABS_Y)
        {
            return Some(Self::SingleTouch);
        }

        Some(Self::None)
    }
}

#[derive(Debug, Clone, Copy)]
enum TouchHint {
    None,
    Property,
    ContactKey(evdev::KeyCode),
}

impl TouchHint {
    fn from_device(device: &Device) -> Self {
        if let Some(contact_key) = device
            .supported_keys()
            .and_then(preferred_touch_contact_key)
        {
            return Self::ContactKey(contact_key);
        }

        if has_touch_properties(device.properties()) {
            Self::Property
        } else {
            Self::None
        }
    }

    fn indicates_touch(self) -> bool {
        !matches!(self, Self::None)
    }

    fn contact_key(self) -> Option<evdev::KeyCode> {
        match self {
            Self::ContactKey(key) => Some(key),
            Self::None | Self::Property => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TouchSignature {
    axes: TouchAxes,
    hint: TouchHint,
}

impl TouchSignature {
    fn from_device(device: &Device) -> Option<Self> {
        Some(Self {
            axes: TouchAxes::from_device(device)?,
            hint: TouchHint::from_device(device),
        })
    }

    fn mode(self) -> TouchMode {
        match self.axes {
            TouchAxes::MultiTouch { slots } => TouchMode::MultiTouch { slots },
            TouchAxes::SingleTouch if self.hint.indicates_touch() => TouchMode::SingleTouch {
                contact_key: self.hint.contact_key(),
            },
            TouchAxes::None | TouchAxes::SingleTouch => TouchMode::None,
        }
    }
}

pub(super) fn inspect_touch_device(device: &Device) -> Option<TouchBootstrap> {
    let signature = TouchSignature::from_device(device)?;
    let mode = signature.mode();
    if matches!(mode, TouchMode::None) {
        return None;
    }

    let mut startup_warnings = Vec::new();
    let abs_state = match device.get_abs_state() {
        Ok(state) => Some(state),
        Err(err) => {
            startup_warnings.push(format!(
                "unable to load touch axis state; inferring touch bounds from incoming events: {err}"
            ));
            None
        }
    };

    let mut x_range = TouchRange::Unknown;
    let mut y_range = TouchRange::Unknown;
    let mut slot_limit = None;

    if let Some((x_axis, y_axis)) = mode.axes() {
        let axis_state = |axis: evdev::AbsoluteAxisCode| {
            abs_state
                .as_ref()
                .and_then(|state| state.get(axis.0 as usize))
        };

        if let Some(info) = axis_state(x_axis) {
            x_range = TouchRange::fixed(info.minimum, info.maximum);
        }
        if let Some(info) = axis_state(y_axis) {
            y_range = TouchRange::fixed(info.minimum, info.maximum);
        }
        if mode.uses_explicit_slots()
            && let Some(info) = axis_state(evdev::AbsoluteAxisCode::ABS_MT_SLOT)
        {
            slot_limit = Some(info.maximum.max(0) as usize + 1);
        }
    }

    if abs_state.is_some() && (!x_range.is_known() || !y_range.is_known()) {
        startup_warnings.push(
            "touch position range is unavailable; inferring touch bounds from incoming events"
                .to_string(),
        );
    }

    Some(TouchBootstrap {
        mode,
        slot_limit,
        x_range,
        y_range,
        startup_warnings,
    })
}

fn has_touch_properties(properties: &evdev::AttributeSetRef<PropType>) -> bool {
    properties.contains(PropType::DIRECT)
        || properties.contains(PropType::BUTTONPAD)
        || properties.contains(PropType::SEMI_MT)
        || properties.contains(PropType::TOPBUTTONPAD)
}

#[cfg(test)]
mod tests {
    use evdev::KeyCode;

    use super::{TouchAxes, TouchHint, TouchSignature};
    use crate::monitor::touch::types::{MultiTouchSlots, TouchMode};

    #[test]
    fn touch_signature_prefers_multi_touch_when_available() {
        let signature = TouchSignature {
            axes: TouchAxes::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            },
            hint: TouchHint::ContactKey(KeyCode::BTN_TOUCH),
        };

        assert_eq!(
            signature.mode(),
            TouchMode::MultiTouch {
                slots: MultiTouchSlots::Explicit,
            }
        );
    }

    #[test]
    fn touch_signature_requires_touch_hint_for_single_touch() {
        let without_hint = TouchSignature {
            axes: TouchAxes::SingleTouch,
            hint: TouchHint::None,
        };
        let with_key = TouchSignature {
            hint: TouchHint::ContactKey(KeyCode::BTN_TOUCH),
            ..without_hint
        };

        assert_eq!(without_hint.mode(), TouchMode::None);
        assert_eq!(
            with_key.mode(),
            TouchMode::SingleTouch {
                contact_key: Some(KeyCode::BTN_TOUCH),
            }
        );
    }
}
