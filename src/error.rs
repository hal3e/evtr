use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io {
        area: ErrorArea,
        context: String,
        source: io::Error,
    },
    Evdev {
        area: ErrorArea,
        context: String,
        source: io::Error,
    },
    NoDevicesFound,
    StreamEnded {
        area: ErrorArea,
        context: &'static str,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorArea {
    App,
    Selector,
    Monitor,
}

impl Error {
    pub fn io(area: ErrorArea, context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            area,
            context: context.into(),
            source,
        }
    }

    pub fn evdev(area: ErrorArea, context: impl Into<String>, source: io::Error) -> Self {
        Self::Evdev {
            area,
            context: context.into(),
            source,
        }
    }

    pub fn stream_ended(area: ErrorArea, context: &'static str) -> Self {
        Self::StreamEnded { area, context }
    }
}

impl fmt::Display for ErrorArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::App => write!(f, "app"),
            Self::Selector => write!(f, "selector"),
            Self::Monitor => write!(f, "monitor"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io {
                area,
                context,
                source,
            } => write!(f, "{area} i/o: {context}: {source}"),
            Error::Evdev {
                area,
                context,
                source,
            } => write!(f, "{area} evdev: {context}: {source}"),
            Error::NoDevicesFound => write!(f, "no input devices found"),
            Error::StreamEnded { area, context } => write!(f, "{area} stream ended: {context}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io { source, .. } | Error::Evdev { source, .. } => Some(source),
            Error::NoDevicesFound | Error::StreamEnded { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error as _, io};

    use super::{Error, ErrorArea};

    #[test]
    fn preserves_monitor_evdev_source() {
        let err = Error::evdev(
            ErrorArea::Monitor,
            "open device stream",
            io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        );

        assert_eq!(err.to_string(), "monitor evdev: open device stream: denied");
        assert_eq!(
            err.source().map(ToString::to_string),
            Some("denied".to_string())
        );
    }

    #[test]
    fn selector_stream_end_reports_area() {
        let err = Error::stream_ended(ErrorArea::Selector, "terminal event stream");

        assert_eq!(
            err.to_string(),
            "selector stream ended: terminal event stream"
        );
        assert!(err.source().is_none());
    }
}
