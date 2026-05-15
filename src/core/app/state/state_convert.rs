// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod step1_settings_to_state {
    use crate::app::state::Step1State;
    use crate::platform_defaults::{
        default_mod_installer_binary, default_weidu_binary, resolve_mod_installer_binary,
        resolve_weidu_binary,
    };
    use crate::settings::model::Step1Settings;

    impl From<Step1Settings> for Step1State {
        fn from(value: Step1Settings) -> Self {
            let install_mode = settings_install_mode(&value);
            let have_weidu_logs = matches!(
                install_mode.as_str(),
                Self::INSTALL_MODE_EXACT_WEIDU_LOGS | Self::INSTALL_MODE_WEIDU_LOGS_REVIEW_EDIT
            );
            Self {
                game_install: value.game_install,
                install_mode,
                have_weidu_logs,
                rust_log_debug: value.rust_log_debug,
                rust_log_trace: value.rust_log_trace,
                custom_scan_depth: value.custom_scan_depth,
                timeout_per_mod_enabled: value.timeout_per_mod_enabled,
                auto_answer_initial_delay_enabled: value.auto_answer_initial_delay_enabled,
                auto_answer_post_send_delay_enabled: value.auto_answer_post_send_delay_enabled,
                prompt_required_sound_enabled: value.prompt_required_sound_enabled,
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
                mod_installer_binary: settings_mod_installer_binary(&value.mod_installer_binary),
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
                weidu_binary: settings_weidu_binary(&value.weidu_binary),
                language: settings_language(value.language),
                depth: default_zero_usize(value.depth, 5),
                skip_installed: value.skip_installed,
                abort_on_warnings: value.abort_on_warnings,
                timeout: default_zero_usize(value.timeout, 3600),
                auto_answer_initial_delay_ms: default_zero_usize(
                    value.auto_answer_initial_delay_ms,
                    2000,
                ),
                auto_answer_post_send_delay_ms: default_zero_usize(
                    value.auto_answer_post_send_delay_ms,
                    5000,
                ),
                weidu_log_mode: settings_weidu_log_mode(value.weidu_log_mode),
                strict_matching: value.strict_matching,
                download: value.download,
                download_archive: value.download_archive,
                mods_archive_folder: value.mods_archive_folder,
                mods_backup_folder: value.mods_backup_folder,
                overwrite: value.overwrite,
                check_last_installed: value.check_last_installed,
                tick: default_zero_u64(value.tick, 500),
                lookback: default_zero_usize(value.lookback, 10),
                casefold: value.casefold,
                backup_targets_before_eet_copy: value.backup_targets_before_eet_copy,
            }
        }
    }

    fn settings_install_mode(value: &Step1Settings) -> String {
        if value.install_mode.trim().is_empty() {
            Step1State::derive_install_mode_from_legacy(
                value.have_weidu_logs,
                value.download_archive,
            )
        } else {
            Step1State::normalize_install_mode(&value.install_mode).to_string()
        }
    }

    fn settings_mod_installer_binary(value: &str) -> String {
        if value.trim().is_empty() {
            default_mod_installer_binary()
        } else {
            resolve_mod_installer_binary(value)
        }
    }

    fn settings_weidu_binary(value: &str) -> String {
        if value.trim().is_empty() {
            default_weidu_binary()
        } else {
            resolve_weidu_binary(value)
        }
    }

    fn settings_language(value: String) -> String {
        if value.is_empty() {
            "en_US".to_string()
        } else {
            value
        }
    }

    fn settings_weidu_log_mode(value: String) -> String {
        if value.is_empty() {
            "autolog,logapp,log-extern".to_string()
        } else {
            value
        }
    }

    const fn default_zero_u64(value: u64, fallback: u64) -> u64 {
        if value == 0 { fallback } else { value }
    }

    const fn default_zero_usize(value: usize, fallback: usize) -> usize {
        if value == 0 { fallback } else { value }
    }
}
mod step1_state_to_settings {
    use crate::app::state::Step1State;
    use crate::settings::model::Step1Settings;

    impl From<Step1State> for Step1Settings {
        fn from(value: Step1State) -> Self {
            let have_weidu_logs = value.uses_source_weidu_logs();
            Self {
                game_install: value.game_install,
                install_mode: value.install_mode,
                have_weidu_logs,
                rust_log_debug: value.rust_log_debug,
                rust_log_trace: value.rust_log_trace,
                custom_scan_depth: value.custom_scan_depth,
                timeout_per_mod_enabled: value.timeout_per_mod_enabled,
                auto_answer_initial_delay_enabled: value.auto_answer_initial_delay_enabled,
                auto_answer_post_send_delay_enabled: value.auto_answer_post_send_delay_enabled,
                prompt_required_sound_enabled: value.prompt_required_sound_enabled,
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
                mod_installer_binary: value.mod_installer_binary,
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
                weidu_binary: value.weidu_binary,
                language: value.language,
                depth: value.depth,
                skip_installed: value.skip_installed,
                abort_on_warnings: value.abort_on_warnings,
                timeout: value.timeout,
                auto_answer_initial_delay_ms: value.auto_answer_initial_delay_ms,
                auto_answer_post_send_delay_ms: value.auto_answer_post_send_delay_ms,
                weidu_log_mode: value.weidu_log_mode,
                strict_matching: value.strict_matching,
                download: value.download,
                download_archive: value.download_archive,
                mods_archive_folder: value.mods_archive_folder,
                mods_backup_folder: value.mods_backup_folder,
                overwrite: value.overwrite,
                check_last_installed: value.check_last_installed,
                tick: value.tick,
                lookback: value.lookback,
                casefold: value.casefold,
                backup_targets_before_eet_copy: value.backup_targets_before_eet_copy,
            }
        }
    }
}
