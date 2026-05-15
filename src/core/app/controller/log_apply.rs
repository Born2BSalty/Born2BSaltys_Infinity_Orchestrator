// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

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

pub fn apply_log_to_mods<S: BuildHasher>(
    mods: &mut [Step2ModState],
    log: &LogFile,
    tp2_allow: Option<&HashSet<String, S>>,
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

        let target_mods = target_mod_indices(mods, &mod_lookup, installed);
        if target_mods.is_empty() {
            if try_apply_eet_end_fallback(mods, installed, next_order, |component, next| {
                check_component(component, next);
                apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            }) {
                matched += 1;
            }
            continue;
        }

        let target_tp2_norm =
            normalize_path_key(format!("{}\\{}", installed.name, installed.tp_file).as_str());
        let target_name =
            normalize_component_name(installed_component_display_name(installed).as_str());
        let mut matched_this_line = false;
        for mod_idx in target_mods {
            if apply_installed_to_mod(
                &mut mods[mod_idx],
                installed,
                &target_tp2_norm,
                &target_name,
                &mod_download_sources,
                next_order,
            ) {
                matched += 1;
                matched_this_line = true;
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
    mod_download_sources: &mod_downloads::ModDownloadsLoad,
) -> HashMap<String, Vec<usize>> {
    let mut mod_lookup: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, mod_state) in mods.iter().enumerate() {
        for key in mod_lookup_keys_for_mod_with_aliases(mod_state, mod_download_sources) {
            mod_lookup.entry(key).or_default().push(idx);
        }
    }
    mod_lookup
}

fn target_mod_indices(
    mods: &[Step2ModState],
    mod_lookup: &HashMap<String, Vec<usize>>,
    installed: &Component,
) -> Vec<usize> {
    let keys = log_lookup_keys(installed.name.as_str(), installed.tp_file.as_str());
    let mut target_mods = lookup_target_mods(mod_lookup, &keys);
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

fn lookup_target_mods(mod_lookup: &HashMap<String, Vec<usize>>, keys: &[String]) -> Vec<usize> {
    let mut target_mods: Vec<usize> = Vec::new();
    let mut seen = HashSet::new();
    for key in keys {
        if let Some(list) = mod_lookup.get(key) {
            for idx in list {
                if seen.insert(*idx) {
                    target_mods.push(*idx);
                }
            }
        }
    }
    target_mods
}

fn apply_installed_to_mod(
    mod_state: &mut Step2ModState,
    installed: &Component,
    target_tp2_norm: &str,
    target_name: &str,
    mod_download_sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
) -> bool {
    let mod_tp_file = mod_state.tp_file.clone();
    if apply_by_component_id(
        &mut mod_state.components,
        installed,
        target_tp2_norm,
        &mod_tp_file,
        mod_download_sources,
        next_order,
    ) {
        return true;
    }
    !target_name.is_empty()
        && apply_by_component_name(
            &mut mod_state.components,
            installed,
            target_tp2_norm,
            target_name,
            &mod_tp_file,
            mod_download_sources,
            next_order,
        )
}

fn apply_by_component_id(
    components: &mut [Step2ComponentState],
    installed: &Component,
    target_tp2_norm: &str,
    mod_tp_file: &str,
    mod_download_sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
) -> bool {
    for component in components {
        if !component_tp2_matches(
            component,
            target_tp2_norm,
            mod_tp_file,
            mod_download_sources,
        ) {
            continue;
        }
        if component.component_id == installed.component {
            check_component(component, next_order);
            apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            return true;
        }
    }
    false
}

fn apply_by_component_name(
    components: &mut [Step2ComponentState],
    installed: &Component,
    target_tp2_norm: &str,
    target_name: &str,
    mod_tp_file: &str,
    mod_download_sources: &mod_downloads::ModDownloadsLoad,
    next_order: &mut usize,
) -> bool {
    for component in components {
        if !component_tp2_matches(
            component,
            target_tp2_norm,
            mod_tp_file,
            mod_download_sources,
        ) {
            continue;
        }
        if normalize_component_name(component.label.as_str()) == target_name {
            check_component(component, next_order);
            apply_wlb_inputs(component, installed.wlb_inputs.as_deref());
            return true;
        }
    }
    false
}

fn component_tp2_matches(
    component: &Step2ComponentState,
    target_tp2_norm: &str,
    mod_tp_file: &str,
    mod_download_sources: &mod_downloads::ModDownloadsLoad,
) -> bool {
    parse_component_tp2_from_raw(component.raw_line.as_str()).is_none_or(|child_tp2| {
        tp2_compatible_with_mod_aliases(
            child_tp2.as_str(),
            target_tp2_norm,
            mod_tp_file,
            mod_download_sources,
        )
    })
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
