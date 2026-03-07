// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::WizardState;

pub(super) fn write_scan_context_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("scan_context.json");
    let payload = if let Some(report) = &state.step2.last_scan_report {
        json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "status": "ok",
            "context": {
                "game_dir": report.game_dir,
                "mods_root": report.mods_root,
                "scan_depth": report.scan_depth,
                "worker_count": report.worker_count,
                "total_groups": report.total_groups,
                "total_tp2": report.total_tp2,
                "tp2_cache_hits": report.tp2_cache_hits,
                "tp2_cache_misses": report.tp2_cache_misses,
                "preferred_locale": report.preferred_locale,
                "preferred_locale_source": report.preferred_locale_source,
                "preferred_locale_baldur_lua": report.preferred_locale_baldur_lua,
            },
            "tp2_runs": report.tp2_reports.iter().map(|r| json!({
                "group_label": r.group_label,
                "tp2_path": r.tp2_path,
                "work_dir": r.work_dir,
                "used_cache": r.used_cache,
                "selected_from_cache": r.selected_from_cache,
                "language_ids_tried": r.language_ids_tried,
                "selected_language_id": r.selected_language_id,
                "parsed_count": r.parsed_count,
                "undefined_count": r.undefined_count,
                "parser_source_file": r.parser_source_file,
                "parser_event_count": r.parser_event_count,
                "parser_warning_count": r.parser_warning_count,
                "parser_error_count": r.parser_error_count,
                "parser_diagnostic_preview": r.parser_diagnostic_preview
            })).collect::<Vec<_>>()
        })
    } else {
        json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "status": "missing_scan_report",
            "reason": if state.step1.have_weidu_logs {
                "Step2 scan skipped because Have WeiDU Logs was enabled."
            } else {
                "No Step2 scan report is present in state."
            }
        })
    };
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
