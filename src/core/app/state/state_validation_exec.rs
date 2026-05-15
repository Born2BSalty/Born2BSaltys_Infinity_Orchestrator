// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::{env, ffi::OsString};

pub(super) fn check_file(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        return;
    }
    *checked += 1;
    let raw = value.trim();
    let path = Path::new(raw);
    if !path.exists() {
        if label.to_ascii_lowercase().contains("binary")
            && !raw.contains('/')
            && !raw.contains('\\')
            && is_command_on_path(raw)
        {
            return;
        }
        let parent_exists = path.parent().is_some_and(std::path::Path::exists);
        if parent_exists {
            errors.push(format!("{label} file was not found"));
        } else {
            errors.push(format!("{label} does not exist"));
        }
    } else if !path.is_file() {
        errors.push(format!("{label} must be a file"));
    }
    if label.contains("WeiDU Log File")
        && path.extension().is_some()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| !ext.eq_ignore_ascii_case("log"))
    {
        errors.push(format!("{label} should use .log extension"));
    }
}

fn is_command_on_path(command: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    #[cfg(target_os = "windows")]
    let mut candidates: Vec<OsString> = vec![OsString::from(command)];
    #[cfg(not(target_os = "windows"))]
    let candidates: Vec<OsString> = vec![OsString::from(command)];
    #[cfg(target_os = "windows")]
    {
        let has_ext = Path::new(command).extension().is_some();
        if !has_ext {
            if let Some(exts) = env::var_os("PATHEXT") {
                for ext in exts
                    .to_string_lossy()
                    .split(';')
                    .filter(|value| !value.trim().is_empty())
                {
                    candidates.push(OsString::from(format!("{command}{ext}")));
                }
            } else {
                candidates.push(OsString::from(format!("{command}.exe")));
            }
        }
    }
    env::split_paths(&path_var).any(|dir| candidates.iter().any(|name| dir.join(name).is_file()))
}
