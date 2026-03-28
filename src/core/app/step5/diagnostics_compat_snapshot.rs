// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ComponentState, Step2ModState, WizardState};

pub(super) fn append_dev_compat_snapshots(state: &WizardState, out: &mut String) {
    out.push_str("\n[Step2 Compatibility Snapshot]\n");
    append_step2_compat_snapshot("BGEE", &state.step2.bgee_mods, out);
    append_step2_compat_snapshot("BG2EE", &state.step2.bg2ee_mods, out);
}

fn append_step2_compat_snapshot(tab: &str, mods: &[Step2ModState], out: &mut String) {
    let mut listed = 0usize;
    out.push_str(&format!("{tab}:\n"));
    for mod_state in mods {
        for component in &mod_state.components {
            if component.compat_kind.is_none() {
                continue;
            }
            listed = listed.saturating_add(1);
            append_step2_component(mod_state, component, out);
        }
    }
    if listed == 0 {
        out.push_str("  no compat markers\n");
    }
}

fn append_step2_component(mod_state: &Step2ModState, component: &Step2ComponentState, out: &mut String) {
    let kind = component.compat_kind.as_deref().unwrap_or("none");
    out.push_str(&format!(
        "  - {} #{} [{}] {}\n",
        mod_state.name, component.component_id, kind, component.label
    ));
    out.push_str(&format!(
        "    state={} checked={}\n",
        if component.disabled { "disabled" } else { "selectable" },
        if component.checked { "yes" } else { "no" }
    ));
    if let Some(reason) = component.disabled_reason.as_deref()
        && !reason.trim().is_empty()
    {
        out.push_str(&format!("    reason: {reason}\n"));
    }
    if let Some(source) = component.compat_source.as_deref()
        && !source.trim().is_empty()
    {
        out.push_str(&format!("    source: {source}\n"));
    }
    if let Some(related_mod) = component.compat_related_mod.as_deref() {
        let related = if let Some(related_component) = component.compat_related_component.as_deref() {
            format!("{related_mod} #{related_component}")
        } else {
            related_mod.to_string()
        };
        out.push_str(&format!("    related: {related}\n"));
    }
}
