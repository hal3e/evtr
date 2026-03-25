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
