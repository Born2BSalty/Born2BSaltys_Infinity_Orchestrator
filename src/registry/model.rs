// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModlistRegistry {
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    #[serde(default)]
    pub entries: Vec<ModlistEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModlistEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub game: Game,
    #[serde(default)]
    pub destination_folder: PathBuf,
    #[serde(default)]
    pub state: ModlistState,
    #[serde(default = "default_registry_timestamp")]
    pub creation_date: DateTime<Utc>,
    #[serde(default = "default_registry_timestamp")]
    pub last_touched_date: DateTime<Utc>,
    #[serde(default)]
    pub install_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_played_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub mod_count: usize,
    #[serde(default)]
    pub component_count: usize,
    #[serde(default)]
    pub total_size_bytes: Option<u64>,
    #[serde(default)]
    pub latest_share_code: Option<String>,
    #[serde(default)]
    pub workspace_file_relpath: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModlistState {
    #[default]
    InProgress,
    Installed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Game {
    #[default]
    BGEE,
    BG2EE,
    IWDEE,
    EET,
}

impl Default for ModlistRegistry {
    fn default() -> Self {
        Self {
            format_version: default_format_version(),
            entries: Vec::new(),
        }
    }
}

impl Default for ModlistEntry {
    fn default() -> Self {
        let timestamp = default_registry_timestamp();
        Self {
            id: String::new(),
            name: String::new(),
            game: Game::default(),
            destination_folder: PathBuf::new(),
            state: ModlistState::default(),
            creation_date: timestamp,
            last_touched_date: timestamp,
            install_date: None,
            last_played_date: None,
            mod_count: 0,
            component_count: 0,
            total_size_bytes: None,
            latest_share_code: None,
            workspace_file_relpath: PathBuf::new(),
        }
    }
}

fn default_format_version() -> u32 {
    1
}

fn default_registry_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::UNIX_EPOCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_round_trips() {
        let registry = ModlistRegistry::default();
        let serialized = serde_json::to_string(&registry).expect("serialize registry");
        let parsed: ModlistRegistry =
            serde_json::from_str(&serialized).expect("deserialize registry");

        assert_eq!(registry, parsed);
    }

    #[test]
    fn missing_additive_entry_fields_parse_with_defaults() {
        let json = r#"{
            "format_version": 1,
            "entries": [
                {
                    "id": "demo",
                    "name": "Demo",
                    "game": "BG2EE",
                    "destination_folder": "demo-install",
                    "state": "InProgress",
                    "creation_date": "2026-05-13T00:00:00Z",
                    "last_touched_date": "2026-05-13T00:00:00Z",
                    "mod_count": 2,
                    "component_count": 4,
                    "workspace_file_relpath": "modlists/demo/workspace.json"
                }
            ]
        }"#;

        let parsed: ModlistRegistry = serde_json::from_str(json).expect("parse older registry");
        let entry = parsed.entries.first().expect("entry");

        assert_eq!(entry.install_date, None);
        assert_eq!(entry.last_played_date, None);
        assert_eq!(entry.total_size_bytes, None);
        assert_eq!(entry.latest_share_code, None);
    }
}
