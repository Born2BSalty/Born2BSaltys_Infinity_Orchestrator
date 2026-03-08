// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::WizardState;

pub(super) fn write_parser_raw_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("parser_raw_manifest.json");
    let Some(report) = state.step2.last_scan_report.as_ref() else {
        fs::write(
            &out_path,
            serde_json::to_string_pretty(&json!({
                "schema_version": 1,
                "generated_at_unix": timestamp_unix_secs,
                "status": "missing_scan_report",
                "files": []
            }))?,
        )?;
        return Ok(out_path);
    };

    let raw_dir = run_dir.join("parser_raw");
    fs::create_dir_all(&raw_dir)?;
    let mut files = Vec::<serde_json::Value>::new();
    for tp2 in &report.tp2_reports {
        let Some(raw_json) = tp2.parser_raw_json.as_deref() else {
            continue;
        };
        let raw_json = raw_json.trim();
        if raw_json.is_empty() {
            continue;
        }
        let file_name = parser_raw_file_name(&tp2.tp2_path);
        let file_path = raw_dir.join(&file_name);
        fs::write(&file_path, raw_json)?;
        files.push(json!({
            "tp2_path": tp2.tp2_path,
            "file": format!("parser_raw/{file_name}"),
            "parser_source_file": tp2.parser_source_file,
            "parser_event_count": tp2.parser_event_count,
            "parser_warning_count": tp2.parser_warning_count,
            "parser_error_count": tp2.parser_error_count,
            "parser_tra_language_requested": tp2.parser_tra_language_requested,
            "parser_tra_language_used": tp2.parser_tra_language_used,
            "parser_flow_node_count": tp2.parser_flow_node_count,
            "parser_flow_event_ref_count": tp2.parser_flow_event_ref_count,
            "parser_event_with_parent_count": tp2.parser_event_with_parent_count,
            "parser_event_with_path_count": tp2.parser_event_with_path_count,
            "parser_option_component_binding_count": tp2.parser_option_component_binding_count,
            "parser_flow_preview": tp2.parser_flow_preview.iter().map(|(id, label)| json!({
                "id": id,
                "label": label
            })).collect::<Vec<_>>()
        }));
    }

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "status": "ok",
        "files": files
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn parser_raw_file_name(tp2_path: &str) -> String {
    let base = Path::new(tp2_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("parser")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();
    let mut hasher = DefaultHasher::new();
    tp2_path.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{base}_{hash:016x}.json")
}
