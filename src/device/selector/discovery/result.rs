use std::{io, path::PathBuf};

use super::issue::DiscoveryIssue;

#[derive(Debug)]
struct DiscoveryError {
    path: PathBuf,
    message: String,
}

impl DiscoveryError {
    fn new(path: impl Into<PathBuf>, err: io::Error) -> Self {
        Self {
            path: path.into(),
            message: err.to_string(),
        }
    }
}

#[derive(Debug)]
struct DiscoveryStats {
    event_nodes: usize,
    permission_denied: usize,
    open_failed: usize,
    read_dir_failed: usize,
    sample_read_dir_error: Option<DiscoveryError>,
    sample_open_error: Option<DiscoveryError>,
}

impl DiscoveryStats {
    fn new() -> Self {
        Self {
            event_nodes: 0,
            permission_denied: 0,
            open_failed: 0,
            read_dir_failed: 0,
            sample_read_dir_error: None,
            sample_open_error: None,
        }
    }

    fn record_event_node(&mut self) {
        self.event_nodes += 1;
    }

    fn record_read_dir_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.read_dir_failed += 1;
        if self.sample_read_dir_error.is_none() {
            self.sample_read_dir_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn record_open_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        if err.kind() == io::ErrorKind::PermissionDenied {
            self.permission_denied += 1;
            return;
        }

        self.open_failed += 1;
        if self.sample_open_error.is_none() {
            self.sample_open_error = Some(DiscoveryError::new(path, err));
        }
    }

    fn total_open_failures(&self) -> usize {
        self.permission_denied + self.open_failed
    }

    fn classify(&self, has_devices: bool) -> Option<DiscoveryIssue> {
        if has_devices {
            return None;
        }

        if self.event_nodes == 0 {
            return self.sample_read_dir_error.as_ref().map_or(
                Some(DiscoveryIssue::NoDevicesFound),
                |error| {
                    Some(DiscoveryIssue::ReadDir {
                        path: error.path.clone(),
                        message: error.message.clone(),
                    })
                },
            );
        }

        let skipped = self.total_open_failures();
        if skipped == 0 {
            return self
                .sample_read_dir_error
                .as_ref()
                .map(|error| DiscoveryIssue::ReadDir {
                    path: error.path.clone(),
                    message: error.message.clone(),
                });
        }

        if self.open_failed == 0 {
            return Some(DiscoveryIssue::PermissionDenied { skipped });
        }

        self.sample_open_error
            .as_ref()
            .map_or(Some(DiscoveryIssue::NoDevicesFound), |error| {
                Some(DiscoveryIssue::OpenFailed {
                    skipped,
                    path: error.path.clone(),
                    message: error.message.clone(),
                })
            })
    }
}

#[derive(Debug)]
pub(crate) struct DiscoveryResult<T> {
    pub(crate) devices: Vec<T>,
    stats: DiscoveryStats,
}

impl<T> DiscoveryResult<T> {
    pub(crate) fn new() -> Self {
        Self {
            devices: Vec::new(),
            stats: DiscoveryStats::new(),
        }
    }

    pub(crate) fn read_dir_failed(path: impl Into<PathBuf>, err: io::Error) -> Self {
        let mut result = Self::new();
        result.record_read_dir_error(path, err);
        result
    }

    pub(crate) fn record_event_node(&mut self) {
        self.stats.record_event_node();
    }

    pub(crate) fn record_read_dir_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.stats.record_read_dir_error(path, err);
    }

    pub(crate) fn record_open_error(&mut self, path: impl Into<PathBuf>, err: io::Error) {
        self.stats.record_open_error(path, err);
    }

    pub(crate) fn push_device(&mut self, device: T) {
        self.devices.push(device);
    }

    pub(crate) fn issue(&self) -> Option<DiscoveryIssue> {
        self.stats.classify(!self.devices.is_empty())
    }

    pub(crate) fn error_message(&self) -> Option<String> {
        self.issue().map(|issue| issue.message())
    }

    #[cfg(test)]
    pub(crate) fn event_nodes(&self) -> usize {
        self.stats.event_nodes
    }

    #[cfg(test)]
    pub(crate) fn total_open_failures(&self) -> usize {
        self.stats.total_open_failures()
    }

    #[cfg(test)]
    pub(crate) fn read_dir_failures(&self) -> usize {
        self.stats.read_dir_failed
    }
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf};

    use super::{DiscoveryIssue, DiscoveryResult};

    #[test]
    fn discovery_issue_reports_no_devices_when_no_event_nodes_exist() {
        let result: DiscoveryResult<()> = DiscoveryResult::new();

        assert_eq!(result.issue(), Some(DiscoveryIssue::NoDevicesFound));
    }

    #[test]
    fn discovery_issue_reports_permission_guidance_when_all_devices_are_skipped() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.record_event_node();
        result.record_event_node();
        result.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.record_open_error(
            "/dev/input/event1",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::PermissionDenied { skipped: 2 })
        );
        assert_eq!(
            result.error_message(),
            Some(
                "found 2 input device node(s), but none were readable; check permissions for /dev/input/event*"
                    .to_string()
            )
        );
    }

    #[test]
    fn discovery_issue_reports_open_failures_when_causes_are_mixed() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.record_event_node();
        result.record_event_node();
        result.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.record_open_error(
            "/dev/input/event1",
            io::Error::new(io::ErrorKind::NotFound, "device disappeared"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::OpenFailed {
                skipped: 2,
                path: PathBuf::from("/dev/input/event1"),
                message: "device disappeared".to_string(),
            })
        );
    }

    #[test]
    fn discovery_issue_prefers_open_failures_over_partial_read_dir_errors() {
        let mut result: DiscoveryResult<()> = DiscoveryResult::new();
        result.record_event_node();
        result.record_read_dir_error(
            "/dev/input",
            io::Error::new(io::ErrorKind::Interrupted, "retry"),
        );
        result.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::PermissionDenied { skipped: 1 })
        );
    }

    #[test]
    fn discovery_issue_reports_read_dir_failures() {
        let result: DiscoveryResult<()> = DiscoveryResult::read_dir_failed(
            "/dev/input",
            io::Error::new(io::ErrorKind::PermissionDenied, "read denied"),
        );

        assert_eq!(
            result.issue(),
            Some(DiscoveryIssue::ReadDir {
                path: PathBuf::from("/dev/input"),
                message: "read denied".to_string(),
            })
        );
    }

    #[test]
    fn discovery_issue_is_suppressed_when_devices_are_found() {
        let mut result = DiscoveryResult::new();
        result.record_event_node();
        result.record_open_error(
            "/dev/input/event0",
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        result.push_device(());

        assert_eq!(result.issue(), None);
        assert_eq!(result.error_message(), None);
    }
}
