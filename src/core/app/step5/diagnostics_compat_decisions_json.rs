// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::app::state::WizardState;

pub(super) fn write_compat_decisions_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("compat_decisions.json");
    let mut rows = Vec::<serde_json::Value>::new();
    for (tab, mods) in [
        ("BGEE", &state.step2.bgee_mods),
        ("BG2EE", &state.step2.bg2ee_mods),
    ] {
        for mod_state in mods {
            for component in &mod_state.components {
                if component.compat_kind.is_none()
                    && !component.disabled
                    && component.disabled_reason.is_none()
                {
                    continue;
                }
                rows.push(json!({
                    "tab": tab,
                    "mod_name": mod_state.name,
                    "tp_file": mod_state.tp_file,
                    "tp2_path": mod_state.tp2_path,
                    "component_id": component.component_id,
                    "label": component.label,
                    "checked": component.checked,
                    "disabled": component.disabled,
                    "disabled_reason": component.disabled_reason,
                    "compat_kind": component.compat_kind,
                    "compat_source": component.compat_source,
                    "compat_related_mod": component.compat_related_mod,
                    "compat_related_component": component.compat_related_component,
                    "compat_graph": component.compat_graph,
                    "compat_evidence": component.compat_evidence
                }));
            }
        }
    }
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "total_rows": rows.len(),
        "rows": rows
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
