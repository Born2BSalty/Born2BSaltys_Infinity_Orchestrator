// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{Step1State, Step2State, Step3State, Step5State};

#[derive(Debug, Clone, Default)]
pub struct WizardState {
    pub current_step: usize,
    pub step1: Step1State,
    pub step1_path_check: Option<(bool, String)>,
    pub step1_mods_folder_has_tp2: Option<bool>,
    pub github_auth_popup_open: bool,
    pub github_auth_running: bool,
    pub github_auth_login: String,
    pub github_auth_user_code: String,
    pub github_auth_verification_uri: String,
    pub github_auth_status_text: String,
    pub modlist_import_window_open: bool,
    pub modlist_import_code: String,
    pub modlist_import_preview: String,
    pub modlist_import_error: String,
    pub modlist_import_ready: bool,
    pub modlist_import_preview_mode: bool,
    pub modlist_import_preview_tab: String,
    pub modlist_import_preview_bgee_log: String,
    pub modlist_import_preview_bg2ee_log: String,
    pub modlist_import_preview_source_overrides: String,
    pub modlist_import_preview_installed_refs: String,
    pub modlist_import_preview_mod_configs: String,
    pub modlist_auto_build_active: bool,
    pub modlist_auto_build_waiting_for_install: bool,
    pub last_step2_sync_signature: Option<String>,
    pub step1_clean_confirm_open: bool,
    pub step4_save_error_open: bool,
    pub step4_save_error_text: String,
    pub step2: Step2State,
    pub step3: Step3State,
    pub step5: Step5State,
}
