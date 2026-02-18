// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::Path;

use crate::ui::state::Step2ModState;

use super::normalize::{ensure_setup_prefix, maybe_last_two_parts, normalize_path_key, strip_setup_prefix};

pub fn tp2_lookup_keys(tp2: &str) -> Vec<String> {
    let norm = normalize_path_key(tp2);
    let mut keys = Vec::<String>::new();
    keys.push(norm.clone());

    let base = Path::new(&norm)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(norm.as_str())
        .to_string();
    keys.push(base.clone());

    if let Some(short) = maybe_last_two_parts(&norm) {
        keys.push(short.clone());
    }

    let base_no_setup = strip_setup_prefix(&base);
    if !base_no_setup.is_empty() {
        keys.push(base_no_setup.clone());
    }

    let stem_no_setup = Path::new(&base_no_setup)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if !stem_no_setup.is_empty() {
        keys.push(stem_no_setup);
    }

    let setup_base = ensure_setup_prefix(&base_no_setup);
    if !setup_base.is_empty() {
        keys.push(setup_base.clone());
    }

    if let Some(short) = maybe_last_two_parts(&norm) {
        let short_parts: Vec<&str> = short.split('\\').filter(|p| !p.is_empty()).collect();
        if let Some(folder) = short_parts.first() {
            keys.push(format!("{folder}\\{base_no_setup}"));
            keys.push(format!("{folder}\\{setup_base}"));
        }
    }

    dedupe(keys)
}

pub fn mod_lookup_keys_for_mod(mod_state: &Step2ModState) -> Vec<String> {
    let mut keys = tp2_lookup_keys(mod_state.tp_file.as_str());
    if !mod_state.tp2_path.trim().is_empty() {
        keys.extend(tp2_lookup_keys(mod_state.tp2_path.as_str()));
    }
    dedupe(keys)
}

pub fn log_lookup_keys(mod_name: &str, tp_file: &str) -> Vec<String> {
    let mut keys = tp2_lookup_keys(tp_file);
    if !mod_name.trim().is_empty() && !tp_file.trim().is_empty() {
        keys.extend(tp2_lookup_keys(format!("{mod_name}\\{tp_file}").as_str()));
    }
    dedupe(keys)
}

fn dedupe(keys: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for key in keys {
        if !key.is_empty() && seen.insert(key.clone()) {
            deduped.push(key);
        }
    }
    deduped
}
