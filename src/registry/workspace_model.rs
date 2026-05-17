// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ModlistWorkspaceState` — serde model for per-modlist
// `modlists/<id>/workspace.json` files.
//
// Per Phase 3 P3.T3: holds the workspace-scoped state that lives outside
// `bio_settings.json` because it is per-modlist (Step 2 / 3 / 4 / 5 reorder
// & selection state, prompt overrides, last share code).
//
// Phase 6 wires the population: on workspace open, the loader reads this
// file and writes its values into the orchestrator's owned `WizardState`
// (Step 2 checked components, Step 3 order arrays, prompt overrides). On
// workspace close / nav-away / debounce tick, the loader extracts current
// `WizardState` values back into this struct and writes via `WorkspaceStore`.
//
// All fields are `#[serde(default)]` so the schema can gain fields additively.
//
// SPEC: §13.1.

// rationale: serde model — `#[must_use]` on trivial query churn (Cat 3);
// deriving `Eq` is a trait-bound surface change, not provably neutral (Cat 3).
#![allow(clippy::must_use_candidate, clippy::derive_partial_eq_without_eq)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A reference to a single TP2 component, identifying it by tp2 path, the
/// component's numeric id, and the chosen install language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ComponentRef {
    /// TP2 path (normalized — `<mod_folder>/<file>.tp2`).
    pub tp2: String,
    /// Numeric component id from the TP2 file.
    pub id: i64,
    /// Selected install language (TP2 LANGUAGE index, typically 0).
    pub language: u8,
}

/// One user-authored override of a TP2 prompt's evaluated answer.
///
/// Stored as plain strings to stay schema-agnostic across Phase 6+'s prompt
/// engine evolution. The orchestrator reads/writes these via the workspace
/// state loader.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptOverride {
    /// The prompt key (`<tp2>:<id>:<prompt_label>`).
    pub key: String,
    /// User-chosen answer (literal string).
    pub answer: String,
    /// True if the user explicitly chose "skip this prompt".
    pub skipped: bool,
}

/// The full per-modlist workspace state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistWorkspaceState {
    /// Step 3 order for BGEE installs.
    pub order_bgee: Vec<ComponentRef>,
    /// Step 3 order for BG2EE installs.
    pub order_bg2ee: Vec<ComponentRef>,
    /// Step 3 order for IWDEE installs (reserved — full IWDEE support arrives
    /// post-v1-alpha; the field is present so the schema stays stable).
    pub order_iwdee: Vec<ComponentRef>,

    /// Step 2 tree-expand state, keyed by `<tab>:<tp2>` and
    /// `<tab>:<tp2>:<parent>`.
    pub expand_state: HashMap<String, bool>,

    /// Step 3 group-collapse state, keyed by group label.
    pub step3_group_collapse: HashMap<String, bool>,

    /// User-authored prompt overrides keyed by `<tp2>:<id>:<prompt_label>`.
    pub prompt_overrides: HashMap<String, PromptOverride>,

    /// Last share / import code captured for this modlist.
    pub last_share_code: Option<String>,

    /// **Dev-scan source folder** (Phase 6 / Run 2b — the #1 fix). When the
    /// dev-only Step-2 scan affordance points BIO's scan at an arbitrary
    /// folder, that folder is recorded here so a **cold resume** (save draft →
    /// quit → relaunch) can re-point `wizard_state.step1.mods_folder` and
    /// re-run BIO's scan (which reads its own persisted scan cache, skipping
    /// WeiDU on a cache hit) to rebuild the scanned mod set the persisted
    /// `order_<tab>` arrays match against.
    ///
    /// `None` for the production path: there is **no** dev scan; the
    /// scannable mods folder is per-install, extracted at prep time by the
    /// Phase-7 P7.T17 pipeline (SPEC §13.12a) — pre-Phase-7 production
    /// legitimately has nothing here and resume legitimately finds nothing.
    /// `#[serde(default)]` (the struct is `#[serde(default)]` already) keeps
    /// older `workspace.json` files backward-compatible.
    pub dev_scanned_mods_folder: Option<String>,
}

impl ModlistWorkspaceState {
    /// True if the workspace state is at default / empty values.
    pub fn is_empty(&self) -> bool {
        self.order_bgee.is_empty()
            && self.order_bg2ee.is_empty()
            && self.order_iwdee.is_empty()
            && self.expand_state.is_empty()
            && self.step3_group_collapse.is_empty()
            && self.prompt_overrides.is_empty()
            && self.last_share_code.is_none()
            && self.dev_scanned_mods_folder.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_default_workspace() {
        let w = ModlistWorkspaceState::default();
        let s = serde_json::to_string(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(w, w2);
    }

    #[test]
    fn round_trip_populated_workspace() {
        let mut w = ModlistWorkspaceState::default();
        w.order_bgee.push(ComponentRef {
            tp2: "EET/EET.TP2".to_string(),
            id: 0,
            language: 0,
        });
        w.expand_state.insert("bgee:EET/EET.TP2".to_string(), true);
        w.last_share_code = Some("ABC-123".to_string());
        let s = serde_json::to_string_pretty(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(w, w2);
    }

    #[test]
    fn missing_fields_default_to_empty() {
        let raw = r"{}";
        let w: ModlistWorkspaceState = serde_json::from_str(raw).expect("parse");
        assert!(w.is_empty());
    }

    /// The #1-fix `dev_scanned_mods_folder` round-trips and `#[serde(default)]`
    /// keeps older files (which lack the key) backward-compatible.
    #[test]
    fn dev_scanned_mods_folder_round_trips_and_defaults() {
        // Older file without the key parses to `None` (no behavior change).
        let legacy: ModlistWorkspaceState =
            serde_json::from_str(r#"{"order_bgee":[]}"#).expect("parse legacy");
        assert_eq!(legacy.dev_scanned_mods_folder, None);
        assert!(legacy.is_empty());

        let mut w = ModlistWorkspaceState::default();
        w.dev_scanned_mods_folder = Some(r"D:\mods\test-corpus".to_string());
        assert!(!w.is_empty(), "a recorded dev-scan folder is not 'empty'");
        let s = serde_json::to_string(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(
            w2.dev_scanned_mods_folder.as_deref(),
            Some(r"D:\mods\test-corpus")
        );
        assert_eq!(w, w2);
    }
}
