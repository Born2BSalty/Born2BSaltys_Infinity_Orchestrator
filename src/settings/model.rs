// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

use crate::platform_defaults::{default_mod_installer_binary, default_weidu_binary};
use crate::settings::redesign_fields::RedesignSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Step1Settings<Flag = bool> {
    pub rust_log_debug: Flag,
    pub rust_log_trace: Flag,
    pub custom_scan_depth: Flag,
    pub timeout_per_mod_enabled: Flag,
    pub auto_answer_initial_delay_enabled: Flag,
    pub auto_answer_post_send_delay_enabled: Flag,
    pub prompt_required_sound_enabled: Flag,
    pub lookback_enabled: Flag,
    pub bio_full_debug: Flag,
    pub tick_dev_enabled: Flag,
    pub log_raw_output_dev: Flag,
    pub mod_installer_binary: String,
    pub bgee_game_folder: String,
    pub bg2ee_game_folder: String,
    pub iwdee_game_folder: String,
    pub global_mods_folder: String,
    pub weidu_binary: String,
    pub language: String,
    pub depth: usize,
    pub skip_installed: Flag,
    pub abort_on_warnings: Flag,
    pub timeout: usize,
    pub auto_answer_initial_delay_ms: usize,
    pub auto_answer_post_send_delay_ms: usize,
    pub strict_matching: Flag,
    pub download: Flag,
    pub download_archive: Flag,
    pub mods_archive_folder: String,
    pub mods_backup_folder: String,
    pub overwrite: Flag,
    pub check_last_installed: Flag,
    pub tick: u64,
    pub lookback: usize,
    pub casefold: Flag,
}

impl Default for Step1Settings {
    fn default() -> Self {
        Self {
            rust_log_debug: false,
            rust_log_trace: false,
            custom_scan_depth: false,
            timeout_per_mod_enabled: false,
            auto_answer_initial_delay_enabled: false,
            auto_answer_post_send_delay_enabled: false,
            prompt_required_sound_enabled: true,
            lookback_enabled: false,
            bio_full_debug: false,
            tick_dev_enabled: false,
            log_raw_output_dev: false,
            mod_installer_binary: default_mod_installer_binary(),
            bgee_game_folder: String::new(),
            bg2ee_game_folder: String::new(),
            iwdee_game_folder: String::new(),
            global_mods_folder: String::new(),
            weidu_binary: default_weidu_binary(),
            language: "en_US".to_string(),
            depth: 5,
            skip_installed: true,
            abort_on_warnings: false,
            timeout: 3600,
            auto_answer_initial_delay_ms: 2000,
            auto_answer_post_send_delay_ms: 5000,
            strict_matching: false,
            download: true,
            download_archive: false,
            mods_archive_folder: String::new(),
            mods_backup_folder: String::new(),
            overwrite: false,
            check_last_installed: true,
            tick: 500,
            lookback: 10,
            casefold: false,
        }
    }
}

impl Step1Settings {
    #[must_use]
    pub fn effective_global_mods_folder(&self) -> &str {
        self.global_mods_folder.trim()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct AppSettings {
    pub exe_fingerprint: String,
    pub step1: Step1Settings,
    pub general: RedesignSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step1_settings_round_trips_global_mods_folder() {
        let s = Step1Settings {
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let s2: Step1Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s2.global_mods_folder, r"C:\global\mods");
    }

    #[test]
    fn a_file_with_per_list_keys_loads_and_drops_them_on_save() {
        let json = r#"{"exe_fingerprint":"x","step1":{"mods_folder":"C:\\old\\mods","generate_directory":"C:\\old\\clone","bgee_game_folder":"C:\\src"}}"#;
        let s: AppSettings = serde_json::from_str(json).expect("deserialize legacy");
        assert_eq!(s.step1.bgee_game_folder, r"C:\src");
        assert_eq!(s.step1.global_mods_folder, "");
        let reserialised = serde_json::to_string(&s).expect("serialize");
        assert!(!reserialised.contains("\"mods_folder\""), "{reserialised}");
        assert!(
            !reserialised.contains("\"generate_directory\""),
            "{reserialised}"
        );
    }

    #[test]
    fn effective_global_mods_folder_returns_global_when_set() {
        let s = Step1Settings {
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        assert_eq!(s.effective_global_mods_folder(), r"C:\global\mods");
    }

    #[test]
    fn effective_global_mods_folder_is_empty_when_global_is_blank() {
        let s = Step1Settings {
            global_mods_folder: "   ".to_string(),
            ..Step1Settings::default()
        };
        assert_eq!(s.effective_global_mods_folder(), "");
    }

    #[test]
    fn app_settings_without_general_block_uses_general_defaults() {
        let json = r#"{"exe_fingerprint":"x","step1":{}}"#;
        let s: AppSettings = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.general, RedesignSettings::default());
    }

    #[test]
    fn app_settings_round_trips_general() {
        let s = AppSettings {
            general: RedesignSettings {
                user_name: "@me".to_string(),
                theme_palette: crate::settings::redesign_fields::ThemeChoice::Light,
                ..RedesignSettings::default()
            },
            ..AppSettings::default()
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let s2: AppSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s2, s);
    }
}
