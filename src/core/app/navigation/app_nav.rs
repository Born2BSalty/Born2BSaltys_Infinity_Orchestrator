// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::{Step2ModState, Step3ItemState, WizardState, exact_log_ready_to_install};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackAction {
    GoBack,
    ReturnToStep1FromLogs,
    SyncThenGoBack,
    SyncThenReturnToStep1FromLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NextAction {
    Blocked,
    OpenModlistImport,
    ApplySavedLogAndAdvance,
    JumpToInstallStep,
    SyncStep3AndAdvance { signature: String },
    NeedStep4SaveThenAdvance,
    Advance,
}

pub(crate) fn can_advance_from_current_step(state: &WizardState) -> bool {
    if !state.can_go_next() {
        return false;
    }
    match state.current_step {
        0 => state.is_step1_valid() && matches!(state.step1_path_check, Some((true, _))),
        1 => {
            if state.step1.installs_exactly_from_weidu_logs() {
                exact_log_ready_to_install(state)
            } else {
                !state.step2.is_scanning && step2_has_selection(state)
            }
        }
        2 => step3_has_items(state) && step3_conflicts_resolved(state),
        3 => {
            !state.step1.installs_exactly_from_weidu_logs()
                || state.step2.exact_log_mod_list_checked
        }
        _ => true,
    }
}

pub(crate) fn should_show_step1_clean_confirm(state: &WizardState) -> bool {
    let uses_fresh_target = if state.step1.game_install == "EET" {
        state.step1.new_pre_eet_dir_enabled || state.step1.new_eet_dir_enabled
    } else {
        state.step1.generate_directory_enabled
    };
    state.current_step == 0
        && !state.step1.imports_modlist()
        && uses_fresh_target
        && state.step1.prepare_target_dirs_before_install
        && !state.step1.backup_targets_before_eet_copy
}

pub(crate) fn decide_back_action(state: &WizardState) -> BackAction {
    let needs_sync = state.current_step == 2;
    let return_to_step1 = state.step1.installs_exactly_from_weidu_logs()
        && (state.current_step == 3 || state.current_step == 4);
    match (needs_sync, return_to_step1) {
        (true, true) => BackAction::SyncThenReturnToStep1FromLogs,
        (true, false) => BackAction::SyncThenGoBack,
        (false, true) => BackAction::ReturnToStep1FromLogs,
        (false, false) => BackAction::GoBack,
    }
}

pub(crate) fn apply_back_action(state: &mut WizardState, action: BackAction) {
    let prev_step = state.current_step;
    match action {
        BackAction::GoBack | BackAction::SyncThenGoBack => state.go_back(),
        BackAction::ReturnToStep1FromLogs | BackAction::SyncThenReturnToStep1FromLogs => {
            state.current_step = 0;
        }
    }
    if prev_step != 0 && state.current_step == 0 {
        state.step1_path_check = None;
    }
}

pub(crate) fn decide_next_action(state: &WizardState) -> NextAction {
    if !can_advance_from_current_step(state) {
        return NextAction::Blocked;
    }
    if state.current_step == 0 && state.step1.imports_modlist() {
        return NextAction::OpenModlistImport;
    }
    if state.current_step == 0 && state.step1.bootstraps_from_weidu_logs() {
        return NextAction::ApplySavedLogAndAdvance;
    }
    if state.current_step == 1 && state.step1.installs_exactly_from_weidu_logs() {
        return NextAction::JumpToInstallStep;
    }
    if state.current_step == 1 {
        let signature = step2_selection_signature(state);
        let should_sync = step3_has_no_real_items(state)
            || state
                .last_step2_sync_signature
                .as_deref()
                .map(|existing| existing != signature)
                .unwrap_or(true);
        if should_sync {
            return NextAction::SyncStep3AndAdvance { signature };
        }
    }
    if state.current_step == 3 {
        return NextAction::NeedStep4SaveThenAdvance;
    }
    NextAction::Advance
}

pub(crate) fn apply_next_action(state: &mut WizardState, action: &NextAction) {
    match action {
        NextAction::Blocked => {}
        NextAction::OpenModlistImport => {
            state.modlist_import_window_open = true;
            state.modlist_import_preview_mode = false;
            state.modlist_import_ready = false;
        }
        NextAction::JumpToInstallStep => state.current_step = 4,
        NextAction::ApplySavedLogAndAdvance
        | NextAction::SyncStep3AndAdvance { .. }
        | NextAction::NeedStep4SaveThenAdvance
        | NextAction::Advance => state.go_next(),
    }
}

pub(crate) fn current_step(state: &WizardState) -> usize {
    state.current_step
}

pub(crate) fn can_go_back(state: &WizardState) -> bool {
    state.can_go_back()
}

pub(crate) fn on_last_step(state: &WizardState) -> bool {
    state.current_step + 1 == WizardState::STEP_COUNT
}

pub(crate) fn step5_install_running(state: &WizardState) -> bool {
    state.current_step == 4 && (state.step5.prep_running || state.step5.install_running)
}

pub(crate) fn step1_clean_confirm_open(state: &WizardState) -> bool {
    state.step1_clean_confirm_open
}

pub(crate) fn step4_save_error_open(state: &WizardState) -> bool {
    state.step4_save_error_open
}

pub(crate) fn step4_save_error_text(state: &WizardState) -> &str {
    &state.step4_save_error_text
}

fn step2_has_selection(state: &WizardState) -> bool {
    let selected_in = |mods: &[Step2ModState]| -> bool {
        mods.iter().any(|m| m.components.iter().any(|c| c.checked))
    };
    match state.step1.game_install.as_str() {
        "BG2EE" => selected_in(&state.step2.bg2ee_mods),
        "EET" => selected_in(&state.step2.bgee_mods) || selected_in(&state.step2.bg2ee_mods),
        _ => selected_in(&state.step2.bgee_mods),
    }
}

fn step3_has_items(state: &WizardState) -> bool {
    let real_items_in = |items: &[Step3ItemState]| -> bool { items.iter().any(|i| !i.is_parent) };
    match state.step1.game_install.as_str() {
        "BG2EE" => real_items_in(&state.step3.bg2ee_items),
        "EET" => real_items_in(&state.step3.bgee_items) || real_items_in(&state.step3.bg2ee_items),
        _ => real_items_in(&state.step3.bgee_items),
    }
}

fn step3_conflicts_resolved(state: &WizardState) -> bool {
    match state.step1.game_install.as_str() {
        "BG2EE" => !state.step3.bg2ee_has_conflict,
        "EET" => !state.step3.bgee_has_conflict && !state.step3.bg2ee_has_conflict,
        _ => !state.step3.bgee_has_conflict,
    }
}

fn step3_has_no_real_items(state: &WizardState) -> bool {
    let bgee_has = state.step3.bgee_items.iter().any(|i| !i.is_parent);
    let bg2_has = state.step3.bg2ee_items.iter().any(|i| !i.is_parent);
    !(bgee_has || bg2_has)
}

fn step2_selection_signature(state: &WizardState) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut collect = |tag: &str, mods: &[Step2ModState]| {
        for m in mods {
            let tp = m.tp_file.to_ascii_uppercase();
            for c in &m.components {
                if c.checked {
                    entries.push(format!(
                        "{tag}|{tp}|{}|{}",
                        c.component_id,
                        c.selected_order.unwrap_or(usize::MAX)
                    ));
                }
            }
        }
    };
    collect("BGEE", &state.step2.bgee_mods);
    collect("BG2EE", &state.step2.bg2ee_mods);
    entries.sort_unstable();
    entries.join(";")
}
