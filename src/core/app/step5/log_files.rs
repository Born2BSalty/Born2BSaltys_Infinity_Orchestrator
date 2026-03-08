// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod log_open {
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::controller::util::open_in_shell;
use crate::ui::state::Step1State;

pub fn open_last_log_file(step1: &Step1State) -> std::io::Result<()> {
    let Some(path) = newest_log_file(step1) else {
        return Ok(());
    };
    open_in_shell(path.to_string_lossy().as_ref())
}

pub fn save_console_log(content: &str) -> std::io::Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("console_{ts}.log"));
    fs::write(&out_path, content)?;
    Ok(out_path)
}

pub fn open_console_logs_folder() -> std::io::Result<()> {
    let dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&dir)?;
    open_in_shell(dir.to_string_lossy().as_ref())
}

fn newest_log_file(step1: &Step1State) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in log_candidate_dirs(step1) {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("log"))
                != Some(true)
            {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            match &best {
                Some((best_time, _)) if modified <= *best_time => {}
                _ => best = Some((modified, path)),
            }
        }
    }
    best.map(|(_, p)| p)
}

fn log_candidate_dirs(step1: &Step1State) -> Vec<PathBuf> {
    let mut dirs = Vec::<PathBuf>::new();
    let mut add = |v: &str| {
        let t = v.trim();
        if !t.is_empty() {
            dirs.push(PathBuf::from(t));
        }
    };
    add(&step1.weidu_log_folder);
    add(&step1.bgee_log_folder);
    add(&step1.bg2ee_log_folder);
    add(&step1.eet_bgee_log_folder);
    add(&step1.eet_bg2ee_log_folder);
    if !step1.bgee_log_file.trim().is_empty()
        && let Some(p) = Path::new(step1.bgee_log_file.trim()).parent()
    {
        dirs.push(p.to_path_buf());
    }
    if !step1.bg2ee_log_file.trim().is_empty()
        && let Some(p) = Path::new(step1.bg2ee_log_file.trim()).parent()
    {
        dirs.push(p.to_path_buf());
    }
    dirs
}
}
mod path_validators {
use std::fs;
use std::path::Path;

use crate::ui::state::{ResumeTargets, Step1State};

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

pub fn validate_resume_paths(step1: &Step1State, resume_targets: &ResumeTargets) -> Result<(), String> {
    let mut checks: Vec<(&str, &str)> = Vec::new();
    if step1.game_install == "EET" {
        let bg1_dir = resume_targets
            .bg1_game_dir
            .as_deref()
            .unwrap_or(step1.eet_bgee_game_folder.trim());
        let bg2_dir = resume_targets
            .bg2_game_dir
            .as_deref()
            .unwrap_or(step1.eet_bg2ee_game_folder.trim());
        checks.push(("Resume BGEE game directory", bg1_dir));
        checks.push(("Resume BG2EE/EET game directory", bg2_dir));
    } else {
        let game_dir = resume_targets.game_dir.as_deref().unwrap_or_else(|| {
            if step1.game_install == "BG2EE" {
                step1.bg2ee_game_folder.trim()
            } else {
                step1.bgee_game_folder.trim()
            }
        });
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
}
mod source_logs {
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::ui::state::Step1State;

#[derive(Debug, Clone)]
pub struct SourceLogInfo {
    pub tag: &'static str,
    pub path: PathBuf,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified: Option<SystemTime>,
}

pub fn copy_source_weidu_logs(step1: &Step1State, out_dir: &Path, suffix: &str) -> Vec<PathBuf> {
    let mut copied = Vec::new();
    let _ = fs::create_dir_all(out_dir);
    for (tag, source) in resolve_source_logs(step1) {
        if !source.is_file() {
            continue;
        }
        let dest = out_dir.join(format!("weidu_{tag}_{suffix}.log"));
        if fs::copy(&source, &dest).is_ok() {
            copied.push(dest);
        }
    }
    copied
}

pub fn copy_saved_weidu_logs(step1: &Step1State, out_dir: &Path, suffix: &str) -> Vec<PathBuf> {
    let mut copied = Vec::new();
    let _ = fs::create_dir_all(out_dir);
    for (tag, source) in resolve_saved_logs(step1) {
        if !source.is_file() {
            continue;
        }
        let dest = out_dir.join(format!("weidu_{tag}_{suffix}.log"));
        if fs::copy(&source, &dest).is_ok() {
            copied.push(dest);
        }
    }
    copied
}

pub fn source_log_infos(step1: &Step1State) -> Vec<SourceLogInfo> {
    resolve_source_logs(step1)
        .into_iter()
        .map(|(tag, path)| {
            let meta = fs::metadata(&path).ok();
            SourceLogInfo {
                tag,
                path,
                exists: meta.as_ref().is_some_and(|m| m.is_file()),
                size_bytes: meta.as_ref().map(|m| m.len()),
                modified: meta.and_then(|m| m.modified().ok()),
            }
        })
        .collect()
}

fn resolve_source_logs(step1: &Step1State) -> Vec<(&'static str, PathBuf)> {
    match step1.game_install.as_str() {
        "EET" => vec![
            ("bgee", resolve_bgee_log_path(step1)),
            ("bg2ee", resolve_bg2_log_path(step1)),
        ],
        "BG2EE" => vec![("bg2ee", resolve_bg2_log_path(step1))],
        _ => vec![("bgee", resolve_bgee_log_path(step1))],
    }
}

fn resolve_bgee_log_path(step1: &Step1State) -> PathBuf {
    if !step1.bgee_log_file.trim().is_empty() {
        PathBuf::from(step1.bgee_log_file.trim())
    } else {
        resolve_saved_bgee_log_path(step1)
    }
}

fn resolve_bg2_log_path(step1: &Step1State) -> PathBuf {
    if !step1.bg2ee_log_file.trim().is_empty() {
        PathBuf::from(step1.bg2ee_log_file.trim())
    } else {
        resolve_saved_bg2_log_path(step1)
    }
}

fn resolve_saved_logs(step1: &Step1State) -> Vec<(&'static str, PathBuf)> {
    match step1.game_install.as_str() {
        "EET" => vec![
            ("bgee", resolve_saved_bgee_log_path(step1)),
            ("bg2ee", resolve_saved_bg2_log_path(step1)),
        ],
        "BG2EE" => vec![("bg2ee", resolve_saved_bg2_log_path(step1))],
        _ => vec![("bgee", resolve_saved_bgee_log_path(step1))],
    }
}

fn resolve_saved_bgee_log_path(step1: &Step1State) -> PathBuf {
    let folder = if step1.game_install == "EET" {
        step1.eet_bgee_log_folder.trim()
    } else {
        step1.bgee_log_folder.trim()
    };
    PathBuf::from(folder).join("weidu.log")
}

fn resolve_saved_bg2_log_path(step1: &Step1State) -> PathBuf {
    let folder = if step1.game_install == "EET" {
        step1.eet_bg2ee_log_folder.trim()
    } else {
        step1.bg2ee_log_folder.trim()
    };
    PathBuf::from(folder).join("weidu.log")
}
}

#[path = "log_files_target_prep.rs"]
mod target_prep;
pub use log_open::{open_console_logs_folder, open_last_log_file, save_console_log};
pub use path_validators::{
    validate_resume_paths, validate_runtime_prep_paths, verify_targets_prepared,
};
pub use source_logs::{SourceLogInfo, copy_saved_weidu_logs, copy_source_weidu_logs, source_log_infos};
pub use target_prep::{TargetPrepResult, prepare_target_dirs_before_install};
