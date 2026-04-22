// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::app::compat_step3_rules::{collect_step3_compat_markers, marker_key};
use crate::app::state::{Step2ModState, Step3ItemState, WizardState};

pub(super) fn write_step3_issue_snapshot_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("step3_issue_snapshot.json");
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "active_tab": state.step3.active_game_tab,
        "tabs": {
            "BGEE": serialize_tab("BGEE", state, &state.step2.bgee_mods, &state.step3.bgee_items),
            "BG2EE": serialize_tab("BG2EE", state, &state.step2.bg2ee_mods, &state.step3.bg2ee_items),
        }
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn serialize_tab(
    tab: &str,
    state: &WizardState,
    mods: &[Step2ModState],
    items: &[Step3ItemState],
) -> serde_json::Value {
    let markers = collect_step3_compat_markers(&state.step1, tab, mods, items);
    let conflict_count = markers
        .values()
        .filter(|marker| marker.kind.eq_ignore_ascii_case("conflict"))
        .count();
    let rows = items
        .iter()
        .enumerate()
        .map(|(row_index, item)| {
            let marker = (!item.is_parent)
                .then(|| markers.get(&marker_key(item)))
                .flatten();
            json!({
                "row_index": row_index,
                "is_parent": item.is_parent,
                "tp_file": item.tp_file,
                "component_id": item.component_id,
                "mod_name": item.mod_name,
                "component_label": item.component_label,
                "block_id": item.block_id,
                "selected_order": item.selected_order,
                "marker": marker.map(|value| json!({
                    "kind": value.kind,
                    "message": value.message,
                    "related_mod": value.related_mod,
                    "related_component": value.related_component,
                    "source": value.source,
                    "raw_evidence": value.raw_evidence,
                })),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "item_count": items.len(),
        "marker_count": markers.len(),
        "conflict_count": conflict_count,
        "has_conflict": conflict_count > 0,
        "rows": rows,
    })
}
