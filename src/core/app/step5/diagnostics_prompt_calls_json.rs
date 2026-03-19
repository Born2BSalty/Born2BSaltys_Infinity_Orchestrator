// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::{Step2ModState, Step2ScanReport, WizardState};

pub(super) fn write_prompt_calls_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("prompt_calls.json");

    let report_by_tp2 = state
        .step2
        .last_scan_report
        .as_ref()
        .map(tp2_report_map)
        .unwrap_or_default();

    let mut groups = Vec::<serde_json::Value>::new();
    append_groups_for_tab("BGEE", &state.step2.bgee_mods, &report_by_tp2, &mut groups);
    append_groups_for_tab(
        "BG2EE",
        &state.step2.bg2ee_mods,
        &report_by_tp2,
        &mut groups,
    );

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "status": if state.step2.last_scan_report.is_some() { "ok" } else { "missing_scan_report" },
        "groups": groups
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn tp2_report_map(report: &Step2ScanReport) -> HashMap<String, serde_json::Value> {
    let mut out = HashMap::<String, serde_json::Value>::new();
    for r in &report.tp2_reports {
        out.insert(
            normalize_path(&r.tp2_path),
            json!({
                "used_cache": r.used_cache,
                "selected_language_id": r.selected_language_id,
                "language_ids_tried": r.language_ids_tried,
                "parsed_count": r.parsed_count,
                "undefined_count": r.undefined_count,
                "parser_source_file": r.parser_source_file,
                "parser_event_count": r.parser_event_count,
                "parser_warning_count": r.parser_warning_count,
                "parser_error_count": r.parser_error_count,
                "parser_diagnostic_preview": r.parser_diagnostic_preview
            }),
        );
    }
    out
}

fn append_groups_for_tab(
    tab: &str,
    mods: &[Step2ModState],
    report_by_tp2: &HashMap<String, serde_json::Value>,
    groups: &mut Vec<serde_json::Value>,
) {
    for mod_state in mods {
        let mut prompt_calls = Vec::<serde_json::Value>::new();

        if let Some(summary) = mod_state.mod_prompt_summary.as_deref() {
            let summary = summary.trim();
            if !summary.is_empty() {
                prompt_calls.push(json!({
                    "scope": "mod",
                    "component_id": null,
                    "component_label": null,
                    "summary": summary
                }));
            }
        }

        for component in &mod_state.components {
            if let Some(summary) = component.prompt_summary.as_deref() {
                let summary = summary.trim();
                if summary.is_empty() {
                    continue;
                }
                prompt_calls.push(json!({
                    "scope": "component",
                    "component_id": component.component_id,
                    "component_label": component.label,
                    "summary": summary
                }));
            }
        }

        if prompt_calls.is_empty() {
            continue;
        }

        let tp2_key = normalize_path(&mod_state.tp2_path);
        let parser_meta = report_by_tp2
            .get(&tp2_key)
            .cloned()
            .unwrap_or_else(|| json!({}));

        groups.push(json!({
            "tab": tab,
            "mod_name": mod_state.name,
            "tp_file": mod_state.tp_file,
            "tp2_path": mod_state.tp2_path,
            "prompt_calls_count": prompt_calls.len(),
            "prompt_calls": prompt_calls,
            "parser_meta": parser_meta
        }));
    }
}

fn normalize_path(input: &str) -> String {
    input.trim().replace('\\', "/").to_ascii_lowercase()
}
