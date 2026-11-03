// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::{Step2ModState, Step2ScanReport};
use crate::parser::PromptSummaryEvent;

pub mod cache;
pub mod discovery;
pub mod parse;
pub mod readme;
pub mod worker;

pub use worker::detect_preferred_game_locale;

pub const ENABLE_TWO_PHASE_PREVIEW: bool = true;
pub const SCAN_CACHE_FILE: &str = "bio_scan_cache.json";
pub const SCAN_CACHE_VERSION: u32 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedComponent {
    pub tp_file: Option<String>,
    pub component_id: String,
    pub display: String,
    pub raw_line: String,
    #[serde(default)]
    pub prompt_summary: Option<String>,
    #[serde(default)]
    pub prompt_events: Vec<PromptSummaryEvent>,
    #[serde(default)]
    pub mod_prompt_summary: Option<String>,
    #[serde(default)]
    pub mod_prompt_events: Vec<PromptSummaryEvent>,
    #[serde(default)]
    pub language_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModInstallLanguages {
    pub bgee: BTreeMap<String, String>,
    pub bg2ee: BTreeMap<String, String>,
}

impl ModInstallLanguages {
    #[must_use]
    pub fn from_scan(
        first_game_mods: &[Step2ModState],
        second_game_mods: &[Step2ModState],
        languages_by_tp2: &BTreeMap<String, String>,
    ) -> Self {
        Self {
            bgee: side_mod_languages(first_game_mods, languages_by_tp2),
            bg2ee: side_mod_languages(second_game_mods, languages_by_tp2),
        }
    }
}

#[must_use]
pub fn languages_by_component_tp2(
    mods_map: &BTreeMap<String, Vec<ScannedComponent>>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for component in mods_map.values().flatten() {
        let language = component.language_name.trim();
        if language.is_empty() {
            continue;
        }
        if let Some(key) = raw_line_tp2_key(&component.raw_line) {
            out.entry(key).or_insert_with(|| language.to_string());
        }
    }
    out
}

fn raw_line_tp2_key(raw_line: &str) -> Option<String> {
    let (tp2, _) = raw_line.trim_start().strip_prefix('~')?.split_once('~')?;
    let key = normalize_mod_download_tp2(tp2);
    (!key.is_empty()).then_some(key)
}

fn side_mod_languages(
    mods: &[Step2ModState],
    languages_by_tp2: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for component in mods.iter().flat_map(|mod_state| &mod_state.components) {
        let Some(key) = raw_line_tp2_key(&component.raw_line) else {
            continue;
        };
        if let Some(language) = languages_by_tp2.get(&key) {
            out.entry(key).or_insert_with(|| language.clone());
        }
    }
    out
}

pub enum Step2ScanEvent {
    Preview {
        bgee_mods: Vec<Step2ModState>,
        bg2ee_mods: Vec<Step2ModState>,
        total: usize,
    },
    Progress {
        current: usize,
        total: usize,
        name: String,
    },
    Finished {
        bgee_mods: Vec<Step2ModState>,
        bg2ee_mods: Vec<Step2ModState>,
        mod_languages: ModInstallLanguages,
        report: Box<Step2ScanReport>,
    },
    Failed(String),
    Canceled,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{ModInstallLanguages, ScannedComponent, languages_by_component_tp2};
    use crate::app::state::{Step2ComponentState, Step2ModState, Step2State};

    fn scanned(tp_file: &str, language_name: &str) -> ScannedComponent {
        ScannedComponent {
            tp_file: Some(tp_file.to_string()),
            component_id: "0".to_string(),
            display: "Component".to_string(),
            raw_line: format!("~{tp_file}~ #0 #0 // Component"),
            prompt_summary: None,
            prompt_events: Vec::new(),
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            language_name: language_name.to_string(),
        }
    }

    fn component(raw_line: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: "0".to_string(),
            label: "Component".to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: raw_line.to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    fn mod_state(tp_file: &str, components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("/mods/{tp_file}"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }
    }

    fn scan_result(
        groups: &[(&str, &[(&str, &str)])],
    ) -> (BTreeMap<String, String>, Vec<Step2ModState>) {
        let mut mods_map = BTreeMap::<String, Vec<ScannedComponent>>::new();
        let mut mods = Vec::new();
        for (group, rows) in groups {
            let scanned_rows = rows
                .iter()
                .map(|(tp_file, language)| scanned(tp_file, language))
                .collect::<Vec<_>>();
            mods.push(mod_state(
                rows.first().map_or("", |(tp_file, _)| tp_file),
                scanned_rows
                    .iter()
                    .map(|row| component(&row.raw_line))
                    .collect(),
            ));
            mods_map.insert((*group).to_string(), scanned_rows);
        }
        (languages_by_component_tp2(&mods_map), mods)
    }

    #[test]
    fn sibling_tp2s_keep_their_own_language() {
        let (by_tp2, mods) = scan_result(&[(
            "main",
            &[
                ("SIBLING.TP2", "english"),
                ("MAIN.TP2", "polish"),
                ("MAIN.TP2", "polish"),
                ("SIBLING.TP2", "english"),
            ],
        )]);
        let mut step2 = Step2State::default();
        step2.replace_mod_languages(ModInstallLanguages::from_scan(&mods, &[], &by_tp2));
        assert_eq!(
            step2.mod_install_language("BGEE", "MAIN.TP2"),
            Some("polish")
        );
        assert_eq!(
            step2.mod_install_language("BGEE", "SIBLING.TP2"),
            Some("english")
        );
        assert_eq!(step2.mod_install_language("BG2EE", "MAIN.TP2"), None);
    }

    #[test]
    fn mod_languages_filled_per_side_and_replaced_on_rescan() {
        let (by_path, mods) = scan_result(&[
            (
                "tweaks",
                &[("SETUP-TWEAKS.TP2", ""), ("SETUP-TWEAKS.TP2", "polish")],
            ),
            ("ascension", &[("ascension.tp2", "english")]),
            ("blank", &[("blank.tp2", "")]),
        ]);
        let (first_game_mods, second_game_mods) = mods.split_at(1);
        let mut step2 = Step2State::default();
        step2.replace_mod_languages(ModInstallLanguages::from_scan(
            first_game_mods,
            second_game_mods,
            &by_path,
        ));

        assert_eq!(
            step2.mod_install_language("BGEE", "SETUP-TWEAKS.TP2"),
            Some("polish")
        );
        assert_eq!(
            step2.mod_install_language("BGEE", "tweaks/setup-tweaks.tp2"),
            Some("polish")
        );
        assert_eq!(
            step2.mod_install_language("BG2EE", "SETUP-TWEAKS.TP2"),
            None
        );
        assert_eq!(
            step2.mod_install_language("BG2EE", "ascension.tp2"),
            Some("english")
        );
        assert_eq!(step2.mod_install_language("BGEE", "ascension.tp2"), None);
        assert_eq!(step2.mod_install_language("BG2EE", "blank.tp2"), None);

        let (by_path, mods) = scan_result(&[("ascension", &[("ascension.tp2", "german")])]);
        step2.replace_mod_languages(ModInstallLanguages::from_scan(&mods, &mods, &by_path));

        assert_eq!(step2.mod_install_language("BGEE", "SETUP-TWEAKS.TP2"), None);
        assert_eq!(
            step2.mod_install_language("BGEE", "ascension.tp2"),
            Some("german")
        );
        assert_eq!(
            step2.mod_install_language("BG2EE", "ascension.tp2"),
            Some("german")
        );
        assert_eq!(step2.bgee_mod_languages.len(), 1);
        assert_eq!(step2.bg2ee_mod_languages.len(), 1);
    }
}
