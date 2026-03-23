use evdev::{Device, PropType};

use super::types::{TouchMode, TouchRange, preferred_touch_contact_key, touch_axes};

pub(super) struct TouchBootstrap {
    pub(super) mode: TouchMode,
    pub(super) slot_limit: Option<usize>,
    pub(super) x_range: TouchRange,
    pub(super) y_range: TouchRange,
    pub(super) startup_warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct TouchCapabilities {
    supports_mt_position: bool,
    supports_slot: bool,
    supports_abs_position: bool,
    has_touch_properties: bool,
    contact_key: Option<evdev::KeyCode>,
}

impl TouchCapabilities {
    fn from_device(device: &Device) -> Option<Self> {
        let axes = device.supported_absolute_axes()?;
        let properties = device.properties();

        Some(Self {
            supports_mt_position: axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_X)
                && axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_Y),
            supports_slot: axes.contains(evdev::AbsoluteAxisCode::ABS_MT_SLOT),
            supports_abs_position: axes.contains(evdev::AbsoluteAxisCode::ABS_X)
                && axes.contains(evdev::AbsoluteAxisCode::ABS_Y),
            has_touch_properties: properties.contains(PropType::DIRECT)
                || properties.contains(PropType::BUTTONPAD)
                || properties.contains(PropType::SEMI_MT)
                || properties.contains(PropType::TOPBUTTONPAD),
            contact_key: device
                .supported_keys()
                .and_then(preferred_touch_contact_key),
        })
    }

    fn mode(self) -> TouchMode {
        if self.supports_mt_position {
            TouchMode::MultiTouch {
                has_slot: self.supports_slot,
            }
        } else if self.supports_abs_position
            && (self.has_touch_properties || self.contact_key.is_some())
        {
            TouchMode::SingleTouch {
                contact_key: self.contact_key,
            }
        } else {
            TouchMode::None
        }
    }
}

pub(super) fn inspect_touch_device(device: &Device) -> Option<TouchBootstrap> {
    let capabilities = TouchCapabilities::from_device(device)?;
    let mode = capabilities.mode();
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

    if let Some((x_axis, y_axis)) = touch_axes(mode) {
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
        if matches!(mode, TouchMode::MultiTouch { has_slot: true })
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

#[cfg(test)]
mod tests {
    use evdev::KeyCode;

    use super::TouchCapabilities;
    use crate::device::monitor::touch::types::TouchMode;

    #[test]
    fn touch_capabilities_prefer_multi_touch_when_available() {
        let capabilities = TouchCapabilities {
            supports_mt_position: true,
            supports_slot: true,
            supports_abs_position: true,
            has_touch_properties: true,
            contact_key: Some(KeyCode::BTN_TOUCH),
        };

        assert_eq!(
            capabilities.mode(),
            TouchMode::MultiTouch { has_slot: true }
        );
    }

    #[test]
    fn touch_capabilities_require_touch_hint_for_single_touch() {
        let without_hint = TouchCapabilities {
            supports_mt_position: false,
            supports_slot: false,
            supports_abs_position: true,
            has_touch_properties: false,
            contact_key: None,
        };
        let with_key = TouchCapabilities {
            contact_key: Some(KeyCode::BTN_TOUCH),
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
