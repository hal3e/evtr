use evdev::{AbsoluteAxisCode, Device, KeyCode};
use std::io::{self, Write};

pub fn discover_joystick_devices() -> Vec<Device> {
    let mut joystick_devices = evdev::enumerate()
        .map(|t| t.1)
        .filter(is_joystick_device)
        .collect::<Vec<_>>();

    joystick_devices.reverse();

    joystick_devices
}

pub fn is_joystick_device(device: &Device) -> bool {
    let has_joystick_axes = device.supported_absolute_axes().map_or(false, |axes| {
        axes.contains(AbsoluteAxisCode::ABS_X) && axes.contains(AbsoluteAxisCode::ABS_Y)
            || axes.contains(AbsoluteAxisCode::ABS_RX) && axes.contains(AbsoluteAxisCode::ABS_RY)
            || axes.contains(AbsoluteAxisCode::ABS_HAT0X)
                && axes.contains(AbsoluteAxisCode::ABS_HAT0Y)
    });

    let has_joystick_keys = device.supported_keys().map_or(false, |keys| {
        keys.contains(KeyCode::BTN_EAST) && keys.contains(KeyCode::BTN_WEST)
            || keys.contains(KeyCode::BTN_NORTH) && keys.contains(KeyCode::BTN_SOUTH)
            || keys.contains(KeyCode::BTN_THUMB) && keys.contains(KeyCode::BTN_THUMB2)
            || keys.contains(KeyCode::BTN_TOP) && keys.contains(KeyCode::BTN_TOP2)
    });

    has_joystick_axes && has_joystick_keys
}

pub fn select_device() -> Result<Device, Box<dyn std::error::Error>> {
    let joystick_devices = discover_joystick_devices();

    if joystick_devices.is_empty() {
        return Err("No joystick devices found!".into());
    }

    println!("Found {} joystick device(s):\n", joystick_devices.len());

    for (i, device) in joystick_devices.iter().enumerate() {
        let name = device.name().unwrap_or("Unknown Device");
        println!("{i}. {name}");
    }

    print!("\nSelect a device (0-{}): ", joystick_devices.len() - 1);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selection: usize = input.trim().parse()?;

    if selection >= joystick_devices.len() {
        return Err("Invalid selection".into());
    }

    Ok(joystick_devices
        .into_iter()
        .nth(selection)
        .expect("already checked bounds"))
}
