use std::path::PathBuf;

use crate::error::Error;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DiscoveryIssue {
    ReadDir {
        path: PathBuf,
        message: String,
    },
    PermissionDenied {
        skipped: usize,
    },
    OpenFailed {
        skipped: usize,
        path: PathBuf,
        message: String,
    },
    NoDevicesFound,
}

impl DiscoveryIssue {
    pub(crate) fn message(&self) -> String {
        match self {
            DiscoveryIssue::ReadDir { path, message } => {
                format!("unable to read {}: {}", path.display(), message)
            }
            DiscoveryIssue::PermissionDenied { skipped } => {
                format!(
                    "found {skipped} input device node(s), but none were readable; check permissions for /dev/input/event*"
                )
            }
            DiscoveryIssue::OpenFailed {
                skipped,
                path,
                message,
            } => {
                format!(
                    "found {skipped} input device node(s), but none could be opened; first error: {}: {}",
                    path.display(),
                    message
                )
            }
            DiscoveryIssue::NoDevicesFound => Error::NoDevicesFound.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::DiscoveryIssue;
    use crate::error::Error;

    #[test]
    fn formats_read_dir_issue_with_path_and_message() {
        let issue = DiscoveryIssue::ReadDir {
            path: PathBuf::from("/dev/input"),
            message: "read denied".to_string(),
        };

        assert_eq!(issue.message(), "unable to read /dev/input: read denied");
    }

    #[test]
    fn formats_permission_denied_issue_with_skipped_count() {
        let issue = DiscoveryIssue::PermissionDenied { skipped: 3 };

        assert_eq!(
            issue.message(),
            "found 3 input device node(s), but none were readable; check permissions for /dev/input/event*"
        );
    }

    #[test]
    fn formats_open_failed_issue_with_path_and_message() {
        let issue = DiscoveryIssue::OpenFailed {
            skipped: 2,
            path: PathBuf::from("/dev/input/event7"),
            message: "device busy".to_string(),
        };

        assert_eq!(
            issue.message(),
            "found 2 input device node(s), but none could be opened; first error: /dev/input/event7: device busy"
        );
    }

    #[test]
    fn no_devices_issue_matches_error_display() {
        let issue = DiscoveryIssue::NoDevicesFound;

        assert_eq!(issue.message(), Error::NoDevicesFound.to_string());
    }
}
