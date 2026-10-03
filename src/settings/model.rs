// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

use crate::platform_defaults::{default_mod_installer_binary, default_weidu_binary};
use crate::settings::redesign_fields::RedesignSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Step1Settings<Flag = bool> {
    pub game_install: String,
    pub install_mode: String,
    pub have_weidu_logs: Flag,
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
    pub weidu_log_mode_enabled: Flag,
    pub new_pre_eet_dir_enabled: Flag,
    pub new_eet_dir_enabled: Flag,
    pub generate_directory_enabled: Flag,
    pub prepare_target_dirs_before_install: Flag,
    pub weidu_log_autolog: Flag,
    pub weidu_log_logapp: Flag,
    pub weidu_log_logextern: Flag,
    pub weidu_log_log_component: Flag,
    pub weidu_log_folder: String,
    pub mod_installer_binary: String,
    pub bgee_game_folder: String,
    pub bgee_log_folder: String,
    pub bgee_log_file: String,
    pub bg2ee_game_folder: String,
    pub bg2ee_log_folder: String,
    pub bg2ee_log_file: String,
    pub iwdee_game_folder: String,
    pub eet_bgee_game_folder: String,
    pub eet_bgee_log_folder: String,
    pub eet_bg2ee_game_folder: String,
    pub eet_bg2ee_log_folder: String,
    pub eet_pre_dir: String,
    pub eet_new_dir: String,
    pub game: String,
    pub log_file: String,
    pub generate_directory: String,
    pub mods_folder: String,
    pub global_mods_folder: String,
    pub weidu_binary: String,
    pub language: String,
    pub depth: usize,
    pub skip_installed: Flag,
    pub abort_on_warnings: Flag,
    pub timeout: usize,
    pub auto_answer_initial_delay_ms: usize,
    pub auto_answer_post_send_delay_ms: usize,
    pub weidu_log_mode: String,
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
    pub backup_targets_before_eet_copy: Flag,
}

impl Default for Step1Settings {
    fn default() -> Self {
        Self {
            game_install: "BGEE".to_string(),
            install_mode: "build_from_scanned_mods".to_string(),
            have_weidu_logs: false,
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
            weidu_log_mode_enabled: true,
            new_pre_eet_dir_enabled: false,
            new_eet_dir_enabled: false,
            generate_directory_enabled: false,
            prepare_target_dirs_before_install: false,
            weidu_log_autolog: true,
            weidu_log_logapp: true,
            weidu_log_logextern: true,
            weidu_log_log_component: false,
            weidu_log_folder: String::new(),
            mod_installer_binary: default_mod_installer_binary(),
            bgee_game_folder: String::new(),
            bgee_log_folder: String::new(),
            bgee_log_file: String::new(),
            bg2ee_game_folder: String::new(),
            bg2ee_log_folder: String::new(),
            bg2ee_log_file: String::new(),
            iwdee_game_folder: String::new(),
            eet_bgee_game_folder: String::new(),
            eet_bgee_log_folder: String::new(),
            eet_bg2ee_game_folder: String::new(),
            eet_bg2ee_log_folder: String::new(),
            eet_pre_dir: String::new(),
            eet_new_dir: String::new(),
            game: String::new(),
            log_file: String::new(),
            generate_directory: String::new(),
            mods_folder: String::new(),
            global_mods_folder: String::new(),
            weidu_binary: default_weidu_binary(),
            language: "en_US".to_string(),
            depth: 5,
            skip_installed: true,
            abort_on_warnings: false,
            timeout: 3600,
            auto_answer_initial_delay_ms: 2000,
            auto_answer_post_send_delay_ms: 5000,
            weidu_log_mode: "autolog,logapp,log-extern".to_string(),
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
            backup_targets_before_eet_copy: false,
        }
    }
}

impl Step1Settings {
    #[must_use]
    pub fn effective_global_mods_folder(&self) -> &str {
        let trimmed = self.global_mods_folder.trim();
        if trimmed.is_empty() {
            &self.mods_folder
        } else {
            trimmed
        }
    }

