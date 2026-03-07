// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{CompatState, Step1State, Step2State, Step3State, Step5State};

#[derive(Debug, Clone)]
pub struct WizardState {
    pub current_step: usize,
    pub step1: Step1State,
    pub step1_path_check: Option<(bool, String)>,
    pub step1_clean_confirm_open: bool,
    pub step4_save_error_open: bool,
    pub step4_save_error_text: String,
    pub step2: Step2State,
    pub step3: Step3State,
    pub step5: Step5State,
    pub compat: CompatState,
}
