// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod compat_snapshot;
mod format;
mod text;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::ui::state::WizardState;
use crate::ui::step4::format::build_weidu_export_lines;
use crate::ui::step5::log_files::copy_source_weidu_logs;
use crate::ui::terminal::EmbeddedTerminal;

pub fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    dev_mode: bool,
) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("bio_diag_{ts}.txt"));

    let copied_source_logs = copy_source_weidu_logs(&state.step1, &out_dir, format!("source_{ts}").as_str());
    let active_order = if state.step3.active_game_tab == "BG2EE" {
        build_weidu_export_lines(&state.step3.bg2ee_items)
    } else {
        build_weidu_export_lines(&state.step3.bgee_items)
    };
    let console_excerpt = terminal
        .map(|t| t.console_excerpt(40_000))
        .unwrap_or_else(|| "Terminal not available".to_string());

    let mut text = text::build_base_text(state, &copied_source_logs, &active_order, &console_excerpt);
    if dev_mode {
        compat_snapshot::append_dev_compat_snapshots(state, &mut text);
    }

    fs::write(&out_path, text)?;
    Ok(out_path)
}
