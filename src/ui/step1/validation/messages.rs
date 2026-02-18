// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

pub(super) fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn step1_validation_messages(s: &Step1State) -> Vec<String> {
    let mut out = Vec::new();
    if !has_value(&s.mods_folder) {
        out.push("Mods Folder is required".to_string());
    }
    if !has_value(&s.weidu_binary) {
        out.push("WeiDU binary is required".to_string());
    }
    match s.game_install.as_str() {
        "BG2EE" => {
            if !has_value(&s.bg2ee_game_folder) {
                out.push("BG2EE Game Folder is required".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required".to_string());
                }
            } else if !has_value(&s.bg2ee_log_folder) {
                out.push("BG2EE WeiDU Log Folder is required".to_string());
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                if !has_value(&s.bgee_game_folder) {
                    out.push("Source BGEE Folder (-p) is required".to_string());
                }
                if !has_value(&s.eet_pre_dir) {
                    out.push("Pre-EET Directory is required when -p is enabled".to_string());
                }
            } else if !has_value(&s.eet_bgee_game_folder) {
                out.push("BGEE Game Folder is required for EET".to_string());
            }
            if s.new_eet_dir_enabled {
                if !has_value(&s.bg2ee_game_folder) {
                    out.push("Source BG2EE Folder (-n) is required".to_string());
                }
                if !has_value(&s.eet_new_dir) {
                    out.push("New EET Directory is required when -n is enabled".to_string());
                }
            } else if !has_value(&s.eet_bg2ee_game_folder) {
                out.push("BG2EE Game Folder is required for EET".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required for EET".to_string());
                }
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required for EET".to_string());
                }
            } else {
                if !has_value(&s.eet_bgee_log_folder) {
                    out.push("BGEE WeiDU Log Folder is required for EET".to_string());
                }
                if !has_value(&s.eet_bg2ee_log_folder) {
                    out.push("BG2EE WeiDU Log Folder is required for EET".to_string());
                }
            }
        }
        _ => {
            if !has_value(&s.bgee_game_folder) {
                out.push("BGEE Game Folder is required".to_string());
            }
            if s.have_weidu_logs {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required".to_string());
                }
            } else if !has_value(&s.bgee_log_folder) {
                out.push("BGEE WeiDU Log Folder is required".to_string());
            }
        }
    }
    out
}
