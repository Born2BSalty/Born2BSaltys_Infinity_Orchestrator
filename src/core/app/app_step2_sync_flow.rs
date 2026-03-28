// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ModState, Step3ItemState};

use super::WizardApp;

pub(super) fn sync_step2_from_step3(app: &mut WizardApp) {
    sync_tab_from_step3(&mut app.state.step2.bgee_mods, &mut app.state.step3.bgee_items);
    sync_tab_from_step3(&mut app.state.step2.bg2ee_mods, &mut app.state.step3.bg2ee_items);
    crate::ui::compat_logic::apply_step2_compat_rules(
        &app.state.step1,
        &mut app.state.step2.bgee_mods,
        &mut app.state.step2.bg2ee_mods,
    );
    super::step3_sync_flow::sync_step3_from_step2(app);
    app.last_step2_sync_signature = Some(step2_selection_signature(
        &app.state.step2.bgee_mods,
        &app.state.step2.bg2ee_mods,
    ));
}

fn sync_tab_from_step3(mods: &mut [Step2ModState], items: &mut [Step3ItemState]) {
    clear_checked_orders(mods);

    let mut next_order = 1usize;
    for item in items.iter_mut().filter(|item| !item.is_parent) {
        item.selected_order = next_order;
        assign_step2_order(mods, &item.tp_file, &item.component_id, next_order);
        next_order += 1;
    }

    assign_fallback_orders(mods, next_order);
}

fn clear_checked_orders(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in mod_state.components.iter_mut().filter(|component| component.checked) {
            component.selected_order = None;
        }
    }
}

fn assign_step2_order(mods: &mut [Step2ModState], tp_file: &str, component_id: &str, order: usize) {
    for mod_state in mods {
        if !mod_state.tp_file.trim().eq_ignore_ascii_case(tp_file.trim()) {
            continue;
        }
        for component in &mut mod_state.components {
            if component.component_id.trim().eq_ignore_ascii_case(component_id.trim())
                && component.checked
            {
                component.selected_order = Some(order);
                return;
            }
        }
    }
}

fn assign_fallback_orders(mods: &mut [Step2ModState], mut next_order: usize) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            if component.checked && component.selected_order.is_none() {
                component.selected_order = Some(next_order);
                next_order += 1;
            }
        }
    }
}

fn step2_selection_signature(bgee_mods: &[Step2ModState], bg2ee_mods: &[Step2ModState]) -> String {
    let mut entries: Vec<String> = Vec::new();
    collect_tab_signature("BGEE", bgee_mods, &mut entries);
    collect_tab_signature("BG2EE", bg2ee_mods, &mut entries);
    entries.sort_unstable();
    entries.join(";")
}

fn collect_tab_signature(tag: &str, mods: &[Step2ModState], out: &mut Vec<String>) {
    for mod_state in mods {
        let tp = mod_state.tp_file.to_ascii_uppercase();
        for component in mod_state.components.iter().filter(|component| component.checked) {
            out.push(format!(
                "{tag}|{tp}|{}|{}",
                component.component_id,
                component.selected_order.unwrap_or(usize::MAX)
            ));
        }
    }
}
