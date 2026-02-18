// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use crate::ui::step5::command::invocation::common_args::{append_common_args, installer_program};
use crate::ui::step5::command::invocation::log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

pub fn build_install_invocation(step1: &Step1State) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let installer = installer_program(step1);
    if step1.game_install == "EET" {
        let bg1_source = if step1.new_pre_eet_dir_enabled && !step1.bgee_game_folder.trim().is_empty()
        {
            step1.bgee_game_folder.trim()
        } else {
            step1.eet_bgee_game_folder.trim()
        };
        let bg2_source = if step1.new_eet_dir_enabled && !step1.bg2ee_game_folder.trim().is_empty() {
            step1.bg2ee_game_folder.trim()
        } else {
            step1.eet_bg2ee_game_folder.trim()
        };
        args.push("eet".to_string());
        args.push("--bg1-game-directory".to_string());
        args.push(bg1_source.to_string());
        args.push("--bg1-log-file".to_string());
        args.push(resolve_bgee_log_file(step1));
        args.push("--bg2-game-directory".to_string());
        args.push(bg2_source.to_string());
        args.push("--bg2-log-file".to_string());
        args.push(resolve_bg2_log_file(step1));
        if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
            args.push("--new-pre-eet-dir".to_string());
            args.push(step1.eet_pre_dir.trim().to_string());
        }
        if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
            args.push("--new-eet-dir".to_string());
            args.push(step1.eet_new_dir.trim().to_string());
        }
    } else {
        args.push("normal".to_string());
        args.push("--game-directory".to_string());
        let game_dir = if step1.game_install == "BG2EE" {
            &step1.bg2ee_game_folder
        } else {
            &step1.bgee_game_folder
        };
        args.push(game_dir.to_string());
        args.push("--log-file".to_string());
        let log_file = if step1.game_install == "BG2EE" {
            resolve_bg2_log_file(step1)
        } else {
            resolve_bgee_log_file(step1)
        };
        args.push(log_file);
        if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
            args.push("--generate-directory".to_string());
            args.push(step1.generate_directory.trim().to_string());
        }
    }
    append_common_args(step1, &mut args);
    (installer, args)
}