    pub fn clear_unreachable_eet_sources(&mut self) -> Option<(String, String)> {
        let pre_eet_unreachable =
            self.new_pre_eet_dir_enabled && !self.bgee_game_folder.trim().is_empty();
        let new_eet_unreachable =
            self.new_eet_dir_enabled && !self.bg2ee_game_folder.trim().is_empty();
        let previous_pre_eet = self.eet_bgee_game_folder.clone();
        let previous_new_eet = self.eet_bg2ee_game_folder.clone();
        let cleared_pre_eet = pre_eet_unreachable && !previous_pre_eet.is_empty();
        let cleared_new_eet = new_eet_unreachable && !previous_new_eet.is_empty();
        if cleared_pre_eet {
            self.eet_bgee_game_folder.clear();
        }
        if cleared_new_eet {
            self.eet_bg2ee_game_folder.clear();
        }
        (cleared_pre_eet || cleared_new_eet).then_some((previous_pre_eet, previous_new_eet))
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
            mods_folder: r"C:\old\mods".to_string(),
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let s2: Step1Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s2.global_mods_folder, r"C:\global\mods");
        assert_eq!(s2.mods_folder, r"C:\old\mods");
    }

    #[test]
    fn step1_settings_legacy_default_missing_global_mods_folder() {
        let json = r#"{"mods_folder":"C:\\old\\mods"}"#;
        let s: Step1Settings = serde_json::from_str(json).expect("deserialize legacy");
        assert_eq!(s.mods_folder, r"C:\old\mods");
        assert_eq!(
            s.global_mods_folder, "",
            "missing key must default to empty"
        );
    }

    #[test]
    fn effective_global_mods_folder_returns_global_when_set() {
        let s = Step1Settings {
            mods_folder: r"C:\old\mods".to_string(),
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        assert_eq!(s.effective_global_mods_folder(), r"C:\global\mods");
    }

    #[test]
    fn effective_global_mods_folder_falls_back_to_mods_folder_when_global_empty() {
        let s = Step1Settings {
            mods_folder: r"C:\old\mods".to_string(),
            global_mods_folder: String::new(),
            ..Step1Settings::default()
        };
        assert_eq!(s.effective_global_mods_folder(), r"C:\old\mods");
    }

    #[test]
    fn effective_global_mods_folder_treats_whitespace_only_global_as_empty() {
        let s = Step1Settings {
            mods_folder: r"C:\old\mods".to_string(),
            global_mods_folder: "   ".to_string(),
            ..Step1Settings::default()
        };
        assert_eq!(s.effective_global_mods_folder(), r"C:\old\mods");
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

    fn eet_step1(pre_on: bool, new_on: bool, plain_filled: bool) -> Step1Settings {
        Step1Settings {
            new_pre_eet_dir_enabled: pre_on,
            new_eet_dir_enabled: new_on,
            bgee_game_folder: if plain_filled { "C:\\Games\\BGEE" } else { "" }.to_string(),
            bg2ee_game_folder: if plain_filled { "C:\\Games\\BG2EE" } else { "" }.to_string(),
            eet_bgee_game_folder: "C:\\Games\\OldEetBgee".to_string(),
            eet_bg2ee_game_folder: "C:\\Games\\OldEetBg2ee".to_string(),
            ..Step1Settings::default()
        }
    }

    #[test]
    fn unreachable_eet_sources_are_cleared() {
        let mut step1 = eet_step1(true, true, true);
        let cleared = step1.clear_unreachable_eet_sources();
        assert_eq!(
            cleared,
            Some((
                "C:\\Games\\OldEetBgee".to_string(),
                "C:\\Games\\OldEetBg2ee".to_string()
            ))
        );
        assert_eq!(step1.eet_bgee_game_folder, "");
        assert_eq!(step1.eet_bg2ee_game_folder, "");
    }

    #[test]
    fn eet_sources_stay_when_the_switch_is_off() {
        let mut step1 = eet_step1(false, false, true);
        assert_eq!(step1.clear_unreachable_eet_sources(), None);
        assert_eq!(step1.eet_bgee_game_folder, "C:\\Games\\OldEetBgee");
        assert_eq!(step1.eet_bg2ee_game_folder, "C:\\Games\\OldEetBg2ee");
    }

    #[test]
    fn eet_sources_stay_when_the_plain_row_is_empty() {
        let mut step1 = eet_step1(true, true, false);
        assert_eq!(step1.clear_unreachable_eet_sources(), None);
        assert_eq!(step1.eet_bgee_game_folder, "C:\\Games\\OldEetBgee");
        assert_eq!(step1.eet_bg2ee_game_folder, "C:\\Games\\OldEetBg2ee");
    }

    #[test]
    fn only_the_reachable_side_is_kept() {
        let mut step1 = eet_step1(true, false, true);
        let cleared = step1.clear_unreachable_eet_sources();
        assert_eq!(
            cleared,
            Some((
                "C:\\Games\\OldEetBgee".to_string(),
                "C:\\Games\\OldEetBg2ee".to_string()
            ))
        );
        assert_eq!(step1.eet_bgee_game_folder, "");
        assert_eq!(step1.eet_bg2ee_game_folder, "C:\\Games\\OldEetBg2ee");
    }

    #[test]
    fn empty_hidden_fields_report_nothing() {
        let mut step1 = eet_step1(true, true, true);
        step1.eet_bgee_game_folder.clear();
        step1.eet_bg2ee_game_folder.clear();
        assert_eq!(step1.clear_unreachable_eet_sources(), None);
    }
}
