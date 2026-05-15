// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::state::WizardState;

const WEIDU_COMPONENT_LOGS_DIR: &str = "weidu_component_logs";

pub fn apply(state: &mut WizardState, destination_folder: &Path) -> Result<(), String> {
    let log_folder = destination_folder.join(WEIDU_COMPONENT_LOGS_DIR);
    std::fs::create_dir_all(&log_folder).map_err(|err| {
        format!(
            "failed to create WeiDU component log folder {}: {err}",
            log_folder.display()
        )
    })?;

    let step1 = &mut state.step1;
    step1.weidu_log_mode_enabled = true;
    step1.weidu_log_autolog = true;
    step1.weidu_log_logapp = true;
    step1.weidu_log_logextern = true;
    step1.weidu_log_log_component = true;
    step1.weidu_log_folder = log_folder.to_string_lossy().to_string();
    rebuild_weidu_log_mode(state);
    Ok(())
}

fn rebuild_weidu_log_mode(state: &mut WizardState) {
    let step1 = &mut state.step1;
    let mut parts = Vec::new();
    if step1.weidu_log_autolog {
        parts.push("autolog".to_string());
    }
    if step1.weidu_log_logapp {
        parts.push("logapp".to_string());
    }
    if step1.weidu_log_logextern {
        parts.push("log-extern".to_string());
    }
    if step1.weidu_log_log_component {
        if step1.weidu_log_folder.trim().is_empty() {
            parts.push("log".to_string());
        } else {
            parts.push(format!("log {}", step1.weidu_log_folder.trim()));
        }
    }
    if parts.is_empty() {
        parts.push("autolog".to_string());
    }
    step1.weidu_log_mode = parts.join(",");
}
