// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;
use std::{env, ffi::OsString};

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};
use crate::ui::state::Step1State;

pub fn run_path_check(s: &Step1State) -> (bool, String) {
    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let required = step1_validation_messages(s);
    if !required.is_empty() {
        errors.extend(required);
    }

    check_dir("Mods Folder", &s.mods_folder, &mut checked, &mut errors);
    if has_value(&s.mods_folder) {
        let mods_path = Path::new(s.mods_folder.trim());
        if mods_path.is_dir() {
            let depth = if s.custom_scan_depth { s.depth } else { 5 };
            if !has_tp2_within_depth(mods_path, depth) {
                errors.push(format!(
                    "Mods Folder has no .tp2 files within scan depth {}",
                    depth
                ));
            }
        }
    }

    let weidu_binary = resolve_weidu_binary(&s.weidu_binary);
    check_file("WeiDU binary", &weidu_binary, &mut checked, &mut errors);
    if has_value(&weidu_binary) {
        let p = Path::new(weidu_binary.trim());
        if let Some(name) = p.file_name().and_then(|v| v.to_str()) {
            let lower = name.to_ascii_lowercase();
            if !lower.contains("weidu") {
                errors.push("WeiDU binary path does not look like a WeiDU executable".to_string());
            }
        }
    }

    let mod_installer_binary = resolve_mod_installer_binary(&s.mod_installer_binary);
    check_file(
        "mod_installer binary",
        &mod_installer_binary,
        &mut checked,
        &mut errors,
    );

    run_mode_checks(s, &mut checked, &mut errors);

    if s.weidu_log_mode_enabled {
        check_dir(
            "Per-component log folder",
            &s.weidu_log_folder,
            &mut checked,
            &mut errors,
        );
        if s.weidu_log_log_component {
            let log_dir = s.weidu_log_folder.trim();
            if !log_dir.is_empty() && log_dir.contains(' ') {
                errors.push(format!(
                    "WeiDU log folder in -u mode cannot contain spaces on this backend. Invalid path: \"{}\". Use a no-space path.",
                    log_dir
                ));
            }
        }
    }

    format_path_check_result(&errors, checked)
}

pub fn sync_weidu_log_mode(s: &mut Step1State) {
    let mut parts = Vec::new();
    if s.weidu_log_autolog {
        parts.push("autolog".to_string());
    }
    if s.weidu_log_logapp {
        parts.push("logapp".to_string());
    }
    if s.weidu_log_logextern {
        parts.push("log-extern".to_string());
    }
    if s.weidu_log_log_component {
        if s.weidu_log_folder.trim().is_empty() {
            parts.push("log".to_string());
        } else {
            parts.push(format!("log {}", s.weidu_log_folder.trim()));
        }
    }
    if parts.is_empty() {
        parts.push("autolog".to_string());
    }
    s.weidu_log_mode = parts.join(",");
}

