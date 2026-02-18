// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use super::path_checks::{check_dir, check_file, check_game_dir};

pub(super) fn run_mode_checks(s: &Step1State, checked: &mut usize, errors: &mut Vec<String>) {
    match s.game_install.as_str() {
        "BG2EE" => {
            check_game_dir("BG2EE Game Folder", &s.bg2ee_game_folder, checked, errors);
            if s.have_weidu_logs {
                check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else {
                check_dir("BG2EE WeiDU Log Folder", &s.bg2ee_log_folder, checked, errors);
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                check_game_dir("Source BGEE Folder (-p)", &s.bgee_game_folder, checked, errors);
                check_dir("Pre-EET Directory", &s.eet_pre_dir, checked, errors);
            } else {
                check_game_dir("BGEE Game Folder", &s.eet_bgee_game_folder, checked, errors);
            }

            if s.new_eet_dir_enabled {
                check_game_dir("Source BG2EE Folder (-n)", &s.bg2ee_game_folder, checked, errors);
                check_dir("New EET Directory", &s.eet_new_dir, checked, errors);
            } else {
                check_game_dir("BG2EE Game Folder", &s.eet_bg2ee_game_folder, checked, errors);
            }

            if s.have_weidu_logs {
                check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
                check_file("BG2EE WeiDU Log File", &s.bg2ee_log_file, checked, errors);
            } else {
                check_dir("BGEE WeiDU Log Folder", &s.eet_bgee_log_folder, checked, errors);
                check_dir("BG2EE WeiDU Log Folder", &s.eet_bg2ee_log_folder, checked, errors);
            }
        }
        _ => {
            check_game_dir("BGEE Game Folder", &s.bgee_game_folder, checked, errors);
            if s.have_weidu_logs {
                check_file("BGEE WeiDU Log File", &s.bgee_log_file, checked, errors);
            } else {
                check_dir("BGEE WeiDU Log Folder", &s.bgee_log_folder, checked, errors);
            }
        }
    }
}
