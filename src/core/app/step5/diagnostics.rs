// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crate::ui::state::WizardState;
use crate::ui::step4::service_step4::build_weidu_export_lines;
use crate::ui::step5::service_diagnostics_run_step5::{current_or_new_run_id, run_dir_from_id};
use crate::ui::step5::service_step5::{
    build_install_invocation, copy_saved_weidu_logs, copy_source_weidu_logs,
};
use crate::ui::terminal::EmbeddedTerminal;

#[path = "diagnostics_appdata_copy.rs"]
mod appdata_copy;
#[path = "diagnostics_compat_decisions_json.rs"]
mod compat_decisions_json;
#[path = "diagnostics_compat_snapshot.rs"]
mod compat_snapshot;
#[path = "diagnostics_compat_summary.rs"]
mod compat_summary;
#[path = "diagnostics_compat_summary_json.rs"]
mod compat_summary_json;
#[path = "diagnostics_export_marker_json.rs"]
mod export_marker_json;
#[path = "diagnostics_format.rs"]
mod format;
#[path = "diagnostics_quick_triage.rs"]
mod quick_triage;
#[path = "diagnostics_scan_context_json.rs"]
mod scan_context_json;
#[path = "diagnostics_prompt_calls_json.rs"]
mod prompt_calls_json;
#[path = "diagnostics_parser_events_json.rs"]
mod parser_events_json;
#[path = "diagnostics_parser_raw_json.rs"]
mod parser_raw_json;
#[path = "diagnostics_text.rs"]
mod text;
#[path = "diagnostics_tp2_layout.rs"]
mod tp2_layout;
#[path = "diagnostics_undefined_summary_json.rs"]
mod undefined_summary_json;
#[path = "diagnostics_write_checks.rs"]
mod write_checks;

pub use write_checks::{AppDataCopySummary, DiagnosticsContext, Tp2LayoutSummary, WriteCheckSummary};

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
    let run_id = current_or_new_run_id(&state.step5);
    let run_dir = run_dir_from_id(&run_id);
    fs::create_dir_all(&run_dir)?;
    let out_path = run_dir.join("bio_diag.txt");

    let source_logs_dir = run_dir.join("source_logs");
    let copied_source_logs = copy_source_weidu_logs(&state.step1, &source_logs_dir, "source");
    let saved_logs_dir = run_dir.join("saved_logs");
    let copied_saved_logs = copy_saved_weidu_logs(&state.step1, &saved_logs_dir, "saved");
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
        &run_id,
        &copied_source_logs,
        &copied_saved_logs,
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
    let mut written_paths = vec![out_path.clone()];
    written_paths.extend(copied_source_logs.iter().cloned());
    written_paths.extend(copied_saved_logs.iter().cloned());
    written_paths.extend(appdata_summary.copied.iter().cloned());

    match quick_triage::write_quick_triage_txt(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(&out_path, &format!("quick_triage_write=FAILED: {err}")),
    }
    match scan_context_json::write_scan_context_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(&out_path, &format!("scan_context_json_write=FAILED: {err}")),
    }
    match prompt_calls_json::write_prompt_calls_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(&out_path, &format!("prompt_calls_json_write=FAILED: {err}")),
    }
    match parser_events_json::write_parser_events_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(&out_path, &format!("parser_events_json_write=FAILED: {err}")),
    }
    match parser_raw_json::write_parser_raw_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(&out_path, &format!("parser_raw_json_write=FAILED: {err}")),
    }
    match undefined_summary_json::write_undefined_summary_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(
            &out_path,
            &format!("undefined_summary_json_write=FAILED: {err}"),
        ),
    }
    match compat_decisions_json::write_compat_decisions_json(&run_dir, state, ts) {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(
            &out_path,
            &format!("compat_decisions_json_write=FAILED: {err}"),
        ),
    }

    if let Err(err) = compat_summary_json::write_compat_summary_json(&run_dir, &state.compat.issues, ts) {
        append_diag_note(&out_path, &format!("compat_summary_json_write=FAILED: {err}"));
    } else {
        written_paths.push(run_dir.join("compat_summary.json"));
    }
    if let Err(err) = export_marker_json::write_export_marker_json(&run_dir, ts, &written_paths) {
        append_diag_note(&out_path, &format!("export_marker_json_write=FAILED: {err}"));
    }
    Ok(out_path)
}

fn append_diag_note(out_path: &PathBuf, line: &str) {
    let note = format!("\n[Diagnostics Notes]\n{line}\n");
    if let Ok(mut f) = fs::OpenOptions::new().append(true).open(out_path) {
        let _ = f.write_all(note.as_bytes());
    }
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
