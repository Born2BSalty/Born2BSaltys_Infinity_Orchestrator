// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::{Step2ModState, WizardState};

pub(super) fn write_parser_events_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("parser_events.json");

    let Some(report) = state.step2.last_scan_report.as_ref() else {
        let payload = json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "status": "missing_scan_report",
            "groups": []
        });
        fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
        return Ok(out_path);
    };

    let mut mapped_by_tp2 = HashMap::<String, Vec<serde_json::Value>>::new();
    append_mapped_for_tab(&state.step2.bgee_mods, &mut mapped_by_tp2);
    append_mapped_for_tab(&state.step2.bg2ee_mods, &mut mapped_by_tp2);

    let tp2_run_by_key = report
        .tp2_reports
        .iter()
        .map(|r| (normalize_path(&r.tp2_path), r))
        .collect::<HashMap<_, _>>();

    let mut groups = Vec::<serde_json::Value>::new();
    for (key, mapped_prompts) in mapped_by_tp2 {
        let Some(tp2_run) = tp2_run_by_key.get(&key) else {
            continue;
        };

        groups.push(json!({
            "group_label": tp2_run.group_label,
            "tp2_path": tp2_run.tp2_path,
            "scan_probe_meta": {
                "used_cache": tp2_run.used_cache,
                "selected_from_cache": tp2_run.selected_from_cache,
                "selected_language_id": tp2_run.selected_language_id,
                "language_ids_tried": tp2_run.language_ids_tried,
                "parsed_count": tp2_run.parsed_count,
                "undefined_count": tp2_run.undefined_count,
                "parser_source_file": tp2_run.parser_source_file,
                "parser_event_count": tp2_run.parser_event_count,
                "parser_warning_count": tp2_run.parser_warning_count,
                "parser_error_count": tp2_run.parser_error_count,
                "parser_diagnostic_preview": tp2_run.parser_diagnostic_preview
            },
            "mapped_prompts_in_ui": &mapped_prompts,
            "parser": {
                "status": "not_rerun_in_diagnostics",
                "note": "Raw parser event replay is intentionally skipped during diagnostics export to keep UI responsive. Use parser counts/diagnostics above plus prompt_calls.json for issue triage."
            }
        }));
    }

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "status": "ok",
        "context": {
            "mods_root": report.mods_root,
            "preferred_locale": report.preferred_locale,
            "preferred_game": state.step1.game_install
        },
        "groups": groups
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn append_mapped_for_tab(
    mods: &[Step2ModState],
    out: &mut HashMap<String, Vec<serde_json::Value>>,
) {
    for mod_state in mods {
        let mut row = Vec::<serde_json::Value>::new();
        if let Some(summary) = mod_state.mod_prompt_summary.as_deref() {
            let summary = summary.trim();
            if !summary.is_empty() {
                row.push(json!({
                    "scope": "mod",
                    "component_id": null,
                    "component_label": null,
                    "summary": summary
                }));
            }
        }
        for c in &mod_state.components {
            if let Some(summary) = c.prompt_summary.as_deref() {
                let summary = summary.trim();
                if summary.is_empty() {
                    continue;
                }
                row.push(json!({
                    "scope": "component",
                    "component_id": c.component_id,
                    "component_label": c.label,
                    "summary": summary
                }));
            }
        }
        if row.is_empty() {
            continue;
        }
        let key = normalize_path(&mod_state.tp2_path);
        out.insert(key, row);
    }
}

fn normalize_path(input: &str) -> String {
    input.trim().replace('\\', "/").to_ascii_lowercase()
}
