use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, MdSeekError>;

#[derive(Debug)]
pub enum MdSeekError {
    Io {
        path: Option<PathBuf>,
        source: io::Error,
    },
    InvalidArgs(String),
    Parse(String),
    Storage(String),
}

impl MdSeekError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: Some(path.into()),
            source,
        }
    }
}

impl fmt::Display for MdSeekError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => match path {
                Some(path) => write!(f, "{}: {}", path.display(), source),
                None => write!(f, "{source}"),
            },
            Self::InvalidArgs(message) => write!(f, "{message}"),
            Self::Parse(message) => write!(f, "parse error: {message}"),
            Self::Storage(message) => write!(f, "storage error: {message}"),
        }
    }
}

impl Error for MdSeekError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidArgs(_) | Self::Parse(_) | Self::Storage(_) => None,
        }
    }
}

impl From<io::Error> for MdSeekError {
    fn from(source: io::Error) -> Self {
        Self::Io { path: None, source }
    }
}
