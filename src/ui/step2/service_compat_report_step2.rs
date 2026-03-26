// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod compat_report_format {
use crate::ui::state::CompatIssueDisplay;

pub(super) fn format_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}

pub(super) fn issue_graph(issue: &CompatIssueDisplay) -> String {
    let affected = format_target(&issue.affected_mod, issue.affected_component);
    let related = format_target(&issue.related_mod, issue.related_component);
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = parse_or_targets_from_reason(&issue.reason) {
            return format!("{affected} requires one of: {}", or_targets.join(" | "));
        }
        return format!("{affected} requires {related}");
    }
    if issue.code.eq_ignore_ascii_case("INCLUDED") {
        return format!("{affected} is included by {related}");
    }
    format!("{affected} -> {related}")
}

fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
    let prefix = "Requires one of:";
    let body = reason.strip_prefix(prefix)?.trim();
    let parts = body
        .split(" OR ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() > 1 {
        Some(parts)
    } else {
        None
    }
}

}

mod compat_report_selected_set {
use crate::ui::state::CompatState;

use super::compat_report_format::{format_target, issue_graph};

pub(super) fn append_selected_set_validator_report(compat: &CompatState, out: &mut String) {
    out.push_str("[Selected-Set Validator]\n");
    out.push_str(&format!(
        "Errors: {} | Warnings: {}\n\n",
        compat.error_count, compat.warning_count
    ));
    if compat.issues.is_empty() {
        out.push_str("No validator issues.\n\n");
        return;
    }
    for issue in &compat.issues {
        let sev = if issue.is_blocking { "ERROR" } else { "WARN" };
        let affected = format_target(&issue.affected_mod, issue.affected_component);
        let related = format_target(&issue.related_mod, issue.related_component);
        out.push_str(&format!("- [{sev}] {} {} -> {}\n", issue.code, affected, related));
        if !issue.reason.trim().is_empty() {
            out.push_str(&format!("  reason: {}\n", issue.reason));
        }
        if !issue.source.trim().is_empty() {
            out.push_str(&format!("  source: {}\n", issue.source));
        }
        out.push_str(&format!("  graph: {}\n", issue_graph(issue)));
        if let Some(raw) = issue.raw_evidence.as_deref()
            && !raw.trim().is_empty()
        {
            out.push_str(&format!("  rule_detail: {raw}\n"));
        }
    }
    out.push('\n');
}

}

mod compat_report_tab {
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

}

mod compat_report {
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::{CompatState, Step2State};

pub fn export_step2_compat_report(step2: &Step2State, compat: &CompatState) -> std::io::Result<PathBuf> {
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let out_path = out_dir.join(format!("compat_step2_{ts}.txt"));

    let mut text = String::new();
    text.push_str("Step 2 Compatibility Report\n");
    text.push_str("===========================\n\n");
    text.push_str(&format!("Active tab: {}\n\n", step2.active_game_tab));

    super::compat_report_tab::append_tab_report("BGEE", &step2.bgee_mods, &mut text);
    super::compat_report_tab::append_tab_report("BG2EE", &step2.bg2ee_mods, &mut text);
    super::compat_report_selected_set::append_selected_set_validator_report(compat, &mut text);

    fs::write(&out_path, text)?;
    Ok(out_path)
}

}


pub(crate) fn export_compat_report(step2: &crate::ui::state::Step2State, compat: &crate::ui::state::CompatState) -> std::io::Result<std::path::PathBuf> {
    compat_report::export_step2_compat_report(step2, compat)
}
