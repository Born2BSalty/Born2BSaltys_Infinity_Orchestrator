// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};
use crate::ui::state::Step1State;

use super::common_checks::{check_dir, check_file};
use super::tp2_depth::has_tp2_within_depth;
use crate::ui::step1::validation::formatting::format_path_check_result;
use crate::ui::step1::validation::messages::{has_value, step1_validation_messages};
use crate::ui::step1::validation::mode_checks::run_mode_checks;

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
