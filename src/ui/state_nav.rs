// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatState, Step1State, Step2State, Step3State, Step5State, WizardState};
use crate::ui::state_validation;

impl WizardState {
    pub const STEP_COUNT: usize = 5;

    pub fn can_go_back(&self) -> bool {
        self.current_step > 0
    }

    pub fn can_go_next(&self) -> bool {
        self.current_step + 1 < Self::STEP_COUNT
    }

    pub fn is_step1_valid(&self) -> bool {
        state_validation::is_step1_valid(&self.step1)
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.current_step -= 1;
        }
    }

    pub fn go_next(&mut self) {
        if self.can_go_next() {
            self.current_step += 1;
        }
    }

    pub fn with_step1(step1: Step1State) -> Self {
        Self {
            current_step: 0,
            step1,
            step1_path_check: None,
            step1_clean_confirm_open: false,
            step2: Step2State::default(),
            step3: Step3State::default(),
            step5: Step5State::default(),
            compat: CompatState::default(),
        }
    }

    pub fn reset_workflow_keep_step1(&mut self) {
        self.current_step = 0;
        self.step1_path_check = None;
        self.step2 = Step2State::default();
        self.step3 = Step3State::default();
        self.step5 = Step5State::default();
        self.compat = CompatState::default();
    }
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            current_step: 0,
            step1: Step1State::default(),
            step1_path_check: None,
            step1_clean_confirm_open: false,
            step2: Step2State::default(),
            step3: Step3State::default(),
            step5: Step5State::default(),
            compat: CompatState::default(),
        }
    }
}
