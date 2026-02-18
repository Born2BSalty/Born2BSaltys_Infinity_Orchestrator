// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step2ModState;

use super::super::WizardApp;

pub(super) fn should_show_step1_clean_confirm(app: &WizardApp) -> bool {
    let uses_fresh_target = if app.state.step1.game_install == "EET" {
        app.state.step1.new_pre_eet_dir_enabled || app.state.step1.new_eet_dir_enabled
    } else {
        app.state.step1.generate_directory_enabled
    };
    app.state.current_step == 0
        && uses_fresh_target
        && !app.state.step1.backup_targets_before_eet_copy
}

pub(super) fn advance_after_next(app: &mut WizardApp) {
    if !app.can_advance_from_current_step() {
        return;
    }
    if app.state.current_step == 0 && app.state.step1.have_weidu_logs {
        // With "Have WeiDU Logs?" enabled, skip Step 2/3 but still land on Step 4
        // so users can review/confirm log sources before install.
        app.state.current_step = 3;
    } else {
        if app.state.current_step == 1 {
            let signature = step2_selection_signature(app);
            let step3_empty = step3_has_no_real_items(app);
            let should_sync = step3_empty
                || app
                    .last_step2_sync_signature
                    .as_ref()
                    .map(|s| s != &signature)
                    .unwrap_or(true);
            if should_sync {
                app.sync_step3_from_step2();
                app.last_step2_sync_signature = Some(signature);
            } else {
                // Even if Step 3 structure didn't need sync, revalidate to refresh
                // mode-dependent issues (e.g. BGEE <-> EET switch).
                app.revalidate_compat();
            }
        }
        if app.state.current_step == 3
            && let Err(err) = app.auto_save_step4_weidu_logs()
        {
            app.state.step5.last_status_text = format!("Step 4 save failed: {err}");
            app.save_settings_best_effort();
            return;
        }
        app.state.go_next();
    }
    app.save_settings_best_effort();
}

fn step3_has_no_real_items(app: &WizardApp) -> bool {
    let bgee_has = app.state.step3.bgee_items.iter().any(|i| !i.is_parent);
    let bg2_has = app.state.step3.bg2ee_items.iter().any(|i| !i.is_parent);
    !(bgee_has || bg2_has)
}

fn step2_selection_signature(app: &WizardApp) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut collect = |tag: &str, mods: &[Step2ModState]| {
        for m in mods {
            let tp = m.tp_file.to_ascii_uppercase();
            for c in &m.components {
                if c.checked {
                    entries.push(format!("{tag}|{tp}|{}", c.component_id));
                }
            }
        }
    };
    collect("BGEE", &app.state.step2.bgee_mods);
    collect("BG2EE", &app.state.step2.bg2ee_mods);
    entries.sort_unstable();
    entries.join(";")
}
