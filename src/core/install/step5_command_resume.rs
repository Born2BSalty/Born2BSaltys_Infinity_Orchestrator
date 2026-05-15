// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::ResumeTargets;

use super::step5_command_common_args::{append_common_args, installer_program};
use super::step5_command_config::InstallCommandConfig;
use super::step5_command_log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

pub(crate) fn capture_resume_targets(config: &InstallCommandConfig) -> ResumeTargets {
    if config.game_install == "EET" {
        ResumeTargets {
            bg1_game_dir: Some(
                if config.new_pre_eet_dir_enabled && !config.eet_pre_dir.trim().is_empty() {
                    config.eet_pre_dir.trim().to_string()
                } else {
                    config.eet_bgee_game_folder.trim().to_string()
                },
            ),
            bg2_game_dir: Some(
                if config.new_eet_dir_enabled && !config.eet_new_dir.trim().is_empty() {
                    config.eet_new_dir.trim().to_string()
                } else {
                    config.eet_bg2ee_game_folder.trim().to_string()
                },
            ),
            game_dir: None,
        }
    } else {
        ResumeTargets {
            bg1_game_dir: None,
            bg2_game_dir: None,
            game_dir: Some(
                if config.generate_directory_enabled && !config.generate_directory.trim().is_empty()
                {
                    config.generate_directory.trim().to_string()
                } else if config.game_install == "BG2EE" {
                    config.bg2ee_game_folder.trim().to_string()
                } else {
                    config.bgee_game_folder.trim().to_string()
                },
            ),
        }
    }
}

pub(crate) fn build_resume_invocation(
    config: &InstallCommandConfig,
    resume_targets: &ResumeTargets,
) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let installer = installer_program(config);
    if config.game_install == "EET" {
        let bg1_dir = resume_targets
            .bg1_game_dir
            .as_deref()
            .unwrap_or_else(|| config.eet_bgee_game_folder.trim());
        let bg2_dir = resume_targets
            .bg2_game_dir
            .as_deref()
            .unwrap_or_else(|| config.eet_bg2ee_game_folder.trim());
        args.push("eet".to_string());
        args.push("--bg1-game-directory".to_string());
        args.push(bg1_dir.to_string());
        args.push("--bg1-log-file".to_string());
        args.push(resolve_bgee_log_file(config));
        args.push("--bg2-game-directory".to_string());
        args.push(bg2_dir.to_string());
        args.push("--bg2-log-file".to_string());
        args.push(resolve_bg2_log_file(config));
    } else {
        args.push("normal".to_string());
        args.push("--game-directory".to_string());
        let game_dir = resume_targets.game_dir.as_deref().unwrap_or_else(|| {
            if config.game_install == "BG2EE" {
                config.bg2ee_game_folder.trim()
            } else {
                config.bgee_game_folder.trim()
            }
        });
        args.push(game_dir.to_string());
        args.push("--log-file".to_string());
        let log_file = if config.game_install == "BG2EE" {
            resolve_bg2_log_file(config)
        } else {
            resolve_bgee_log_file(config)
        };
        args.push(log_file);
    }
    append_common_args(config, &mut args);
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
