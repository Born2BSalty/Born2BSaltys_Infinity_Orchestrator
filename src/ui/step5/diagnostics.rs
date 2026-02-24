// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod appdata_copy;
mod compat_snapshot;
mod compat_summary;
mod compat_summary_json;
mod format;
mod text;
mod tp2_layout;
mod write_checks;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use chrono::Local;

use crate::ui::state::WizardState;
use crate::ui::step4::format::build_weidu_export_lines;
use crate::ui::step5::command::build_install_invocation;
use crate::ui::step5::log_files::copy_source_weidu_logs;
use crate::ui::terminal::EmbeddedTerminal;

#[derive(Debug, Clone)]
pub struct DiagnosticsContext {
    pub dev_mode: bool,
    pub exe_fingerprint: String,
}

#[derive(Debug, Default, Clone)]
pub struct AppDataCopySummary {
    pub copied: Vec<PathBuf>,
    pub missing: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct WriteCheckSummary {
    pub lines: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Tp2LayoutSummary {
    pub lines: Vec<String>,
}

pub fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    ctx: &DiagnosticsContext,
) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let root_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&root_dir)?;
    let run_stamp = Local::now().format("%Y-%m-%d_%H-%M-%S_%3f").to_string();
    let run_dir = root_dir.join(format!("run_{run_stamp}"));
    fs::create_dir_all(&run_dir)?;
    let out_path = run_dir.join("bio_diag.txt");

    let source_logs_dir = run_dir.join("source_logs");
    let copied_source_logs = copy_source_weidu_logs(&state.step1, &source_logs_dir, "source");
    let appdata_summary = appdata_copy::copy_appdata_configs(&run_dir);
    let write_check_summary = write_checks::run_write_checks(state, ts);
    let tp2_layout_summary = tp2_layout::build_tp2_layout_summary(state);
    let active_order = if state.step3.active_game_tab == "BG2EE" {
        build_weidu_export_lines(&state.step3.bg2ee_items)
    } else {
        build_weidu_export_lines(&state.step3.bgee_items)
    };
    let console_excerpt = terminal
        .map(|t| t.console_excerpt(40_000))
        .unwrap_or_else(|| fallback_console_excerpt(state, 40_000));
    let (installer_program, installer_args) = build_install_invocation(&state.step1);

    let mut text = text::build_base_text(
        state,
        &copied_source_logs,
        &active_order,
        &console_excerpt,
        ts,
        ctx,
        &installer_program,
        &installer_args,
        &appdata_summary,
        &write_check_summary,
        &tp2_layout_summary,
    );
    if ctx.dev_mode {
        compat_snapshot::append_dev_compat_snapshots(state, &mut text);
    }

    fs::write(&out_path, text)?;
    if let Err(err) = compat_summary_json::write_compat_summary_json(&run_dir, &state.compat.issues, ts) {
        let note = format!(
            "\n[Diagnostics Notes]\ncompat_summary_json_write=FAILED: {}\n",
            err
        );
        if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&out_path) {
            let _ = f.write_all(note.as_bytes());
        }
    }
    Ok(out_path)
}

fn fallback_console_excerpt(state: &WizardState, limit_chars: usize) -> String {
    let text = state.step5.console_output.as_str();
    if text.trim().is_empty() {
        return "Terminal not available".to_string();
    }
    let total = text.chars().count();
    if total <= limit_chars {
        return text.to_string();
    }
    let start = total.saturating_sub(limit_chars);
    text.chars().skip(start).collect()
}
