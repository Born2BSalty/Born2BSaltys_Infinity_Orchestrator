// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app::state::{Step2ModState, WizardState};

use super::write_checks::WriteCheckSummary;

pub(super) fn write_quick_triage_txt(
    run_dir: &Path,
    state: &WizardState,
    write_check_summary: &WriteCheckSummary,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("quick_triage.txt");
    let first_failure = detect_first_failure(state, write_check_summary);
    let mut text = String::new();
    text.push_str("BIO quick triage\n");
    text.push_str("================\n\n");
    text.push_str(&format!("generated_at_unix={timestamp_unix_secs}\n"));
    text.push_str(&format!("first_failure={first_failure}\n"));
    text.push_str(&format!(
        "step1_path_check={}\n",
        match &state.step1_path_check {
            Some((ok, _)) => {
                if *ok { "ok" } else { "failed" }
            }
            None => "not_run",
        }
    ));
    text.push_str(&format!(
        "step2_status={}\n",
        state.step2.scan_status.trim()
    ));
    text.push_str(&format!(
        "install_exit_code={:?}\n",
        state.step5.last_exit_code
    ));
    text.push_str(&format!(
        "install_status={}\n\n",
        state.step5.last_status_text.trim()
    ));
    text.push_str("open_these_files_first:\n");
    text.push_str("- bio_diag.txt\n");
    text.push_str("- scan_context.json\n");
    text.push_str("- step2_render_order.json\n");
    text.push_str("- step3_issue_snapshot.json\n");
    text.push_str("- runtime_assumptions.json\n");
    text.push_str("- undefined_summary.json\n");
    text.push_str("- compat_decisions.json\n");
    text.push_str("- compat_rule_inventory.json\n");
    text.push_str("- compat_rule_trace.json\n");
    text.push_str("- compat_rule_matches_summary.json\n");
    text.push_str("- logs/\n");
    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn detect_first_failure(state: &WizardState, write_check_summary: &WriteCheckSummary) -> String {
    if let Some((ok, msg)) = &state.step1_path_check
        && !ok
    {
        return format!("step1_validation_failed: {msg}");
    }
    if let Some(line) = first_write_failure(write_check_summary) {
        return format!("write_check_failed: {line}");
    }
    let step2_status = state.step2.scan_status.to_ascii_lowercase();
    if step2_status.contains("scan failed") {
        return format!("step2_scan_failed: {}", state.step2.scan_status.trim());
    }
    if !state.step1.have_weidu_logs
        && state.current_step > 1
        && state.step2.last_scan_report.is_none()
    {
        return "step2_scan_report_missing".to_string();
    }
    if let Some(conflict) = first_step2_conflict(state) {
        return conflict;
    }
    if state.step3.bgee_has_conflict || state.step3.bg2ee_has_conflict {
        return format!(
            "step3_conflict_present: BGEE={} BG2EE={}",
            state.step3.bgee_has_conflict, state.step3.bg2ee_has_conflict
        );
    }
    if let Some(code) = state.step5.last_exit_code
        && code != 0
    {
        return format!("step5_install_failed: exit_code={code}");
    }
    "none".to_string()
}

fn first_write_failure(summary: &WriteCheckSummary) -> Option<String> {
    summary
        .lines
        .iter()
        .find(|line| line.starts_with("FAIL |"))
        .cloned()
}

fn first_step2_conflict(state: &WizardState) -> Option<String> {
    for (tab, mods) in [
        ("BGEE", &state.step2.bgee_mods),
        ("BG2EE", &state.step2.bg2ee_mods),
    ] {
        if let Some(hit) = first_conflict_in_mods(mods) {
            return Some(format!("step2_compat_conflict: {tab} | {hit}"));
        }
    }
    None
}

fn first_conflict_in_mods(mods: &[Step2ModState]) -> Option<String> {
    for mod_state in mods {
        for component in &mod_state.components {
            if !component.checked || component.compat_kind.as_deref() != Some("conflict") {
                continue;
            }
            return Some(format!(
                "{} #{} | {}",
                mod_state.tp_file, component.component_id, component.label
            ));
        }
    }
    None
}
