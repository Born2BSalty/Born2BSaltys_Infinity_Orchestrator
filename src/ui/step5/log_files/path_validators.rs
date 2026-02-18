// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use crate::ui::state::Step1State;

use super::target_prep::paths_point_to_same_dir;

pub fn validate_runtime_prep_paths(step1: &Step1State) -> Result<(), String> {
    let mut checks: Vec<(&str, &str, &str)> = Vec::new();
    if step1.game_install == "EET" {
        if step1.new_pre_eet_dir_enabled {
            checks.push((
                "Source BGEE Folder (-p)",
                step1.bgee_game_folder.trim(),
                step1.eet_pre_dir.trim(),
            ));
        }
        if step1.new_eet_dir_enabled {
            checks.push((
                "Source BG2EE Folder (-n)",
                step1.bg2ee_game_folder.trim(),
                step1.eet_new_dir.trim(),
            ));
        }
    } else if step1.generate_directory_enabled {
        let source = if step1.game_install == "BG2EE" {
            step1.bg2ee_game_folder.trim()
        } else {
            step1.bgee_game_folder.trim()
        };
        checks.push((
            "Source Game Folder (-g)",
            source,
            step1.generate_directory.trim(),
        ));
    }

    for (source_label, source, target) in checks {
        if source.is_empty() {
            return Err(format!("{source_label} is required"));
        }
        if target.is_empty() {
            return Err("Target directory is required for fresh install mode".to_string());
        }
        let source_path = Path::new(source);
        if !source_path.is_dir() {
            return Err(format!("{source_label} must be an existing folder"));
        }
        if !source_path.join("chitin.key").is_file() {
            return Err(format!("{source_label} is missing chitin.key"));
        }
        let target_path = Path::new(target);
        if !target_path.exists() || !target_path.is_dir() {
            return Err(format!(
                "Target directory must already exist and be a folder: {}",
                target
            ));
        }
        if paths_point_to_same_dir(source_path, target_path) {
            return Err(format!(
                "{source_label} and target directory cannot be the same: {}",
                source
            ));
        }
    }

    Ok(())
}

pub fn verify_targets_prepared(step1: &Step1State) -> Result<(), String> {
    let mut targets: Vec<(&str, &str)> = Vec::new();
    if step1.game_install == "EET" {
        if step1.new_pre_eet_dir_enabled {
            targets.push(("Pre-EET Directory", step1.eet_pre_dir.trim()));
        }
        if step1.new_eet_dir_enabled {
            targets.push(("New EET Directory", step1.eet_new_dir.trim()));
        }
    } else if step1.generate_directory_enabled {
        targets.push(("Generate Directory (-g)", step1.generate_directory.trim()));
    }

    for (label, target) in targets {
        if target.is_empty() {
            return Err(format!("{label} is required"));
        }
        let p = Path::new(target);
        if !p.exists() || !p.is_dir() {
            return Err(format!("{label} does not exist after prep: {target}"));
        }
        let mut entries = fs::read_dir(p).map_err(|e| format!("{label} read failed: {e}"))?;
        if entries.next().is_some() {
            return Err(format!("{label} is not empty after prep: {target}"));
        }
    }

    Ok(())
}

pub fn validate_resume_paths(step1: &Step1State) -> Result<(), String> {
    let mut checks: Vec<(&str, &str)> = Vec::new();
    if step1.game_install == "EET" {
        let bg1_dir = if step1.new_pre_eet_dir_enabled {
            step1.eet_pre_dir.trim()
        } else {
            step1.eet_bgee_game_folder.trim()
        };
        let bg2_dir = if step1.new_eet_dir_enabled {
            step1.eet_new_dir.trim()
        } else {
            step1.eet_bg2ee_game_folder.trim()
        };
        checks.push(("Resume BGEE game directory", bg1_dir));
        checks.push(("Resume BG2EE/EET game directory", bg2_dir));
    } else {
        let game_dir = if step1.generate_directory_enabled {
            step1.generate_directory.trim()
        } else if step1.game_install == "BG2EE" {
            step1.bg2ee_game_folder.trim()
        } else {
            step1.bgee_game_folder.trim()
        };
        checks.push(("Resume game directory", game_dir));
    }

    for (label, value) in checks {
        if value.is_empty() {
            return Err(format!("{label} is required"));
        }
        let p = Path::new(value);
        if !p.is_dir() {
            return Err(format!("{label} must be an existing folder: {value}"));
        }
        if !p.join("chitin.key").is_file() {
            return Err(format!("{label} missing chitin.key: {value}"));
        }
    }

    Ok(())
}
