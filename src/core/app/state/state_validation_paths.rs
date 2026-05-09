// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::Step1State;
use crate::app::state_validation_exec as exec;
use crate::app::state_validation_fs as fs_checks;
use crate::app::state_validation_modes;
use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};

use super::step1_validation_messages;

pub(super) fn run_path_check(s: &Step1State) -> (bool, String) {
    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let required = step1_validation_messages(s);
    if !required.is_empty() {
        errors.extend(required);
    }

    fs_checks::check_dir("Mods Folder", &s.mods_folder, &mut checked, &mut errors);
    if has_value(&s.mods_folder) {
        let mods_path = Path::new(s.mods_folder.trim());
        if mods_path.is_dir() {
            let depth = if s.custom_scan_depth { s.depth } else { 5 };
            if !(fs_checks::has_tp2_within_depth(mods_path, depth)
                || (s.have_weidu_logs && s.download_archive)
                || s.imports_modlist())
            {
                errors.push(format!(
                    "Mods Folder has no .tp2 files within scan depth {}",
                    depth
                ));
            }
        }
    }

    if s.download_archive {
        fs_checks::check_dir(
            "Mods Archive",
            &s.mods_archive_folder,
            &mut checked,
            &mut errors,
        );
        fs_checks::check_dir("Backup", &s.mods_backup_folder, &mut checked, &mut errors);
        if has_value(&s.mods_folder)
            && has_value(&s.mods_backup_folder)
            && !fs_checks::same_windows_drive(&s.mods_folder, &s.mods_backup_folder)
        {
            errors.push("Backup must be on the same drive as Mods Folder".to_string());
        }
    }

    let weidu_binary = resolve_weidu_binary(&s.weidu_binary);
    exec::check_file("WeiDU binary", &weidu_binary, &mut checked, &mut errors);
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
    exec::check_file(
        "mod_installer binary",
        &mod_installer_binary,
        &mut checked,
        &mut errors,
    );

    state_validation_modes::run_mode_checks(s, &mut checked, &mut errors);

    if s.weidu_log_mode_enabled {
        fs_checks::check_dir(
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

pub(super) fn step1_mods_folder_has_tp2(s: &Step1State) -> bool {
    let mods_path = Path::new(s.mods_folder.trim());
    let depth = if s.custom_scan_depth { s.depth } else { 5 };
    mods_path.is_dir() && fs_checks::has_tp2_within_depth(mods_path, depth)
}

pub(super) fn same_windows_drive(left: &str, right: &str) -> bool {
    fs_checks::same_windows_drive(left, right)
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
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
