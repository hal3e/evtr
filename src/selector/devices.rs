use std::path::Path;

use super::{DeviceInfo, discovery::DiscoveryResult};

pub(crate) struct DeviceCatalog {
    devices: Vec<DeviceInfo>,
    labels: Vec<String>,
}

impl DeviceCatalog {
    pub(crate) fn from_discovery(discovery: DiscoveryResult<DeviceInfo>) -> (Self, Option<String>) {
        let error_message = discovery.error_message();
        let devices = discovery.devices;
        let labels = devices.iter().map(device_label).collect();

        (Self { devices, labels }, error_message)
    }

    pub(crate) fn labels(&self) -> &[String] {
        &self.labels
    }

    pub(crate) fn label(&self, index: usize) -> Option<&str> {
        self.labels.get(index).map(String::as_str)
    }

    pub(crate) fn take_selected(&mut self, index: Option<usize>) -> Option<DeviceInfo> {
        let index = index?;
        if index >= self.devices.len() {
            return None;
        }

        self.labels.swap_remove(index);
        Some(self.devices.swap_remove(index))
    }
}

pub(crate) fn device_label(device: &DeviceInfo) -> String {
    format_device_label(&device.name, &device.path)
}

fn format_device_label(name: &str, path: &Path) -> String {
    format!("{name} ({})", path.display())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::format_device_label;

    #[test]
    fn format_device_label_uses_structured_name_and_path() {
        assert_eq!(
            format_device_label("Gamepad", Path::new("/dev/input/event7")),
            "Gamepad (/dev/input/event7)"
        );
    }
}
