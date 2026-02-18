// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::PathBuf;

use rfd::FileDialog;

use crate::mods::log_file::LogFile;
use crate::ui::controller::log_apply::{apply_log_to_mods, normalize_path_key};

use super::WizardApp;

pub(super) fn apply_weidu_log_selection(app: &mut WizardApp, bgee: bool) {
    let picked = pick_weidu_log_file(app, bgee);
    let log_path = picked.or_else(|| {
        if bgee {
            resolve_bgee_weidu_log_path(app)
        } else {
            resolve_bg2_weidu_log_path(app)
        }
    });

    let Some(path) = log_path else {
        app.state.step2.scan_status = "No WeiDU log selected".to_string();
        return;
    };

    let log = match LogFile::from_path(&path) {
        Ok(v) => v,
        Err(err) => {
            app.state.step2.scan_status = format!("Failed to parse log: {err}");
            return;
        }
    };

    let mut next_order = app.state.step2.next_selection_order;
    let matched = match (app.state.step1.game_install.as_str(), bgee) {
        ("EET", true) => {
            let picked_bgee =
                apply_log_to_mods(&mut app.state.step2.bgee_mods, &log, None, true, &mut next_order);
            let allow = HashSet::from([normalize_path_key(r"EET\EET.TP2")]);
            let picked_eet_core = apply_log_to_mods(
                &mut app.state.step2.bg2ee_mods,
                &log,
                Some(&allow),
                false,
                &mut next_order,
            );
            picked_bgee + picked_eet_core
        }
        (_, true) => apply_log_to_mods(&mut app.state.step2.bgee_mods, &log, None, true, &mut next_order),
        _ => apply_log_to_mods(&mut app.state.step2.bg2ee_mods, &log, None, true, &mut next_order),
    };
    app.state.step2.next_selection_order = next_order;
    let label = if bgee { "BGEE" } else { "BG2EE" };
    app.state.step2.scan_status = format!("{label} selected from log: {matched}");
    // Treat explicit "select via WeiDU log" as a fresh Step 2 source of truth.
    // This avoids carrying stale/custom Step 3 blocks when re-applying logs.
    app.last_step2_sync_signature = None;
    app.sync_step3_from_step2();
}

fn pick_weidu_log_file(app: &mut WizardApp, bgee: bool) -> Option<PathBuf> {
    let current = if bgee {
        resolve_bgee_weidu_log_path(app)
    } else {
        resolve_bg2_weidu_log_path(app)
    };

    let mut dialog = FileDialog::new()
        .add_filter("WeiDU Log", &["log"])
        .set_title(if bgee {
            "Select BGEE WeiDU log"
        } else {
            "Select BG2EE WeiDU log"
        });
    if let Some(cur) = &current
        && let Some(dir) = cur.parent()
    {
        dialog = dialog.set_directory(dir);
    }
    let picked = dialog.pick_file()?;
    let picked_str = picked.to_string_lossy().to_string();
    if bgee {
        app.state.step1.bgee_log_file = picked_str;
    } else {
        app.state.step1.bg2ee_log_file = picked_str;
    }
    app.save_settings_best_effort();
    Some(picked)
}

fn resolve_bgee_weidu_log_path(app: &WizardApp) -> Option<PathBuf> {
    let s = &app.state.step1;
    if s.have_weidu_logs && !s.bgee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bgee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bgee_log_folder.trim()
    } else {
        s.bgee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}

fn resolve_bg2_weidu_log_path(app: &WizardApp) -> Option<PathBuf> {
    let s = &app.state.step1;
    if s.have_weidu_logs && !s.bg2ee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bg2ee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bg2ee_log_folder.trim()
    } else {
        s.bg2ee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}
