use evdev::{AbsoluteAxisCode, Device, EventType};
use std::io::{self, Write};

pub fn discover_joystick_devices() -> Result<Vec<(Device, String)>, Box<dyn std::error::Error>> {
    let mut joystick_devices = Vec::new();

    for entry in std::fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name() {
            if let Some(filename_str) = filename.to_str() {
                if filename_str.starts_with("event") {
                    if let Ok(device) = Device::open(&path) {
                        if is_joystick_device(&device) {
                            joystick_devices.push((device, path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(joystick_devices)
}

pub fn is_joystick_device(device: &Device) -> bool {
    let supported_events = device.supported_events();

    // Must support absolute positioning (analog sticks/triggers)
    if !supported_events.contains(EventType::ABSOLUTE) {
        return false;
    }

    // Check for joystick-specific absolute axes
    let has_joystick_axes = device.supported_absolute_axes().map_or(false, |axes| {
        // Look for typical joystick axes
        axes.contains(AbsoluteAxisCode::ABS_X) && axes.contains(AbsoluteAxisCode::ABS_Y)
            || axes.contains(AbsoluteAxisCode::ABS_RX) && axes.contains(AbsoluteAxisCode::ABS_RY)
            || axes.contains(AbsoluteAxisCode::ABS_HAT0X)
            || axes.contains(AbsoluteAxisCode::ABS_HAT0Y)
    });

    // Check for gamepad buttons - if it has key events, assume it might be a gamepad
    let has_gamepad_buttons = device.supported_events().contains(EventType::KEY);

    // Accept devices that have joystick axes OR gamepad buttons
    has_joystick_axes || has_gamepad_buttons
}

pub fn select_device(devices: &[(Device, String)]) -> Result<String, Box<dyn std::error::Error>> {
    println!("Found {} joystick device(s):\n", devices.len());

    for (i, (device, path)) in devices.iter().enumerate() {
        let name = device.name().unwrap_or("Unknown Device");
        println!("{}. {} ({})", i + 1, name, path);
    }

    print!("\nSelect a device (1-{}): ", devices.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selection: usize = input.trim().parse()?;

    if selection == 0 || selection > devices.len() {
        return Err("Invalid selection".into());
    }

    println!(
        "\nSelected: {}\n",
        devices[selection - 1].0.name().unwrap_or("Unknown")
    );

    Ok(devices[selection - 1].1.clone())
}
