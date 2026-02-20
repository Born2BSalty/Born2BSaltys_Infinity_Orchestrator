// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::platform_defaults::{
    default_mod_installer_binary, default_weidu_binary, resolve_mod_installer_binary,
    resolve_weidu_binary,
};
use crate::settings::model::Step1Settings;
use crate::ui::state::Step1State;

impl From<Step1Settings> for Step1State {
    fn from(value: Step1Settings) -> Self {
        Self {
            game_install: value.game_install,
            have_weidu_logs: value.have_weidu_logs,
            rust_log_debug: value.rust_log_debug,
            rust_log_trace: value.rust_log_trace,
            custom_scan_depth: value.custom_scan_depth,
            timeout_per_mod_enabled: value.timeout_per_mod_enabled,
            auto_answer_initial_delay_enabled: value.auto_answer_initial_delay_enabled,
            auto_answer_post_send_delay_enabled: value.auto_answer_post_send_delay_enabled,
            lookback_enabled: value.lookback_enabled,
            bio_full_debug: value.bio_full_debug,
            tick_dev_enabled: value.tick_dev_enabled,
            log_raw_output_dev: value.log_raw_output_dev,
            weidu_log_mode_enabled: value.weidu_log_mode_enabled,
            new_pre_eet_dir_enabled: value.new_pre_eet_dir_enabled,
            new_eet_dir_enabled: value.new_eet_dir_enabled,
            generate_directory_enabled: value.generate_directory_enabled,
            prepare_target_dirs_before_install: value.prepare_target_dirs_before_install,
            weidu_log_autolog: value.weidu_log_autolog,
            weidu_log_logapp: value.weidu_log_logapp,
            weidu_log_logextern: value.weidu_log_logextern,
            weidu_log_log_component: value.weidu_log_log_component,
            weidu_log_folder: value.weidu_log_folder,
            mod_installer_binary: if value.mod_installer_binary.trim().is_empty() {
                default_mod_installer_binary()
            } else {
                resolve_mod_installer_binary(&value.mod_installer_binary)
            },
            bgee_game_folder: value.bgee_game_folder,
            bgee_log_folder: value.bgee_log_folder,
            bgee_log_file: value.bgee_log_file,
            bg2ee_game_folder: value.bg2ee_game_folder,
            bg2ee_log_folder: value.bg2ee_log_folder,
            bg2ee_log_file: value.bg2ee_log_file,
            eet_bgee_game_folder: value.eet_bgee_game_folder,
            eet_bgee_log_folder: value.eet_bgee_log_folder,
            eet_bg2ee_game_folder: value.eet_bg2ee_game_folder,
            eet_bg2ee_log_folder: value.eet_bg2ee_log_folder,
            eet_pre_dir: value.eet_pre_dir,
            eet_new_dir: value.eet_new_dir,
            game: value.game,
            log_file: value.log_file,
            generate_directory: value.generate_directory,
            mods_folder: value.mods_folder,
            weidu_binary: if value.weidu_binary.trim().is_empty() {
                default_weidu_binary()
            } else {
                resolve_weidu_binary(&value.weidu_binary)
            },
            language: if value.language.is_empty() {
                "en_US".to_string()
            } else {
                value.language
            },
            depth: if value.depth == 0 { 5 } else { value.depth },
            skip_installed: value.skip_installed,
            abort_on_warnings: value.abort_on_warnings,
            timeout: if value.timeout == 0 { 3600 } else { value.timeout },
            auto_answer_initial_delay_ms: if value.auto_answer_initial_delay_ms == 0 {
                2000
            } else {
                value.auto_answer_initial_delay_ms
            },
            auto_answer_post_send_delay_ms: if value.auto_answer_post_send_delay_ms == 0 {
                5000
            } else {
                value.auto_answer_post_send_delay_ms
            },
            weidu_log_mode: if value.weidu_log_mode.is_empty() {
                "autolog,logapp,log-extern".to_string()
            } else {
                value.weidu_log_mode
            },
            strict_matching: value.strict_matching,
            download: value.download,
            overwrite: value.overwrite,
            check_last_installed: value.check_last_installed,
            tick: if value.tick == 0 { 500 } else { value.tick },
            lookback: if value.lookback == 0 { 10 } else { value.lookback },
            casefold: value.casefold,
            backup_targets_before_eet_copy: value.backup_targets_before_eet_copy,
        }
    }
}
