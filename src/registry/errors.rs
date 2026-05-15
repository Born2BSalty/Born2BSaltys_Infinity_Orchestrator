// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum RegistryError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Corrupt {
        path: PathBuf,
        message: String,
    },
    Serialize {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "registry I/O error at {}: {source}", path.display())
            }
            Self::Corrupt { path, message } => {
                write!(f, "registry is corrupt at {}: {message}", path.display())
            }
            Self::Serialize { path, message } => {
                write!(
                    f,
                    "failed serializing registry for {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl Error for RegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Corrupt { .. } | Self::Serialize { .. } => None,
        }
    }
}
