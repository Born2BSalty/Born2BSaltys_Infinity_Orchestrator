// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::platform_defaults::app_config_file;
use crate::settings::model::AppSettings;

pub const SETTINGS_FILE_NAME: &str = "bio_settings.json";

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    #[must_use]
    pub fn new_default() -> Self {
        let path = app_config_file(SETTINGS_FILE_NAME, ".");
        Self { path }
    }

    #[must_use]
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<AppSettings> {
        let raw = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed reading settings file {}", self.path.display()))?;
        let settings = serde_json::from_str::<AppSettings>(&raw)
            .with_context(|| format!("failed parsing settings file {}", self.path.display()))?;
        Ok(settings)
    }

    pub fn backup_corrupt_file(&self) -> std::io::Result<PathBuf> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let new_path = self.path.with_extension(format!("json.corrupt-{ts}"));
        std::fs::rename(&self.path, &new_path)?;
        Ok(new_path)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        let raw =
            serde_json::to_string_pretty(settings).context("failed serializing settings json")?;
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed creating settings directory {}", parent.display())
            })?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, raw.as_bytes())
            .with_context(|| format!("failed writing settings tmp {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &self.path)
            .with_context(|| format!("failed renaming settings file to {}", self.path.display()))?;
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
            "bio_settings_test_{}_{}_{}.json",
            std::process::id(),
            n,
            label
        ))
    }

    #[test]
    fn load_missing_returns_err() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new_with_path(&path);
        assert!(store.load().is_err(), "missing file is an Err");
    }

    #[test]
    fn round_trip() {
        let path = temp_path("round_trip");
        let store = SettingsStore::new_with_path(&path);
        let s = AppSettings {
            exe_fingerprint: "abc".to_string(),
            ..Default::default()
        };
        store.save(&s).expect("save");
        let s2 = store.load().expect("load");
        assert_eq!(s2.exe_fingerprint, "abc");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_returns_err_and_load_stays_pure() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"{ not json").expect("write garbage");
        let store = SettingsStore::new_with_path(&path);
        assert!(store.load().is_err(), "corrupt file is an Err");

        let still_there = std::fs::read_to_string(&path).expect("file intact");
        assert_eq!(still_there, "{ not json");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn backup_corrupt_file_renames_to_unix_ts_suffix() {
        let path = temp_path("backup");
        std::fs::write(&path, b"{ bad").expect("write");
        let store = SettingsStore::new_with_path(&path);
        let new_path = store.backup_corrupt_file().expect("rename");
        assert!(!path.exists(), "original moved aside");
        assert!(new_path.exists(), "backup created");
        let name = new_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(
            name.contains("corrupt-"),
            "name `{name}` has timestamp suffix"
        );
        let _ = std::fs::remove_file(&new_path);
    }
}
