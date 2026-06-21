// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use crate::app::state::{ResumeTargets, Step1State};
use crate::install::plan::InstallPlan;
use crate::install::runner::check_missing_mod_folders;
use crate::mods::log_file::LogFile;
use crate::platform_defaults::compose_weidu_log_path;

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
                "Target directory must already exist and be a folder: {target}"
            ));
        }
        if paths_point_to_same_dir(source_path, target_path) {
            return Err(format!(
                "{source_label} and target directory cannot be the same: {source}"
            ));
        }
    }

    validate_mod_folders_for_log(step1)?;

    Ok(())
}

fn resolve_log_file_path(step1: &Step1State) -> Option<String> {
    let candidate = if !step1.bgee_log_file.trim().is_empty() {
        step1.bgee_log_file.trim().to_string()
    } else if !step1.bg2ee_log_file.trim().is_empty() {
        step1.bg2ee_log_file.trim().to_string()
    } else {
        let folder = if step1.game_install == "EET" {
            step1.eet_bgee_log_folder.trim()
        } else if step1.game_install == "BG2EE" {
            step1.bg2ee_log_folder.trim()
        } else {
            step1.bgee_log_folder.trim()
        };
        compose_weidu_log_path(folder)
    };
    if candidate.is_empty() {
        None
    } else {
        Some(candidate)
    }
}

fn validate_mod_folders_for_log(step1: &Step1State) -> Result<(), String> {
    let mods_dir = step1.mods_folder.trim();
    if mods_dir.is_empty() {
        return Ok(());
    }
    let Some(log_path) = resolve_log_file_path(step1) else {
        return Ok(());
    };
    let log_path = Path::new(&log_path);
    if !log_path.is_file() {
        return Ok(());
    }
    let log_file = LogFile::from_path(log_path).map_err(|err| format!("{err}"))?;
    if log_file.is_empty() {
        return Ok(());
    }
    let plan = InstallPlan::from_log_file(&log_file);
    check_missing_mod_folders(Path::new(mods_dir), step1.depth, &plan.components)
        .map(|_| ())
        .map_err(|err| format!("{err}"))
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
        let path = Path::new(target);
        if !path.exists() || !path.is_dir() {
            return Err(format!("{label} does not exist after prep: {target}"));
        }
        let mut entries =
            fs::read_dir(path).map_err(|err| format!("{label} read failed: {err}"))?;
        if entries.next().is_some() {
            return Err(format!("{label} is not empty after prep: {target}"));
        }
    }

    Ok(())
}

pub fn validate_resume_paths(
    step1: &Step1State,
    resume_targets: &ResumeTargets,
) -> Result<(), String> {
    let mut checks: Vec<(&str, &str)> = Vec::new();
    if step1.game_install == "EET" {
        let bg1_dir = resume_targets
            .bg1_game_dir
            .as_deref()
            .unwrap_or_else(|| step1.eet_bgee_game_folder.trim());
        let bg2_dir = resume_targets
            .bg2_game_dir
            .as_deref()
            .unwrap_or_else(|| step1.eet_bg2ee_game_folder.trim());
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
        let path = Path::new(value);
        if !path.is_dir() {
            return Err(format!("{label} must be an existing folder: {value}"));
        }
        if !path.join("chitin.key").is_file() {
            return Err(format!("{label} missing chitin.key: {value}"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::app::state::Step1State;

    use super::validate_mod_folders_for_log;

    fn temp_dir() -> PathBuf {
        let mut base = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        base.push(format!("bio_validators_test_{ts}"));
        base
    }

    #[test]
    fn missing_mod_folder_returns_err_naming_component() {
        let root = temp_dir();
        let mods_dir = root.join("mods");
        let log_dir = root.join("game");
        fs::create_dir_all(&mods_dir).expect("create mods dir");
        fs::create_dir_all(&log_dir).expect("create game dir");
        let log_path = log_dir.join("weidu.log");
        fs::write(&log_path, b"~ISNF/ISNF.TP2~ #0 #0 // ISNF:1.0\n").expect("write log");

        let step1 = Step1State {
            mods_folder: mods_dir.to_string_lossy().into_owned(),
            bgee_log_file: log_path.to_string_lossy().into_owned(),
            ..Default::default()
        };

        let result = validate_mod_folders_for_log(&step1);
        assert!(result.is_err(), "missing mod folder must be an error");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("ISNF") || msg.contains("missing mod folder"),
            "error must name the missing component: {msg}"
        );
    }

    #[test]
    fn present_mod_folder_with_tp2_passes() {
        let root = temp_dir();
        let mods_dir = root.join("mods");
        let mod_folder = mods_dir.join("ISNF");
        let log_dir = root.join("game");
        fs::create_dir_all(&mod_folder).expect("create mod folder");
        fs::create_dir_all(&log_dir).expect("create game dir");
        fs::write(mod_folder.join("ISNF.TP2"), b"// stub").expect("write tp2");
        let log_path = log_dir.join("weidu.log");
        fs::write(&log_path, b"~ISNF/ISNF.TP2~ #0 #0 // ISNF:1.0\n").expect("write log");

        let step1 = Step1State {
            mods_folder: mods_dir.to_string_lossy().into_owned(),
            bgee_log_file: log_path.to_string_lossy().into_owned(),
            ..Default::default()
        };

        let result = validate_mod_folders_for_log(&step1);
        assert!(result.is_ok(), "present mod folder must pass: {result:?}");
    }

    #[test]
    fn no_log_file_skips_check() {
        let step1 = Step1State {
            mods_folder: "/nonexistent/mods".to_string(),
            ..Default::default()
        };
        let result = validate_mod_folders_for_log(&step1);
        assert!(result.is_ok(), "no log file path must skip the check");
    }

    #[test]
    fn empty_mods_folder_skips_check() {
        let step1 = Step1State::default();
        let result = validate_mod_folders_for_log(&step1);
        assert!(result.is_ok(), "empty mods_folder must skip the check");
    }
}
