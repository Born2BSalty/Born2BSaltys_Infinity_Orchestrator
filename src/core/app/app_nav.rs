// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::WizardApp;

pub(super) fn can_advance_from_current_step(app: &WizardApp) -> bool {
    if !app.state.can_go_next() {
        return false;
    }
    match app.state.current_step {
        0 => {
            app.state.is_step1_valid()
                && matches!(app.state.step1_path_check, Some((true, _)))
        }
        1 => !app.state.step2.is_scanning && step2_has_selection(app),
        2 => step3_has_items(app) && step3_conflicts_resolved(app),
        _ => true,
    }
}

fn step2_has_selection(app: &WizardApp) -> bool {
    let selected_in = |mods: &[crate::ui::state::Step2ModState]| -> bool {
        mods.iter().any(|m| m.components.iter().any(|c| c.checked))
    };
    match app.state.step1.game_install.as_str() {
        "BG2EE" => selected_in(&app.state.step2.bg2ee_mods),
        "EET" => selected_in(&app.state.step2.bgee_mods) || selected_in(&app.state.step2.bg2ee_mods),
        _ => selected_in(&app.state.step2.bgee_mods),
    }
}

fn step3_has_items(app: &WizardApp) -> bool {
    let real_items_in = |items: &[crate::ui::state::Step3ItemState]| -> bool { items.iter().any(|i| !i.is_parent) };
    match app.state.step1.game_install.as_str() {
        "BG2EE" => real_items_in(&app.state.step3.bg2ee_items),
        "EET" => real_items_in(&app.state.step3.bgee_items) || real_items_in(&app.state.step3.bg2ee_items),
        _ => real_items_in(&app.state.step3.bgee_items),
    }
}

fn step3_conflicts_resolved(app: &WizardApp) -> bool {
    match app.state.step1.game_install.as_str() {
        "BG2EE" => !app.state.step3.bg2ee_has_conflict,
        "EET" => !app.state.step3.bgee_has_conflict && !app.state.step3.bg2ee_has_conflict,
        _ => !app.state.step3.bgee_has_conflict,
    }
}
