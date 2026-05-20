// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::state::{Step1State, Step2State, Step3State, Step5State, WizardState};
use super::state_validation;

impl WizardState {
    pub const STEP_COUNT: usize = 5;

    #[must_use]
    pub const fn can_go_back(&self) -> bool {
        self.current_step > 0
    }

    #[must_use]
    pub const fn can_go_next(&self) -> bool {
        self.current_step + 1 < Self::STEP_COUNT
    }

    #[must_use]
    pub fn is_step1_valid(&self) -> bool {
        state_validation::is_step1_valid(&self.step1)
    }

    pub fn run_step1_path_check(&mut self) {
        self.step1_path_check = Some(state_validation::run_path_check(&self.step1));
        self.step1_mods_folder_has_tp2 =
            Some(state_validation::step1_mods_folder_has_tp2(&self.step1));
    }

    pub const fn open_step1_clean_confirm(&mut self) {
        self.step1_clean_confirm_open = true;
    }

    pub const fn clear_step1_clean_confirm(&mut self) {
        self.step1_clean_confirm_open = false;
    }

    pub fn record_step4_save_error(&mut self, msg: String) {
        self.step5.last_status_text.clone_from(&msg);
        self.step4_save_error_text = msg;
        self.step4_save_error_open = true;
    }

    pub const fn dismiss_step4_save_error(&mut self) {
        self.step4_save_error_open = false;
    }

    pub fn set_last_step2_sync_signature(&mut self, signature: String) {
        self.last_step2_sync_signature = Some(signature);
    }

    pub fn clear_last_step2_sync_signature(&mut self) {
        self.last_step2_sync_signature = None;
    }

    pub const fn go_back(&mut self) {
        if self.can_go_back() {
            self.current_step -= 1;
        }
    }

    pub const fn go_next(&mut self) {
        if self.can_go_next() {
            self.current_step += 1;
        }
    }

    #[must_use]
    pub fn with_step1(step1: Step1State) -> Self {
        Self {
            current_step: 0,
            step1,
            step1_path_check: None,
            step1_mods_folder_has_tp2: None,
            github_auth_popup_open: false,
            github_auth_running: false,
            github_auth_login: String::new(),
            github_auth_user_code: String::new(),
            github_auth_verification_uri: String::new(),
            github_auth_status_text: String::new(),
            modlist_import_window_open: false,
            modlist_import_code: String::new(),
            modlist_import_preview: String::new(),
            modlist_import_error: String::new(),
            modlist_import_ready: false,
            modlist_import_preview_mode: false,
            modlist_import_preview_tab: "Summary".to_string(),
            modlist_import_preview_bgee_log: String::new(),
            modlist_import_preview_bg2ee_log: String::new(),
            modlist_import_preview_source_overrides: String::new(),
            modlist_import_preview_installed_refs: String::new(),
            modlist_import_preview_mod_configs: String::new(),
            modlist_auto_build_active: false,
            modlist_auto_build_waiting_for_install: false,
            last_step2_sync_signature: None,
            step1_clean_confirm_open: false,
            step4_save_error_open: false,
            step4_save_error_text: String::new(),
            step2: Step2State::default(),
            step3: Step3State::default(),
            step5: Step5State::default(),
        }
    }

    pub fn reset_workflow_keep_step1(&mut self) {
        self.current_step = 0;
        self.step1_path_check = None;
        self.step1_mods_folder_has_tp2 = None;
        self.github_auth_popup_open = false;
        self.github_auth_running = false;
        self.github_auth_login.clear();
        self.github_auth_user_code.clear();
        self.github_auth_verification_uri.clear();
        self.github_auth_status_text.clear();
        self.modlist_import_window_open = false;
        self.modlist_import_code.clear();
        self.modlist_import_preview.clear();
        self.modlist_import_error.clear();
        self.modlist_import_ready = false;
        self.modlist_import_preview_mode = false;
        self.modlist_import_preview_tab = "Summary".to_string();
        self.modlist_import_preview_bgee_log.clear();
        self.modlist_import_preview_bg2ee_log.clear();
        self.modlist_import_preview_source_overrides.clear();
        self.modlist_import_preview_installed_refs.clear();
        self.modlist_import_preview_mod_configs.clear();
        self.modlist_auto_build_active = false;
        self.modlist_auto_build_waiting_for_install = false;
        self.last_step2_sync_signature = None;
        self.step1_clean_confirm_open = false;
        self.step4_save_error_open = false;
        self.step4_save_error_text.clear();
        self.step2 = Step2State::default();
        self.step3 = Step3State::default();
        self.step5 = Step5State::default();
    }
}
