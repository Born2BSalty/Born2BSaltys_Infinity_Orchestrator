// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::install::step5_command_config::{
    DirectoryOptions, InstallCommandConfig, LogOptions, SafetyOptions, ScanOptions, TimingOptions,
    TransferOptions,
};

#[must_use]
pub(crate) fn build_install_command_config(step1: &Step1State) -> InstallCommandConfig {
    InstallCommandConfig {
        game_install: step1.game_install.clone(),
        logs: LogOptions {
            exact_weidu_logs: step1.installs_exactly_from_weidu_logs(),
            include_mode: step1.weidu_log_mode_enabled,
        },
        directories: DirectoryOptions {
            pre_eet_override: step1.new_pre_eet_dir_enabled,
            eet_override: step1.new_eet_dir_enabled,
            generate_output: step1.generate_directory_enabled,
        },
        mod_installer_binary: step1.mod_installer_binary.clone(),
        bgee_game_folder: step1.bgee_game_folder.clone(),
        bgee_log_folder: step1.bgee_log_folder.clone(),
        bgee_log_file: step1.bgee_log_file.clone(),
        bg2ee_game_folder: step1.bg2ee_game_folder.clone(),
        bg2ee_log_folder: step1.bg2ee_log_folder.clone(),
        bg2ee_log_file: step1.bg2ee_log_file.clone(),
        eet_bgee_game_folder: step1.eet_bgee_game_folder.clone(),
        eet_bgee_log_folder: step1.eet_bgee_log_folder.clone(),
        eet_bg2ee_game_folder: step1.eet_bg2ee_game_folder.clone(),
        eet_bg2ee_log_folder: step1.eet_bg2ee_log_folder.clone(),
        eet_pre_dir: step1.eet_pre_dir.clone(),
        eet_new_dir: step1.eet_new_dir.clone(),
        generate_directory: step1.generate_directory.clone(),
        mods_folder: step1.mods_folder.clone(),
        weidu_binary: step1.weidu_binary.clone(),
        language: step1.language.clone(),
        scan: ScanOptions {
            custom_depth: step1.custom_scan_depth,
            casefold: step1.casefold,
            depth: step1.depth,
        },
        safety: SafetyOptions {
            skip_installed: step1.skip_installed,
            abort_on_warnings: step1.abort_on_warnings,
            check_last_installed: step1.check_last_installed,
        },
        timing: TimingOptions {
            per_mod_timeout: step1.timeout_per_mod_enabled,
            bounded_lookback: step1.lookback_enabled,
            dev_tick: step1.tick_dev_enabled,
            timeout: step1.timeout,
            tick: step1.tick,
            lookback: step1.lookback,
        },
        weidu_log_mode: step1.weidu_log_mode.clone(),
        transfer: TransferOptions {
            strict_matching: step1.strict_matching,
            download: step1.download,
            overwrite: step1.overwrite,
        },
    }
}
