use std::{
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::types::DiscoveryResult;

const INPUT_DIR: &str = "/dev/input";
const INPUT_EVENT_PREFIX: &[u8] = b"event";

pub(crate) fn discover_devices<T, F>(open_device: F) -> DiscoveryResult<T>
where
    F: FnMut(&Path) -> io::Result<T>,
{
    let entries = match fs::read_dir(INPUT_DIR) {
        Ok(entries) => entries,
        Err(err) => return DiscoveryResult::read_dir_failed(INPUT_DIR, err),
    };

    discover_from_entries(
        entries.map(|entry| entry.map(|entry| entry.path())),
        open_device,
    )
}

pub(crate) fn discover_from_entries<T, I, F>(entries: I, mut open_device: F) -> DiscoveryResult<T>
where
    I: IntoIterator<Item = io::Result<PathBuf>>,
    F: FnMut(&Path) -> io::Result<T>,
{
    let mut result = DiscoveryResult::new();

    for entry in entries {
        match entry {
            Ok(path) => scan_path(&path, &mut result, &mut open_device),
            Err(err) => result.stats.record_read_dir_error(INPUT_DIR, err),
        }
    }

    result
}

fn scan_path<T, F>(path: &Path, result: &mut DiscoveryResult<T>, open_device: &mut F)
where
    F: FnMut(&Path) -> io::Result<T>,
{
    if !is_event_node(path) {
        return;
    }

    result.stats.event_nodes += 1;
    match open_device(path) {
        Ok(device) => result.devices.push(device),
        Err(err) => result.stats.record_open_error(path, err),
    }
}

fn is_event_node(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };

    name.as_bytes().starts_with(INPUT_EVENT_PREFIX)
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::{Path, PathBuf},
    };

    use super::discover_from_entries;

    #[test]
    fn discover_from_entries_counts_skips_and_filters_non_event_nodes() {
        let entries = vec![
            Ok(PathBuf::from("/dev/input/mice")),
            Ok(PathBuf::from("/dev/input/event1")),
            Err(io::Error::new(io::ErrorKind::Interrupted, "retry")),
            Ok(PathBuf::from("/dev/input/event0")),
        ];

        let result = discover_from_entries(entries, |path: &Path| {
            if path.ends_with("event1") {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "permission denied",
                ));
            }

            Ok(path.display().to_string())
        });

        assert_eq!(result.stats.event_nodes, 2);
        assert_eq!(result.stats.total_open_failures(), 1);
        assert_eq!(result.stats.read_dir_failed, 1);
        assert_eq!(result.devices, vec!["/dev/input/event0".to_string()]);
    }
}
