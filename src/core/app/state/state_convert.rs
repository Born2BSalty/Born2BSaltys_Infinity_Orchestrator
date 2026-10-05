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
            let mod_installer_binary =
                resolve_mod_installer_binary_setting(&value.mod_installer_binary);
            let weidu_binary = resolve_weidu_binary_setting(&value.weidu_binary);
            let language = string_or_default(value.language, "en_US");
            let depth = usize_or_default(value.depth, 5);
            let timeout = usize_or_default(value.timeout, 3600);
            let auto_answer_initial_delay_ms =
                usize_or_default(value.auto_answer_initial_delay_ms, 2000);
            let auto_answer_post_send_delay_ms =
                usize_or_default(value.auto_answer_post_send_delay_ms, 5000);
            let tick = u64_or_default(value.tick, 500);
            let lookback = usize_or_default(value.lookback, 10);
            Self {
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
                mod_installer_binary,
                bgee_game_folder: value.bgee_game_folder,
                bg2ee_game_folder: value.bg2ee_game_folder,
                iwdee_game_folder: value.iwdee_game_folder,
                global_mods_folder: value.global_mods_folder,
                weidu_binary,
                language,
                depth,
                skip_installed: value.skip_installed,
                abort_on_warnings: value.abort_on_warnings,
                timeout,
                auto_answer_initial_delay_ms,
                auto_answer_post_send_delay_ms,
                strict_matching: value.strict_matching,
                download: value.download,
                download_archive: value.download_archive,
                mods_archive_folder: value.mods_archive_folder,
                mods_backup_folder: value.mods_backup_folder,
                overwrite: value.overwrite,
                check_last_installed: value.check_last_installed,
                tick,
                lookback,
                casefold: value.casefold,
                ..Self::default()
            }
        }
    }

    fn resolve_mod_installer_binary_setting(value: &str) -> String {
        if value.trim().is_empty() {
            default_mod_installer_binary()
        } else {
            resolve_mod_installer_binary(value)
        }
    }

    fn resolve_weidu_binary_setting(value: &str) -> String {
        if value.trim().is_empty() {
            default_weidu_binary()
        } else {
            resolve_weidu_binary(value)
        }
    }

    fn string_or_default(value: String, fallback: &str) -> String {
        if value.is_empty() {
            fallback.to_string()
        } else {
            value
        }
    }

    const fn usize_or_default(value: usize, fallback: usize) -> usize {
        if value == 0 { fallback } else { value }
    }

    const fn u64_or_default(value: u64, fallback: u64) -> u64 {
        if value == 0 { fallback } else { value }
    }
}
#[cfg(test)]
mod tests {
    use crate::app::state::Step1State;
    use crate::settings::model::Step1Settings;

    fn per_list_fields_of(state: &Step1State) -> Step1State {
        Step1State {
            game_install: state.game_install.clone(),
            install_mode: state.install_mode.clone(),
            have_weidu_logs: state.have_weidu_logs,
            weidu_log_mode_enabled: state.weidu_log_mode_enabled,
            new_pre_eet_dir_enabled: state.new_pre_eet_dir_enabled,
            new_eet_dir_enabled: state.new_eet_dir_enabled,
            generate_directory_enabled: state.generate_directory_enabled,
            prepare_target_dirs_before_install: state.prepare_target_dirs_before_install,
            backup_targets_before_eet_copy: state.backup_targets_before_eet_copy,
            weidu_log_autolog: state.weidu_log_autolog,
            weidu_log_logapp: state.weidu_log_logapp,
            weidu_log_logextern: state.weidu_log_logextern,
            weidu_log_log_component: state.weidu_log_log_component,
            weidu_log_folder: state.weidu_log_folder.clone(),
            weidu_log_mode: state.weidu_log_mode.clone(),
            bgee_log_folder: state.bgee_log_folder.clone(),
            bgee_log_file: state.bgee_log_file.clone(),
            bg2ee_log_folder: state.bg2ee_log_folder.clone(),
            bg2ee_log_file: state.bg2ee_log_file.clone(),
            eet_bgee_log_folder: state.eet_bgee_log_folder.clone(),
            eet_bg2ee_log_folder: state.eet_bg2ee_log_folder.clone(),
            eet_pre_dir: state.eet_pre_dir.clone(),
            eet_new_dir: state.eet_new_dir.clone(),
            game: state.game.clone(),
            log_file: state.log_file.clone(),
            generate_directory: state.generate_directory.clone(),
            mods_folder: state.mods_folder.clone(),
            eet_bgee_game_folder: state.eet_bgee_game_folder.clone(),
            eet_bg2ee_game_folder: state.eet_bg2ee_game_folder.clone(),
            ..Step1State::default()
        }
    }

