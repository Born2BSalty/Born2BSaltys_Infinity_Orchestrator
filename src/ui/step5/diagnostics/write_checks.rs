// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};
use crate::ui::state::WizardState;

use super::WriteCheckSummary;

pub(super) fn run_write_checks(state: &WizardState, ts: u64) -> WriteCheckSummary {
    let mut summary = WriteCheckSummary::default();
    let s = &state.step1;
    let mut seen: HashSet<String> = HashSet::new();

    push_dir_check("Mods Folder", &s.mods_folder, ts, &mut seen, &mut summary);
    push_dir_check(
        "Generate Directory",
        &s.generate_directory,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "WeiDU log folder",
        &s.weidu_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BGEE game folder",
        &s.bgee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BG2EE game folder",
        &s.bg2ee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BGEE game folder",
        &s.eet_bgee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BG2EE game folder",
        &s.eet_bg2ee_game_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BGEE log folder",
        &s.bgee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "BG2EE log folder",
        &s.bg2ee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BGEE log folder",
        &s.eet_bgee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET BG2EE log folder",
        &s.eet_bg2ee_log_folder,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET pre directory",
        &s.eet_pre_dir,
        ts,
        &mut seen,
        &mut summary,
    );
    push_dir_check(
        "EET new directory",
        &s.eet_new_dir,
        ts,
        &mut seen,
        &mut summary,
    );

    let weidu_binary = resolve_weidu_binary(&s.weidu_binary);
    push_parent_check("WeiDU binary", &weidu_binary, ts, &mut seen, &mut summary);
    let mod_installer_binary = resolve_mod_installer_binary(&s.mod_installer_binary);
    push_parent_check(
        "mod_installer binary",
        &mod_installer_binary,
        ts,
        &mut seen,
        &mut summary,
    );
    push_parent_check("WeiDU log file", &s.log_file, ts, &mut seen, &mut summary);
    push_parent_check(
        "BGEE WeiDU log file",
        &s.bgee_log_file,
        ts,
        &mut seen,
        &mut summary,
    );
    push_parent_check(
        "BG2EE WeiDU log file",
        &s.bg2ee_log_file,
        ts,
        &mut seen,
        &mut summary,
    );

    if summary.lines.is_empty() {
        summary
            .lines
            .push("INFO | write checks | no configured paths to test".to_string());
    }
    summary
}

fn push_dir_check(
    label: &str,
    raw: &str,
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    let value = raw.trim();
    if value.is_empty() {
        return;
    }
    let key = format!("dir:{value}");
    if !seen.insert(key) {
        return;
    }
    let path = PathBuf::from(value);
    let line = match probe_write_dir(&path, ts) {
        Ok(()) => format!("OK | {label} | {}", path.display()),
        Err(err) => format!("FAIL | {label} | {} | {err}", path.display()),
    };
    summary.lines.push(line);
}

fn push_parent_check(
    label: &str,
    raw: &str,
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    let value = raw.trim();
    if value.is_empty() {
        return;
    }
    let path = PathBuf::from(value);
    let Some(parent) = path.parent() else {
        summary
            .lines
            .push(format!("INFO | {label} parent | {value} | no parent directory"));
        return;
    };
    let key = format!("dir:{}", parent.display());
    if !seen.insert(key) {
        return;
    }
    let line = match probe_write_dir(parent, ts) {
        Ok(()) => format!("OK | {label} parent | {}", parent.display()),
        Err(err) => format!("FAIL | {label} parent | {} | {err}", parent.display()),
    };
    summary.lines.push(line);
}

fn probe_write_dir(dir: &Path, ts: u64) -> Result<(), String> {
    if !dir.exists() {
        return Err("directory does not exist".to_string());
    }
    if !dir.is_dir() {
        return Err("path is not a directory".to_string());
    }
    let probe = dir.join(format!(
        ".bio_diag_write_probe_{}_{}.tmp",
        process::id(),
        ts
    ));
    fs::write(&probe, b"bio_diagnostics_probe")
        .map_err(|e| format!("write failed: {e}"))?;
    if let Err(e) = fs::remove_file(&probe) {
        return Err(format!("cleanup failed: {e}"));
    }
    Ok(())
}
