// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum RegistryError {
    Io(io::Error),

    Parse(serde_json::Error),

    Corrupt { path: PathBuf, message: String },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "registry IO error: {err}"),
            Self::Parse(err) => write!(f, "registry parse error: {err}"),
            Self::Corrupt { path, message } => {
                write!(
                    f,
                    "registry file is corrupt ({}): {message}",
                    path.display()
                )
            }
        }
    }
}

impl Error for RegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
            Self::Corrupt { .. } => None,
        }
    }
}

impl From<io::Error> for RegistryError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value)
    }
}

impl RegistryError {
    pub fn corrupt(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Corrupt {
            path: path.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_variant_displays() {
        let err = RegistryError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "nope"));
        let s = format!("{err}");
        assert!(s.contains("nope"));
    }

    #[test]
    fn corrupt_variant_displays_path_and_message() {
        let err = RegistryError::corrupt("/tmp/x.json", "bad json");
        let s = format!("{err}");
        assert!(s.contains("/tmp/x.json"));
        assert!(s.contains("bad json"));
    }
}
