// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::state::Step1State;

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
        copy_file_group(logs_dir, "BG2EE WeiDU Log File", step1.bg2ee_log_file.trim()),
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
