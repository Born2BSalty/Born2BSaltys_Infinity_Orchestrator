// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::WizardState;

use super::undefined_detect::looks_like_undefined_signal;

pub(super) fn write_undefined_summary_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("undefined_summary.json");
    let payload = if let Some(report) = &state.step2.last_scan_report {
        let rows = report
            .tp2_reports
            .iter()
            .filter(|r| r.undefined_count > 0)
            .map(|r| {
                json!({
                    "tp2_path": r.tp2_path,
                    "selected_language_id": r.selected_language_id,
                    "work_dir": r.work_dir,
                    "undefined_count": r.undefined_count,
                    "parsed_count": r.parsed_count
                })
            })
            .collect::<Vec<_>>();
        let total_undefined = report.tp2_reports.iter().map(|r| r.undefined_count).sum::<usize>();
        json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "source": "scan_report",
            "tp2_with_undefined": rows.len(),
            "total_undefined_components": total_undefined,
            "rows": rows
        })
    } else {
        let mut grouped = BTreeMap::<String, usize>::new();
        for mods in [&state.step2.bgee_mods, &state.step2.bg2ee_mods] {
            for mod_state in mods {
                let mut hits = 0usize;
                for component in &mod_state.components {
                    if looks_like_undefined_signal(&component.label)
                        || looks_like_undefined_signal(&component.raw_line)
                    {
                        hits = hits.saturating_add(1);
                    }
                }
                if hits > 0 {
                    *grouped.entry(mod_state.tp2_path.clone()).or_default() += hits;
                }
            }
        }
        let rows = grouped
            .into_iter()
            .map(|(tp2_path, undefined_count)| {
                json!({
                    "tp2_path": tp2_path,
                    "undefined_count": undefined_count
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "source": "step2_fallback",
            "tp2_with_undefined": rows.len(),
            "rows": rows
        })
    };
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
