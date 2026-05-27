// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{Step1State, Step2State, Step3State, Step5State};

#[derive(Debug, Clone, Default)]
pub struct WizardState<Flag = bool> {
    pub current_step: usize,
    pub step1: Step1State,
    pub step1_path_check: Option<(bool, String)>,
    pub step1_mods_folder_has_tp2: Option<bool>,
    pub github_auth_popup_open: Flag,
    pub github_auth_running: Flag,
    pub github_auth_login: String,
    pub github_auth_user_code: String,
    pub github_auth_verification_uri: String,
    pub github_auth_status_text: String,
    pub modlist_import_window_open: Flag,
    pub modlist_import_code: String,
    pub modlist_import_preview: String,
    pub modlist_import_error: String,
    pub modlist_import_ready: Flag,
    pub modlist_import_preview_mode: Flag,
    pub modlist_import_preview_tab: String,
    pub modlist_import_preview_bgee_log: String,
    pub modlist_import_preview_bg2ee_log: String,
    pub modlist_import_preview_source_overrides: String,
    pub modlist_import_preview_installed_refs: String,
    pub modlist_import_preview_mod_configs: String,
    pub(crate) modlist_share_name: Option<String>,
    pub(crate) modlist_share_author: Option<String>,
    pub(crate) modlist_share_forked_from: Vec<crate::app::modlist_share::ForkAncestor>,
    pub modlist_auto_build_active: Flag,
    pub modlist_auto_build_waiting_for_install: Flag,
    pub last_step2_sync_signature: Option<String>,
    pub step1_clean_confirm_open: Flag,
    pub step4_save_error_open: Flag,
    pub step4_save_error_text: String,
    pub step2: Step2State,
    pub step3: Step3State,
    pub step5: Step5State,
}
