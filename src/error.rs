use std::{error, fmt};

#[derive(Clone)]
pub enum Error {
    Io { context: String, message: String },
    Evdev { context: String, message: String },
    Terminal { context: String, message: String },
    NoDevicesFound,
    StreamEnded { context: String },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io(context: impl Into<String>, message: impl fmt::Display) -> Self {
        Error::Io {
            context: context.into(),
            message: message.to_string(),
        }
    }

    pub fn evdev(context: impl Into<String>, message: impl fmt::Display) -> Self {
        Error::Evdev {
            context: context.into(),
            message: message.to_string(),
        }
    }

    pub fn terminal(context: impl Into<String>, message: impl fmt::Display) -> Self {
        Error::Terminal {
            context: context.into(),
            message: message.to_string(),
        }
    }

    pub fn stream_ended(context: impl Into<String>) -> Self {
        Error::StreamEnded {
            context: context.into(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, message } => write!(f, "i/o: {}: {}", context, message),
            Error::Evdev { context, message } => write!(f, "evdev: {}: {}", context, message),
            Error::Terminal { context, message } => {
                write!(f, "terminal: {}: {}", context, message)
            }
            Error::NoDevicesFound => write!(f, "no input devices found"),
            Error::StreamEnded { context } => write!(f, "stream ended: {}", context),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl error::Error for Error {}
