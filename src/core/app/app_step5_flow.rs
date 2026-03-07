// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::ui::step4::action_step4::Step4Action;
use crate::ui::step4::service_step4::build_weidu_export_lines;
use crate::ui::terminal::EmbeddedTerminal;

use super::WizardApp;

pub(super) fn ensure_step5_terminal(app: &mut WizardApp, ctx: &eframe::egui::Context) {
    if app.step5_terminal.is_some() || app.step5_terminal_error.is_some() {
        return;
    }
    match EmbeddedTerminal::new(ctx) {
        Ok(term) => {
            app.step5_terminal = Some(term);
        }
        Err(err) => {
            app.step5_terminal_error = Some(err.to_string());
        }
    }
}

pub(super) fn handle_step4_action(
    app: &mut WizardApp,
    _ctx: &eframe::egui::Context,
    action: Step4Action,
) {
    match action {
        Step4Action::SaveWeiduLog => {
            let _ = auto_save_step4_weidu_logs(app);
        }
    }
}

fn save_weidu_logs_from_step4(app: &WizardApp) -> anyhow::Result<()> {
    let game = app.state.step1.game_install.as_str();
    let header = [
        "// Log of Currently Installed WeiDU Mods",
        "// The top of the file is the 'oldest' mod",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
    ];

    let write_target = |folder: &str, lines: Vec<String>| -> anyhow::Result<()> {
        let folder = folder.trim();
        if folder.is_empty() {
            return Ok(());
        }
        let dir = PathBuf::from(folder);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("weidu.log");
        let mut out: Vec<String> = header.iter().map(|s| s.to_string()).collect();
        out.extend(lines);
        std::fs::write(path, out.join("\n"))?;
        Ok(())
    };

    match game {
        "EET" => {
            let bgee_lines = build_weidu_export_lines(&app.state.step3.bgee_items);
            let bg2_lines = build_weidu_export_lines(&app.state.step3.bg2ee_items);
            write_target(&app.state.step1.eet_bgee_log_folder, bgee_lines)?;
            write_target(&app.state.step1.eet_bg2ee_log_folder, bg2_lines)?;
        }
        "BG2EE" => {
            let lines = build_weidu_export_lines(&app.state.step3.bg2ee_items);
            write_target(&app.state.step1.bg2ee_log_folder, lines)?;
        }
        _ => {
            let lines = build_weidu_export_lines(&app.state.step3.bgee_items);
            write_target(&app.state.step1.bgee_log_folder, lines)?;
        }
    }
    Ok(())
}

pub(super) fn auto_save_step4_weidu_logs(app: &mut WizardApp) -> Result<(), String> {
    if app.state.step1.have_weidu_logs {
        app.state.step2.scan_status = "Using source WeiDU log file(s) from Step 1".to_string();
        return Ok(());
    }
    match save_weidu_logs_from_step4(app) {
        Ok(()) => {
            app.state.step2.scan_status = "Saved weidu.log file(s)".to_string();
            Ok(())
        }
        Err(err) => {
            let msg = format!("Save weidu.log failed: {err}");
            app.state.step2.scan_status = msg.clone();
            app.state.step5.last_status_text = msg.clone();
            Err(msg)
        }
    }
}