    #[test]
    fn global_mods_folder_survives_settings_to_state() {
        let s = Step1Settings {
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        let state = Step1State::from(s);
        assert_eq!(state.global_mods_folder, r"C:\global\mods");
        assert_eq!(state.mods_folder, "");
    }

    #[test]
    fn settings_to_state_fills_per_list_fields_from_defaults() {
        let s = Step1Settings {
            bgee_game_folder: r"D:\src\BGEE".to_string(),
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        let state = Step1State::from(s);
        assert_eq!(
            per_list_fields_of(&state),
            per_list_fields_of(&Step1State::default())
        );
        assert_eq!(
            state.install_mode,
            Step1State::INSTALL_MODE_BUILD_FROM_SCANNED_MODS
        );
        assert!(!state.have_weidu_logs);
        assert_eq!(state.bgee_game_folder, r"D:\src\BGEE");
    }

    #[test]
    fn state_to_settings_copies_globals_only() {
        let state = Step1State {
            mods_folder: r"C:\list\mods".to_string(),
            generate_directory: r"C:\list\clone".to_string(),
            prepare_target_dirs_before_install: true,
            eet_bgee_game_folder: r"C:\old\eet".to_string(),
            global_mods_folder: r"C:\global\mods".to_string(),
            bgee_game_folder: r"D:\src\BGEE".to_string(),
            ..Step1State::default()
        };
        let s = Step1Settings::from(state);
        assert_eq!(s.global_mods_folder, r"C:\global\mods");
        assert_eq!(s.bgee_game_folder, r"D:\src\BGEE");
        let json = serde_json::to_string(&s).expect("serialize");
        for key in [
            "\"mods_folder\"",
            "\"generate_directory\"",
            "\"prepare_target_dirs_before_install\"",
            "\"eet_bgee_game_folder\"",
        ] {
            assert!(!json.contains(key), "{key} must not be saved: {json}");
        }
    }

    #[test]
    fn global_mods_folder_round_trips_settings_state_settings() {
        let original = Step1Settings {
            global_mods_folder: r"C:\global\mods".to_string(),
            ..Step1Settings::default()
        };
        let round_tripped = Step1Settings::from(Step1State::from(original.clone()));
        assert_eq!(round_tripped, original);
    }
}

mod step1_state_to_settings {
    use crate::app::state::Step1State;
    use crate::settings::model::Step1Settings;

    impl From<Step1State> for Step1Settings {
        fn from(value: Step1State) -> Self {
            Self {
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
                mod_installer_binary: value.mod_installer_binary,
                bgee_game_folder: value.bgee_game_folder,
                bg2ee_game_folder: value.bg2ee_game_folder,
                iwdee_game_folder: value.iwdee_game_folder,
                global_mods_folder: value.global_mods_folder,
                weidu_binary: value.weidu_binary,
                language: value.language,
                depth: value.depth,
                skip_installed: value.skip_installed,
                abort_on_warnings: value.abort_on_warnings,
                timeout: value.timeout,
                auto_answer_initial_delay_ms: value.auto_answer_initial_delay_ms,
                auto_answer_post_send_delay_ms: value.auto_answer_post_send_delay_ms,
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
            }
        }
    }
}
