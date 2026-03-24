// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::model::{
    CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity,
};
use super::RuleEvalContext;

pub(super) fn handle_require_installed_any(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    targets: &[(String, Option<u32>)],
    raw_line: &str,
    line: usize,
) {
    let hit = targets.iter().any(|(target_mod, target_component)| match target_component {
        Some(cid) => ctx.selected_set.contains(&(target_mod.clone(), *cid)),
        None => ctx.selected_set.iter().any(|(m, _)| m == target_mod),
    });

    if !hit {
        let related_text = targets
            .iter()
            .map(|(m, c)| match c {
                Some(id) => format!("{m} #{id}"),
                None => format!("{m} (any component)"),
            })
            .collect::<Vec<_>>()
            .join(" OR ");
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
            reason: format!("Requires one of: {related_text}"),
            raw_evidence: Some(raw_line.to_string()),
        }));
        return;
    }

    let mut matched_orders: Vec<usize> = Vec::new();
    for (target_mod, target_component) in targets {
        match target_component {
            Some(cid) => {
                if ctx.selected_set.contains(&(target_mod.clone(), *cid))
                    && let Some(order) = ctx.order_map.get(&(target_mod.clone(), *cid))
                {
                    matched_orders.push(*order);
                }
            }
            None => {
                for ((mod_key, _), order) in ctx.order_map {
                    if mod_key == target_mod {
                        matched_orders.push(*order);
                    }
                }
            }
        }
    }
    matched_orders.sort_unstable();
    matched_orders.dedup();

    if matched_orders.is_empty() || matched_orders.iter().any(|order| *order <= ctx.component.order) {
        return;
    }

    let (related_mod, related_component) = targets
        .first()
        .cloned()
        .unwrap_or_else(|| ("unknown".to_string(), None));
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::OrderWarn,
        severity: Severity::Warning,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod,
        related_component,
        reason: format!(
            "Requires one of: {} but all matching selected targets are ordered after this component",
            targets
                .iter()
                .map(|(m, c)| match c {
                    Some(id) => format!("{m} #{id}"),
                    None => format!("{m} (any component)"),
                })
                .collect::<Vec<_>>()
                .join(" OR ")
        ),
        raw_evidence: Some(raw_line.to_string()),
    }));
}
