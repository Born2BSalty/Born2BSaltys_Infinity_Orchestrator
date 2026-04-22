// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};

use super::step5_command_config::InstallCommandConfig;

pub(crate) fn installer_program(config: &InstallCommandConfig) -> String {
    resolve_mod_installer_binary(&config.mod_installer_binary)
}

pub(crate) fn append_common_args(config: &InstallCommandConfig, args: &mut Vec<String>) {
    let bool_s = |value: bool| if value { "true" } else { "false" }.to_string();
    args.push("--language".to_string());
    args.push(config.language.clone());
    args.push("--mod-directories".to_string());
    args.push(config.mods_folder.clone());
    args.push("--weidu-binary".to_string());
    args.push(resolve_weidu_binary(&config.weidu_binary));
    if config.weidu_log_mode_enabled {
        args.push("--weidu-log-mode".to_string());
        args.push(config.weidu_log_mode.clone());
    }
    args.push("--skip-installed".to_string());
    args.push(bool_s(config.skip_installed));
    args.push("--strict-matching".to_string());
    args.push(bool_s(config.strict_matching));
    args.push("--download".to_string());
    args.push(bool_s(config.download));
    args.push("--overwrite".to_string());
    args.push(bool_s(config.overwrite));
    args.push("--abort-on-warnings".to_string());
    args.push(bool_s(config.abort_on_warnings));
    args.push("--check-last-installed".to_string());
    args.push(bool_s(config.check_last_installed));
    if config.custom_scan_depth {
        args.push("--depth".to_string());
        args.push(config.depth.to_string());
    }
    if config.timeout_per_mod_enabled {
        args.push("--timeout".to_string());
        args.push(config.timeout.to_string());
    }
    if config.lookback_enabled {
        args.push("--lookback".to_string());
        args.push(config.lookback.to_string());
    }
    if config.tick_dev_enabled {
        args.push("--tick".to_string());
        args.push(config.tick.to_string());
    }
    if config.casefold {
        args.push("--casefold".to_string());
        args.push("true".to_string());
    }
}
