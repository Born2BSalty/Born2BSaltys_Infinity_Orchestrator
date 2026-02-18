// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
