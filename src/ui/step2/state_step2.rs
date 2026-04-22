// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step2ModState, Step2State, WizardState};

#[derive(Debug, Clone, Default)]
pub struct Step2Details {
    pub mod_name: Option<String>,
    pub component_label: Option<String>,
    pub component_id: Option<String>,
    pub shown_component_count: Option<usize>,
    pub hidden_component_count: Option<usize>,
    pub raw_component_count: Option<usize>,
    pub component_lang: Option<String>,
    pub component_version: Option<String>,
    pub selected_order: Option<usize>,
    pub is_checked: Option<bool>,
    pub is_disabled: Option<bool>,
    pub compat_kind: Option<String>,
    pub compat_role: Option<String>,
    pub compat_code: Option<String>,
    pub disabled_reason: Option<String>,
    pub compat_source: Option<String>,
    pub compat_related_target: Option<String>,
    pub compat_graph: Option<String>,
    pub compat_evidence: Option<String>,
    pub compat_component_block: Option<String>,
    pub raw_line: Option<String>,
    pub tp_file: Option<String>,
    pub tp2_folder: Option<String>,
    pub tp2_path: Option<String>,
    pub readme_path: Option<String>,
    pub web_url: Option<String>,
    pub package_source_status: Option<String>,
    pub package_source_name: Option<String>,
    pub package_latest_version: Option<String>,
    pub package_source_url: Option<String>,
    pub package_source_github: Option<String>,
    pub package_update_locked: Option<bool>,
    pub package_can_check_updates: bool,
}

pub fn normalize_active_tab(state: &mut WizardState) {
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let active_is_visible = (state.step2.active_game_tab == "BGEE" && show_bgee)
        || (state.step2.active_game_tab == "BG2EE" && show_bg2ee);
    if active_is_visible {
        return;
    }
    if show_bgee {
        state.step2.active_game_tab = "BGEE".to_string();
    } else if show_bg2ee {
        state.step2.active_game_tab = "BG2EE".to_string();
    }
}

pub fn active_mods_mut(step2: &mut Step2State) -> &mut Vec<Step2ModState> {
    if step2.active_game_tab == "BGEE" {
        &mut step2.bgee_mods
    } else {
        &mut step2.bg2ee_mods
    }
}

pub fn review_edit_waiting_for_first_scan(state: &WizardState) -> bool {
    state.step1.bootstraps_from_weidu_logs() && state.step2.last_scan_report.is_none()
}

pub fn review_edit_scan_complete(state: &WizardState) -> bool {
    state.step1.bootstraps_from_weidu_logs() && state.step2.last_scan_report.is_some()
}

pub fn review_edit_any_log_applied(state: &WizardState) -> bool {
    state.step2.review_edit_bgee_log_applied || state.step2.review_edit_bg2ee_log_applied
}

pub fn non_scan_controls_locked(state: &WizardState) -> bool {
    state.step2.is_scanning || review_edit_waiting_for_first_scan(state)
}

#[cfg(test)]
#[path = "state_step2_tests.rs"]
mod state_step2_tests;
