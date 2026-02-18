// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::{env, ffi::OsString};

pub(crate) fn check_dir(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        return;
    }
    *checked += 1;
    let p = Path::new(value.trim());
    if !p.exists() {
        errors.push(format!("{label} does not exist"));
    } else if !p.is_dir() {
        errors.push(format!("{label} must be a folder"));
    }
}

pub(crate) fn check_file(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        return;
    }
    *checked += 1;
    let raw = value.trim();
    let p = Path::new(raw);
    if !p.exists() {
        if label.to_ascii_lowercase().contains("binary")
            && !raw.contains('/')
            && !raw.contains('\\')
            && is_command_on_path(raw)
        {
            return;
        }
        let parent_exists = p.parent().is_some_and(|parent| parent.exists());
        if parent_exists {
            errors.push(format!("{label} file was not found"));
        } else {
            errors.push(format!("{label} does not exist"));
        }
    } else if !p.is_file() {
        errors.push(format!("{label} must be a file"));
    }
    if label.contains("WeiDU Log File")
        && p.extension().is_some()
        && p.extension()
            .and_then(|v| v.to_str())
            .map(|ext| !ext.eq_ignore_ascii_case("log"))
            .unwrap_or(false)
    {
        errors.push(format!("{label} should use .log extension"));
    }
}

fn is_command_on_path(command: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    let mut candidates: Vec<OsString> = vec![OsString::from(command)];
    #[cfg(target_os = "windows")]
    {
        let has_ext = Path::new(command).extension().is_some();
        if !has_ext {
            if let Some(exts) = env::var_os("PATHEXT") {
                for ext in exts.to_string_lossy().split(';').filter(|v| !v.trim().is_empty()) {
                    candidates.push(OsString::from(format!("{command}{ext}")));
                }
            } else {
                candidates.push(OsString::from(format!("{command}.exe")));
            }
        }
    }
    env::split_paths(&path_var).any(|dir| {
        candidates
            .iter()
            .any(|name| dir.join(name).is_file())
    })
}

pub(crate) fn check_game_dir(
    label: &str,
    value: &str,
    checked: &mut usize,
    errors: &mut Vec<String>,
) {
    check_dir(label, value, checked, errors);
    let t = value.trim();
    if t.is_empty() {
        return;
    }
    let p = Path::new(t);
    if p.is_dir() {
        let has_chitin = p.join("chitin.key").is_file();
        let has_lang = p.join("lang").is_dir();
        if !has_chitin || !has_lang {
            errors.push(format!(
                "{label} does not look like an Infinity Engine game folder (missing chitin.key/lang)"
            ));
        }
    }
}
