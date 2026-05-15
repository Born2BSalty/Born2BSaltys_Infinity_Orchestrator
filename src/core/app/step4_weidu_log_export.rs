// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::app::state::WizardState;
use crate::app::step5::diagnostics::build_weidu_export_lines;

fn save_weidu_logs_from_step4(state: &WizardState) -> anyhow::Result<()> {
    let game = state.step1.game_install.as_str();
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
        let mut out: Vec<String> = header
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        out.extend(lines);
        std::fs::write(path, out.join("\n"))?;
        Ok(())
    };

    match game {
        "EET" => {
            let bgee_lines = build_weidu_export_lines(&state.step3.bgee_items);
            let bg2_lines = build_weidu_export_lines(&state.step3.bg2ee_items);
            write_target(&state.step1.eet_bgee_log_folder, bgee_lines)?;
            write_target(&state.step1.eet_bg2ee_log_folder, bg2_lines)?;
        }
        "BG2EE" => {
            let lines = build_weidu_export_lines(&state.step3.bg2ee_items);
            write_target(&state.step1.bg2ee_log_folder, lines)?;
        }
        _ => {
            let lines = build_weidu_export_lines(&state.step3.bgee_items);
            write_target(&state.step1.bgee_log_folder, lines)?;
        }
    }
    Ok(())
}

pub(crate) fn auto_save_step4_weidu_logs(state: &mut WizardState) -> Result<(), String> {
    if state.step1.installs_exactly_from_weidu_logs() {
        state.step2.scan_status = "Using source WeiDU log file(s) from Step 1".to_string();
        return Ok(());
    }
    match save_weidu_logs_from_step4(state) {
        Ok(()) => {
            state.step2.scan_status = "Saved weidu.log file(s)".to_string();
            Ok(())
        }
        Err(err) => {
            let msg = format!("Save weidu.log failed: {err}");
            state.step2.scan_status.clone_from(&msg);
            state.step5.last_status_text.clone_from(&msg);
            Err(msg)
        }
    }
}
