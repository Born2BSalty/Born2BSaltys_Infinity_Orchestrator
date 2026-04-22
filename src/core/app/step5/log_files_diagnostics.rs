// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;

use super::source_logs::{copy_saved_weidu_logs, copy_source_weidu_logs};
use crate::app::state::{Step1State, Step5State};

pub fn begin_new_run(step5: &mut Step5State) -> String {
    prune_old_diagnostics(None);
    let run_id = make_run_id();
    step5.diagnostics_run_id = Some(run_id.clone());
    run_id
}

pub fn current_or_new_run_id(step5: &Step5State) -> String {
    step5.diagnostics_run_id.clone().unwrap_or_else(make_run_id)
}

pub fn run_dir_from_id(run_id: &str) -> PathBuf {
    PathBuf::from("diagnostics").join(format!("run_{run_id}"))
}

pub fn prune_old_diagnostics(keep_run_id: Option<&str>) {
    let diagnostics_dir = Path::new("diagnostics");
    let entries = match fs::read_dir(diagnostics_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() && name.starts_with("run_") {
            let keep_name = keep_run_id.map(|id| format!("run_{id}"));
            if keep_name.as_deref() == Some(name.as_ref()) {
                continue;
            }
            let _ = fs::remove_dir_all(&path);
        }
    }
}

pub fn copy_weidu_logs_for_diagnostics(step1: &Step1State, run_id: &str) {
    if !step1.bio_full_debug && !step1.log_raw_output_dev {
        return;
    }
    let run_dir = run_dir_from_id(run_id);
    let source_logs_dir = run_dir.join("source_logs");
    let _ = copy_source_weidu_logs(step1, &source_logs_dir, "original");
    let saved_logs_dir = run_dir.join("saved_logs");
    let _ = copy_saved_weidu_logs(step1, &saved_logs_dir, "original");
}

fn make_run_id() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S_%3f").to_string()
}

#[derive(Debug, Clone)]
pub struct DiagnosticLogGroup {
    pub label: String,
    pub copied_paths: Vec<PathBuf>,
}

pub fn copy_diagnostic_origin_logs(step1: &Step1State, logs_dir: &Path) -> Vec<DiagnosticLogGroup> {
    let _ = fs::create_dir_all(logs_dir);
    let mut groups = vec![
        copy_directory_group(
            logs_dir,
            "BGEE Game Folder",
            &resolve_bgee_game_folder(step1),
            should_check_weidu_bgee_log(step1, "BGEE Game Folder"),
        ),
        copy_directory_group(
            logs_dir,
            "BG2EE Game Folder",
            &resolve_bg2_game_folder(step1),
            should_check_weidu_bgee_log(step1, "BG2EE Game Folder"),
        ),
        copy_directory_group(
            logs_dir,
            "BGEE WeiDU Log Folder",
            &resolve_bgee_log_folder(step1),
            should_check_weidu_bgee_log(step1, "BGEE WeiDU Log Folder"),
        ),
        copy_directory_group(
            logs_dir,
            "BG2EE WeiDU Log Folder",
            &resolve_bg2_log_folder(step1),
            should_check_weidu_bgee_log(step1, "BG2EE WeiDU Log Folder"),
        ),
        copy_file_group(logs_dir, "BGEE WeiDU Log File", step1.bgee_log_file.trim()),
        copy_file_group(
            logs_dir,
            "BG2EE WeiDU Log File",
            step1.bg2ee_log_file.trim(),
        ),
    ];

    if !step1.eet_pre_dir.trim().is_empty() {
        groups.push(copy_directory_group(
            logs_dir,
            "Pre-EET Directory",
            step1.eet_pre_dir.trim(),
            should_check_weidu_bgee_log(step1, "Pre-EET Directory"),
        ));
    }
    if !step1.eet_new_dir.trim().is_empty() {
        groups.push(copy_directory_group(
            logs_dir,
            "New EET Directory",
            step1.eet_new_dir.trim(),
            should_check_weidu_bgee_log(step1, "New EET Directory"),
        ));
    }
    if !step1.generate_directory.trim().is_empty() {
        groups.push(copy_directory_group(
            logs_dir,
            "Generate Directory (-g)",
            step1.generate_directory.trim(),
            should_check_weidu_bgee_log(step1, "Generate Directory (-g)"),
        ));
    }

    groups
}

