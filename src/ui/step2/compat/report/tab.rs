// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step2ComponentState, Step2ModState};

pub(super) fn append_tab_report(tab_name: &str, mods: &[Step2ModState], out: &mut String) {
    let mut conflicts = 0usize;
    let mut warnings = 0usize;
    let mut included = 0usize;
    let mut disabled = 0usize;
    let mut total_components = 0usize;

    for mod_state in mods {
        for component in &mod_state.components {
            total_components = total_components.saturating_add(1);
            if component.disabled {
                disabled = disabled.saturating_add(1);
            }
            match component.compat_kind.as_deref().unwrap_or_default() {
                "conflict" | "not_compatible" => conflicts = conflicts.saturating_add(1),
                "warning" => warnings = warnings.saturating_add(1),
                "included" | "not_needed" => included = included.saturating_add(1),
                _ => {}
            }
        }
    }

    out.push_str(&format!("[{tab_name}]\n"));
    out.push_str(&format!("Mods: {}\n", mods.len()));
    out.push_str(&format!("Components: {total_components}\n"));
    out.push_str(&format!(
        "Conflicts: {conflicts} | Warnings: {warnings} | Included: {included} | Disabled: {disabled}\n\n"
    ));

    for mod_state in mods {
        let mod_has_compat = mod_state.components.iter().any(|c| c.compat_kind.is_some());
        if !mod_has_compat {
            continue;
        }
        out.push_str(&format!("- {}\n", mod_state.name));
        out.push_str(&format!("  TP2: {}\n", mod_state.tp_file));
        for component in &mod_state.components {
            append_component_report(component, out);
        }
        out.push('\n');
    }
}

fn append_component_report(component: &Step2ComponentState, out: &mut String) {
    let Some(kind) = component.compat_kind.as_deref() else {
        return;
    };
    out.push_str(&format!(
        "  * #{} {} [{}]\n",
        component.component_id, component.label, kind
    ));
    out.push_str(&format!(
        "    state: {} | checked: {}\n",
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
        let related =
            if let Some(related_component) = component.compat_related_component.as_deref() {
                format!("{related_mod} #{related_component}")
            } else {
                related_mod.to_string()
            };
        out.push_str(&format!("    related: {related}\n"));
    }
}
