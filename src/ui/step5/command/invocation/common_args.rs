// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;
use crate::platform_defaults::{resolve_mod_installer_binary, resolve_weidu_binary};

pub(crate) fn installer_program(step1: &Step1State) -> String {
    resolve_mod_installer_binary(&step1.mod_installer_binary)
}

pub(crate) fn append_common_args(step1: &Step1State, args: &mut Vec<String>) {
    let bool_s = |v: bool| if v { "true" } else { "false" }.to_string();
    args.push("--language".to_string());
    args.push(step1.language.clone());
    args.push("--mod-directories".to_string());
    args.push(step1.mods_folder.clone());
    args.push("--weidu-binary".to_string());
    args.push(resolve_weidu_binary(&step1.weidu_binary));
    if step1.weidu_log_mode_enabled {
        args.push("--weidu-log-mode".to_string());
        args.push(step1.weidu_log_mode.clone());
    }
    args.push("--skip-installed".to_string());
    args.push(bool_s(step1.skip_installed));
    args.push("--strict-matching".to_string());
    args.push(bool_s(step1.strict_matching));
    args.push("--download".to_string());
    args.push(bool_s(step1.download));
    args.push("--overwrite".to_string());
    args.push(bool_s(step1.overwrite));
    args.push("--abort-on-warnings".to_string());
    args.push(bool_s(step1.abort_on_warnings));
    args.push("--check-last-installed".to_string());
    args.push(bool_s(step1.check_last_installed));
    if step1.custom_scan_depth {
        args.push("--depth".to_string());
        args.push(step1.depth.to_string());
    }
    if step1.timeout_per_mod_enabled {
        args.push("--timeout".to_string());
        args.push(step1.timeout.to_string());
    }
    if step1.lookback_enabled {
        args.push("--lookback".to_string());
        args.push(step1.lookback.to_string());
    }
    if step1.tick_dev_enabled {
        args.push("--tick".to_string());
        args.push(step1.tick.to_string());
    }
    if step1.casefold {
        args.push("--casefold".to_string());
        args.push("true".to_string());
    }
}
