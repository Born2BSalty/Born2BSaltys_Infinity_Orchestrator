// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::Step1State;

pub struct TargetPrepResult {
    pub backups: Vec<PathBuf>,
    pub cleaned: Vec<PathBuf>,
}

pub fn prepare_target_dirs_before_install(step1: &Step1State) -> std::io::Result<TargetPrepResult> {
    let mut backups = Vec::new();
    let mut cleaned = Vec::new();

    if step1.new_pre_eet_dir_enabled {
        if step1.backup_targets_before_eet_copy {
            if let Some(path) = backup_target_dir_if_nonempty(&step1.eet_pre_dir)? {
                backups.push(path);
            }
        } else if let Some(path) = clean_target_dir_if_nonempty(&step1.eet_pre_dir)? {
            cleaned.push(path);
        }
    }

    if step1.new_eet_dir_enabled {
        if step1.backup_targets_before_eet_copy {
            if let Some(path) = backup_target_dir_if_nonempty(&step1.eet_new_dir)? {
                backups.push(path);
            }
        } else if let Some(path) = clean_target_dir_if_nonempty(&step1.eet_new_dir)? {
            cleaned.push(path);
        }
    }

    if step1.generate_directory_enabled {
        if step1.backup_targets_before_eet_copy {
            if let Some(path) = backup_target_dir_if_nonempty(&step1.generate_directory)? {
                backups.push(path);
            }
        } else if let Some(path) = clean_target_dir_if_nonempty(&step1.generate_directory)? {
            cleaned.push(path);
        }
    }

    Ok(TargetPrepResult { backups, cleaned })
}

pub(super) fn paths_point_to_same_dir(a: &Path, b: &Path) -> bool {
    let ac = fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let bc = fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    #[cfg(target_os = "windows")]
    {
        return ac.to_string_lossy().eq_ignore_ascii_case(&bc.to_string_lossy());
    }
    #[cfg(not(target_os = "windows"))]
    {
        ac == bc
    }
}

fn backup_target_dir_if_nonempty(target: &str) -> std::io::Result<Option<PathBuf>> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(None);
    }
    let target_path = PathBuf::from(target);
    if !target_path.exists() || !target_path.is_dir() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(&target_path)?;
    if entries.next().is_none() {
        return Ok(None);
    }

    let parent = target_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("target");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = parent.join(format!("_bio_backup_{name}_{ts}"));

    fs::rename(&target_path, &backup)?;
    fs::create_dir_all(&target_path)?;
    Ok(Some(backup))
}

fn clean_target_dir_if_nonempty(target: &str) -> std::io::Result<Option<PathBuf>> {
    let target = target.trim();
    if target.is_empty() {
        return Ok(None);
    }
    let target_path = PathBuf::from(target);
    if !target_path.exists() || !target_path.is_dir() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(&target_path)?;
    if entries.next().is_none() {
        return Ok(None);
    }

    fs::remove_dir_all(&target_path)?;
    fs::create_dir_all(&target_path)?;
    Ok(Some(target_path))
}
