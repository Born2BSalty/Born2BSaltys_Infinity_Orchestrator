// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use crate::ui::state::CompatIssueDisplay;

use super::format::{format_target, issue_graph};

#[derive(Debug, Clone)]
pub(super) struct GroupCount {
    pub group: String,
    pub count: usize,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CompatSummary {
    pub total_issues: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub by_code: BTreeMap<String, usize>,
    pub conflict_groups: Vec<GroupCount>,
    pub missing_dep_groups: Vec<GroupCount>,
    pub order_warn_groups: Vec<GroupCount>,
}

pub(super) fn build_compat_summary(issues: &[CompatIssueDisplay]) -> CompatSummary {
    let mut by_code = BTreeMap::<String, usize>::new();
    let mut conflict_groups = BTreeMap::<String, usize>::new();
    let mut missing_groups = BTreeMap::<String, usize>::new();
    let mut order_groups = BTreeMap::<String, usize>::new();

    for issue in issues {
        *by_code.entry(issue.code.clone()).or_default() += 1;

        let key = match issue.code.to_ascii_uppercase().as_str() {
            "REQ_MISSING" => missing_group_key(issue),
            "FORBID_HIT" => conflict_or_order_group_key(issue),
            "ORDER_WARN" => conflict_or_order_group_key(issue),
            _ => continue,
        };
        match issue.code.to_ascii_uppercase().as_str() {
            "FORBID_HIT" => *conflict_groups.entry(key).or_default() += 1,
            "REQ_MISSING" => *missing_groups.entry(key).or_default() += 1,
            "ORDER_WARN" => *order_groups.entry(key).or_default() += 1,
            _ => {}
        }
    }

    let total_errors = issues.iter().filter(|i| i.is_blocking).count();
    let total_issues = issues.len();
    let total_warnings = total_issues.saturating_sub(total_errors);

    CompatSummary {
        total_issues,
        total_errors,
        total_warnings,
        by_code,
        conflict_groups: sorted_group_entries(&conflict_groups),
        missing_dep_groups: sorted_group_entries(&missing_groups),
        order_warn_groups: sorted_group_entries(&order_groups),
    }
}

fn conflict_or_order_group_key(issue: &CompatIssueDisplay) -> String {
    format!(
        "{} -> {}",
        format_target(&issue.affected_mod, None),
        format_target(&issue.related_mod, issue.related_component)
    )
}

fn missing_group_key(issue: &CompatIssueDisplay) -> String {
    // issue_graph() preserves OR dependencies ("requires one of: A | B"),
    // which avoids collapsing REQ_MISSING groups to only the first related target.
    issue_graph(issue)
}

fn sorted_group_entries(groups: &BTreeMap<String, usize>) -> Vec<GroupCount> {
    let mut pairs: Vec<(&String, &usize)> = groups.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    pairs
        .into_iter()
        .map(|(group, count)| GroupCount {
            group: group.clone(),
            count: *count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(code: &str, affected: &str, related: &str, reason: &str, is_blocking: bool) -> CompatIssueDisplay {
        CompatIssueDisplay {
            issue_id: "id".to_string(),
            code: code.to_string(),
            severity: if is_blocking { "Error".to_string() } else { "Warning".to_string() },
            is_blocking,
            affected_mod: affected.to_string(),
            affected_component: Some(1),
            related_mod: related.to_string(),
            related_component: Some(0),
            reason: reason.to_string(),
            source: "TP2".to_string(),
            raw_evidence: None,
        }
    }

    #[test]
    fn groups_missing_or_dependencies_by_full_or_set() {
        let issues = vec![issue(
            "REQ_MISSING",
            "ArtisansKitpack_npc",
            "emily",
            "Requires one of: emily #0 OR skitianpcs #0",
            true,
        )];
        let summary = build_compat_summary(&issues);
        assert_eq!(summary.missing_dep_groups.len(), 1);
        assert!(
            summary.missing_dep_groups[0]
                .group
                .contains("requires one of: emily #0 | skitianpcs #0")
        );
    }
}
