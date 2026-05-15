// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::log_files::{
    DiagnosticLogGroup, copy_diagnostic_origin_logs, current_or_new_run_id, prune_old_diagnostics,
    run_dir_from_id,
};
use crate::app::state::{Step3ItemState, WizardState};
use crate::app::step5::command_config::build_install_command_config;
use crate::app::terminal::EmbeddedTerminal;
use crate::install::step5_command_install::build_install_invocation;
use crate::platform_defaults::normalize_weidu_like_line;
use anyhow::Result;

#[path = "diagnostics_appdata_copy.rs"]
mod appdata_copy;
#[path = "diagnostics_compat_decisions_json.rs"]
mod compat_decisions_json;
#[path = "diagnostics_compat_rule_inventory_json.rs"]
mod compat_rule_inventory_json;
#[path = "diagnostics_compat_rule_matches_summary_json.rs"]
mod compat_rule_matches_summary_json;
#[path = "diagnostics_compat_rule_trace_json.rs"]
mod compat_rule_trace_json;
#[path = "diagnostics_compat_snapshot.rs"]
mod compat_snapshot;
#[path = "diagnostics_export_marker_json.rs"]
mod export_marker_json;
#[path = "diagnostics_mod_downloads.rs"]
mod mod_downloads_diagnostics;
#[path = "diagnostics_parser_events_json.rs"]
mod parser_events_json;
#[path = "diagnostics_parser_raw_json.rs"]
mod parser_raw_json;
#[path = "diagnostics_prompt_calls_json.rs"]
mod prompt_calls_json;
#[path = "diagnostics_quick_triage.rs"]
mod quick_triage;
#[path = "diagnostics_runtime_assumptions_json.rs"]
mod runtime_assumptions_json;
#[path = "diagnostics_scan_context_json.rs"]
mod scan_context_json;
#[path = "diagnostics_step2_component_audit_json.rs"]
mod step2_component_audit_json;
#[path = "diagnostics_step2_component_audit_txt.rs"]
mod step2_component_audit_txt;
#[path = "diagnostics_step2_render_order_json.rs"]
mod step2_render_order_json;
#[path = "diagnostics_step3_issue_snapshot_json.rs"]
mod step3_issue_snapshot_json;
#[path = "diagnostics_text.rs"]
mod text;
#[path = "diagnostics_tp2_layout.rs"]
mod tp2_layout;
#[path = "diagnostics_undefined_detect.rs"]
mod undefined_detect;
#[path = "diagnostics_undefined_summary_json.rs"]
mod undefined_summary_json;
#[path = "diagnostics_write_checks.rs"]
pub mod write_checks;

pub(crate) use write_checks::{
    AppDataCopySummary, DiagnosticsContext, Tp2LayoutSummary, WriteCheckSummary,
};

pub(crate) fn build_weidu_export_lines(items: &[Step3ItemState]) -> Vec<String> {
    items
        .iter()
        .filter(|i| !i.is_parent)
        .map(format_step4_item)
        .collect()
}

pub(crate) fn format_step4_item(item: &Step3ItemState) -> String {
    if item.raw_line.trim().is_empty() {
        let folder = item.mod_name.replace('/', "\\");
        format!(
            "~{}\\{}~ #0 #{} // {}",
            folder, item.tp_file, item.component_id, item.component_label
        )
    } else {
        normalize_weidu_like_line(&item.raw_line)
    }
}

pub(crate) fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    ctx: &DiagnosticsContext,
) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let run_id = current_or_new_run_id(&state.step5);
    prune_old_diagnostics(Some(&run_id));
    let root_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&root_dir)?;
    let run_dir = run_dir_from_id(&run_id);
    fs::create_dir_all(&run_dir)?;
    let out_path = run_dir.join("bio_diag.txt");

    let dirs = prepare_diagnostics_dirs(&run_dir)?;
    let mut log_groups = copy_diagnostic_origin_logs(&state.step1, &dirs.logs);
    log_groups.extend(write_step4_weidu_log_snapshots(state, &dirs.logs)?);
    let appdata_summary = appdata_copy::copy_appdata_configs(&run_dir);
    let write_check_summary = write_checks::run_write_checks(state, ts);
    let tp2_layout_summary = tp2_layout::build_tp2_layout_summary(state);
    let mut text = build_diagnostics_text(
        state,
        terminal,
        ctx,
        &TextBuildArtifacts {
            run_id: &run_id,
            log_groups: &log_groups,
            appdata_summary: &appdata_summary,
            write_check_summary: &write_check_summary,
            tp2_layout_summary: &tp2_layout_summary,
            timestamp_unix_secs: ts,
        },
    );
    if ctx.dev_mode {
        compat_snapshot::append_dev_compat_snapshots(state, &mut text);
    }

    fs::write(&out_path, text)?;
    let mut written_paths = vec![out_path.clone()];
    for group in &log_groups {
        written_paths.extend(group.copied_paths.iter().cloned());
    }
    written_paths.extend(appdata_summary.copied.iter().cloned());

    write_diagnostic_artifacts(
        state,
        &dirs,
        &out_path,
        &mut written_paths,
        &write_check_summary,
        ts,
    );

    if let Err(err) = export_marker_json::write_export_marker_json(&run_dir, ts, &written_paths) {
        append_diag_note(
            &out_path,
            &format!("export_marker_json_write=FAILED: {err}"),
        );
    }
    Ok(out_path)
}

