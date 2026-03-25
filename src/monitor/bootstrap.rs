use evdev::{Device, InputId};

use super::{model::InputCollection, plan::Counts, state::MonitorState, touch::TouchState};

pub(super) struct Bootstrapped<T> {
    value: T,
    startup_warnings: Vec<String>,
}

impl<T> Bootstrapped<T> {
    pub(super) fn new(value: T) -> Self {
        Self::with_warnings(value, Vec::new())
    }

    pub(super) fn with_warnings(value: T, startup_warnings: Vec<String>) -> Self {
        Self {
            value,
            startup_warnings,
        }
    }

    fn into_parts(self) -> (T, Vec<String>) {
        (self.value, self.startup_warnings)
    }
}

pub(super) struct MonitorBootstrap {
    pub(super) inputs: InputCollection,
    pub(super) touch: TouchState,
    pub(super) state: MonitorState,
}

impl MonitorBootstrap {
    pub(super) fn from_device(device: &Device) -> Self {
        let (inputs, mut startup_warnings) = InputCollection::from_device(device).into_parts();
        let (touch, touch_warnings) = TouchState::from_device(device).into_parts();
        startup_warnings.extend(touch_warnings);

        let counts = Counts::new(
            inputs.absolute_inputs().len(),
            inputs.relative_inputs().len(),
            inputs.button_inputs().len(),
        );
        let info_lines = device_info_lines(
            device.driver_version(),
            device.input_id(),
            device.physical_path(),
            &startup_warnings,
        );

        Self {
            inputs,
            touch,
            state: MonitorState::new(counts, info_lines),
        }
    }
}

fn device_info_lines(
    driver_version: (u8, u8, u8),
    input_id: InputId,
    phys: Option<&str>,
    startup_warnings: &[String],
) -> Vec<String> {
    let (major, minor, patch) = driver_version;
    let bus = input_id.bus_type().0;
    let vendor = input_id.vendor();
    let product = input_id.product();
    let version = input_id.version();
    let phys = phys.unwrap_or("n/a");
    let mut lines = vec![
        format!("Input driver version: {major}.{minor}.{patch}"),
        format!(
            "Input device ID: bus {bus:#x}, vendor {vendor:#x}, product {product:#x}, version {version:#x}"
        ),
        format!("Input device phys: {phys}"),
    ];

    for warning in startup_warnings {
        lines.push(format!("Startup warning: {warning}"));
    }

    lines
}

#[cfg(test)]
mod tests {
    use evdev::{BusType, InputId};

    use super::device_info_lines;

    #[test]
    fn device_info_lines_include_metadata_and_startup_warnings() {
        let lines = device_info_lines(
            (1, 2, 3),
            InputId::new(BusType::BUS_USB, 0x1234, 0xabcd, 0x0001),
            Some("usb-1/input0"),
            &["touch bounds unavailable".to_string()],
        );

        assert_eq!(
            lines,
            vec![
                "Input driver version: 1.2.3".to_string(),
                "Input device ID: bus 0x3, vendor 0x1234, product 0xabcd, version 0x1".to_string(),
                "Input device phys: usb-1/input0".to_string(),
                "Startup warning: touch bounds unavailable".to_string(),
            ]
        );
    }

    #[test]
    fn device_info_lines_use_na_for_missing_phys_path() {
        let lines = device_info_lines(
            (0, 0, 1),
            InputId::new(BusType::BUS_VIRTUAL, 0, 0, 0),
            None,
            &[],
        );

        assert_eq!(lines[2], "Input device phys: n/a");
    }

    #[test]
    fn device_info_lines_append_multiple_warnings_in_order_after_metadata() {
        let lines = device_info_lines(
            (2, 3, 4),
            InputId::new(BusType::BUS_USB, 0x1, 0x2, 0x3),
            Some("usb-2/input0"),
            &[
                "touch bounds unavailable".to_string(),
                "slot limit inferred".to_string(),
            ],
        );

        assert_eq!(
            lines,
            vec![
                "Input driver version: 2.3.4".to_string(),
                "Input device ID: bus 0x3, vendor 0x1, product 0x2, version 0x3".to_string(),
                "Input device phys: usb-2/input0".to_string(),
                "Startup warning: touch bounds unavailable".to_string(),
                "Startup warning: slot limit inferred".to_string(),
            ]
        );
    }
}
