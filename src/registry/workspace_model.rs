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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A reference to a single TP2 component, identifying it by tp2 path, the
/// component's numeric id, and the chosen install language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ComponentRef {
    /// TP2 path (normalized — `<mod_folder>/<file>.tp2`).
    pub tp2: String,
    /// Numeric component id from the TP2 file.
    pub id: i64,
    /// Selected install language (TP2 LANGUAGE index, typically 0).
    pub language: u8,
}

impl Default for ComponentRef {
    fn default() -> Self {
        Self {
            tp2: String::new(),
            id: 0,
            language: 0,
        }
    }
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
        let raw = r#"{}"#;
        let w: ModlistWorkspaceState = serde_json::from_str(raw).expect("parse");
        assert!(w.is_empty());
    }
}
