// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use crate::mods::log_file::LogFile;
use crate::ui::controller::log_apply_keys::{
    find_mods_by_tp2_filename, find_unique_mod_by_tp2_stem, log_lookup_keys, mod_lookup_keys_for_mod,
};
use crate::ui::controller::log_apply_match::{
    installed_component_display_name, is_allowed_tp2, normalize_component_name,
    parse_component_tp2_from_raw, tp2_compatible, try_apply_eet_end_fallback,
};

pub fn apply_log_to_mods(
    mods: &mut [crate::ui::state::Step2ModState],
    log: &LogFile,
    tp2_allow: Option<&HashSet<String>>,
    reset_before_apply: bool,
    next_order: &mut usize,
) -> usize {
    if reset_before_apply {
        for mod_state in mods.iter_mut() {
            for component in &mut mod_state.components {
                component.checked = false;
                component.selected_order = None;
            }
            mod_state.checked = false;
        }
    }

    let mut mod_lookup: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, mod_state) in mods.iter().enumerate() {
        for key in mod_lookup_keys_for_mod(mod_state) {
            mod_lookup.entry(key).or_default().push(idx);
        }
    }

    let mut matched = 0usize;
    for installed in log.components() {
        if let Some(allow) = tp2_allow
            && !is_allowed_tp2(allow, installed)
        {
            continue;
        }

        let target_tp2_norm =
            normalize_path_key(format!("{}\\{}", installed.name, installed.tp_file).as_str());
        let keys = log_lookup_keys(installed.name.as_str(), installed.tp_file.as_str());
        let mut target_mods: Vec<usize> = Vec::new();
        let mut seen = HashSet::new();
        for key in &keys {
            if let Some(list) = mod_lookup.get(key) {
                for idx in list {
                    if seen.insert(*idx) {
                        target_mods.push(*idx);
                    }
                }
            }
        }

        if target_mods.is_empty() {
            target_mods = find_mods_by_tp2_filename(mods, &installed.tp_file);
        }

        if target_mods.is_empty() {
            if let Some(idx) = find_unique_mod_by_tp2_stem(mods, &installed.tp_file) {
                target_mods.push(idx);
            }
        }

        if target_mods.is_empty() {
            if try_apply_eet_end_fallback(mods, installed, next_order, |component, next| {
                check_component(component, next);
                apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            }) {
                matched += 1;
            }
            continue;
        }

        let target_name = normalize_component_name(installed_component_display_name(installed).as_str());
        let mut matched_this_line = false;
        for mod_idx in target_mods {
            let mod_state = &mut mods[mod_idx];
            let mut picked = false;

            for component in &mut mod_state.components {
                if let Some(child_tp2) = parse_component_tp2_from_raw(component.raw_line.as_str())
                    && !tp2_compatible(child_tp2.as_str(), target_tp2_norm.as_str())
                {
                    continue;
                }
                if component.component_id == installed.component {
                    check_component(component, next_order);
                    apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
                    matched += 1;
                    matched_this_line = true;
                    picked = true;
                    break;
                }
            }

            if !picked && !target_name.is_empty() {
                for component in &mut mod_state.components {
                    if let Some(child_tp2) = parse_component_tp2_from_raw(component.raw_line.as_str())
                        && !tp2_compatible(child_tp2.as_str(), target_tp2_norm.as_str())
                    {
                        continue;
                    }
                    if normalize_component_name(component.label.as_str()) == target_name {
                        check_component(component, next_order);
                        apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
                        matched += 1;
                        matched_this_line = true;
                        break;
                    }
                }
            }
        }
        if !matched_this_line
            && try_apply_eet_end_fallback(mods, installed, next_order, |component, next| {
                check_component(component, next);
                apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            })
        {
            matched += 1;
        }
    }

    for mod_state in mods.iter_mut() {
        let checkable = mod_state.components.len();
        let checked = mod_state.components.iter().filter(|c| c.checked).count();
        mod_state.checked = checkable > 0 && checkable == checked;
    }

    matched
}

fn check_component(component: &mut crate::ui::state::Step2ComponentState, next_order: &mut usize) {
    if component.disabled {
        component.checked = false;
        component.selected_order = None;
        return;
    }
    component.checked = true;
    if component.selected_order.is_none() {
        component.selected_order = Some(*next_order);
        *next_order += 1;
    }
}

fn apply_wlb_inputs(component: &mut crate::ui::state::Step2ComponentState, wlb_inputs: Option<&str>) {
    let Some(inputs) = wlb_inputs.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };
    let base = strip_wlb_marker(component.raw_line.as_str());
    component.raw_line = format!("{base} // @wlb-inputs: {inputs}");
}

fn strip_wlb_marker(raw_line: &str) -> String {
    let marker = "@wlb-inputs:";
    let lower = raw_line.to_ascii_lowercase();
    if let Some(start) = lower.find(marker) {
        let mut head = raw_line[..start].to_string();
        while head.ends_with(' ') || head.ends_with('\t') {
            head.pop();
        }
        if head.ends_with("//") {
            head.truncate(head.len().saturating_sub(2));
            while head.ends_with(' ') || head.ends_with('\t') {
                head.pop();
            }
        }
        head
    } else {
        raw_line.trim().to_string()
    }
}

pub use crate::ui::controller::log_apply_keys::normalize_path_key;
