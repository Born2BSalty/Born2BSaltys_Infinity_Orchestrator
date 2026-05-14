// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `RedesignSettingsStore` — load/save for `bio_redesign_settings.json`.
//
// Per Phase 4 P4.T10:
//   - Mirrors `bio::settings::store::SettingsStore` (load/save with the same
//     platform-default config-dir lookup).
//   - Sibling file in the same config dir; **never** merged into
//     `bio_settings.json`.
//   - Atomic write via temp file + rename, parent-dir auto-create.
//   - Missing file returns `Ok(default)`; corrupt file returns an `anyhow`
//     error (the Settings tab can surface this as a friendly banner — Phase
//     4 ships with a log-and-default-fallback to keep the General tab
//     functional even on first-launch with a malformed sibling file).
//
// SPEC: §11.1.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::platform_defaults::app_config_file;
use crate::settings::redesign_fields::RedesignSettings;

const REDESIGN_SETTINGS_FILE_NAME: &str = "bio_redesign_settings.json";

#[derive(Debug, Clone)]
pub struct RedesignSettingsStore {
    path: PathBuf,
}

impl RedesignSettingsStore {
    /// Construct the store pointing at the platform-default config dir.
    pub fn new_default() -> Self {
        let path = app_config_file(REDESIGN_SETTINGS_FILE_NAME, ".");
        Self { path }
    }

    /// Override path (used for tests + custom-rooted callers).
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Load the redesign settings, returning the default on missing file.
    /// A corrupt file returns an `Err`; the caller may choose to log and
    /// fall back to the default to keep the General tab functional.
    pub fn load(&self) -> Result<RedesignSettings> {
        match std::fs::read_to_string(&self.path) {
            Ok(raw) => {
                let parsed = serde_json::from_str::<RedesignSettings>(&raw).with_context(|| {
                    format!(
                        "failed parsing redesign settings file {}",
                        self.path.display()
                    )
                })?;
                Ok(parsed)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok(RedesignSettings::default())
            }
            Err(err) => Err(err).with_context(|| {
                format!(
                    "failed reading redesign settings file {}",
                    self.path.display()
                )
            }),
        }
    }

    /// Persist atomically via temp file + rename.
    pub fn save(&self, settings: &RedesignSettings) -> Result<()> {
        let raw = serde_json::to_string_pretty(settings)
            .context("failed serializing redesign settings json")?;
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed creating redesign settings directory {}",
                    parent.display()
                )
            })?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, raw.as_bytes()).with_context(|| {
            format!(
                "failed writing redesign settings tmp {}",
                tmp_path.display()
            )
        })?;
        std::fs::rename(&tmp_path, &self.path).with_context(|| {
            format!(
                "failed renaming redesign settings file to {}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        let n = C.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bio_redesign_test_{}_{}_{}.json",
            std::process::id(),
            n,
            label
        ))
    }

    #[test]
    fn load_missing_returns_default() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let store = RedesignSettingsStore::new_with_path(&path);
        let s = store.load().expect("ok");
        assert_eq!(s, RedesignSettings::default());
    }

    #[test]
    fn round_trip() {
        let path = temp_path("round_trip");
        let store = RedesignSettingsStore::new_with_path(&path);
        let s = RedesignSettings {
            user_name: "Alice".to_string(),
            ..Default::default()
        };
        store.save(&s).expect("save");
        let s2 = store.load().expect("load");
        assert_eq!(s2.user_name, "Alice");
        let _ = std::fs::remove_file(&path);
    }
}
