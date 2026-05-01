// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

pub(super) fn check_dir(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        return;
    }
    *checked += 1;
    let path = Path::new(value.trim());
    if !path.exists() {
        errors.push(format!("{label} does not exist"));
    } else if !path.is_dir() {
        errors.push(format!("{label} must be a folder"));
    }
}

pub(super) fn check_game_dir(
    label: &str,
    value: &str,
    checked: &mut usize,
    errors: &mut Vec<String>,
) {
    check_dir(label, value, checked, errors);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    let path = Path::new(trimmed);
    if path.is_dir() {
        let has_chitin = path.join("chitin.key").is_file();
        let has_lang = path.join("lang").is_dir();
        if !has_chitin || !has_lang {
            errors.push(format!(
                "{label} does not look like an Infinity Engine game folder (missing chitin.key/lang)"
            ));
        }
    }
}

pub(super) fn same_windows_drive(left: &str, right: &str) -> bool {
    match (windows_drive_prefix(left), windows_drive_prefix(right)) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        _ => true,
    }
}

pub(super) fn has_tp2_within_depth(root: &Path, depth: usize) -> bool {
    fn walk(dir: &Path, remaining: usize) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let is_tp2 = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("tp2"))
                    .unwrap_or(false);
                if is_tp2 {
                    return true;
                }
            } else if path.is_dir() && remaining > 0 && walk(&path, remaining - 1) {
                return true;
            }
        }
        false
    }

    walk(root, depth)
}

fn windows_drive_prefix(path: &str) -> Option<&str> {
    let path = path.trim();
    (path.len() >= 2 && path.as_bytes()[0].is_ascii_alphabetic() && path.as_bytes()[1] == b':')
        .then(|| &path[..2])
}
