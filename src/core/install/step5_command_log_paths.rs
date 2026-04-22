// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::platform_defaults::compose_weidu_log_path;

use super::step5_command_config::InstallCommandConfig;

pub(crate) fn resolve_bgee_log_file(config: &InstallCommandConfig) -> String {
    if config.have_weidu_logs && !config.bgee_log_file.trim().is_empty() {
        return config.bgee_log_file.trim().to_string();
    }
    let folder = if config.game_install == "EET" {
        config.eet_bgee_log_folder.trim()
    } else {
        config.bgee_log_folder.trim()
    };
    compose_weidu_log_path(folder)
}

pub(crate) fn resolve_bg2_log_file(config: &InstallCommandConfig) -> String {
    if config.have_weidu_logs && !config.bg2ee_log_file.trim().is_empty() {
        return config.bg2ee_log_file.trim().to_string();
    }
    let folder = if config.game_install == "EET" {
        config.eet_bg2ee_log_folder.trim()
    } else {
        config.bg2ee_log_folder.trim()
    };
    compose_weidu_log_path(folder)
}
