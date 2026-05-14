// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `RegistryError` — error class for the modlist registry + per-modlist
// workspace stores.
//
// Per Phase 3 P3.T4:
//   - `Io(io::Error)`             — non-parse IO failure (permission denied,
//                                   disk full, file locked, missing-directory
//                                   on save, etc.).
//   - `Parse(serde_json::Error)`  — JSON parse failure with the upstream error.
//   - `Corrupt { path, message }` — high-level "the file exists but is
//                                   unreadable" error pre-formatted for the
//                                   terminal error UI.
//
// All variants implement `Display + Error` for use in `?`/`Result`. The terminal
// error UI in `src/ui/orchestrator/registry_error_panel.rs` prints `path` +
// `message` in monospace.
//
// SPEC: §13.14.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum RegistryError {
    /// Lower-level IO failure (permission denied, disk full, etc.).
    Io(io::Error),
    /// JSON parse failure. The wrapped serde error includes the line/col offset.
    Parse(serde_json::Error),
    /// Pre-formatted "file is corrupt" error. Used when load detects a parse
    /// failure and wants to surface both the path and a friendly message.
    Corrupt {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Io(err) => write!(f, "registry IO error: {err}"),
            RegistryError::Parse(err) => write!(f, "registry parse error: {err}"),
            RegistryError::Corrupt { path, message } => {
                write!(f, "registry file is corrupt ({}): {message}", path.display())
            }
        }
    }
}

impl Error for RegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RegistryError::Io(err) => Some(err),
            RegistryError::Parse(err) => Some(err),
            RegistryError::Corrupt { .. } => None,
        }
    }
}

impl From<io::Error> for RegistryError {
    fn from(value: io::Error) -> Self {
        RegistryError::Io(value)
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(value: serde_json::Error) -> Self {
        RegistryError::Parse(value)
    }
}

impl RegistryError {
    /// Build a `Corrupt` error from a path + a parse error message.
    pub fn corrupt(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        RegistryError::Corrupt {
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
