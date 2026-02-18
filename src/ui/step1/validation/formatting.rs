// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

pub(super) fn format_path_check_result(errors: &[String], checked: usize) -> (bool, String) {
    if errors.is_empty() {
        (true, format!("Path check passed ({checked} path(s) validated)"))
    } else {
        (false, format!("Path check failed: {}", errors.join(" | ")))
    }
}

pub fn sync_weidu_log_mode(s: &mut Step1State) {
    let mut parts = Vec::new();
    if s.weidu_log_autolog {
        parts.push("autolog".to_string());
    }
    if s.weidu_log_logapp {
        parts.push("logapp".to_string());
    }
    if s.weidu_log_logextern {
        parts.push("log-extern".to_string());
    }
    if s.weidu_log_log_component {
        if s.weidu_log_folder.trim().is_empty() {
            parts.push("log".to_string());
        } else {
            parts.push(format!("log {}", s.weidu_log_folder.trim()));
        }
    }
    if parts.is_empty() {
        parts.push("autolog".to_string());
    }
    s.weidu_log_mode = parts.join(",");
}
