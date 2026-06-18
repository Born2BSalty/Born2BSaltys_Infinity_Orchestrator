// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub fn apply_weidu_log_selection_for_orchestrator(orchestrator: &mut OrchestratorApp, bgee: bool) {
    let current = if bgee {
        resolve_bgee_weidu_log_path(&orchestrator.wizard_state.step1)
    } else {
        resolve_bg2_weidu_log_path(&orchestrator.wizard_state.step1)
    };

    let picked = pick_weidu_log_file(current.as_deref(), bgee);

    let Some(path) = picked else {
        return;
    };

    let picked_str = path.to_string_lossy().to_string();
    if bgee {
        orchestrator.wizard_state.step1.bgee_log_file = picked_str;
    } else {
        orchestrator.wizard_state.step1.bg2ee_log_file = picked_str;
    }
    orchestrator
        .bio_settings_last_dirty_at
        .get_or_insert_with(Instant::now);

    apply_weidu_log_selection_from_path(&mut orchestrator.wizard_state, bgee, Some(path));

    if bgee {
        orchestrator.wizard_state.step3.bgee_items.clear();
    } else {
        orchestrator.wizard_state.step3.bg2ee_items.clear();
    }
}

fn pick_weidu_log_file(current: Option<&Path>, bgee: bool) -> Option<PathBuf> {
    let mut dialog = FileDialog::new()
        .add_filter("WeiDU Log", &["log"])
        .set_title(if bgee {
            "Select BGEE WeiDU log"
        } else {
            "Select BG2EE WeiDU log"
        });
    if let Some(cur) = current
        && let Some(dir) = cur.parent()
    {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}
