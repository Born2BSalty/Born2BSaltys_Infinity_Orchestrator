// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::time::Instant;

use crate::settings::redesign_fields::RedesignSettings;
use crate::ui::shared::redesign_tokens::ThemePalette;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Paths,
    Tools,
    Accounts,
    Advanced,
}

impl SettingsTab {
    pub const ORDERED: [Self; 5] = [
        Self::General,
        Self::Paths,
        Self::Tools,
        Self::Accounts,
        Self::Advanced,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Paths => "Paths",
            Self::Tools => "Tools",
            Self::Accounts => "Accounts",
            Self::Advanced => "Advanced",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsScreenState {
    pub active_tab: SettingsTab,
    pub general_changed_this_frame: bool,
    pub path_edit_debounce: HashMap<&'static str, Instant>,
    pub user_name: String,
    pub selected_theme: ThemePalette,
    pub language: String,
    pub validate_paths_on_startup: bool,
    pub diagnostic_mode: bool,
    pub bgee_source_path: String,
    pub bg2ee_source_path: String,
    pub iwdee_source_path: String,
    pub mods_archive_path: String,
    pub mods_backup_path: String,
    pub tools_path: String,
    pub temp_path: String,
    pub weidu_executable_path: String,
    pub mod_installer_executable_path: String,
    pub seven_zip_executable_path: String,
    pub git_executable_path: String,
    pub custom_scan_depth: String,
    pub mod_install_timeout: String,
    pub mod_install_timeout_per_mod: String,
    pub auto_answer_initial_delay: String,
    pub auto_answer_post_send_delay: String,
    pub tick_dev: String,
    pub prompt_context_lookback: String,
    pub sound_cue_when_prompt_input_required: bool,
    pub download_missing_mods_and_keep_archives: bool,
    pub case_insensitive_component_matching: bool,
    pub abort_on_warnings: bool,
    pub strict_matching: bool,
    pub overwrite_mod_folder: bool,
}

impl Default for SettingsScreenState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::default(),
            general_changed_this_frame: false,
            path_edit_debounce: HashMap::new(),
            user_name: "@b2bs".to_string(),
            selected_theme: ThemePalette::Dark,
            language: "English".to_string(),
            validate_paths_on_startup: true,
            diagnostic_mode: false,
            bgee_source_path: String::new(),
            bg2ee_source_path: String::new(),
            iwdee_source_path: String::new(),
            mods_archive_path: String::new(),
            mods_backup_path: String::new(),
            tools_path: String::new(),
            temp_path: String::new(),
            weidu_executable_path: String::new(),
            mod_installer_executable_path: String::new(),
            seven_zip_executable_path: String::new(),
            git_executable_path: String::new(),
            custom_scan_depth: String::new(),
            mod_install_timeout: String::new(),
            mod_install_timeout_per_mod: String::new(),
            auto_answer_initial_delay: String::new(),
            auto_answer_post_send_delay: String::new(),
            tick_dev: String::new(),
            prompt_context_lookback: String::new(),
            sound_cue_when_prompt_input_required: true,
            download_missing_mods_and_keep_archives: true,
            case_insensitive_component_matching: false,
            abort_on_warnings: false,
            strict_matching: false,
            overwrite_mod_folder: false,
        }
    }
}

impl SettingsScreenState {
    pub fn from_redesign_settings(settings: &RedesignSettings) -> Self {
        Self {
            user_name: settings.user_name.clone(),
            selected_theme: settings.theme_palette,
            language: settings.language.clone(),
            validate_paths_on_startup: settings.validate_paths_on_startup,
            diagnostic_mode: settings.diagnostic_mode,
            ..Default::default()
        }
    }

    pub fn current_redesign_settings(&self) -> RedesignSettings {
        RedesignSettings {
            user_name: self.user_name.clone(),
            theme_palette: self.selected_theme,
            language: self.language.clone(),
            validate_paths_on_startup: self.validate_paths_on_startup,
            diagnostic_mode: self.diagnostic_mode,
        }
    }

    pub fn mark_general_changed(&mut self) {
        self.general_changed_this_frame = true;
    }

    pub fn take_general_changed(&mut self) -> bool {
        let changed = self.general_changed_this_frame;
        self.general_changed_this_frame = false;
        changed
    }
}
