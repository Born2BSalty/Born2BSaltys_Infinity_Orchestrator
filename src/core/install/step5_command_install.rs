// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::step5_command_common_args::{append_common_args, installer_program};
use super::step5_command_config::InstallCommandConfig;
use super::step5_command_log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

#[must_use]
pub(crate) fn build_install_invocation(config: &InstallCommandConfig) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let installer = installer_program(config);
    if config.game_install == "EET" {
        let bg1_source =
            if config.directories.pre_eet_override && !config.bgee_game_folder.trim().is_empty() {
                config.bgee_game_folder.trim()
            } else {
                config.eet_bgee_game_folder.trim()
            };
        let bg2_source =
            if config.directories.eet_override && !config.bg2ee_game_folder.trim().is_empty() {
                config.bg2ee_game_folder.trim()
            } else {
                config.eet_bg2ee_game_folder.trim()
            };
        args.push("eet".to_string());
        args.push("--bg1-game-directory".to_string());
        args.push(bg1_source.to_string());
        args.push("--bg1-log-file".to_string());
        args.push(resolve_bgee_log_file(config));
        args.push("--bg2-game-directory".to_string());
        args.push(bg2_source.to_string());
        args.push("--bg2-log-file".to_string());
        args.push(resolve_bg2_log_file(config));
        if config.directories.pre_eet_override && !config.eet_pre_dir.trim().is_empty() {
            args.push("--new-pre-eet-dir".to_string());
            args.push(config.eet_pre_dir.trim().to_string());
        }
        if config.directories.eet_override && !config.eet_new_dir.trim().is_empty() {
            args.push("--new-eet-dir".to_string());
            args.push(config.eet_new_dir.trim().to_string());
        }
    } else {
        args.push("normal".to_string());
        args.push("--game-directory".to_string());
        let game_dir = if config.game_install == "BG2EE" {
            &config.bg2ee_game_folder
        } else {
            &config.bgee_game_folder
        };
        args.push(game_dir.clone());
        args.push("--log-file".to_string());
        let log_file = if config.game_install == "BG2EE" {
            resolve_bg2_log_file(config)
        } else {
            resolve_bgee_log_file(config)
        };
        args.push(log_file);
        if config.directories.generate_output && !config.generate_directory.trim().is_empty() {
            args.push("--generate-directory".to_string());
            args.push(config.generate_directory.trim().to_string());
        }
    }
    append_common_args(config, &mut args);
    (installer, args)
}
