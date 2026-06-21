// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ui::install::state_install::DestChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModsSource {
    #[default]
    InstallationFolder,
    GlobalModsFolder,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ComponentRef {
    pub tp2: String,

    pub id: i64,

    pub language: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub wlb_inputs: Option<String>,
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scratch_mods_folder: Option<String>,

    pub dev_scanned_mods_folder: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_destination_prep: Option<DestChoice>,

    #[serde(default)]
    pub mods_source: ModsSource,

    #[serde(default)]
    pub last_rescanned_mods_source: ModsSource,
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
            && self.scratch_mods_folder.is_none()
            && self.dev_scanned_mods_folder.is_none()
            && self.pending_destination_prep.is_none()
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
            wlb_inputs: None,
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
    fn pending_destination_prep_round_trips_and_defaults() {
        let legacy: ModlistWorkspaceState =
            serde_json::from_str(r#"{"order_bgee":[]}"#).expect("parse legacy");
        assert_eq!(legacy.pending_destination_prep, None);
        assert!(legacy.is_empty());

        let w = ModlistWorkspaceState {
            pending_destination_prep: Some(DestChoice::Clear),
            ..Default::default()
        };
        assert!(!w.is_empty(), "a deferred destination prep is not 'empty'");
        let s = serde_json::to_string(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(w2.pending_destination_prep, Some(DestChoice::Clear));
        assert_eq!(w, w2);

        for choice in [DestChoice::Clear, DestChoice::Backup, DestChoice::Continue] {
            let with = ModlistWorkspaceState {
                pending_destination_prep: Some(choice),
                ..Default::default()
            };
            let s = serde_json::to_string(&with).unwrap();
            let r: ModlistWorkspaceState = serde_json::from_str(&s).unwrap();
            assert_eq!(r.pending_destination_prep, Some(choice));
        }
    }

    #[test]
    fn none_pending_destination_prep_is_omitted_on_serialize() {
        let default_ws = ModlistWorkspaceState::default();
        let s = serde_json::to_string(&default_ws).expect("serialize");
        assert!(
            !s.contains("pending_destination_prep"),
            "the new field MUST be omitted on serialize when None so the \
             emitted JSON is byte-identical to a pre-field workspace.json; \
             got: {s}"
        );
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

    #[test]
    fn mods_source_round_trips_and_defaults() {
        let legacy: ModlistWorkspaceState =
            serde_json::from_str(r#"{"order_bgee":[]}"#).expect("parse legacy");
        assert_eq!(legacy.mods_source, ModsSource::InstallationFolder);
        assert_eq!(
            legacy.last_rescanned_mods_source,
            ModsSource::InstallationFolder
        );
        assert!(legacy.is_empty());

        let w = ModlistWorkspaceState {
            mods_source: ModsSource::GlobalModsFolder,
            last_rescanned_mods_source: ModsSource::GlobalModsFolder,
            ..Default::default()
        };
        let s = serde_json::to_string(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(w2.mods_source, ModsSource::GlobalModsFolder);
        assert_eq!(w2.last_rescanned_mods_source, ModsSource::GlobalModsFolder);
        assert_eq!(w, w2);

        let with_install = ModlistWorkspaceState {
            mods_source: ModsSource::InstallationFolder,
            last_rescanned_mods_source: ModsSource::InstallationFolder,
            ..Default::default()
        };
        let s2 = serde_json::to_string(&with_install).expect("serialize");
        let w3: ModlistWorkspaceState = serde_json::from_str(&s2).expect("deserialize");
        assert_eq!(w3.mods_source, ModsSource::InstallationFolder);
        assert_eq!(
            w3.last_rescanned_mods_source,
            ModsSource::InstallationFolder
        );
    }

    #[test]
    fn scratch_mods_folder_round_trips_and_defaults() {
        let legacy: ModlistWorkspaceState =
            serde_json::from_str(r#"{"order_bgee":[]}"#).expect("parse legacy");
        assert_eq!(legacy.scratch_mods_folder, None);
        assert!(legacy.is_empty());

        let w = ModlistWorkspaceState {
            scratch_mods_folder: Some(r"D:\BIO\my-list\mods".to_string()),
            ..Default::default()
        };
        assert!(!w.is_empty(), "a scratch mods folder is not 'empty'");
        let s = serde_json::to_string(&w).expect("serialize");
        let w2: ModlistWorkspaceState = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(
            w2.scratch_mods_folder.as_deref(),
            Some(r"D:\BIO\my-list\mods")
        );
        assert_eq!(w, w2);
    }

    #[test]
    fn none_wlb_inputs_is_omitted_on_serialize() {
        let r = ComponentRef {
            tp2: "MOD/MOD.TP2".to_string(),
            id: 0,
            language: 0,
            wlb_inputs: None,
        };
        let s = serde_json::to_string(&r).expect("serialize");
        assert!(
            !s.contains("wlb_inputs"),
            "wlb_inputs must be omitted when None so the emitted JSON is \
             byte-identical to a pre-field order entry; got: {s}"
        );
    }

    #[test]
    fn some_wlb_inputs_round_trips() {
        let r = ComponentRef {
            tp2: "MOD/MOD.TP2".to_string(),
            id: 5,
            language: 0,
            wlb_inputs: Some(r"y,D:\test1".to_string()),
        };
        let s = serde_json::to_string(&r).expect("serialize");
        let r2: ComponentRef = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(r2.wlb_inputs.as_deref(), Some(r"y,D:\test1"));
        assert_eq!(r, r2);
    }

    #[test]
    fn legacy_order_entry_without_wlb_inputs_parses_to_none() {
        let raw = r#"{"tp2":"MOD/MOD.TP2","id":3,"language":0}"#;
        let r: ComponentRef = serde_json::from_str(raw).expect("parse legacy");
        assert_eq!(
            r.wlb_inputs, None,
            "legacy entry without the field must parse to None"
        );
    }
}