struct DiagnosticsDirs {
    run: PathBuf,
    summary: PathBuf,
    scan: PathBuf,
    compat: PathBuf,
    logs: PathBuf,
}

fn prepare_diagnostics_dirs(run_dir: &std::path::Path) -> Result<DiagnosticsDirs> {
    let dirs = DiagnosticsDirs {
        run: run_dir.to_path_buf(),
        summary: run_dir.join("summary"),
        scan: run_dir.join("scan"),
        compat: run_dir.join("compat"),
        logs: run_dir.join("logs"),
    };
    fs::create_dir_all(&dirs.summary)?;
    fs::create_dir_all(&dirs.scan)?;
    fs::create_dir_all(&dirs.compat)?;
    fs::create_dir_all(&dirs.logs)?;
    Ok(dirs)
}

struct TextBuildArtifacts<'a> {
    run_id: &'a str,
    log_groups: &'a [DiagnosticLogGroup],
    appdata_summary: &'a AppDataCopySummary,
    write_check_summary: &'a WriteCheckSummary,
    tp2_layout_summary: &'a Tp2LayoutSummary,
    timestamp_unix_secs: u64,
}

fn build_diagnostics_text(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    ctx: &DiagnosticsContext,
    artifacts: &TextBuildArtifacts<'_>,
) -> String {
    let active_order = if state.step3.active_game_tab == "BG2EE" {
        build_weidu_export_lines(&state.step3.bg2ee_items)
    } else {
        build_weidu_export_lines(&state.step3.bgee_items)
    };
    let console_excerpt = terminal.map_or_else(
        || fallback_console_excerpt(state, 40_000),
        |t| t.console_excerpt(40_000),
    );
    let install_config = build_install_command_config(&state.step1);
    let (installer_program, installer_args) = build_install_invocation(&install_config);
    text::build_base_text(&text::BuildBaseTextInput {
        state,
        diagnostics_run_id: artifacts.run_id,
        log_groups: artifacts.log_groups,
        active_order: &active_order,
        console_excerpt: &console_excerpt,
        timestamp_unix_secs: artifacts.timestamp_unix_secs,
        diag_ctx: ctx,
        installer_program: &installer_program,
        installer_args: &installer_args,
        appdata_summary: artifacts.appdata_summary,
        write_check_summary: artifacts.write_check_summary,
        tp2_layout_summary: artifacts.tp2_layout_summary,
    })
}

fn write_diagnostic_artifacts(
    state: &WizardState,
    dirs: &DiagnosticsDirs,
    out_path: &PathBuf,
    written_paths: &mut Vec<PathBuf>,
    write_check_summary: &WriteCheckSummary,
    ts: u64,
) {
    write_summary_artifacts(
        state,
        dirs,
        out_path,
        written_paths,
        write_check_summary,
        ts,
    );
    write_scan_artifacts(state, dirs, out_path, written_paths, ts);
    write_compat_artifacts(state, dirs, out_path, written_paths, ts);
    match mod_downloads_diagnostics::write_mod_download_diagnostics(&dirs.run, state, ts) {
        Ok(paths) => written_paths.extend(paths),
        Err(err) => append_diag_note(
            out_path,
            &format!("mod_download_diagnostics_write=FAILED: {err}"),
        ),
    }
}

fn push_artifact_result(
    out_path: &PathBuf,
    written_paths: &mut Vec<PathBuf>,
    result: Result<PathBuf>,
    label: &str,
) {
    match result {
        Ok(path) => written_paths.push(path),
        Err(err) => append_diag_note(out_path, &format!("{label}=FAILED: {err}")),
    }
}

