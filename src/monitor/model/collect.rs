use std::io;

use evdev::{AbsoluteAxisCode, AttributeSetRef, Device, KeyCode, RelativeAxisCode};

use super::types::{AbsoluteState, DeviceInput};
use crate::monitor::config;

pub(super) struct BootstrapEntries {
    pub(super) absolute: Vec<(u16, DeviceInput)>,
    pub(super) relative: Vec<(u16, DeviceInput)>,
    pub(super) buttons: Vec<(u16, DeviceInput)>,
    pub(super) startup_warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AxisSnapshot {
    min: i32,
    max: i32,
    value: i32,
}

pub(super) fn collect_device_inputs(device: &Device) -> BootstrapEntries {
    let mut startup_warnings = Vec::new();

    let abs_state = load_startup_state(
        device.supported_absolute_axes().is_some(),
        &mut startup_warnings,
        || device.get_abs_state(),
        |err| {
            format!(
                "unable to load absolute axis state; using fallback defaults until events arrive: {err}"
            )
        },
    );

    let key_state = load_startup_state(
        device.supported_keys().is_some(),
        &mut startup_warnings,
        || device.get_key_state(),
        |err| {
            format!(
                "unable to load key/button state; buttons start released until events arrive: {err}"
            )
        },
    );

    BootstrapEntries {
        absolute: absolute_entries(device, |code| {
            abs_state.as_ref().and_then(|state| {
                state.get(code as usize).map(|info| AxisSnapshot {
                    min: info.minimum,
                    max: info.maximum,
                    value: info.value,
                })
            })
        }),
        relative: relative_entries(device),
        buttons: button_entries(device, key_state.as_deref()),
        startup_warnings,
    }
}

fn absolute_entries(
    device: &Device,
    snapshot_for: impl Fn(u16) -> Option<AxisSnapshot>,
) -> Vec<(u16, DeviceInput)> {
    let mut absolute = Vec::new();

    if let Some(axes) = device.supported_absolute_axes() {
        for axis in axes.iter() {
            let code = axis.0;
            absolute.push((
                code,
                DeviceInput::absolute(
                    absolute_name(code),
                    absolute_state_from_snapshot(snapshot_for(code)),
                ),
            ));
        }
    }

    absolute
}

fn relative_entries(device: &Device) -> Vec<(u16, DeviceInput)> {
    let mut relative = Vec::new();

    if let Some(axes) = device.supported_relative_axes() {
        for axis in axes.iter() {
            let code = axis.0;
            relative.push((code, DeviceInput::relative(relative_name(code))));
        }
    }

    relative
}

fn button_entries(
    device: &Device,
    key_state: Option<&AttributeSetRef<KeyCode>>,
) -> Vec<(u16, DeviceInput)> {
    let mut buttons = Vec::new();

    if let Some(keys) = device.supported_keys() {
        for key in keys.iter() {
            if is_touch_contact_button(key) {
                continue;
            }

            let code = key.0;
            buttons.push((
                code,
                DeviceInput::button(button_name(key), is_key_pressed(key, key_state)),
            ));
        }
    }

    buttons
}

fn absolute_state_from_snapshot(snapshot: Option<AxisSnapshot>) -> AbsoluteState {
    if let Some(snapshot) = snapshot {
        AbsoluteState::kernel(snapshot.min, snapshot.max, snapshot.value)
    } else {
        AbsoluteState::fallback(
            config::DEFAULT_AXIS_RANGE.0,
            config::DEFAULT_AXIS_RANGE.1,
            0,
        )
    }
}

fn load_startup_state<T>(
    supported: bool,
    startup_warnings: &mut Vec<String>,
    load: impl FnOnce() -> io::Result<T>,
    warning: impl FnOnce(&io::Error) -> String,
) -> Option<T> {
    if !supported {
        return None;
    }

    match load() {
        Ok(state) => Some(state),
        Err(err) => {
            startup_warnings.push(warning(&err));
            None
        }
    }
}

fn absolute_name(code: u16) -> String {
    format!("{:?}", AbsoluteAxisCode(code)).to_lowercase()
}

fn relative_name(code: u16) -> String {
    format!("{:?}", RelativeAxisCode(code)).to_lowercase()
}

fn button_name(code: KeyCode) -> String {
    strip_btn_prefix(&format!("{code:?}").to_lowercase())
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
    use crate::monitor::config;

    #[test]
    fn absolute_state_from_snapshot_uses_kernel_values() {
        assert_eq!(
            absolute_state_from_snapshot(Some(AxisSnapshot {
                min: -10,
                max: 20,
                value: 7,
            })),
            AbsoluteState::kernel(-10, 20, 7)
        );
    }

    #[test]
    fn absolute_state_from_snapshot_uses_explicit_fallback_defaults() {
        assert_eq!(
            absolute_state_from_snapshot(None),
            AbsoluteState::fallback(
                config::DEFAULT_AXIS_RANGE.0,
                config::DEFAULT_AXIS_RANGE.1,
                0
            )
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
