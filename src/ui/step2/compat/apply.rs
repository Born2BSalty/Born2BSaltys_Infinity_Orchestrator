// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step1State, Step2ModState};

use super::loader::load_rules;
use super::matcher::{match_rule, rule_disables_component};

pub fn apply_step2_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    let rules = load_rules();
    if rules.is_empty() {
        clear_all_disables(bgee_mods);
        clear_all_disables(bg2ee_mods);
        return;
    }
    apply_for_tab(step1, "BGEE", bgee_mods, &rules);
    apply_for_tab(step1, "BG2EE", bg2ee_mods, &rules);
}

fn clear_all_disables(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.disabled_reason = None;
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

fn apply_for_tab(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
    rules: &[super::model::Step2CompatRule],
) {
    for mod_state in mods {
        let mod_name = mod_state.name.clone();
        let tp_file = mod_state.tp_file.clone();
        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.disabled_reason = None;
            for rule in rules {
                if !match_rule(rule, step1, tab, &mod_name, &tp_file, component) {
                    continue;
                }
                component.compat_kind = Some(rule.kind.trim().to_ascii_lowercase());
                component.compat_source = Some(
                    rule.source
                        .clone()
                        .unwrap_or_else(|| "step2_compat_rules.toml".to_string()),
                );
                component.compat_related_mod = rule.related_mod.clone();
                component.compat_related_component = rule.related_component.clone();
                if rule_disables_component(rule) {
                    component.disabled = true;
                    component.checked = false;
                    component.selected_order = None;
                }
                if !rule.message.trim().is_empty() {
                    component.disabled_reason = Some(rule.message.clone());
                }
                break;
            }
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}
