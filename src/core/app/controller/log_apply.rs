// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};

use super::log_apply_keys::{
    find_mods_by_tp2_filename, find_unique_mod_by_tp2_stem, log_lookup_keys,
    mod_lookup_keys_for_mod, tp2_lookup_keys,
};
use super::log_apply_match::{
    installed_component_display_name, is_allowed_tp2, normalize_component_name,
    parse_component_tp2_from_raw, tp2_compatible, try_apply_eet_end_fallback,
};
use crate::app::mod_downloads;
use crate::app::state::{Step2ComponentState, Step2ModState};
use crate::mods::component::Component;
use crate::mods::log_file::LogFile;

pub fn apply_log_to_mods(
    mods: &mut [Step2ModState],
    log: &LogFile,
    tp2_allow: Option<&HashSet<String, RandomState>>,
    reset_before_apply: bool,
    next_order: &mut usize,
) -> usize {
    if reset_before_apply {
        reset_mod_selection(mods);
    }

    let mod_download_sources = mod_downloads::load_mod_download_sources();
    let mod_lookup = build_mod_lookup(mods, &mod_download_sources);
    let mut matched = 0usize;
    for installed in log.components() {
        if let Some(allow) = tp2_allow
            && !is_allowed_tp2(allow, installed)
        {
            continue;
        }

        matched += apply_installed_component(
            mods,
            installed,
            &mod_lookup,
            &mod_download_sources,
            next_order,
        );
    }

    update_mod_checked_state(mods);
    matched
}

fn reset_mod_selection(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            component.checked = false;
            component.selected_order = None;
        }
        mod_state.checked = false;
    }
}

fn build_mod_lookup(
    mods: &[Step2ModState],
    sources: &mod_downloads::ModDownloadsLoad,
) -> HashMap<String, Vec<usize>> {
    let mut mod_lookup: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, mod_state) in mods.iter().enumerate() {
        for key in mod_lookup_keys_for_mod_with_aliases(mod_state, sources) {
            mod_lookup.entry(key).or_default().push(idx);
        }
    }
    mod_lookup
}

fn apply_installed_component(
    mods: &mut [Step2ModState],
    installed: &Component,
    mod_lookup: &HashMap<String, Vec<usize>>,
    sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
) -> usize {
    let target_mods = target_mods_for_installed(mods, installed, mod_lookup);
    if target_mods.is_empty() {
        return usize::from(apply_eet_end_fallback(mods, installed, next_order));
    }

    let target_tp2_norm =
        normalize_path_key(format!("{}\\{}", installed.name, installed.tp_file).as_str());
    let target_name = normalize_component_name(&installed_component_display_name(installed));
    let mut matched = 0usize;
    for mod_idx in target_mods {
        matched += apply_installed_to_mod(
            &mut mods[mod_idx],
            installed,
            &target_tp2_norm,
            &target_name,
            sources,
            next_order,
        );
    }
    if matched == 0 && apply_eet_end_fallback(mods, installed, next_order) {
        matched += 1;
    }
    matched
}

fn target_mods_for_installed(
    mods: &[Step2ModState],
    installed: &Component,
    mod_lookup: &HashMap<String, Vec<usize>>,
) -> Vec<usize> {
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
    if target_mods.is_empty()
        && let Some(idx) = find_unique_mod_by_tp2_stem(mods, &installed.tp_file)
    {
        target_mods.push(idx);
    }
    target_mods
}

fn apply_installed_to_mod(
    mod_state: &mut Step2ModState,
    installed: &Component,
    target_tp2_norm: &str,
    target_name: &str,
    sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
) -> usize {
    let mod_tp_file = mod_state.tp_file.clone();
    if apply_matching_component(
        &mut mod_state.components,
        installed,
        target_tp2_norm,
        &mod_tp_file,
        sources,
        next_order,
        |component| component.component_id == installed.component,
    ) {
        return 1;
    }
    if target_name.is_empty() {
        return 0;
    }
    usize::from(apply_matching_component(
        &mut mod_state.components,
        installed,
        target_tp2_norm,
        &mod_tp_file,
        sources,
        next_order,
        |component| normalize_component_name(&component.label) == target_name,
    ))
}

fn apply_matching_component(
    components: &mut [Step2ComponentState],
    installed: &Component,
    target_tp2_norm: &str,
    mod_tp_file: &str,
    sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
    matches_component: impl Fn(&Step2ComponentState) -> bool,
) -> bool {
    for component in components {
        if !component_targets_log_tp2(component, target_tp2_norm, mod_tp_file, sources) {
            continue;
        }
        if matches_component(component) {
            check_component(component, next_order);
            apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            return true;
        }
    }
    false
}

fn component_targets_log_tp2(
    component: &Step2ComponentState,
    target_tp2_norm: &str,
    mod_tp_file: &str,
    sources: &mod_downloads::ModDownloadsLoad,
) -> bool {
    parse_component_tp2_from_raw(&component.raw_line).is_none_or(|child_tp2| {
        tp2_compatible_with_mod_aliases(&child_tp2, target_tp2_norm, mod_tp_file, sources)
    })
}

fn apply_eet_end_fallback(
    mods: &mut [Step2ModState],
    installed: &Component,
    next_order: &mut usize,
) -> bool {
    try_apply_eet_end_fallback(mods, installed, next_order, |component, next| {
        check_component(component, next);
        apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
    })
}

fn update_mod_checked_state(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        let checkable = mod_state.components.len();
        let checked = mod_state.components.iter().filter(|c| c.checked).count();
        mod_state.checked = checkable > 0 && checkable == checked;
    }
}

fn mod_lookup_keys_for_mod_with_aliases(
    mod_state: &Step2ModState,
    sources: &mod_downloads::ModDownloadsLoad,
) -> Vec<String> {
    let mut keys = mod_lookup_keys_for_mod(mod_state);
    for source in sources.find_sources(&mod_state.tp_file) {
        keys.extend(tp2_lookup_keys(&source.tp2));
        for alias in source.aliases {
            keys.extend(tp2_lookup_keys(&alias));
        }
    }
    let mut seen = HashSet::new();
    keys.into_iter()
        .filter(|key| !key.is_empty() && seen.insert(key.clone()))
        .collect()
}

fn tp2_compatible_with_mod_aliases(
    child_tp2: &str,
    target_tp2: &str,
    mod_tp_file: &str,
    sources: &mod_downloads::ModDownloadsLoad,
) -> bool {
    if tp2_compatible(child_tp2, target_tp2) {
        return true;
    }
    sources.find_sources(mod_tp_file).into_iter().any(|source| {
        tp2_compatible(child_tp2, &source.tp2)
            || source
                .aliases
                .iter()
                .any(|alias| tp2_compatible(child_tp2, alias))
    })
}

const fn check_component(component: &mut Step2ComponentState, next_order: &mut usize) {
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

fn apply_wlb_inputs(component: &mut Step2ComponentState, wlb_inputs: Option<&str>) {
    let Some(inputs) = wlb_inputs.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };
    let base = strip_wlb_marker(component.raw_line.as_str());
    component.raw_line = format!("{base} // @wlb-inputs: {inputs}");
}

fn strip_wlb_marker(raw_line: &str) -> String {
    let marker = "@wlb-inputs:";
    let lower = raw_line.to_ascii_lowercase();
    lower.find(marker).map_or_else(
        || raw_line.trim().to_string(),
        |start| {
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
        },
    )
}

pub use super::log_apply_keys::normalize_path_key;