pub fn split_path_check_lines(msg: &str) -> Vec<String> {
    let details = msg.strip_prefix("Path check failed: ").unwrap_or(msg);
    details
        .split(" | ")
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

fn step1_validation_messages(s: &Step1State) -> Vec<String> {
    let mut out = Vec::new();
    if !has_value(&s.mods_folder) {
        out.push("Mods Folder is required".to_string());
    }
    if !has_value(&s.weidu_binary) {
        out.push("WeiDU binary is required".to_string());
    }
    match s.game_install.as_str() {
        "BG2EE" => {
            if !has_value(&s.bg2ee_game_folder) {
                out.push("BG2EE Game Folder is required".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required".to_string());
                }
            } else if !has_value(&s.bg2ee_log_folder) {
                out.push("BG2EE WeiDU Log Folder is required".to_string());
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                if !has_value(&s.bgee_game_folder) {
                    out.push("Source BGEE Folder (-p) is required".to_string());
                }
                if !has_value(&s.eet_pre_dir) {
                    out.push("Pre-EET Directory is required when -p is enabled".to_string());
                }
            } else if !has_value(&s.eet_bgee_game_folder) {
                out.push("BGEE Game Folder is required for EET".to_string());
            }
            if s.new_eet_dir_enabled {
                if !has_value(&s.bg2ee_game_folder) {
                    out.push("Source BG2EE Folder (-n) is required".to_string());
                }
                if !has_value(&s.eet_new_dir) {
                    out.push("New EET Directory is required when -n is enabled".to_string());
                }
            } else if !has_value(&s.eet_bg2ee_game_folder) {
                out.push("BG2EE Game Folder is required for EET".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required for EET".to_string());
                }
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required for EET".to_string());
                }
            } else {
                if !has_value(&s.eet_bgee_log_folder) {
                    out.push("BGEE WeiDU Log Folder is required for EET".to_string());
                }
                if !has_value(&s.eet_bg2ee_log_folder) {
                    out.push("BG2EE WeiDU Log Folder is required for EET".to_string());
                }
            }
        }
        _ => {
            if !has_value(&s.bgee_game_folder) {
                out.push("BGEE Game Folder is required".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required".to_string());
                }
            } else if !has_value(&s.bgee_log_folder) {
                out.push("BGEE WeiDU Log Folder is required".to_string());
            }
        }
    }
    out
}

fn run_mode_checks(s: &Step1State, checked: &mut usize, errors: &mut Vec<String>) {
    match s.game_install.as_str() {
        "BG2EE" => {
            check_game_dir("BG2EE Game Folder", &s.bg2ee_game_folder, checked, errors);
            if s.have_weidu_logs {
                check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else {
                check_dir(
                    "BG2EE WeiDU Log Folder",
                    &s.bg2ee_log_folder,
                    checked,
                    errors,
                );
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                check_game_dir(
                    "Source BGEE Folder (-p)",
                    &s.bgee_game_folder,
                    checked,
                    errors,
                );
                check_dir("Pre-EET Directory", &s.eet_pre_dir, checked, errors);
            } else {
                check_game_dir("BGEE Game Folder", &s.eet_bgee_game_folder, checked, errors);
            }

            if s.new_eet_dir_enabled {
                check_game_dir(
                    "Source BG2EE Folder (-n)",
                    &s.bg2ee_game_folder,
                    checked,
                    errors,
                );
                check_dir("New EET Directory", &s.eet_new_dir, checked, errors);
            } else {
                check_game_dir(
                    "BG2EE Game Folder",
                    &s.eet_bg2ee_game_folder,
                    checked,
                    errors,
                );
            }

            if s.have_weidu_logs {
                check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
                check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else {
                check_dir(
                    "BGEE WeiDU Log Folder",
                    &s.eet_bgee_log_folder,
                    checked,
                    errors,
                );
                check_dir(
                    "BG2EE WeiDU Log Folder",
                    &s.eet_bg2ee_log_folder,
                    checked,
                    errors,
                );
            }
        }
        _ => {
            check_game_dir("BGEE Game Folder", &s.bgee_game_folder, checked, errors);
            if s.have_weidu_logs {
                check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
            } else {
                check_dir("BGEE WeiDU Log Folder", &s.bgee_log_folder, checked, errors);
            }
        }
    }
}

fn format_path_check_result(errors: &[String], checked: usize) -> (bool, String) {
    if errors.is_empty() {
        (
            true,
            format!("Path check passed ({checked} path(s) validated)"),
        )
    } else {
        (false, format!("Path check failed: {}", errors.join(" | ")))
    }
}

fn check_dir(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
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

fn check_file(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
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
                for ext in exts
                    .to_string_lossy()
                    .split(';')
                    .filter(|v| !v.trim().is_empty())
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

fn check_game_dir(label: &str, value: &str, checked: &mut usize, errors: &mut Vec<String>) {
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

fn has_tp2_within_depth(root: &Path, depth: usize) -> bool {
    fn walk(dir: &Path, remaining: usize) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let is_tp2 = path
                    .extension()
                    .and_then(|v| v.to_str())
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
