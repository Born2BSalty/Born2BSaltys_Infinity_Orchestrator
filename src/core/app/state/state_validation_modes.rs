// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::app::state_validation_exec as exec;
use crate::app::state_validation_fs as fs_checks;

pub(crate) fn run_mode_checks(s: &Step1State, checked: &mut usize, errors: &mut Vec<String>) {
    match s.game_install.as_str() {
        "BG2EE" => {
            fs_checks::check_game_dir("BG2EE Game Folder", &s.bg2ee_game_folder, checked, errors);
            if s.installs_exactly_from_weidu_logs() {
                exec::check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else if !s.bootstraps_from_weidu_logs() && !s.imports_modlist() {
                fs_checks::check_dir(
                    "BG2EE WeiDU Log Folder",
                    &s.bg2ee_log_folder,
                    checked,
                    errors,
                );
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                fs_checks::check_game_dir(
                    "Source BGEE Folder (-p)",
                    &s.bgee_game_folder,
                    checked,
                    errors,
                );
                fs_checks::check_dir("Pre-EET Directory", &s.eet_pre_dir, checked, errors);
            } else {
                fs_checks::check_game_dir(
                    "BGEE Game Folder",
                    &s.eet_bgee_game_folder,
                    checked,
                    errors,
                );
            }

            if s.new_eet_dir_enabled {
                fs_checks::check_game_dir(
                    "Source BG2EE Folder (-n)",
                    &s.bg2ee_game_folder,
                    checked,
                    errors,
                );
                fs_checks::check_dir("New EET Directory", &s.eet_new_dir, checked, errors);
            } else {
                fs_checks::check_game_dir(
                    "BG2EE Game Folder",
                    &s.eet_bg2ee_game_folder,
                    checked,
                    errors,
                );
            }

            if s.installs_exactly_from_weidu_logs() {
                exec::check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
                exec::check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else if !s.bootstraps_from_weidu_logs() && !s.imports_modlist() {
                fs_checks::check_dir(
                    "BGEE WeiDU Log Folder",
                    &s.eet_bgee_log_folder,
                    checked,
                    errors,
                );
                fs_checks::check_dir(
                    "BG2EE WeiDU Log Folder",
                    &s.eet_bg2ee_log_folder,
                    checked,
                    errors,
                );
            }
        }
        _ => {
            fs_checks::check_game_dir("BGEE Game Folder", &s.bgee_game_folder, checked, errors);
            if s.installs_exactly_from_weidu_logs() {
                exec::check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
            } else if !s.bootstraps_from_weidu_logs() && !s.imports_modlist() {
                fs_checks::check_dir("BGEE WeiDU Log Folder", &s.bgee_log_folder, checked, errors);
            }
        }
    }
}
