// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

use super::super::undefined_detect::looks_like_undefined_signal;

pub(super) fn append_step2_sections(out: &mut String, state: &WizardState) {
    append_step2_summary(out, state);
    append_step2_selected_components(out, state);
    append_step2_scan_undefined_signals(out, state);
    append_wlb_inputs_map(out, state);
}

fn append_step2_summary(out: &mut String, state: &WizardState) {
    out.push_str("[Step2]\n");
    out.push_str(&format!("selected_count={}\n", state.step2.selected_count));
    out.push_str(&format!("total_count={}\n", state.step2.total_count));
    out.push_str(&format!("active_tab={}\n\n", state.step2.active_game_tab));
}

fn append_step2_selected_components(out: &mut String, state: &WizardState) {
    out.push_str("[Step2 Selected Components]\n");
    let mut listed = 0usize;
    for (tab, mods) in [("BGEE", &state.step2.bgee_mods), ("BG2EE", &state.step2.bg2ee_mods)] {
        for mod_state in mods {
            for component in &mod_state.components {
                if !component.checked {
                    continue;
                }
                listed = listed.saturating_add(1);
                out.push_str(&format!(
                    "{tab} | {} #{} | {}\n",
                    mod_state.tp_file, component.component_id, component.label
                ));
            }
        }
    }
    if listed == 0 {
        out.push_str("none\n");
    }
    out.push('\n');
}

fn append_step2_scan_undefined_signals(out: &mut String, state: &WizardState) {
    out.push_str("[Step2 Scan Undefined Signals]\n");
    let mut listed = 0usize;
    for (tab, mods) in [("BGEE", &state.step2.bgee_mods), ("BG2EE", &state.step2.bg2ee_mods)] {
        for mod_state in mods {
            for component in &mod_state.components {
                let label_hit = looks_like_scan_undefined(&component.label);
                let raw_hit = looks_like_scan_undefined(&component.raw_line);
                if !(label_hit || raw_hit) {
                    continue;
                }
                listed = listed.saturating_add(1);
                out.push_str(&format!(
                    "{tab} | {} | tp2={} | #{} | label={} | raw={}\n",
                    mod_state.name,
                    mod_state.tp2_path,
                    component.component_id,
                    component.label,
                    component.raw_line
                ));
            }
        }
    }
    if listed == 0 {
        out.push_str("none\n");
    } else if listed > 200 {
        out.push_str("note=high count; consider narrowing scan set\n");
    }
    out.push('\n');
}

fn looks_like_scan_undefined(text: &str) -> bool {
    looks_like_undefined_signal(text)
}

fn append_wlb_inputs_map(out: &mut String, state: &WizardState) {
    out.push_str("[@wlb-inputs Map]\n");
    let mut listed = 0usize;
    for (tab, items) in [("BGEE", &state.step3.bgee_items), ("BG2EE", &state.step3.bg2ee_items)] {
        for item in items {
            if item.is_parent {
                continue;
            }
            let Some(inputs) = extract_wlb_inputs(&item.raw_line) else {
                continue;
            };
            listed = listed.saturating_add(1);
            out.push_str(&format!(
                "{tab} | {} #{} | {inputs}\n",
                item.tp_file, item.component_id
            ));
        }
    }
    if listed == 0 {
        out.push_str("none\n");
    }
    out.push('\n');
}

fn extract_wlb_inputs(raw_line: &str) -> Option<String> {
    let marker = "@wlb-inputs:";
    let lower = raw_line.to_ascii_lowercase();
    let start = lower.find(marker)?;
    let tail = raw_line[start + marker.len()..].trim();
    if tail.is_empty() {
        None
    } else {
        Some(tail.to_string())
    }
}
