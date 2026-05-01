// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::app::state::{Step2ModState, Step2Selection, WizardState};

pub(super) fn write_step2_render_order_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("step2_render_order.json");

    let serialize_tab = |tab_name: &str, mods: &[Step2ModState]| {
        mods.iter()
            .enumerate()
            .map(|(mod_index, mod_state)| {
                json!({
                    "tab": tab_name,
                    "mod_index": mod_index,
                    "mod_name": mod_state.name,
                    "tp_file": mod_state.tp_file,
                    "tp2_path": mod_state.tp2_path,
                    "checked": mod_state.checked,
                    "component_count": mod_state.components.len(),
                    "components": mod_state.components.iter().enumerate().map(|(component_index, component)| json!({
                        "component_index": component_index,
                        "component_id": component.component_id,
                        "label": component.label,
                        "checked": component.checked,
                        "selected_order": component.selected_order,
                        "disabled": component.disabled,
                        "weidu_group": component.weidu_group,
                        "collapsible_group": component.collapsible_group,
                        "collapsible_group_is_umbrella": component.collapsible_group_is_umbrella,
                        "is_meta_mode_component": component.is_meta_mode_component,
                        "compat_kind": component.compat_kind,
                    })).collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
    };

    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "active_tab": state.step2.active_game_tab,
        "search_query": state.step2.search_query,
        "selected": match &state.step2.selected {
            Some(Step2Selection::Mod { game_tab, tp_file }) => json!({
                "kind": "mod",
                "game_tab": game_tab,
                "tp_file": tp_file,
            }),
            Some(Step2Selection::Component { game_tab, tp_file, component_id, component_key }) => json!({
                "kind": "component",
                "game_tab": game_tab,
                "tp_file": tp_file,
                "component_id": component_id,
                "component_key": component_key,
            }),
            None => serde_json::Value::Null,
        },
        "tabs": {
            "BGEE": serialize_tab("BGEE", &state.step2.bgee_mods),
            "BG2EE": serialize_tab("BG2EE", &state.step2.bg2ee_mods),
        }
    });

    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
