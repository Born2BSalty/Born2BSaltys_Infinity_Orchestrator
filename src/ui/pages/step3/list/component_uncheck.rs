// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ModState, Step3ItemState, WizardState};

pub(super) fn apply_component_unchecks(state: &mut WizardState, tab_id: &str, requests: &[(String, String)]) {
    if tab_id.eq_ignore_ascii_case("BGEE") {
        for (tp_file, component_id) in requests {
            uncheck_component_in_step2(&mut state.step2.bgee_mods, tp_file, component_id);
            remove_component_from_step3_items(
                &mut state.step3.bgee_items,
                &mut state.step3.bgee_selected,
                tp_file,
                component_id,
            );
        }
    } else {
        for (tp_file, component_id) in requests {
            uncheck_component_in_step2(&mut state.step2.bg2ee_mods, tp_file, component_id);
            remove_component_from_step3_items(
                &mut state.step3.bg2ee_items,
                &mut state.step3.bg2ee_selected,
                tp_file,
                component_id,
            );
        }
    }
}

fn uncheck_component_in_step2(mods: &mut [Step2ModState], tp_file: &str, component_id: &str) {
    let target_tp = tp_file.trim();
    let target_comp = component_id.trim();
    for mod_state in mods {
        if mod_state.tp_file.trim().eq_ignore_ascii_case(target_tp) {
            for component in &mut mod_state.components {
                if component.component_id.trim() == target_comp {
                    component.checked = false;
                    component.selected_order = None;
                }
            }
            mod_state.checked = mod_state.components.iter().any(|c| c.checked);
            return;
        }
    }
}

fn remove_component_from_step3_items(
    items: &mut Vec<Step3ItemState>,
    selected: &mut Vec<usize>,
    tp_file: &str,
    component_id: &str,
) {
    let target_tp = tp_file.trim();
    let target_comp = component_id.trim();
    let Some(component_idx) = items.iter().position(|it| {
        !it.is_parent
            && it.tp_file.trim().eq_ignore_ascii_case(target_tp)
            && it.component_id.trim() == target_comp
    }) else {
        return;
    };
    let block_id = items[component_idx].block_id.clone();
    remove_row_at(items, selected, component_idx);

    let block_has_children = items.iter().any(|it| !it.is_parent && it.block_id == block_id);
    if !block_has_children
        && let Some(parent_idx) = items
            .iter()
            .position(|it| it.is_parent && it.block_id == block_id)
    {
        remove_row_at(items, selected, parent_idx);
    }
}

fn remove_row_at(items: &mut Vec<Step3ItemState>, selected: &mut Vec<usize>, idx: usize) {
    let _ = items.remove(idx);
    selected.retain(|v| *v != idx);
    for s in selected.iter_mut() {
        if *s > idx {
            *s -= 1;
        }
    }
}
