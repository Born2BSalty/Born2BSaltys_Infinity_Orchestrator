// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};

use super::step5_command_config::InstallCommandConfig;

#[must_use]
pub(crate) fn installer_program(config: &InstallCommandConfig) -> String {
    resolve_mod_installer_binary(&config.mod_installer_binary)
}

pub(crate) fn append_common_args(config: &InstallCommandConfig, args: &mut Vec<String>) {
    args.push("--language".to_string());
    args.push(config.language.clone());
    args.push("--mod-directories".to_string());
    args.push(config.mods_folder.clone());
    args.push("--weidu-binary".to_string());
    args.push(resolve_weidu_binary(&config.weidu_binary));
    if config.logs.include_mode {
        args.push("--weidu-log-mode".to_string());
        args.push(config.weidu_log_mode.clone());
    }
    args.push("--skip-installed".to_string());
    args.push(config.safety.skip_installed.to_string());
    args.push("--strict-matching".to_string());
    args.push(config.transfer.strict_matching.to_string());
    args.push("--download".to_string());
    args.push(config.transfer.download.to_string());
    args.push("--overwrite".to_string());
    args.push(config.transfer.overwrite.to_string());
    args.push("--abort-on-warnings".to_string());
    args.push(config.safety.abort_on_warnings.to_string());
    args.push("--check-last-installed".to_string());
    args.push(config.safety.check_last_installed.to_string());
    if config.scan.custom_depth {
        args.push("--depth".to_string());
        args.push(config.scan.depth.to_string());
    }
    if config.timing.per_mod_timeout {
        args.push("--timeout".to_string());
        args.push(config.timing.timeout.to_string());
    }
    if config.timing.bounded_lookback {
        args.push("--lookback".to_string());
        args.push(config.timing.lookback.to_string());
    }
    if config.timing.dev_tick {
        args.push("--tick".to_string());
        args.push(config.timing.tick.to_string());
    }
    if config.scan.casefold {
        args.push("--casefold".to_string());
        args.push("true".to_string());
    }
}
