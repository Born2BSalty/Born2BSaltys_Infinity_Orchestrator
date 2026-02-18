// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::settings::model::AppSettings;

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new_default() -> Self {
        let path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("bio_settings.json");
        Self { path }
    }

    pub fn load(&self) -> Result<AppSettings> {
        let raw = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed reading settings file {}", self.path.display()))?;
        let settings = serde_json::from_str::<AppSettings>(&raw)
            .with_context(|| format!("failed parsing settings file {}", self.path.display()))?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        let raw =
            serde_json::to_string_pretty(settings).context("failed serializing settings json")?;
        std::fs::write(&self.path, raw)
            .with_context(|| format!("failed writing settings file {}", self.path.display()))?;
        Ok(())
    }
}
