// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;
use crate::platform_defaults::compose_weidu_log_path;

pub(crate) fn resolve_bgee_log_file(step1: &Step1State) -> String {
    if step1.have_weidu_logs && !step1.bgee_log_file.trim().is_empty() {
        return step1.bgee_log_file.trim().to_string();
    }
    let folder = if step1.game_install == "EET" {
        step1.eet_bgee_log_folder.trim()
    } else {
        step1.bgee_log_folder.trim()
    };
    compose_weidu_log_path(folder)
}

pub(crate) fn resolve_bg2_log_file(step1: &Step1State) -> String {
    if step1.have_weidu_logs && !step1.bg2ee_log_file.trim().is_empty() {
        return step1.bg2ee_log_file.trim().to_string();
    }
    let folder = if step1.game_install == "EET" {
        step1.eet_bg2ee_log_folder.trim()
    } else {
        step1.bg2ee_log_folder.trim()
    };
    compose_weidu_log_path(folder)
}