fn write_summary_artifacts(
    state: &WizardState,
    dirs: &DiagnosticsDirs,
    out_path: &PathBuf,
    written_paths: &mut Vec<PathBuf>,
    write_check_summary: &WriteCheckSummary,
    ts: u64,
) {
    push_artifact_result(
        out_path,
        written_paths,
        runtime_assumptions_json::write_runtime_assumptions_json(&dirs.summary, state, ts),
        "runtime_assumptions_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        quick_triage::write_quick_triage_txt(&dirs.run, state, write_check_summary, ts),
        "quick_triage_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        prompt_calls_json::write_prompt_calls_json(&dirs.summary, state, ts),
        "prompt_calls_json_write",
    );
}

fn write_scan_artifacts(
    state: &WizardState,
    dirs: &DiagnosticsDirs,
    out_path: &PathBuf,
    written_paths: &mut Vec<PathBuf>,
    ts: u64,
) {
    push_artifact_result(
        out_path,
        written_paths,
        scan_context_json::write_scan_context_json(&dirs.scan, state, ts),
        "scan_context_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        step2_render_order_json::write_step2_render_order_json(&dirs.scan, state, ts),
        "step2_render_order_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        step2_component_audit_json::write_step2_component_audit_json(&dirs.scan, state, ts),
        "step2_component_audit_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        step2_component_audit_txt::write_step2_component_audit_txt(&dirs.scan, state, ts),
        "step2_component_audit_txt_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        parser_events_json::write_parser_events_json(&dirs.scan, state, ts),
        "parser_events_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        parser_raw_json::write_parser_raw_json(&dirs.scan, state, ts),
        "parser_raw_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        undefined_summary_json::write_undefined_summary_json(&dirs.scan, state, ts),
        "undefined_summary_json_write",
    );
}

fn write_compat_artifacts(
    state: &WizardState,
    dirs: &DiagnosticsDirs,
    out_path: &PathBuf,
    written_paths: &mut Vec<PathBuf>,
    ts: u64,
) {
    push_artifact_result(
        out_path,
        written_paths,
        compat_decisions_json::write_compat_decisions_json(&dirs.compat, state, ts),
        "compat_decisions_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        compat_rule_inventory_json::write_compat_rule_inventory_json(&dirs.compat, ts),
        "compat_rule_inventory_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        compat_rule_trace_json::write_compat_rule_trace_json(&dirs.compat, state, ts),
        "compat_rule_trace_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        compat_rule_matches_summary_json::write_compat_rule_matches_summary_json(&dirs.compat, ts),
        "compat_rule_matches_summary_json_write",
    );
    push_artifact_result(
        out_path,
        written_paths,
        step3_issue_snapshot_json::write_step3_issue_snapshot_json(&dirs.compat, state, ts),
        "step3_issue_snapshot_json_write",
    );
}

fn write_step4_weidu_log_snapshots(
    state: &WizardState,
    logs_dir: &std::path::Path,
) -> Result<Vec<DiagnosticLogGroup>> {
    let header = [
        "// Log of Currently Installed WeiDU Mods",
        "// The top of the file is the 'oldest' mod",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
    ];

    let write_group =
        |label: &str, file_name: &str, lines: Vec<String>| -> Result<DiagnosticLogGroup> {
            let dest_dir = logs_dir.join("Save WeiDU logs").join(label);
            fs::create_dir_all(&dest_dir)?;
            let dest = dest_dir.join(file_name);
            let mut out: Vec<String> = header
                .iter()
                .map(std::string::ToString::to_string)
                .collect();
            out.extend(lines);
            fs::write(&dest, out.join("\n"))?;
            Ok(DiagnosticLogGroup {
                label: format!("Save WeiDU logs/{label}"),
                copied_paths: vec![dest],
            })
        };

    let groups = match state.step1.game_install.as_str() {
        "EET" => vec![
            write_group(
                "BGEE",
                "weidu.log",
                build_weidu_export_lines(&state.step3.bgee_items),
            )?,
            write_group(
                "BG2EE",
                "weidu.log",
                build_weidu_export_lines(&state.step3.bg2ee_items),
            )?,
        ],
        "BG2EE" => vec![write_group(
            "BG2EE",
            "weidu.log",
            build_weidu_export_lines(&state.step3.bg2ee_items),
        )?],
        _ => vec![write_group(
            "BGEE",
            "weidu.log",
            build_weidu_export_lines(&state.step3.bgee_items),
        )?],
    };

    Ok(groups)
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
