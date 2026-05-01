// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::app::step4_action::Step4Action;
use crate::app::step4_weidu_log_export::auto_save_step4_weidu_logs;

pub(crate) fn handle_step4_action(state: &mut WizardState, action: Step4Action) {
    match action {
        Step4Action::SaveWeiduLog => {
            if let Err(err) = auto_save_step4_weidu_logs(state) {
                state.step5.last_status_text = err.clone();
            }
        }
        Step4Action::CheckMissingMods => {
            let active_game_tab = active_step4_game_tab(state).to_string();
            super::app_step2_saved_log_flow::queue_exact_log_update_preview(
                state,
                &active_game_tab,
                false,
            );
        }
    }
}

fn active_step4_game_tab(state: &WizardState) -> &str {
    match state.step1.game_install.as_str() {
        "BG2EE" => "BG2EE",
        "EET" if state.step3.active_game_tab == "BG2EE" => "BG2EE",
        _ => "BGEE",
    }
}
