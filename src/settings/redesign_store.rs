// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::platform_defaults::app_config_file;
use crate::settings::redesign_fields::RedesignSettings;

#[derive(Debug, Clone)]
pub struct RedesignSettingsStore {
    path: PathBuf,
}

impl RedesignSettingsStore {
    pub fn new_default() -> Self {
        Self {
            path: app_config_file("bio_redesign_settings.json", "."),
        }
    }

    pub fn load(&self) -> RedesignSettings {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return RedesignSettings::default();
        };
        serde_json::from_str::<RedesignSettings>(&raw).unwrap_or_default()
    }

    pub fn save(&self, settings: &RedesignSettings) -> Result<()> {
        let raw = serde_json::to_string_pretty(settings)
            .context("failed serializing redesign settings json")?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed creating redesign settings directory {}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(&self.path, raw).with_context(|| {
            format!(
                "failed writing redesign settings file {}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}
