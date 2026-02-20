// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use crate::ui::step5::command::invocation::common_args::{append_common_args, installer_program};
use crate::ui::step5::command::invocation::log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};

pub fn build_resume_invocation(step1: &Step1State) -> (String, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let installer = installer_program(step1);
    if step1.game_install == "EET" {
        let bg1_dir = if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
            step1.eet_pre_dir.trim()
        } else {
            step1.eet_bgee_game_folder.trim()
        };
        let bg2_dir = if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
            step1.eet_new_dir.trim()
        } else {
            step1.eet_bg2ee_game_folder.trim()
        };
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
        let game_dir =
            if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
                step1.generate_directory.trim()
            } else if step1.game_install == "BG2EE" {
                step1.bg2ee_game_folder.trim()
            } else {
                step1.bgee_game_folder.trim()
            };
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
    if step1.new_pre_eet_dir_enabled || step1.new_eet_dir_enabled || step1.generate_directory_enabled
    {
        force_skip_installed_on(&mut args);
    }
    (installer, args)
}

fn force_skip_installed_on(args: &mut [String]) {
    if let Some(idx) = args.iter().position(|a| a == "--skip-installed")
        && let Some(value) = args.get_mut(idx + 1)
    {
        *value = "true".to_string();
    }
}
