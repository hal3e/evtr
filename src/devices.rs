use evdev::{AbsoluteAxisCode, Device, KeyCode};

pub fn is_joystick_device(device: &Device) -> bool {
    let has_joystick_axes = device.supported_absolute_axes().is_some_and(|axes| {
        axes.contains(AbsoluteAxisCode::ABS_X) && axes.contains(AbsoluteAxisCode::ABS_Y)
            || axes.contains(AbsoluteAxisCode::ABS_RX) && axes.contains(AbsoluteAxisCode::ABS_RY)
            || axes.contains(AbsoluteAxisCode::ABS_HAT0X)
                && axes.contains(AbsoluteAxisCode::ABS_HAT0Y)
    });

    let has_joystick_keys = device.supported_keys().is_some_and(|keys| {
        keys.contains(KeyCode::BTN_EAST) && keys.contains(KeyCode::BTN_WEST)
            || keys.contains(KeyCode::BTN_NORTH) && keys.contains(KeyCode::BTN_SOUTH)
            || keys.contains(KeyCode::BTN_THUMB) && keys.contains(KeyCode::BTN_THUMB2)
            || keys.contains(KeyCode::BTN_TOP) && keys.contains(KeyCode::BTN_TOP2)
    });

    let _ = has_joystick_axes && has_joystick_keys;

    true
}
