// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod format;
mod selected_set;
mod tab;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::{CompatState, Step2State};

pub fn export_step2_compat_report(step2: &Step2State, compat: &CompatState) -> std::io::Result<PathBuf> {
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let out_path = out_dir.join(format!("compat_step2_{ts}.txt"));

    let mut text = String::new();
    text.push_str("Step 2 Compatibility Report\n");
    text.push_str("===========================\n\n");
    text.push_str(&format!("Active tab: {}\n\n", step2.active_game_tab));

    tab::append_tab_report("BGEE", &step2.bgee_mods, &mut text);
    tab::append_tab_report("BG2EE", &step2.bg2ee_mods, &mut text);
    selected_set::append_selected_set_validator_report(compat, &mut text);

    fs::write(&out_path, text)?;
    Ok(out_path)
}
