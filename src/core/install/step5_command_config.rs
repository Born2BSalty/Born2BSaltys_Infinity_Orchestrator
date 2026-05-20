// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Default)]
pub(crate) struct InstallCommandConfig {
    pub(crate) game_install: String,
    pub(crate) logs: LogOptions,
    pub(crate) directories: DirectoryOptions,
    pub(crate) mod_installer_binary: String,
    pub(crate) bgee_game_folder: String,
    pub(crate) bgee_log_folder: String,
    pub(crate) bgee_log_file: String,
    pub(crate) bg2ee_game_folder: String,
    pub(crate) bg2ee_log_folder: String,
    pub(crate) bg2ee_log_file: String,
    pub(crate) eet_bgee_game_folder: String,
    pub(crate) eet_bgee_log_folder: String,
    pub(crate) eet_bg2ee_game_folder: String,
    pub(crate) eet_bg2ee_log_folder: String,
    pub(crate) eet_pre_dir: String,
    pub(crate) eet_new_dir: String,
    pub(crate) generate_directory: String,
    pub(crate) mods_folder: String,
    pub(crate) weidu_binary: String,
    pub(crate) language: String,
    pub(crate) scan: ScanOptions,
    pub(crate) safety: SafetyOptions,
    pub(crate) timing: TimingOptions,
    pub(crate) weidu_log_mode: String,
    pub(crate) transfer: TransferOptions,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LogOptions {
    pub(crate) exact_weidu_logs: bool,
    pub(crate) include_mode: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DirectoryOptions {
    pub(crate) pre_eet_override: bool,
    pub(crate) eet_override: bool,
    pub(crate) generate_output: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ScanOptions {
    pub(crate) custom_depth: bool,
    pub(crate) casefold: bool,
    pub(crate) depth: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SafetyOptions {
    pub(crate) skip_installed: bool,
    pub(crate) abort_on_warnings: bool,
    pub(crate) check_last_installed: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TimingOptions {
    pub(crate) per_mod_timeout: bool,
    pub(crate) bounded_lookback: bool,
    pub(crate) dev_tick: bool,
    pub(crate) timeout: usize,
    pub(crate) tick: u64,
    pub(crate) lookback: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TransferOptions {
    pub(crate) strict_matching: bool,
    pub(crate) download: bool,
    pub(crate) overwrite: bool,
}
