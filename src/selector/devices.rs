use std::{cmp::Ordering, path::Path};

use super::{DeviceInfo, discovery::DiscoveryResult};
use crate::config::SortOrder;

pub(super) struct DeviceCatalog {
    devices: Vec<DeviceInfo>,
    labels: Vec<String>,
}

impl DeviceCatalog {
    pub(super) fn from_discovery(
        discovery: DiscoveryResult<DeviceInfo>,
        sort: SortOrder,
    ) -> (Self, Option<String>) {
        let (devices, labels, error_message) =
            catalog_parts_from_discovery(discovery, device_label);
        let (devices, labels) = sort_catalog(devices, labels, sort);

        (Self { devices, labels }, error_message)
    }

    pub(super) fn labels(&self) -> &[String] {
        &self.labels
    }

    pub(super) fn label(&self, index: usize) -> Option<&str> {
        self.labels.get(index).map(String::as_str)
    }

    pub(super) fn take_selected(&mut self, index: Option<usize>) -> Option<DeviceInfo> {
        take_selected_item(&mut self.devices, &mut self.labels, index)
    }
}

pub(crate) fn device_label(device: &DeviceInfo) -> String {
    format_device_label(&device.name, &device.path)
}

fn catalog_parts_from_discovery<T>(
    discovery: DiscoveryResult<T>,
    mut label_for: impl FnMut(&T) -> String,
) -> (Vec<T>, Vec<String>, Option<String>) {
    let error_message = discovery.error_message();
    let devices = discovery.devices;
    let labels = devices.iter().map(&mut label_for).collect();

    (devices, labels, error_message)
}

fn format_device_label(name: &str, path: &Path) -> String {
    format!("{name} ({})", path.display())
}

fn take_selected_item<T>(
    items: &mut Vec<T>,
    labels: &mut Vec<String>,
    index: Option<usize>,
) -> Option<T> {
    let index = index?;
    if index >= items.len() {
        return None;
    }

    labels.swap_remove(index);
    Some(items.swap_remove(index))
}

fn sort_catalog(
    devices: Vec<DeviceInfo>,
    labels: Vec<String>,
    sort: SortOrder,
) -> (Vec<DeviceInfo>, Vec<String>) {
    sort_entries(devices.into_iter().zip(labels).collect(), |left, right| {
        compare_devices(left, right, sort)
    })
}

fn compare_devices(left: &DeviceInfo, right: &DeviceInfo, sort: SortOrder) -> Ordering {
    match sort {
        SortOrder::Path => left
            .path
            .cmp(&right.path)
            .then_with(|| left.name.cmp(&right.name)),
        SortOrder::Name => left
            .name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path)),
    }
}

fn sort_entries<T>(
    mut entries: Vec<(T, String)>,
    mut compare: impl FnMut(&T, &T) -> Ordering,
) -> (Vec<T>, Vec<String>) {
    entries.sort_unstable_by(|(left, _), (right, _)| compare(left, right));
    entries.into_iter().unzip()
}

#[cfg(test)]
mod tests {
    use std::{io, path::Path};

    use super::{
        DeviceCatalog, catalog_parts_from_discovery, format_device_label, sort_entries,
        take_selected_item,
    };
    use crate::selector::discovery::DiscoveryResult;

    #[test]
    fn format_device_label_uses_structured_name_and_path() {
        assert_eq!(
            format_device_label("Gamepad", Path::new("/dev/input/event7")),
            "Gamepad (/dev/input/event7)"
        );
    }

    #[test]
    fn catalog_parts_from_discovery_suppresses_partial_errors_when_devices_exist() {
        let mut discovery = DiscoveryResult::new();
        discovery.record_read_dir_error(
            "/dev/input",
            io::Error::new(io::ErrorKind::PermissionDenied, "read denied"),
        );
        discovery.push_device(("Pad A", "/dev/input/event3"));
        discovery.push_device(("Pad B", "/dev/input/event4"));

        let (devices, labels, error_message) =
            catalog_parts_from_discovery(discovery, |(name, path)| {
                format_device_label(name, Path::new(path))
            });

        assert_eq!(
            devices,
            vec![
                ("Pad A", "/dev/input/event3"),
                ("Pad B", "/dev/input/event4")
            ]
        );
        assert_eq!(
            labels,
            vec![
                "Pad A (/dev/input/event3)".to_string(),
                "Pad B (/dev/input/event4)".to_string()
            ]
        );
        assert_eq!(error_message, None);
    }

    #[test]
    fn label_returns_some_for_valid_index() {
        let catalog = DeviceCatalog {
            devices: Vec::new(),
            labels: vec!["Pad A (/dev/input/event3)".to_string()],
        };

        assert_eq!(catalog.label(0), Some("Pad A (/dev/input/event3)"));
    }

    #[test]
    fn label_returns_none_for_out_of_bounds_index() {
        let catalog = DeviceCatalog {
            devices: Vec::new(),
            labels: vec!["Pad A (/dev/input/event3)".to_string()],
        };

        assert_eq!(catalog.label(1), None);
    }

    #[test]
    fn take_selected_item_returns_none_for_none_index() {
        let mut items = vec![10, 20];
        let mut labels = vec!["Pad A".to_string(), "Pad B".to_string()];

        assert_eq!(take_selected_item(&mut items, &mut labels, None), None);
        assert_eq!(items, vec![10, 20]);
        assert_eq!(labels, vec!["Pad A".to_string(), "Pad B".to_string()]);
    }

    #[test]
    fn take_selected_item_returns_none_for_stale_index() {
        let mut items = vec![10];
        let mut labels = vec!["Pad A".to_string()];

        assert_eq!(take_selected_item(&mut items, &mut labels, Some(1)), None);
        assert_eq!(items, vec![10]);
        assert_eq!(labels, vec!["Pad A".to_string()]);
    }

    #[test]
    fn take_selected_item_removes_matching_item_and_label() {
        let mut items = vec![10, 20, 30];
        let mut labels = vec![
            "Pad A".to_string(),
            "Pad B".to_string(),
            "Pad C".to_string(),
        ];

        assert_eq!(
            take_selected_item(&mut items, &mut labels, Some(1)),
            Some(20)
        );
        assert_eq!(items.len(), 2);
        assert_eq!(labels.len(), 2);
        assert!(!items.contains(&20));
        assert!(!labels.iter().any(|label| label == "Pad B"));
        assert_eq!(items.len(), labels.len());
    }

    #[test]
    fn sort_entries_reorders_without_index_invalidation() {
        let entries = vec![
            (("Pad C", 9), "Pad C (/dev/input/event9)".to_string()),
            (("Pad A", 3), "Pad A (/dev/input/event3)".to_string()),
            (("Pad B", 7), "Pad B (/dev/input/event7)".to_string()),
        ];
        let (devices, labels) = sort_entries(entries, |left, right| left.1.cmp(&right.1));

        assert_eq!(devices, vec![("Pad A", 3), ("Pad B", 7), ("Pad C", 9)]);
        assert_eq!(
            labels,
            vec![
                "Pad A (/dev/input/event3)".to_string(),
                "Pad B (/dev/input/event7)".to_string(),
                "Pad C (/dev/input/event9)".to_string(),
            ]
        );
    }
}
