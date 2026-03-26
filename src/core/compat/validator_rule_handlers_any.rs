// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::model::{
    CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity,
};
use super::validator_helpers as helpers;
use super::RuleEvalContext;

pub(super) fn handle_require_installed_any(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    targets: &[(String, Option<u32>)],
    raw_line: &str,
    line: usize,
) {
    let related_text = targets
        .iter()
        .map(|(m, c)| match c {
            Some(id) => format!("{m} #{id}"),
            None => format!("{m} (any component)"),
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    let matched_orders = helpers::matching_orders_for_targets(ctx.order_map, targets);
    if matched_orders.is_empty() {
        let (related_mod, related_component) = targets
            .first()
            .cloned()
            .unwrap_or_else(|| ("unknown".to_string(), None));
        issues.push(CompatIssue::new(CompatIssueInit {
            code: CompatIssueCode::ReqMissing,
            severity: Severity::Error,
            source: IssueSource::Tp2 {
                file: ctx.metadata.tp_file.clone(),
                line,
            },
            affected_mod: ctx.component.mod_name.clone(),
            affected_component: Some(ctx.component.component_id),
            related_mod,
            related_component,
            reason: helpers::resolved_reason_or(
                ctx.metadata,
                raw_line,
                format!("Requires one of: {related_text}"),
            ),
            raw_evidence: Some(raw_line.to_string()),
            component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
        }));
        return;
    }

    if matched_orders.iter().any(|(_, _, order)| *order <= ctx.component.order) {
        return;
    }

    let mut later = matched_orders;
    later.sort_by_key(|(_, _, order)| *order);
    let (related_mod, related_component, _) = later[0].clone();
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::OrderBlock,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod,
        related_component: Some(related_component),
        reason: format!(
            "Requires one of: {} before this component, but all matching selected targets are currently ordered after it",
            related_text
        ),
        raw_evidence: Some(raw_line.to_string()),
        component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
    }));
}
