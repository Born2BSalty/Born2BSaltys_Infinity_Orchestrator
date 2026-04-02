// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::WizardState;

pub(super) fn write_step2_component_audit_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("step2_component_audit.json");

    let serialize_tab = |tab_name: &str, mods: &[crate::ui::state::Step2ModState]| {
        mods.iter()
            .enumerate()
            .map(|(mod_index, mod_state)| {
                json!({
                    "tab": tab_name,
                    "mod_index": mod_index,
                    "mod_name": mod_state.name,
                    "tp_file": mod_state.tp_file,
                    "tp2_path": mod_state.tp2_path,
                    "raw_count": mod_state.components.len() + mod_state.hidden_components.len(),
                    "shown_count": mod_state.components.len(),
                    "hidden_count": mod_state.hidden_components.len(),
                    "shown_components": mod_state.components.iter().map(|component| json!({
                        "component_id": component.component_id,
                        "label": component.label,
                    })).collect::<Vec<_>>(),
                    "hidden_components": mod_state.hidden_components.iter().map(|component| json!({
                        "component_id": component.component_id,
                        "label": component.label,
                        "raw_line": component.raw_line,
                        "reason": component.reason,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>()
    };

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "tabs": {
            "BGEE": serialize_tab("BGEE", &state.step2.bgee_mods),
            "BG2EE": serialize_tab("BG2EE", &state.step2.bg2ee_mods),
        }
    });

    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
