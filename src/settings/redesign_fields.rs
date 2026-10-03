// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    #[default]
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

impl UiLanguage {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
            Self::French => "Français",
            Self::Spanish => "Español",
            Self::Italian => "Italiano",
            Self::Polish => "Polski",
            Self::Portuguese => "Português",
            Self::Czech => "Čeština",
            Self::Turkish => "Türkçe",
            Self::Ukrainian => "Українська",
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedesignSettings {
    pub user_name: String,

    pub theme_palette: ThemeChoice,

    pub language: UiLanguage,

    pub diagnostic_mode: bool,

    #[serde(default = "default_true")]
    pub validate_paths_on_startup: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub step2_details_width: Option<u16>,
}

const fn default_true() -> bool {
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
            step2_details_width: None,
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
        let raw = r"{}";
        let s: RedesignSettings = serde_json::from_str(raw).expect("backward-compat");
        assert_eq!(s.user_name, "");
        assert_eq!(s.theme_palette, ThemeChoice::Dark);
        assert_eq!(s.language, UiLanguage::English);
        assert!(!s.diagnostic_mode);
        assert!(s.validate_paths_on_startup);
        assert!(s.step2_details_width.is_none());
    }

    #[test]
    fn round_trip_populated() {
        let s = RedesignSettings {
            user_name: "Tester".to_string(),
            theme_palette: ThemeChoice::Light,
            language: UiLanguage::French,
            diagnostic_mode: true,
            validate_paths_on_startup: false,
            step2_details_width: Some(700),
        };
        let raw = serde_json::to_string_pretty(&s).expect("serialize");
        let s2: RedesignSettings = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(s, s2);
    }

    #[test]
    fn step2_details_width_round_trips() {
        let s = RedesignSettings {
            step2_details_width: Some(700),
            ..RedesignSettings::default()
        };
        let raw = serde_json::to_string(&s).expect("serialize");
        let s2: RedesignSettings = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(s2.step2_details_width, Some(700));
    }

    #[test]
    fn step2_details_width_absent_when_none() {
        let s = RedesignSettings::default();
        let raw = serde_json::to_string(&s).expect("serialize");
        assert!(!raw.contains("step2_details_width"));
    }

    #[test]
    fn an_unknown_general_key_is_ignored() {
        let s: RedesignSettings =
            serde_json::from_str(r#"{"gallery_index_url":"x","user_name":"Tester"}"#)
                .expect("unknown keys are ignored");
        assert_eq!(s.user_name, "Tester");
    }
}
