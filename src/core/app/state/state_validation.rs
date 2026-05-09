// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::state::Step1State;
#[path = "state_validation_paths.rs"]
mod paths;

pub fn is_step1_valid(s: &Step1State) -> bool {
    if !has_value(&s.mods_folder) || !has_value(&s.weidu_binary) {
        return false;
    }
    if s.uses_source_weidu_logs() && !s.download_archive {
        return false;
    }
    if s.download_archive
        && (!has_value(&s.mods_archive_folder)
            || !has_value(&s.mods_backup_folder)
            || !paths::same_windows_drive(&s.mods_folder, &s.mods_backup_folder))
    {
        return false;
    }

    match s.game_install.as_str() {
        "BG2EE" => {
            if !has_value(&s.bg2ee_game_folder) {
                return false;
            }
            if s.installs_exactly_from_weidu_logs() {
                has_value(&s.bg2ee_log_file)
            } else if s.bootstraps_from_weidu_logs() || s.imports_modlist() {
                true
            } else {
                has_value(&s.bg2ee_log_folder)
            }
        }
        "EET" => {
            if s.new_pre_eet_dir_enabled {
                if !has_value(&s.bgee_game_folder) || !has_value(&s.eet_pre_dir) {
                    return false;
                }
            } else if !has_value(&s.eet_bgee_game_folder) {
                return false;
            }

            if s.new_eet_dir_enabled {
                if !has_value(&s.bg2ee_game_folder) || !has_value(&s.eet_new_dir) {
                    return false;
                }
            } else if !has_value(&s.eet_bg2ee_game_folder) {
                return false;
            }
            if s.installs_exactly_from_weidu_logs() {
                has_value(&s.bgee_log_file) && has_value(&s.bg2ee_log_file)
            } else if s.bootstraps_from_weidu_logs() || s.imports_modlist() {
                true
            } else {
                has_value(&s.eet_bgee_log_folder) && has_value(&s.eet_bg2ee_log_folder)
            }
        }
        _ => {
            if !has_value(&s.bgee_game_folder) {
                return false;
            }
            if s.installs_exactly_from_weidu_logs() {
                has_value(&s.bgee_log_file)
            } else if s.bootstraps_from_weidu_logs() || s.imports_modlist() {
                true
            } else {
                has_value(&s.bgee_log_folder)
            }
        }
    }
}

pub fn run_path_check(s: &Step1State) -> (bool, String) {
    paths::run_path_check(s)
}

pub fn split_path_check_lines(msg: &str) -> Vec<String> {
    let details = msg.strip_prefix("Path check failed: ").unwrap_or(msg);
    details
        .split(" | ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn step1_mods_folder_has_tp2(s: &Step1State) -> bool {
    paths::step1_mods_folder_has_tp2(s)
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(super) fn step1_validation_messages(s: &Step1State) -> Vec<String> {
    let mut out = Vec::new();
    if !has_value(&s.mods_folder) {
        out.push("Mods Folder is required".to_string());
    }
    if !has_value(&s.weidu_binary) {
        out.push("WeiDU binary is required".to_string());
    }
    if s.uses_source_weidu_logs() && !s.download_archive {
        out.push(
            "Download Missing Mods and Keep Archives is required for WeiDU log install modes"
                .to_string(),
        );
    }
    if s.download_archive && !has_value(&s.mods_archive_folder) {
        out.push("Mods Archive is required when Download Archive is enabled".to_string());
    }
    if s.download_archive && !has_value(&s.mods_backup_folder) {
        out.push("Backup is required when Download Archive is enabled".to_string());
    }
    match s.game_install.as_str() {
        "BG2EE" => {
            if !has_value(&s.bg2ee_game_folder) {
                out.push("BG2EE Game Folder is required".to_string());
            }
            if s.installs_exactly_from_weidu_logs() {
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required".to_string());
                }
            } else if !s.imports_modlist() && !has_value(&s.bg2ee_log_folder) {
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
            if s.installs_exactly_from_weidu_logs() {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required for EET".to_string());
                }
                if !has_value(&s.bg2ee_log_file) {
                    out.push("BG2EE WeiDU Log File is required for EET".to_string());
                }
            } else if !s.imports_modlist() {
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
            if s.installs_exactly_from_weidu_logs() {
                if !has_value(&s.bgee_log_file) {
                    out.push("BGEE WeiDU Log File is required".to_string());
                }
            } else if !s.imports_modlist() && !has_value(&s.bgee_log_folder) {
                out.push("BGEE WeiDU Log Folder is required".to_string());
            }
        }
    }
    out
}
