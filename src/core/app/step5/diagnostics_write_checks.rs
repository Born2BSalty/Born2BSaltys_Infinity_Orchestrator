// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use crate::app::state::WizardState;
use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsContext {
    pub(crate) dev_mode: bool,
    pub(crate) exe_fingerprint: String,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct AppDataCopySummary {
    pub(crate) copied: Vec<PathBuf>,
    pub(crate) missing: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct WriteCheckSummary {
    pub(crate) lines: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Tp2LayoutSummary {
    pub(crate) lines: Vec<String>,
}

pub(super) fn run_write_checks(state: &WizardState, ts: u64) -> WriteCheckSummary {
    let mut summary = WriteCheckSummary::default();
    let s = &state.step1;
    let mut seen: HashSet<String> = HashSet::new();

    push_dir_checks(
        &[
            ("Mods Folder", &s.mods_folder),
            ("Generate Directory", &s.generate_directory),
            ("WeiDU log folder", &s.weidu_log_folder),
            ("BGEE game folder", &s.bgee_game_folder),
            ("BG2EE game folder", &s.bg2ee_game_folder),
            ("EET BGEE game folder", &s.eet_bgee_game_folder),
            ("EET BG2EE game folder", &s.eet_bg2ee_game_folder),
            ("BGEE log folder", &s.bgee_log_folder),
            ("BG2EE log folder", &s.bg2ee_log_folder),
            ("EET BGEE log folder", &s.eet_bgee_log_folder),
            ("EET BG2EE log folder", &s.eet_bg2ee_log_folder),
            ("EET pre directory", &s.eet_pre_dir),
            ("EET new directory", &s.eet_new_dir),
        ],
        ts,
        &mut seen,
        &mut summary,
    );

    let weidu_binary = resolve_weidu_binary(&s.weidu_binary);
    let mod_installer_binary = resolve_mod_installer_binary(&s.mod_installer_binary);
    push_file_checks(
        &[
            ("WeiDU binary", weidu_binary.as_str()),
            ("mod_installer binary", mod_installer_binary.as_str()),
            ("WeiDU log file", &s.log_file),
            ("BGEE WeiDU log file", &s.bgee_log_file),
            ("BG2EE WeiDU log file", &s.bg2ee_log_file),
        ],
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

fn push_dir_checks(
    checks: &[(&str, &str)],
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    for (label, raw) in checks {
        push_dir_check(label, raw, ts, seen, summary);
    }
}

fn push_file_checks(
    checks: &[(&str, &str)],
    ts: u64,
    seen: &mut HashSet<String>,
    summary: &mut WriteCheckSummary,
) {
    for (label, raw) in checks {
        push_file_check(label, raw, ts, seen, summary);
    }
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

fn push_file_check(
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
    let target_key = format!("file:{}", path.display());
    if seen.insert(target_key) {
        match probe_file_target(&path) {
            Ok(true) => summary
                .lines
                .push(format!("OK | {label} target_exists | {}", path.display())),
            Ok(false) => summary.lines.push(format!(
                "FAIL | {label} target_exists | {} | file does not exist",
                path.display()
            )),
            Err(err) => summary.lines.push(format!(
                "FAIL | {label} target_is_file | {} | {err}",
                path.display()
            )),
        }
    }
    let Some(parent) = path.parent() else {
        summary.lines.push(format!(
            "INFO | {label} parent_writable | {value} | no parent directory"
        ));
        return;
    };
    let key = format!("dir:{}", parent.display());
    if !seen.insert(key) {
        return;
    }
    let line = match probe_write_dir(parent, ts) {
        Ok(()) => format!("OK | {label} parent_writable | {}", parent.display()),
        Err(err) => format!(
            "FAIL | {label} parent_writable | {} | {err}",
            parent.display()
        ),
    };
    summary.lines.push(line);
}

fn probe_file_target(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    if !path.is_file() {
        return Err("path exists but is not a file".to_string());
    }
    Ok(true)
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
    fs::write(&probe, b"bio_diagnostics_probe").map_err(|e| format!("write failed: {e}"))?;
    if let Err(e) = fs::remove_file(&probe) {
        return Err(format!("cleanup failed: {e}"));
    }
    Ok(())
}
