// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use rfd::FileDialog;

use super::WizardApp;

pub(super) fn apply_weidu_log_selection(app: &mut WizardApp, bgee: bool) {
    let current = if bgee {
        crate::app::app_step2_log::resolve_bgee_weidu_log_path(&app.state.step1)
    } else {
        crate::app::app_step2_log::resolve_bg2_weidu_log_path(&app.state.step1)
    };
    let picked = pick_weidu_log_file(current.as_ref(), bgee);
    let log_path = picked.clone().or_else(|| {
        if bgee {
            crate::app::app_step2_log::resolve_bgee_weidu_log_path(&app.state.step1)
        } else {
            crate::app::app_step2_log::resolve_bg2_weidu_log_path(&app.state.step1)
        }
    });
    if let Some(path) = picked {
        let picked_str = path.to_string_lossy().to_string();
        if bgee {
            app.state.step1.bgee_log_file = picked_str;
        } else {
            app.state.step1.bg2ee_log_file = picked_str;
        }
        app.save_settings_best_effort();
    }
    crate::app::app_step2_log::apply_weidu_log_selection_from_path(&mut app.state, bgee, log_path);
}

fn pick_weidu_log_file(current: Option<&PathBuf>, bgee: bool) -> Option<PathBuf> {
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
