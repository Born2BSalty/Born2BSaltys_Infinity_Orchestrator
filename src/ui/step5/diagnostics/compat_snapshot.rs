// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step2ComponentState, Step2ModState, WizardState};

use super::compat_summary::build_compat_summary;
use super::format::{format_target, issue_graph};

pub(super) fn append_dev_compat_snapshots(state: &WizardState, out: &mut String) {
    out.push_str("\n[Step2 Compatibility Snapshot]\n");
    append_step2_compat_snapshot("BGEE", &state.step2.bgee_mods, out);
    append_step2_compat_snapshot("BG2EE", &state.step2.bg2ee_mods, out);

    out.push_str("\n[Step3 Compatibility Snapshot]\n");
    append_step3_compat_snapshot(&state.compat.issues, out);
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

fn append_step3_compat_snapshot(issues: &[CompatIssueDisplay], out: &mut String) {
    if issues.is_empty() {
        out.push_str("  no issues\n");
        return;
    }
    append_step3_compact_summary(issues, out);
    out.push_str("  -- Full Issue List --\n");
    for issue in issues {
        let sev = if issue.is_blocking { "ERROR" } else { "WARN" };
        let affected = format_target(&issue.affected_mod, issue.affected_component);
        let related = format_target(&issue.related_mod, issue.related_component);
        out.push_str(&format!("  - [{sev}] {} {} -> {}\n", issue.code, affected, related));
        if !issue.reason.trim().is_empty() {
            out.push_str(&format!("    reason: {}\n", issue.reason));
        }
        if !issue.source.trim().is_empty() {
            out.push_str(&format!("    source: {}\n", issue.source));
        }
        out.push_str(&format!("    graph: {}\n", issue_graph(issue)));
        if let Some(raw) = issue.raw_evidence.as_deref()
            && !raw.trim().is_empty()
        {
            out.push_str(&format!("    rule_detail: {raw}\n"));
        }
    }
}

fn append_step3_compact_summary(issues: &[CompatIssueDisplay], out: &mut String) {
    let summary = build_compat_summary(issues);
    out.push_str(&format!(
        "  summary: total={} errors={} warnings={}\n",
        summary.total_issues, summary.total_errors, summary.total_warnings
    ));
    out.push_str("  by_code:\n");
    for (code, count) in summary.by_code {
        out.push_str(&format!("    - {}: {}\n", code, count));
    }
    append_top_groups(out, "top_conflict_groups", &summary.conflict_groups);
    append_top_groups(out, "top_missing_dep_groups", &summary.missing_dep_groups);
    append_top_groups(out, "top_order_warn_groups", &summary.order_warn_groups);
}

fn append_top_groups(
    out: &mut String,
    title: &str,
    groups: &[super::compat_summary::GroupCount],
) {
    out.push_str(&format!("  {}:\n", title));
    if groups.is_empty() {
        out.push_str("    - none\n");
        return;
    }
    for (idx, entry) in groups.iter().take(12).enumerate() {
        out.push_str(&format!("    {}. {} ({})\n", idx + 1, entry.group, entry.count));
    }
}
