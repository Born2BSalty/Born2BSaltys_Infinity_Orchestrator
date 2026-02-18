// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::CompatState;

use super::format::{format_target, issue_graph};

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
