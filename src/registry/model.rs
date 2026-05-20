// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistRegistry {
    pub format_version: u32,

    pub entries: Vec<ModlistEntry>,
}

impl Default for ModlistRegistry {
    fn default() -> Self {
        Self {
            format_version: 1,
            entries: Vec::new(),
        }
    }
}

impl ModlistRegistry {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn find(&self, id: &str) -> Option<&ModlistEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut ModlistEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistEntry {
    pub id: String,

    pub name: String,

    pub game: Game,

    pub destination_folder: String,

    pub state: ModlistState,

    pub creation_date: DateTime<Utc>,

    pub last_touched_date: DateTime<Utc>,

    pub install_date: Option<DateTime<Utc>>,

    #[serde(default)]
    pub install_started_at: Option<DateTime<Utc>>,

    pub last_played_date: Option<DateTime<Utc>>,

    pub mod_count: u32,

    pub component_count: u32,

    pub paused_at_step: Option<u8>,

    pub total_size_bytes: Option<u64>,

    pub latest_share_code: Option<String>,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub(crate) forked_from: Vec<crate::app::modlist_share::ForkAncestor>,

    pub workspace_file_relpath: PathBuf,
}

impl Default for ModlistEntry {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(),
            name: String::new(),
            game: Game::default(),
            destination_folder: String::new(),
            state: ModlistState::default(),
            creation_date: now,
            last_touched_date: now,
            install_date: None,
            install_started_at: None,
            last_played_date: None,
            mod_count: 0,
            component_count: 0,
            paused_at_step: None,
            total_size_bytes: None,
            latest_share_code: None,
            author: None,
            forked_from: Vec::new(),
            workspace_file_relpath: PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModlistState {
    #[default]
    InProgress,

    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Game {
    #[default]
    BGEE,
    BG2EE,
    IWDEE,
    EET,
}

impl Game {
    #[must_use]
    pub const fn to_legacy_string(self) -> &'static str {
        match self {
            Self::BGEE => "BGEE",
            Self::BG2EE => "BG2EE",
            Self::IWDEE => "IWDEE",
            Self::EET => "EET",
        }
    }

    #[must_use]
    pub fn from_legacy_string(s: &str) -> Self {
        match s.trim() {
            "BG2EE" => Self::BG2EE,
            "IWDEE" => Self::IWDEE,
            "EET" => Self::EET,
            _ => Self::BGEE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_registry() {
        let r = ModlistRegistry::default();
        let s = serde_json::to_string(&r).expect("serialize");
        let r2: ModlistRegistry = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(r, r2);
    }

    #[test]
    fn round_trip_one_entry() {
        let mut r = ModlistRegistry::default();
        r.entries.push(ModlistEntry {
            id: "ABC0123456789".to_string(),
            name: "Test Modlist".to_string(),
            game: Game::EET,
            destination_folder: "/games/eet".to_string(),
            state: ModlistState::InProgress,
            mod_count: 47,
            component_count: 312,
            total_size_bytes: Some(1024 * 1024 * 100),
            latest_share_code: Some("FAKE-CODE".to_string()),
            workspace_file_relpath: PathBuf::from("modlists/ABC0123456789/workspace.json"),
            ..Default::default()
        });
        let s = serde_json::to_string_pretty(&r).expect("serialize");
        let r2: ModlistRegistry = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(r.entries.len(), r2.entries.len());
        assert_eq!(r.entries[0].id, r2.entries[0].id);
        assert_eq!(r.entries[0].game, r2.entries[0].game);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let raw = r#"{"format_version":1,"entries":[]}"#;
        let r: ModlistRegistry = serde_json::from_str(raw).expect("backward-compat");
        assert_eq!(r.format_version, 1);
        assert!(r.entries.is_empty());
    }

    #[test]
    fn entry_with_legacy_state_lowercase() {
        let raw = r#"{
            "format_version": 1,
            "entries": [{
                "id": "ABCDEFGHIJKL",
                "name": "demo",
                "game": "BGEE",
                "state": "in_progress",
                "workspace_file_relpath": "modlists/ABCDEFGHIJKL/workspace.json"
            }]
        }"#;
        let r: ModlistRegistry = serde_json::from_str(raw).expect("parse");
        assert_eq!(r.entries[0].state, ModlistState::InProgress);
    }

    #[test]
    fn provenance_fields_serde_default_round_trip() {
        use crate::app::modlist_share::ForkAncestor;

        let raw = r#"{
            "format_version": 1,
            "entries": [{
                "id": "ABCDEFGHIJKL",
                "name": "legacy",
                "game": "EET",
                "state": "in_progress",
                "workspace_file_relpath": "modlists/ABCDEFGHIJKL/workspace.json"
            }]
        }"#;
        let r: ModlistRegistry = serde_json::from_str(raw).expect("backward-compat parse");
        assert_eq!(r.entries[0].author, None);
        assert!(r.entries[0].forked_from.is_empty());

        assert_eq!(r.entries[0].install_started_at, None);

        let mut reg = ModlistRegistry::default();
        reg.entries.push(ModlistEntry {
            id: "ZYXWVUTSRQPO".to_string(),
            name: "Forked".to_string(),
            game: Game::EET,
            author: Some("@me".to_string()),
            forked_from: vec![
                ForkAncestor {
                    name: "Original".to_string(),
                    author: "@root".to_string(),
                },
                ForkAncestor {
                    name: "Mid".to_string(),
                    author: "@mid".to_string(),
                },
            ],
            ..Default::default()
        });
        let s = serde_json::to_string_pretty(&reg).expect("serialize");
        let back: ModlistRegistry = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(reg, back);
        assert_eq!(back.entries[0].author.as_deref(), Some("@me"));
        assert_eq!(back.entries[0].forked_from.len(), 2);
        assert_eq!(back.entries[0].forked_from[0].name, "Original");
        assert_eq!(back.entries[0].forked_from[1].author, "@mid");
    }

    #[test]
    fn game_legacy_round_trip() {
        for g in [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET] {
            assert_eq!(Game::from_legacy_string(g.to_legacy_string()), g);
        }
    }

    #[test]
    fn unknown_game_string_defaults_to_bgee() {
        assert_eq!(Game::from_legacy_string("???"), Game::BGEE);
    }
}
