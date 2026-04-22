// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::app::state::{Step1State, Step2ModState};

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

pub fn group_tp2s(mod_root: &Path, depth: usize) -> Result<Vec<(String, Vec<PathBuf>)>, String> {
    let mut tp2_paths = Vec::<PathBuf>::new();
    for entry in WalkDir::new(mod_root).follow_links(false).max_depth(depth) {
        let entry = entry
            .map_err(|err| format!("failed to scan mods folder {}: {err}", mod_root.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".tp2") {
            tp2_paths.push(entry.path().to_path_buf());
        }
    }

    let tp2_dirs: BTreeSet<PathBuf> = tp2_paths
        .iter()
        .filter_map(|tp2| tp2.parent().map(Path::to_path_buf))
        .collect();

    let mut grouped: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in tp2_paths {
        let group_key = mod_group_key(mod_root, &path, &tp2_dirs);
        grouped.entry(group_key).or_default().push(path);
    }
    Ok(grouped.into_iter().collect())
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
                package_marker: None,
                latest_checked_version: None,
                update_locked: false,
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
    if let Some(named_group) = named_package_group_key(mod_root, tp2_path, tp2_dirs) {
        return named_group;
    }
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

fn named_package_group_key(
    mod_root: &Path,
    tp2_path: &Path,
    tp2_dirs: &BTreeSet<PathBuf>,
) -> Option<String> {
    let tp2_stem = tp2_path.file_stem()?.to_str()?;
    let tp2_key = tp2_stem
        .strip_prefix("setup-")
        .unwrap_or(tp2_stem)
        .to_ascii_lowercase();
    let mut current = tp2_path.parent()?;

    while let Ok(rel_current) = current.strip_prefix(mod_root) {
        if tp2_dirs.contains(current)
            && current
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(&tp2_key))
        {
            let rel = rel_current.display().to_string();
            if !rel.trim().is_empty() {
                return Some(rel);
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

    None
}

#[cfg(test)]
mod tests {
    use super::mod_group_key;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn nested_self_named_tp2_uses_its_own_group() {
        let mods_root = PathBuf::from("/mods");
        let tp2_path = PathBuf::from(
            "/mods/EET/other/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker.tp2",
        );
        let tp2_dirs = BTreeSet::from([
            PathBuf::from("/mods/EET"),
            PathBuf::from("/mods/EET/other/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker"),
        ]);

        let group = mod_group_key(&mods_root, &tp2_path, &tp2_dirs);

        assert_eq!(
            group,
            "EET/other/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker"
        );
    }

    #[test]
    fn helper_tp2_without_named_package_stays_under_parent_group() {
        let mods_root = PathBuf::from("/mods");
        let tp2_path = PathBuf::from("/mods/questpack/simwork/sahuagin.tp2");
        let tp2_dirs = BTreeSet::from([
            PathBuf::from("/mods/questpack"),
            PathBuf::from("/mods/questpack/simwork"),
        ]);

        let group = mod_group_key(&mods_root, &tp2_path, &tp2_dirs);

        assert_eq!(group, "questpack");
    }
}
