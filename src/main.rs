use evdev::{AbsoluteAxisType, Device, EventType, Key};
use std::collections::HashMap;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_joystick_devices()?;

    if devices.is_empty() {
        println!("No joystick devices found!");
        return Ok(());
    }

    let device_path = select_device(&devices)?;
    let mut device = Device::open(&device_path)?;
    monitor_device(&mut device)?;

    Ok(())
}

fn discover_joystick_devices() -> Result<Vec<(Device, String)>, Box<dyn std::error::Error>> {
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

fn is_joystick_device(device: &Device) -> bool {
    let supported_events = device.supported_events();

    // Must support absolute positioning (analog sticks/triggers)
    if !supported_events.contains(EventType::ABSOLUTE) {
        return false;
    }

    // Check for joystick-specific absolute axes
    let has_joystick_axes = device.supported_absolute_axes().map_or(false, |axes| {
        // Look for typical joystick axes
        axes.contains(AbsoluteAxisType::ABS_X) && axes.contains(AbsoluteAxisType::ABS_Y)
            || axes.contains(AbsoluteAxisType::ABS_RX) && axes.contains(AbsoluteAxisType::ABS_RY)
            || axes.contains(AbsoluteAxisType::ABS_HAT0X)
            || axes.contains(AbsoluteAxisType::ABS_HAT0Y)
    });

    // Check for gamepad buttons (using raw button codes)
    let has_gamepad_buttons = device.supported_keys().map_or(false, |keys| {
        // Check for common gamepad button codes
        keys.contains(Key::new(0x130)) || // BTN_A
        keys.contains(Key::new(0x131)) || // BTN_B
        keys.contains(Key::new(0x132)) || // BTN_C
        keys.contains(Key::new(0x133)) || // BTN_X
        keys.contains(Key::new(0x134)) || // BTN_Y
        keys.contains(Key::new(0x135)) || // BTN_Z
        keys.contains(Key::new(0x136)) || // BTN_TL
        keys.contains(Key::new(0x137)) || // BTN_TR
        keys.contains(Key::new(0x13a)) || // BTN_START
        keys.contains(Key::new(0x13b)) // BTN_SELECT
    });

    // Accept devices that have joystick axes OR gamepad buttons
    has_joystick_axes || has_gamepad_buttons
}

fn select_device(devices: &[(Device, String)]) -> Result<String, Box<dyn std::error::Error>> {
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

fn monitor_device(device: &mut Device) -> Result<(), Box<dyn std::error::Error>> {
    println!("Monitoring device. Press Ctrl+C to exit.\n");

    let mut axis_values: HashMap<u16, i32> = HashMap::new();
    let mut button_states: HashMap<u16, bool> = HashMap::new();

    loop {
        for event in device.fetch_events()? {
            match event.event_type() {
                EventType::ABSOLUTE => {
                    let code = event.code();
                    let value = event.value();
                    axis_values.insert(code, value);
                    print_status(&axis_values, &button_states);
                }
                EventType::KEY => {
                    let code = event.code();
                    let pressed = event.value() == 1;
                    button_states.insert(code, pressed);
                    print_status(&axis_values, &button_states);
                }
                _ => {}
            }
        }
    }
}

fn print_status(axis_values: &HashMap<u16, i32>, button_states: &HashMap<u16, bool>) {
    print!("\x1B[2J\x1B[1;1H");
    println!("=== Joystick Monitor ===\n");

    if !axis_values.is_empty() {
        println!("Axes:");
        for (&code, &value) in axis_values {
            let axis_name = match code {
                0 => "Left X",
                1 => "Left Y",
                2 => "Right X",
                3 => "Right Y",
                4 => "Left Trigger",
                5 => "Right Trigger",
                16 => "D-Pad X",
                17 => "D-Pad Y",
                _ => "Unknown",
            };
            println!("  {}: {} (raw: {})", axis_name, value, code);
        }
        println!();
    }

    if !button_states.is_empty() {
        println!("Buttons:");
        for (&code, &pressed) in button_states {
            if pressed {
                let button_name = match code {
                    304 => "A/X",
                    305 => "B/Circle",
                    306 => "X/Square",
                    307 => "Y/Triangle",
                    308 => "L1/LB",
                    309 => "R1/RB",
                    310 => "Back/Select",
                    311 => "Start",
                    312 => "Home/Guide",
                    313 => "Left Stick",
                    314 => "Right Stick",
                    _ => "Unknown",
                };
                println!("  {} ({}): PRESSED", button_name, code);
            }
        }
        println!();
    }

    println!("Press Ctrl+C to exit");
}
