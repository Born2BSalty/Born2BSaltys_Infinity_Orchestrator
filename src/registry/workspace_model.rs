// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ComponentRef {
    pub tp2: String,

    pub id: i64,

    pub language: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptOverride {
    pub key: String,

    pub answer: String,

    pub skipped: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModlistWorkspaceState {
    pub order_bgee: Vec<ComponentRef>,

    pub order_bg2ee: Vec<ComponentRef>,

    pub order_iwdee: Vec<ComponentRef>,

    pub expand_state: HashMap<String, bool>,

    pub step3_group_collapse: HashMap<String, bool>,

    pub prompt_overrides: HashMap<String, PromptOverride>,

    pub last_share_code: Option<String>,

    pub dev_scanned_mods_folder: Option<String>,
}

impl ModlistWorkspaceState {
    #[must_use]
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

    #[test]
    fn dev_scanned_mods_folder_round_trips_and_defaults() {
        let legacy: ModlistWorkspaceState =
            serde_json::from_str(r#"{"order_bgee":[]}"#).expect("parse legacy");
        assert_eq!(legacy.dev_scanned_mods_folder, None);
        assert!(legacy.is_empty());

        let w = ModlistWorkspaceState {
            dev_scanned_mods_folder: Some(r"D:\mods\test-corpus".to_string()),
            ..Default::default()
        };
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
