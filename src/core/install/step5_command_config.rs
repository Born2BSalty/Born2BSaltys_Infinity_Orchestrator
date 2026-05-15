// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "install command config mirrors stable command-generation inputs"
)]
pub(crate) struct InstallCommandConfig {
    pub game_install: String,
    pub have_weidu_logs: bool,
    pub custom_scan_depth: bool,
    pub timeout_per_mod_enabled: bool,
    pub lookback_enabled: bool,
    pub tick_dev_enabled: bool,
    pub weidu_log_mode_enabled: bool,
    pub new_pre_eet_dir_enabled: bool,
    pub new_eet_dir_enabled: bool,
    pub generate_directory_enabled: bool,
    pub mod_installer_binary: String,
    pub bgee_game_folder: String,
    pub bgee_log_folder: String,
    pub bgee_log_file: String,
    pub bg2ee_game_folder: String,
    pub bg2ee_log_folder: String,
    pub bg2ee_log_file: String,
    pub eet_bgee_game_folder: String,
    pub eet_bgee_log_folder: String,
    pub eet_bg2ee_game_folder: String,
    pub eet_bg2ee_log_folder: String,
    pub eet_pre_dir: String,
    pub eet_new_dir: String,
    pub generate_directory: String,
    pub mods_folder: String,
    pub weidu_binary: String,
    pub language: String,
    pub depth: usize,
    pub skip_installed: bool,
    pub abort_on_warnings: bool,
    pub timeout: usize,
    pub weidu_log_mode: String,
    pub strict_matching: bool,
    pub download: bool,
    pub overwrite: bool,
    pub check_last_installed: bool,
    pub tick: u64,
    pub lookback: usize,
    pub casefold: bool,
}
