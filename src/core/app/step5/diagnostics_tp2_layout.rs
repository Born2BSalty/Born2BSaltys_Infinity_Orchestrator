// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::state::WizardState;

use super::Tp2LayoutSummary;

pub(super) fn build_tp2_layout_summary(state: &WizardState) -> Tp2LayoutSummary {
    let mut summary = Tp2LayoutSummary::default();
    let mods_root_raw = state.step1.mods_folder.trim();
    let mods_root = if mods_root_raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(mods_root_raw))
    };

    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut selected_tp2_paths: Vec<PathBuf> = Vec::new();
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let selected = mod_state.checked || mod_state.components.iter().any(|c| c.checked);
        if !selected {
            continue;
        }
        let tp2 = mod_state.tp2_path.trim();
        if tp2.is_empty() {
            continue;
        }
        let key = tp2.to_ascii_lowercase();
        if !seen_paths.insert(key) {
            continue;
        }
        selected_tp2_paths.push(PathBuf::from(tp2));
    }

    if selected_tp2_paths.is_empty() {
        summary
            .lines
            .push("no selected TP2 paths from Step2".to_string());
        return summary;
    }

    selected_tp2_paths.sort();
    let limit = selected_tp2_paths.len().min(120);
    for tp2_path in selected_tp2_paths.into_iter().take(limit) {
        append_tp2_layout_lines(&mut summary.lines, &tp2_path, mods_root.as_deref());
    }
    if limit >= 120 {
        summary
            .lines
            .push("... truncated after 120 selected TP2 entries".to_string());
    }
    summary
}

fn append_tp2_layout_lines(lines: &mut Vec<String>, tp2_path: &Path, mods_root: Option<&Path>) {
    lines.push(format!("tp2={}", tp2_path.display()));
    lines.push(format!("tp2_exists={}", tp2_path.is_file()));

    if let Some(root) = mods_root {
        match tp2_path.strip_prefix(root) {
            Ok(rel) => {
                lines.push(format!("relative_from_mods={}", rel.display()));
                lines.push(format!("relative_depth={}", rel.components().count()));
            }
            Err(_) => {
                lines.push("relative_from_mods=<outside mods folder>".to_string());
            }
        }
    } else {
        lines.push("relative_from_mods=<mods folder not set>".to_string());
    }

    let Some(parent) = tp2_path.parent() else {
        lines.push("parent=<none>".to_string());
        lines.push(String::new());
        return;
    };
    lines.push(format!("parent={}", parent.display()));
    lines.push(format!("parent_exists={}", parent.is_dir()));

    let parent_entries = list_parent_entries(parent, 24);
    lines.push(format!(
        "parent_entries={}",
        if parent_entries.is_empty() {
            "<none>".to_string()
        } else {
            parent_entries.join(", ")
        }
    ));

    let hint_dirs = collect_hint_dirs(parent, 2, 30);
    lines.push(format!(
        "nearby_hint_dirs={}",
        if hint_dirs.is_empty() {
            "<none>".to_string()
        } else {
            hint_dirs.join(", ")
        }
    ));

    let tra_count = count_ext_files_limited(parent, "tra", 3, 5000);
    lines.push(format!("nearby_tra_file_count={tra_count}"));
    lines.push(String::new());
}

fn list_parent_entries(parent: &Path, max_items: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let file_type = e.file_type().ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            if file_type.is_dir() {
                Some(format!("[D]{name}"))
            } else if file_type.is_file() {
                Some(format!("[F]{name}"))
            } else {
                None
            }
        })
        .collect();
    names.sort_by_key(|v| v.to_ascii_lowercase());
    names.into_iter().take(max_items).collect()
}

fn collect_hint_dirs(dir: &Path, depth: usize, max_items: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    collect_hint_dirs_recursive(dir, depth, max_items, &mut out);
    out.sort_by_key(|v| v.to_ascii_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

fn collect_hint_dirs_recursive(dir: &Path, depth: usize, max_items: usize, out: &mut Vec<String>) {
    if depth == 0 || out.len() >= max_items {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= max_items {
            break;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "tra" | "lang" | "language" | "languages" | "lib" | "tpa" | "tph"
        ) {
            out.push(entry.path().display().to_string());
        }
        collect_hint_dirs_recursive(&entry.path(), depth.saturating_sub(1), max_items, out);
    }
}

fn count_ext_files_limited(dir: &Path, ext: &str, depth: usize, hard_limit: usize) -> usize {
    let mut total = 0usize;
    count_ext_files_recursive(dir, ext, depth, hard_limit, &mut total);
    total
}

fn count_ext_files_recursive(
    dir: &Path,
    ext: &str,
    depth: usize,
    hard_limit: usize,
    total: &mut usize,
) {
    if depth == 0 || *total >= hard_limit {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *total >= hard_limit {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            count_ext_files_recursive(&path, ext, depth.saturating_sub(1), hard_limit, total);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let is_match = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false);
        if is_match {
            *total = total.saturating_add(1);
        }
    }
}
