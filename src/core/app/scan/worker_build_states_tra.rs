// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) fn load_tp2_setup_tra_map(tp2_path: &Path) -> HashMap<String, String> {
    let Some(base) = tp2_path.parent() else {
        return HashMap::new();
    };

    let mut candidates = Vec::<std::path::PathBuf>::new();
    let tp2_stem = tp2_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let mod_key = tp2_stem.strip_prefix("setup-").unwrap_or(tp2_stem);
    let custom_setup_name = if mod_key.is_empty() {
        None
    } else {
        Some(format!("{mod_key}setup.tra"))
    };
    let preferred = [
        base.join("lang/english/setup.tra"),
        base.join("lang/english").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/english").join(name))
            .unwrap_or_default(),
        base.join("lang/en_us/setup.tra"),
        base.join("lang/en_us").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/en_us").join(name))
            .unwrap_or_default(),
        base.join("lang/en_US/setup.tra"),
        base.join("lang/en_US").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/en_US").join(name))
            .unwrap_or_default(),
        base.join("setup.tra"),
        base.join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join(name))
            .unwrap_or_default(),
    ];
    for path in preferred {
        if path.is_file() && !candidates.iter().any(|existing| existing == &path) {
            candidates.push(path);
        }
    }
    if candidates.is_empty() {
        for path in walk_setup_tra_files(base) {
            if !candidates.iter().any(|existing| existing == &path) {
                candidates.push(path);
            }
        }
    }

    let mut merged = HashMap::new();
    for path in candidates {
        if let Ok(text) = fs::read_to_string(&path) {
            let map = parse_tra_string_map(&text);
            for (key, value) in map {
                merged.entry(key).or_insert(value);
            }
        }
    }
    merged
}

pub(super) fn resolve_group_token_label(
    token: &str,
    tra_map: &HashMap<String, String>,
) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('@') {
        return tra_map.get(trimmed).cloned();
    }
    if let Some(rest) = trimmed.strip_prefix('~') {
        return Some(rest.split('~').next().unwrap_or_default().trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        return Some(rest.split('"').next().unwrap_or_default().trim().to_string());
    }
    Some(trimmed.to_string())
}

fn walk_setup_tra_files(base: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::<std::path::PathBuf>::new();
    let Ok(read_dir) = fs::read_dir(base) else {
        return out;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_setup_tra_files(&path));
            continue;
        }
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| {
                name.ends_with(".tra") && name.to_ascii_lowercase().contains("setup")
            })
        {
            out.push(path);
        }
    }
    out
}

fn parse_tra_string_map(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            continue;
        }
        let Some((key, rhs)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let rhs = rhs.trim();
        let value = if let Some(rest) = rhs.strip_prefix('~') {
            rest.split('~').next().unwrap_or_default().trim().to_string()
        } else if let Some(rest) = rhs.strip_prefix('"') {
            rest.split('"').next().unwrap_or_default().trim().to_string()
        } else {
            continue;
        };
        if !value.is_empty() {
            out.insert(key, value);
        }
    }
    out
}
