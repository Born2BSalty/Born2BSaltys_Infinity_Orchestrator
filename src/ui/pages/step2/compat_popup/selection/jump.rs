// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{Step2Selection, WizardState};

use super::key::normalize_mod_key;

pub(crate) fn jump_to_target(
    state: &mut WizardState,
    game_tab: &str,
    mod_ref: &str,
    component_ref: Option<u32>,
) {
    let target_key = normalize_mod_key(mod_ref);
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    for mod_state in mods {
        if let Some(target_component) = component_ref {
            if let Some(component) = mod_state.components.iter().find(|c| {
                let c_key = parse_component_tp2_from_raw(&c.raw_line)
                    .map(|tp2| normalize_mod_key(&tp2))
                    .unwrap_or_else(|| normalize_mod_key(&mod_state.tp_file));
                c.component_id.trim().parse::<u32>().ok() == Some(target_component)
                    && c_key == target_key
            }) {
                state.step2.selected = Some(Step2Selection::Component {
                    game_tab: game_tab.to_string(),
                    tp_file: mod_state.tp_file.clone(),
                    component_id: component.component_id.clone(),
                    component_key: component.raw_line.clone(),
                });
                return;
            }
        } else if let Some(component) = mod_state.components.iter().find(|c| {
            parse_component_tp2_from_raw(&c.raw_line)
                .map(|tp2| normalize_mod_key(&tp2))
                .unwrap_or_else(|| normalize_mod_key(&mod_state.tp_file))
                == target_key
        }) {
            state.step2.selected = Some(Step2Selection::Component {
                game_tab: game_tab.to_string(),
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
            });
            return;
        }
        if normalize_mod_key(&mod_state.tp_file) != target_key {
            continue;
        }
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: game_tab.to_string(),
            tp_file: mod_state.tp_file.clone(),
        });
        return;
    }
}