fn resolve_bgee_game_folder(step1: &Step1State) -> String {
    if step1.game_install == "EET" {
        step1.eet_bgee_game_folder.trim().to_string()
    } else {
        step1.bgee_game_folder.trim().to_string()
    }
}

fn resolve_bg2_game_folder(step1: &Step1State) -> String {
    if step1.game_install == "EET" {
        step1.eet_bg2ee_game_folder.trim().to_string()
    } else {
        step1.bg2ee_game_folder.trim().to_string()
    }
}

fn resolve_bgee_log_folder(step1: &Step1State) -> String {
    if step1.game_install == "EET" {
        step1.eet_bgee_log_folder.trim().to_string()
    } else {
        step1.bgee_log_folder.trim().to_string()
    }
}

fn resolve_bg2_log_folder(step1: &Step1State) -> String {
    if step1.game_install == "EET" {
        step1.eet_bg2ee_log_folder.trim().to_string()
    } else {
        step1.bg2ee_log_folder.trim().to_string()
    }
}

fn should_check_weidu_bgee_log(step1: &Step1State, label: &str) -> bool {
    step1.game_install == "EET"
        && matches!(
            label,
            "BGEE Game Folder"
                | "BGEE WeiDU Log Folder"
                | "Pre-EET Directory"
                | "New EET Directory"
                | "Generate Directory (-g)"
        )
}

fn copy_directory_group(
    logs_dir: &Path,
    label: &str,
    source_dir: &str,
    include_weidu_bgee_log: bool,
) -> DiagnosticLogGroup {
    let mut copied_paths = Vec::new();
    let source_dir = source_dir.trim();
    if source_dir.is_empty() {
        return DiagnosticLogGroup {
            label: label.to_string(),
            copied_paths,
        };
    }
    let source_dir = Path::new(source_dir);
    if !source_dir.is_dir() {
        return DiagnosticLogGroup {
            label: label.to_string(),
            copied_paths,
        };
    }

    let dest_dir = logs_dir.join(label);
    let _ = fs::create_dir_all(&dest_dir);
    let mut candidates = vec!["weidu.log"];
    if include_weidu_bgee_log {
        candidates.push("WeiDU-BGEE.log");
    }
    for name in candidates {
        let source = source_dir.join(name);
        if !source.is_file() {
            continue;
        }
        let dest = dest_dir.join(name);
        if fs::copy(&source, &dest).is_ok() {
            copied_paths.push(dest);
        }
    }

    DiagnosticLogGroup {
        label: label.to_string(),
        copied_paths,
    }
}

fn copy_file_group(logs_dir: &Path, label: &str, source_file: &str) -> DiagnosticLogGroup {
    let mut copied_paths = Vec::new();
    let source_file = source_file.trim();
    if source_file.is_empty() {
        return DiagnosticLogGroup {
            label: label.to_string(),
            copied_paths,
        };
    }
    let source = Path::new(source_file);
    if !source.is_file() {
        return DiagnosticLogGroup {
            label: label.to_string(),
            copied_paths,
        };
    }

    let dest_dir = logs_dir.join(label);
    let _ = fs::create_dir_all(&dest_dir);
    let file_name = source.file_name().unwrap_or_default();
    let dest = dest_dir.join(file_name);
    if fs::copy(source, &dest).is_ok() {
        copied_paths.push(dest);
    }

    DiagnosticLogGroup {
        label: label.to_string(),
        copied_paths,
    }
}
