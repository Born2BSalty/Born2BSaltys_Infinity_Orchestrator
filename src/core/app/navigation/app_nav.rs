// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::{self, GameSlot};
use crate::app::state::{Step2ModState, Step3ItemState, WizardState, exact_log_ready_to_install};

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
                .is_none_or(|existing| existing != signature);
        if should_sync {
            return NextAction::SyncStep3AndAdvance { signature };
        }
    }
    if state.current_step == 3 {
        return NextAction::NeedStep4SaveThenAdvance;
    }
    NextAction::Advance
}

fn step2_has_selection(state: &WizardState) -> bool {
    let selected_in = |mods: &[Step2ModState]| -> bool {
        mods.iter().any(|m| m.components.iter().any(|c| c.checked))
    };
    let game = state.step1.game_install.as_str();
    (game_authority::has_slot(game, GameSlot::First) && selected_in(&state.step2.bgee_mods))
        || (game_authority::has_slot(game, GameSlot::Second)
            && selected_in(&state.step2.bg2ee_mods))
}

fn step3_has_items(state: &WizardState) -> bool {
    let real_items_in = |items: &[Step3ItemState]| -> bool { items.iter().any(|i| !i.is_parent) };
    let game = state.step1.game_install.as_str();
    (game_authority::has_slot(game, GameSlot::First) && real_items_in(&state.step3.bgee_items))
        || (game_authority::has_slot(game, GameSlot::Second)
            && real_items_in(&state.step3.bg2ee_items))
}

fn step3_conflicts_resolved(state: &WizardState) -> bool {
    let game = state.step1.game_install.as_str();
    (!game_authority::has_slot(game, GameSlot::First) || !state.step3.bgee_has_conflict)
        && (!game_authority::has_slot(game, GameSlot::Second) || !state.step3.bg2ee_has_conflict)
}

pub(crate) fn step3_has_no_real_items(state: &WizardState) -> bool {
    let bgee_has = state.step3.bgee_items.iter().any(|i| !i.is_parent);
    let bg2_has = state.step3.bg2ee_items.iter().any(|i| !i.is_parent);
    !(bgee_has || bg2_has)
}

pub(crate) fn step2_selection_signature(state: &WizardState) -> String {
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
