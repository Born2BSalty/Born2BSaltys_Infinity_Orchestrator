// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::ui::state::{Step1State, Step2ModState};

pub fn resolve_scan_game_dir(step1: &Step1State) -> Option<PathBuf> {
    let mut candidates: Vec<&str> = Vec::new();
    match step1.game_install.as_str() {
        "BG2EE" => {
            candidates.push(step1.bg2ee_game_folder.trim());
            candidates.push(step1.eet_bg2ee_game_folder.trim());
            candidates.push(step1.bgee_game_folder.trim());
            candidates.push(step1.eet_bgee_game_folder.trim());
        }
        "EET" => {
            candidates.push(step1.eet_bg2ee_game_folder.trim());
            candidates.push(step1.bg2ee_game_folder.trim());
        }
        _ => {
            candidates.push(step1.bgee_game_folder.trim());
            candidates.push(step1.eet_bgee_game_folder.trim());
            candidates.push(step1.bg2ee_game_folder.trim());
            candidates.push(step1.eet_bg2ee_game_folder.trim());
        }
    }

    let mut first_existing: Option<PathBuf> = None;
    for raw in candidates {
        if raw.is_empty() {
            continue;
        }
        let path = PathBuf::from(raw);
        if !path.exists() {
            continue;
        }
        if first_existing.is_none() {
            first_existing = Some(path.clone());
        }
        if path.join("chitin.key").exists() {
            return Some(path);
        }
    }
    first_existing
}

pub fn group_tp2s(mod_root: &Path, depth: usize) -> Vec<(String, Vec<PathBuf>)> {
    let tp2_paths: Vec<PathBuf> = WalkDir::new(mod_root)
        .follow_links(false)
        .max_depth(depth)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name.ends_with(".tp2").then(|| entry.path().to_path_buf())
        })
        .collect();

    let tp2_dirs: BTreeSet<PathBuf> = tp2_paths
        .iter()
        .filter_map(|tp2| tp2.parent().map(Path::to_path_buf))
        .collect();

    let mut grouped: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in tp2_paths {
        let group_key = mod_group_key(mod_root, &path, &tp2_dirs);
        grouped.entry(group_key).or_default().push(path);
    }
    grouped.into_iter().collect()
}

pub fn build_preview_mods(grouped: &[(String, Vec<PathBuf>)]) -> Vec<Step2ModState> {
    grouped
        .iter()
        .map(|(group_key, tp2_paths)| {
            let display_name = display_name_from_group_key(group_key);
            let tp2_path = tp2_paths
                .first()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let tp_file = Path::new(&tp2_path)
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| display_name.clone());
            Step2ModState {
                name: display_name,
                tp_file,
                tp2_path,
                readme_path: None,
                web_url: None,
                mod_prompt_summary: None,
                mod_prompt_events: Vec::new(),
                checked: false,
                hidden_components: Vec::new(),
                components: Vec::new(),
            }
        })
        .collect()
}

pub fn display_name_from_group_key(group_key: &str) -> String {
    Path::new(group_key)
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| group_key.to_string())
}

fn mod_group_key(mod_root: &Path, tp2_path: &Path, tp2_dirs: &BTreeSet<PathBuf>) -> String {
    if let Some(parent) = tp2_path.parent() {
        let mut current = parent;
        let mut best_group: Option<String> = None;
        while let Ok(rel_current) = current.strip_prefix(mod_root) {
            if tp2_dirs.contains(current) {
                let rel_current = rel_current.display().to_string();
                if !rel_current.trim().is_empty() {
                    best_group = Some(rel_current);
                }
            }
            let Some(next) = current.parent() else {
                break;
            };
            if next == current || !next.starts_with(mod_root) {
                break;
            }
            current = next;
        }
        if let Some(best_group) = best_group {
            return best_group;
        }
    }
    tp2_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| tp2_path.display().to_string())
}
