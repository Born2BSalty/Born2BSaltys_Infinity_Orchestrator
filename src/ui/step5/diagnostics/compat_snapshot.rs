// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step2ComponentState, Step2ModState, WizardState};
use std::collections::BTreeMap;

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
    let mut code_counts = BTreeMap::<String, usize>::new();
    for issue in issues {
        *code_counts.entry(issue.code.clone()).or_default() += 1;
    }
    let errors = issues.iter().filter(|i| i.is_blocking).count();
    let warnings = issues.len().saturating_sub(errors);
    out.push_str(&format!(
        "  summary: total={} errors={} warnings={}\n",
        issues.len(),
        errors,
        warnings
    ));
    out.push_str("  by_code:\n");
    for (code, count) in code_counts {
        out.push_str(&format!("    - {}: {}\n", code, count));
    }

    let mut conflict_groups = BTreeMap::<String, usize>::new();
    let mut missing_groups = BTreeMap::<String, usize>::new();
    let mut order_groups = BTreeMap::<String, usize>::new();
    for issue in issues {
        let key = format!(
            "{} -> {}",
            format_target(&issue.affected_mod, None),
            format_target(&issue.related_mod, issue.related_component)
        );
        match issue.code.to_ascii_uppercase().as_str() {
            "FORBID_HIT" => *conflict_groups.entry(key).or_default() += 1,
            "REQ_MISSING" => *missing_groups.entry(key).or_default() += 1,
            "ORDER_WARN" => *order_groups.entry(key).or_default() += 1,
            _ => {}
        }
    }

    append_top_groups(out, "top_conflict_groups", &conflict_groups);
    append_top_groups(out, "top_missing_dep_groups", &missing_groups);
    append_top_groups(out, "top_order_warn_groups", &order_groups);
}

fn append_top_groups(out: &mut String, title: &str, groups: &BTreeMap<String, usize>) {
    out.push_str(&format!("  {}:\n", title));
    if groups.is_empty() {
        out.push_str("    - none\n");
        return;
    }
    let mut pairs: Vec<(&String, &usize)> = groups.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (idx, (name, count)) in pairs.into_iter().take(12).enumerate() {
        out.push_str(&format!("    {}. {} ({})\n", idx + 1, name, count));
    }
}
