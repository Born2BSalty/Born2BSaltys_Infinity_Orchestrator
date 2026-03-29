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
    append_mapped_for_tab("BGEE", &state.step2.bgee_mods, &mut mapped_by_tp2);
    append_mapped_for_tab("BG2EE", &state.step2.bg2ee_mods, &mut mapped_by_tp2);

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
                "parser_diagnostic_preview": tp2_run.parser_diagnostic_preview,
                "parser_tra_language_requested": tp2_run.parser_tra_language_requested,
                "parser_tra_language_used": tp2_run.parser_tra_language_used,
                "parser_flow_node_count": tp2_run.parser_flow_node_count,
                "parser_flow_event_ref_count": tp2_run.parser_flow_event_ref_count,
                "parser_event_with_parent_count": tp2_run.parser_event_with_parent_count,
                "parser_event_with_path_count": tp2_run.parser_event_with_path_count,
                "parser_option_component_binding_count": tp2_run.parser_option_component_binding_count,
                "parser_flow_preview": tp2_run.parser_flow_preview.iter().map(|(id, label)| json!({
                    "id": id,
                    "label": label
                })).collect::<Vec<_>>()
            },
            "mapped_prompts_in_ui": &mapped_prompts,
            "parser": {
                "status": "not_rerun_in_diagnostics",
                "note": "Raw parser replay is intentionally skipped during diagnostics export to keep UI responsive. When available, the original Lapdu parser JSON is exported separately under parser_raw/. Use that plus prompt_calls.json for issue triage."
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
    tab: &str,
    mods: &[Step2ModState],
    out: &mut HashMap<String, Vec<serde_json::Value>>,
) {
    for mod_state in mods {
        let mut component_rows = Vec::<serde_json::Value>::new();
        let mod_summary = mod_state
            .mod_prompt_summary
            .as_deref()
            .map(str::trim)
            .filter(|summary| !summary.is_empty())
            .map(str::to_string);
        for c in &mod_state.components {
            let summary = c
                .prompt_summary
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            if summary.is_none() && c.prompt_events.is_empty() {
                continue;
            }
            component_rows.push(json!({
                "component_id": c.component_id,
                "component_label": c.label,
                "summary": summary,
                "prompt_event_count": c.prompt_events.len(),
                "prompt_events": &c.prompt_events,
            }));
        }
        if mod_summary.is_none() && mod_state.mod_prompt_events.is_empty() && component_rows.is_empty() {
            continue;
        }
        let key = normalize_path(&mod_state.tp2_path);
        out.entry(key).or_default().push(json!({
            "tab": tab,
            "mod_name": mod_state.name,
            "tp_file": mod_state.tp_file,
            "tp2_path": mod_state.tp2_path,
            "mod_prompt_summary": mod_summary,
            "mod_prompt_event_count": mod_state.mod_prompt_events.len(),
            "mod_prompt_events": &mod_state.mod_prompt_events,
            "component_prompts": component_rows,
        }));
    }
}

fn normalize_path(input: &str) -> String {
    input.trim().replace('\\', "/").to_ascii_lowercase()
}
