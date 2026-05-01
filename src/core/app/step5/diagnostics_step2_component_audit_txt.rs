// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::app::state::{Step2ModState, WizardState};

pub(super) fn write_step2_component_audit_txt(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("step2_component_audit.txt");
    let mut text = String::new();
    text.push_str("BIO Step 2 component audit\n");
    text.push_str("==========================\n\n");
    let _ = writeln!(text, "generated_at_unix={timestamp_unix_secs}");
    let _ = writeln!(text, "active_tab={}", state.step2.active_game_tab);
    let _ = writeln!(text, "search_query={}", state.step2.search_query);
    text.push('\n');

    append_tab(&mut text, "BGEE", &state.step2.bgee_mods);
    append_tab(&mut text, "BG2EE", &state.step2.bg2ee_mods);

    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn append_tab(text: &mut String, tab_name: &str, mods: &[Step2ModState]) {
    let _ = writeln!(text, "[{tab_name}]");
    text.push('\n');
    for mod_state in mods {
        let raw_count = mod_state.components.len() + mod_state.hidden_components.len();
        let _ = writeln!(
            text,
            "- {} | tp_file={} | raw={} shown={} hidden={}",
            mod_state.name,
            mod_state.tp_file,
            raw_count,
            mod_state.components.len(),
            mod_state.hidden_components.len()
        );
        let _ = writeln!(text, "  tp2_path={}", mod_state.tp2_path);
        text.push_str("  shown_components:\n");
        if mod_state.components.is_empty() {
            text.push_str("    - none\n");
        } else {
            for component in &mod_state.components {
                let _ = writeln!(
                    text,
                    "    - #{} {}",
                    component.component_id, component.label
                );
            }
        }
        text.push_str("  hidden_components:\n");
        if mod_state.hidden_components.is_empty() {
            text.push_str("    - none\n");
        } else {
            for component in &mod_state.hidden_components {
                let _ = writeln!(
                    text,
                    "    - #{} {} [{}]",
                    component.component_id, component.label, component.reason
                );
            }
        }
        text.push('\n');
    }
}
