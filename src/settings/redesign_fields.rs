// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `RedesignSettings` — sibling settings file for the redesign-only fields.
//
// Per Phase 4 P4.T10:
//   - Stored in `bio_redesign_settings.json`, **not** as a new field on
//     `bio::settings::model::AppSettings`. Per the CRITICAL DIRECTIVE,
//     existing BIO serde structs are not extended.
//   - Fields: `user_name`, `theme_palette`, `language`, `diagnostic_mode`.
//   - All `#[serde(default)]` so adding new fields stays backward-compat.
//
// SPEC: §11.1.

use serde::{Deserialize, Serialize};

/// Theme choice persisted alongside the redesign's other UI prefs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    Light,
    Dark,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        ThemeChoice::Dark
    }
}

/// UI language choice. Per SPEC §11.1, only English drives real text rendering
/// in v1 alpha — the other entries are a visual stub. The persisted value
/// stays stable across phases so the future i18n layer can read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    English,
    German,
    French,
    Spanish,
    Italian,
    Polish,
    Portuguese,
    Czech,
    Turkish,
    Ukrainian,
}

impl Default for UiLanguage {
    fn default() -> Self {
        UiLanguage::English
    }
}

impl UiLanguage {
    /// Stable display label for the General sub-tab ComboBox.
    pub fn label(self) -> &'static str {
        match self {
            UiLanguage::English => "English",
            UiLanguage::German => "Deutsch",
            UiLanguage::French => "Français",
            UiLanguage::Spanish => "Español",
            UiLanguage::Italian => "Italiano",
            UiLanguage::Polish => "Polski",
            UiLanguage::Portuguese => "Português",
            UiLanguage::Czech => "Čeština",
            UiLanguage::Turkish => "Türkçe",
            UiLanguage::Ukrainian => "Українська",
        }
    }

    /// The full ordered list shown in the ComboBox (SPEC §11.1).
    pub fn all() -> &'static [UiLanguage] {
        const ALL: [UiLanguage; 10] = [
            UiLanguage::English,
            UiLanguage::German,
            UiLanguage::French,
            UiLanguage::Spanish,
            UiLanguage::Italian,
            UiLanguage::Polish,
            UiLanguage::Portuguese,
            UiLanguage::Czech,
            UiLanguage::Turkish,
            UiLanguage::Ukrainian,
        ];
        &ALL
    }
}

/// All redesign-only persistent fields. Sibling to BIO's `AppSettings`;
/// **never** merged into `AppSettings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedesignSettings {
    /// User display name — feeds the share-code author field (SPEC §11.1) and
    /// the General sub-tab NameRow.
    pub user_name: String,
    /// Active theme palette persisted across launches.
    pub theme_palette: ThemeChoice,
    /// UI language choice (visual-stub for v1 alpha — see SPEC §11.1).
    pub language: UiLanguage,
    /// Persistent Diagnostic-mode toggle. OR'd with the CLI `-d` flag at app
    /// launch per M12.
    pub diagnostic_mode: bool,
    /// Whether path validation runs automatically at app launch.
    ///
    /// `true` (default): `OrchestratorApp::new` seeds
    /// `settings_screen_state.path_validation_results` with a synchronous
    /// `validate_now::run_now` pass so prefilled paths show their inline
    /// status the moment the user opens Settings → Paths.
    /// `false`: the seeding pass is skipped; validation runs only in response
    /// to user edits via the debounce cycle.
    #[serde(default = "default_true")]
    pub validate_paths_on_startup: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RedesignSettings {
    fn default() -> Self {
        Self {
            user_name: String::new(),
            theme_palette: ThemeChoice::default(),
            language: UiLanguage::default(),
            diagnostic_mode: false,
            validate_paths_on_startup: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_default() {
        let s = RedesignSettings::default();
        let raw = serde_json::to_string(&s).expect("serialize");
        let s2: RedesignSettings = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(s, s2);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let raw = r#"{}"#;
        let s: RedesignSettings = serde_json::from_str(raw).expect("backward-compat");
        assert_eq!(s.user_name, "");
        assert_eq!(s.theme_palette, ThemeChoice::Dark);
        assert_eq!(s.language, UiLanguage::English);
        assert!(!s.diagnostic_mode);
        assert!(s.validate_paths_on_startup);
    }

    #[test]
    fn round_trip_populated() {
        let s = RedesignSettings {
            user_name: "Tester".to_string(),
            theme_palette: ThemeChoice::Light,
            language: UiLanguage::French,
            diagnostic_mode: true,
            validate_paths_on_startup: false,
        };
        let raw = serde_json::to_string_pretty(&s).expect("serialize");
        let s2: RedesignSettings = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(s, s2);
    }
}
