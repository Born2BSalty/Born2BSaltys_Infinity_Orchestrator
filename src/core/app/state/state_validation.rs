// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::state::Step1State;
#[path = "state_validation_paths.rs"]
mod paths;

#[must_use]
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
        "IWDEE" => {
            if !has_value(&s.iwdee_game_folder) {
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

#[must_use]
pub fn settings_paths_ok(s: &Step1State) -> bool {
    if !has_value(&s.weidu_binary) {
        return false;
    }
    if s.uses_source_weidu_logs() && !s.download_archive {
        return false;
    }
    if s.download_archive
        && (!has_value(&s.mods_archive_folder) || !has_value(&s.mods_backup_folder))
    {
        return false;
    }
    if s.download_archive
        && has_value(&s.mods_folder)
        && !paths::same_windows_drive(&s.mods_folder, &s.mods_backup_folder)
    {
        return false;
    }
    has_value(&s.bgee_game_folder)
        || has_value(&s.bg2ee_game_folder)
        || has_value(&s.iwdee_game_folder)
}

#[must_use]
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

#[must_use]
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
        "IWDEE" => {
            if !has_value(&s.iwdee_game_folder) {
                out.push("IWDEE Game Folder is required".to_string());
            }
            if s.installs_exactly_from_weidu_logs() {
                if !has_value(&s.bgee_log_file) {
                    out.push("IWDEE WeiDU Log File is required".to_string());
                }
            } else if !s.imports_modlist() && !has_value(&s.bgee_log_folder) {
                out.push("IWDEE WeiDU Log Folder is required".to_string());
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

#[cfg(test)]
mod tests {
    use super::{is_step1_valid, settings_paths_ok, step1_validation_messages};
    use crate::app::state::Step1State;

    fn base() -> Step1State {
        Step1State {
            mods_folder: "/mods".to_string(),
            weidu_binary: "/weidu".to_string(),
            install_mode: Step1State::INSTALL_MODE_IMPORT_MODLIST.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn step1_valid_requires_the_lists_own_game_folder() {
        for game in ["BGEE", "BG2EE", "IWDEE"] {
            let mut ok_state = base();
            ok_state.game_install = game.to_string();
            match game {
                "BGEE" => ok_state.bgee_game_folder = "/games/bgee".to_string(),
                "BG2EE" => ok_state.bg2ee_game_folder = "/games/bg2ee".to_string(),
                _ => ok_state.iwdee_game_folder = "/games/iwdee".to_string(),
            }
            assert!(is_step1_valid(&ok_state), "{game}: own folder set");

            let mut hole_state = base();
            hole_state.game_install = game.to_string();
            if game != "BGEE" {
                hole_state.bgee_game_folder = "/games/bgee".to_string();
            }
            if game != "BG2EE" {
                hole_state.bg2ee_game_folder = "/games/bg2ee".to_string();
            }
            if game != "IWDEE" {
                hole_state.iwdee_game_folder = "/games/iwdee".to_string();
            }
            assert!(
                !is_step1_valid(&hole_state),
                "{game}: only other folders set must stay invalid"
            );
        }
    }

    #[test]
    fn iwdee_validation_message_names_iwdee() {
        let s = Step1State {
            mods_folder: "/mods".to_string(),
            weidu_binary: "/weidu".to_string(),
            game_install: "IWDEE".to_string(),
            ..Default::default()
        };
        let messages = step1_validation_messages(&s);
        assert!(messages.contains(&"IWDEE Game Folder is required".to_string()));
        assert!(messages.contains(&"IWDEE WeiDU Log Folder is required".to_string()));
    }

    #[test]
    fn settings_paths_ok_ignores_last_opened_game() {
        let s = Step1State {
            mods_folder: "/mods".to_string(),
            weidu_binary: "/weidu".to_string(),
            game_install: "BGEE".to_string(),
            iwdee_game_folder: "/games/iwdee".to_string(),
            ..Default::default()
        };
        assert!(settings_paths_ok(&s));

        let mut none_set = s.clone();
        none_set.iwdee_game_folder = String::new();
        assert!(!settings_paths_ok(&none_set));

        let mut no_list_opened_yet = s.clone();
        no_list_opened_yet.mods_folder = String::new();
        assert!(
            settings_paths_ok(&no_list_opened_yet),
            "the per-install mods folder is filled by opening a list, never by Settings"
        );

        let mut tools_missing = s;
        tools_missing.weidu_binary = String::new();
        assert!(!settings_paths_ok(&tools_missing));
    }
}
