// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::mods::component::Component;
use crate::ui::controller::log_apply_keys::{
    log_lookup_keys, mod_lookup_keys_for_mod, normalize_path_key, tp2_lookup_keys,
};
use crate::ui::state::{Step2ComponentState, Step2ModState};

pub fn normalize_component_name(value: &str) -> String {
    let mut s = value.replace('\u{2013}', "-").replace('\u{2014}', "-");
    if let Some((head, tail)) = s.rsplit_once(':') {
        let t = tail.trim();
        let looks_like_version =
            t.starts_with('v') || t.starts_with('V') || t.chars().next().is_some_and(|c| c.is_ascii_digit());
        if looks_like_version {
            s = head.to_string();
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

pub fn parse_component_tp2_from_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('~') {
        return None;
    }
    let start = trimmed.find('~')?;
    let rest = &trimmed[start + 1..];
    let end = rest.find('~')?;
    Some(normalize_path_key(&rest[..end]))
}

pub fn installed_component_display_name(installed: &Component) -> String {
    if installed.sub_component.trim().is_empty() {
        installed.component_name.clone()
    } else {
        format!(
            "{} -> {}",
            installed.component_name.trim(),
            installed.sub_component.trim()
        )
    }
}

pub fn tp2_compatible(child_tp2_norm: &str, target_tp2_norm: &str) -> bool {
    let child_keys: HashSet<String> = tp2_lookup_keys(child_tp2_norm).into_iter().collect();
    let target_keys: HashSet<String> = tp2_lookup_keys(target_tp2_norm).into_iter().collect();
    !child_keys.is_disjoint(&target_keys)
}

pub fn is_eet_end_line(installed: &Component) -> bool {
    let tp2 = normalize_path_key(format!("{}\\{}", installed.name, installed.tp_file).as_str());
    if tp2 != normalize_path_key(r"EET_END\EET_END.TP2") {
        return false;
    }
    if installed.component != "0" {
        return false;
    }
    let target_name = normalize_component_name(installed_component_display_name(installed).as_str());
    let expected_name =
        normalize_component_name("EET end (last mod in install order) -> Standard installation");
    target_name == expected_name
}

pub fn try_apply_eet_end_fallback(
    mods: &mut [Step2ModState],
    installed: &Component,
    next_order: &mut usize,
    check_component: impl Fn(&mut Step2ComponentState, &mut usize),
) -> bool {
    if !is_eet_end_line(installed) {
        return false;
    }
    let expected_name =
        normalize_component_name("EET end (last mod in install order) -> Standard installation");

    let eet_keys: HashSet<String> = tp2_lookup_keys(r"EET\EET.TP2").into_iter().collect();
    let eet_end_keys: HashSet<String> = tp2_lookup_keys(r"EET_END\EET_END.TP2").into_iter().collect();
    let mut name_only_candidate: Option<(usize, usize)> = None;
    for (mod_idx, mod_state) in mods.iter_mut().enumerate() {
        let mod_keys: HashSet<String> = mod_lookup_keys_for_mod(mod_state).into_iter().collect();
        if eet_keys.is_disjoint(&mod_keys) && eet_end_keys.is_disjoint(&mod_keys) {
            continue;
        }
        for (idx, component) in mod_state.components.iter_mut().enumerate() {
            if normalize_component_name(component.label.as_str()) == expected_name {
                if component.component_id == "0" {
                    check_component(component, next_order);
                    return true;
                }
                if name_only_candidate.is_none() {
                    name_only_candidate = Some((mod_idx, idx));
                }
            }
        }
    }
    if let Some((mod_idx, comp_idx)) = name_only_candidate
        && let Some(mod_state) = mods.get_mut(mod_idx)
        && let Some(component) = mod_state.components.get_mut(comp_idx)
    {
        check_component(component, next_order);
        return true;
    }
    false
}

pub fn is_allowed_tp2(allow: &HashSet<String>, installed: &Component) -> bool {
    let keys = log_lookup_keys(installed.name.as_str(), installed.tp_file.as_str());
    keys.iter().any(|k| allow.contains(k))
}
