// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{ResumeTargets, Step1State};

use super::common_args::{append_common_args, installer_program};
use super::log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

pub(crate) fn capture_resume_targets(step1: &Step1State) -> ResumeTargets {
    if step1.game_install == "EET" {
        ResumeTargets {
            bg1_game_dir: Some(
                if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                    step1.eet_pre_dir.trim().to_string()
                } else {
                    step1.eet_bgee_game_folder.trim().to_string()
                },
            ),
            bg2_game_dir: Some(
                if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
                    step1.eet_new_dir.trim().to_string()
                } else {
                    step1.eet_bg2ee_game_folder.trim().to_string()
                },
            ),
            game_dir: None,
        }
    } else {
        ResumeTargets {
            bg1_game_dir: None,
            bg2_game_dir: None,
            game_dir: Some(
                if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
                    step1.generate_directory.trim().to_string()
                } else if step1.game_install == "BG2EE" {
                    step1.bg2ee_game_folder.trim().to_string()
                } else {
                    step1.bgee_game_folder.trim().to_string()
                },
            ),
        }
    }
}

pub(crate) fn build_resume_invocation(
    step1: &Step1State,
    resume_targets: &ResumeTargets,
) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let installer = installer_program(step1);
    if step1.game_install == "EET" {
        let bg1_dir = resume_targets
            .bg1_game_dir
            .as_deref()
            .unwrap_or(step1.eet_bgee_game_folder.trim());
        let bg2_dir = resume_targets
            .bg2_game_dir
            .as_deref()
            .unwrap_or(step1.eet_bg2ee_game_folder.trim());
        args.push("eet".to_string());
        args.push("--bg1-game-directory".to_string());
        args.push(bg1_dir.to_string());
        args.push("--bg1-log-file".to_string());
        args.push(resolve_bgee_log_file(step1));
        args.push("--bg2-game-directory".to_string());
        args.push(bg2_dir.to_string());
        args.push("--bg2-log-file".to_string());
        args.push(resolve_bg2_log_file(step1));
    } else {
        args.push("normal".to_string());
        args.push("--game-directory".to_string());
        let game_dir = resume_targets.game_dir.as_deref().unwrap_or_else(|| {
            if step1.game_install == "BG2EE" {
                step1.bg2ee_game_folder.trim()
            } else {
                step1.bgee_game_folder.trim()
            }
        });
        args.push(game_dir.to_string());
        args.push("--log-file".to_string());
        let log_file = if step1.game_install == "BG2EE" {
            resolve_bg2_log_file(step1)
        } else {
            resolve_bgee_log_file(step1)
        };
        args.push(log_file);
    }
    append_common_args(step1, &mut args);
    force_skip_installed_on(&mut args);
    (installer, args)
}

fn force_skip_installed_on(args: &mut [String]) {
    if let Some(index) = args.iter().position(|arg| arg == "--skip-installed")
        && let Some(value) = args.get_mut(index + 1)
    {
        *value = "true".to_string();
    }
}
