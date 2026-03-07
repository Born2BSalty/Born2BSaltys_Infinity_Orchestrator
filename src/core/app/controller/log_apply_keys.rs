// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod lookup {
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
}
mod matchers {
use std::path::Path;

use crate::ui::state::Step2ModState;

use super::normalize::{normalize_path_key, strip_setup_prefix};

pub fn extract_tp2_filename(tp_file: &str) -> String {
    let norm = normalize_path_key(tp_file);
    Path::new(&norm)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&norm)
        .to_ascii_uppercase()
}

pub fn extract_tp2_stem(tp_file: &str) -> String {
    let filename = extract_tp2_filename(tp_file);
    let without_ext = Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename)
        .to_ascii_uppercase();
    strip_setup_prefix(&without_ext)
}

pub fn find_mods_by_tp2_filename(mods: &[Step2ModState], tp_file: &str) -> Vec<usize> {
    let target_filename = extract_tp2_filename(tp_file);
    let mut results = Vec::new();
    for (idx, mod_state) in mods.iter().enumerate() {
        let mod_filename = extract_tp2_filename(&mod_state.tp_file);
        if mod_filename == target_filename {
            results.push(idx);
        }
    }
    results
}

pub fn find_unique_mod_by_tp2_stem(mods: &[Step2ModState], tp_file: &str) -> Option<usize> {
    let target_stem = extract_tp2_stem(tp_file);
    if target_stem.is_empty() {
        return None;
    }
    let mut candidates = Vec::new();
    for (idx, mod_state) in mods.iter().enumerate() {
        let mod_stem = extract_tp2_stem(&mod_state.tp_file);
        if mod_stem == target_stem {
            candidates.push(idx);
        }
    }
    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}
}
mod normalize {
pub(super) fn maybe_last_two_parts(path_like: &str) -> Option<String> {
    let norm = normalize_path_key(path_like);
    let parts: Vec<&str> = norm.split('\\').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 {
        Some(format!("{}\\{}", parts[parts.len() - 2], parts[parts.len() - 1]))
    } else {
        None
    }
}

pub(super) fn strip_setup_prefix(value: &str) -> String {
    let upper = value.to_ascii_uppercase();
    upper
        .strip_prefix("SETUP-")
        .or_else(|| upper.strip_prefix("SETUP_"))
        .unwrap_or(upper.as_str())
        .to_string()
}

pub(super) fn ensure_setup_prefix(value: &str) -> String {
    let upper = value.to_ascii_uppercase();
    if upper.starts_with("SETUP-") || upper.starts_with("SETUP_") {
        upper
    } else {
        format!("SETUP-{upper}")
    }
}

pub fn normalize_path_key(value: &str) -> String {
    let mut normalized = value.trim().trim_matches('"').replace('/', "\\");
    if normalized.starts_with(".\\") {
        normalized = normalized[2..].to_string();
    }
    normalized.to_ascii_uppercase()
}
}

pub use lookup::{log_lookup_keys, mod_lookup_keys_for_mod, tp2_lookup_keys};
pub use matchers::{
    find_mods_by_tp2_filename, find_unique_mod_by_tp2_stem,
};
pub use normalize::normalize_path_key;
