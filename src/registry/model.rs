// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ModlistRegistry` — serde model for `modlists.json`.
//
// Per Phase 3 P3.T1 (SPEC §13.1):
//   - `ModlistRegistry { format_version, entries }` — top-level container.
//   - `ModlistEntry { id, name, game, destination_folder, state, … }` — one row.
//   - `ModlistState { InProgress, Installed }` — lifecycle stage.
//   - `Game { BGEE, BG2EE, IWDEE, EET }` — chosen game family per entry.
//
// All fields are `#[serde(default)]` at the struct level so the schema can
// gain new fields additively without breaking older `modlists.json` files
// already on disk. `#[serde(rename_all = "snake_case")]` keeps the JSON
// human-readable.
//
// Timestamps use `chrono::DateTime<Utc>` (already a `Cargo.toml` dep).
//
// SPEC: §13.1, §15 (additive schema evolution).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The top-level `modlists.json` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistRegistry {
    /// Schema version. Incremented when a non-backward-compatible change is
    /// introduced (we expect to stay at 1 for the v1 alpha lifecycle).
    pub format_version: u32,
    /// All known modlists. Ordering is the on-disk write order; the Home
    /// screen sorts at render time.
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
    /// True if no entries are stored. Used by Home's first-launch empty state
    /// and the statusbar's `0 modlists` segment.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up an entry by id.
    pub fn find(&self, id: &str) -> Option<&ModlistEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Mutably look up an entry by id.
    pub fn find_mut(&mut self, id: &str) -> Option<&mut ModlistEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }
}

/// One modlist row in the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistEntry {
    /// Stable opaque id (12-char base32 ULID via `ids::new_modlist_id`).
    pub id: String,
    /// Display name shown in Home cards / Workspace title.
    pub name: String,
    /// Chosen game family for this modlist.
    pub game: Game,
    /// Final install folder (per-modlist, not per-app).
    pub destination_folder: String,
    /// Lifecycle stage — in progress vs installed.
    pub state: ModlistState,
    /// When the entry was first created (UTC).
    pub creation_date: DateTime<Utc>,
    /// Last time the user touched the workspace (UTC). Bumped on every
    /// successful workspace state write.
    pub last_touched_date: DateTime<Utc>,
    /// When the install completed (None for in-progress entries).
    pub install_date: Option<DateTime<Utc>>,
    /// When the user last clicked "Play" / opened the install folder.
    pub last_played_date: Option<DateTime<Utc>>,
    /// Cached mod count for the Home card meta line.
    pub mod_count: u32,
    /// Cached component count for the Home card meta line.
    pub component_count: u32,
    /// Cached workspace step (2–5) the in-progress build is paused at, for
    /// the Home card meta line (`… · paused at Step <K>`). Denormalized onto
    /// the registry the same way `mod_count` / `component_count` are, so Home
    /// renders without loading `workspace.json`. `None` for installed entries
    /// (their meta line shows `installed <when>` instead). Kept in sync by
    /// the Phase 6 workspace-persistence cycle on every workspace write.
    pub paused_at_step: Option<u8>,
    /// Cached total install footprint (computed post-install in Phase 7).
    pub total_size_bytes: Option<u64>,
    /// Last share/import code captured for this modlist.
    pub latest_share_code: Option<String>,
    /// Workspace state file path **relative** to the orchestrator's config
    /// dir. Always `modlists/<id>/workspace.json` in practice.
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
            last_played_date: None,
            mod_count: 0,
            component_count: 0,
            paused_at_step: None,
            total_size_bytes: None,
            latest_share_code: None,
            workspace_file_relpath: PathBuf::new(),
        }
    }
}

/// The lifecycle stage of a modlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModlistState {
    /// Workspace edits ongoing; install not yet completed.
    InProgress,
    /// Install completed; entry is read-only from Home (Reinstall = clone).
    Installed,
}

impl Default for ModlistState {
    fn default() -> Self {
        ModlistState::InProgress
    }
}

/// Game family for an entry. Mirrors BIO's `game_install` string set but is
/// strongly typed for the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Game {
    BGEE,
    BG2EE,
    IWDEE,
    EET,
}

impl Default for Game {
    fn default() -> Self {
        Game::BGEE
    }
}

impl Game {
    /// Map to the BIO `game_install` legacy string used by `Step1State`.
    pub fn to_legacy_string(self) -> &'static str {
        match self {
            Game::BGEE => "BGEE",
            Game::BG2EE => "BG2EE",
            Game::IWDEE => "IWDEE",
            Game::EET => "EET",
        }
    }

    /// Parse the legacy `Step1State::game_install` string back into a `Game`.
    /// Defaults to `BGEE` for unrecognized inputs (matches BIO's behavior).
    pub fn from_legacy_string(s: &str) -> Self {
        match s.trim() {
            "BG2EE" => Game::BG2EE,
            "IWDEE" => Game::IWDEE,
            "EET" => Game::EET,
            _ => Game::BGEE,
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
        // Older `modlists.json` that lacks `total_size_bytes` parses fine.
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
