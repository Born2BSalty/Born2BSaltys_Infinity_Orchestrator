// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::model::{CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity};
use super::validator_helpers::{self as helpers, SameModBlockMeaning};
use super::RuleEvalContext;

pub(super) fn handle_require_game_or_installed_any(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    allowed_games: &[String],
    targets: &[(String, Option<u32>)],
    raw_line: &str,
    line: usize,
) {
    let current_game = helpers::normalize_game_mode(ctx.game_mode);
    let game_ok = helpers::game_allowed(&current_game, allowed_games);
    if game_ok {
        return;
    }

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
                format!(
                    "Requires GAME_IS {} OR one of: {}",
                    allowed_games.join(","),
                    related_text
                ),
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
            "Requires GAME_IS {} OR one of: {} before this component, but all matching selected targets are currently ordered after it",
            allowed_games.join(","),
            related_text
        ),
        raw_evidence: Some(raw_line.to_string()),
        component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
    }));
}

pub(super) fn handle_require_installed_mod(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
) {
    let matching_orders = helpers::matching_orders_for_target(ctx.order_map, target_mod, target_component);
    if matching_orders.is_empty() {
        let related_text = match target_component {
            Some(cid) => format!("{target_mod} #{cid}"),
            None => format!("{target_mod} (any component)"),
        };
        issues.push(CompatIssue::new(CompatIssueInit {
            code: CompatIssueCode::ReqMissing,
            severity: Severity::Error,
            source: IssueSource::Tp2 {
                file: ctx.metadata.tp_file.clone(),
                line,
            },
            affected_mod: ctx.component.mod_name.clone(),
            affected_component: Some(ctx.component.component_id),
            related_mod: target_mod.to_string(),
            related_component: target_component,
            reason: helpers::resolved_reason_or(
                ctx.metadata,
                raw_line,
                format!("Requires installed component: {related_text}"),
            ),
            raw_evidence: Some(raw_line.to_string()),
            component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
        }));
        return;
    }

    if matching_orders.iter().any(|(_, order)| *order <= ctx.component.order) {
        return;
    }

    let mut later = matching_orders;
    later.sort_by_key(|(_, order)| *order);
    let (related_component, _) = later[0];
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::OrderBlock,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: target_mod.to_string(),
        related_component: Some(related_component),
        reason: format!(
            "Requires installed component: {} #{} before this component, but it is currently ordered after it",
            target_mod, related_component
        ),
        raw_evidence: Some(raw_line.to_string()),
        component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
    }));
}

pub(super) fn handle_forbid_installed(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
) {
    let matching_orders: Vec<(u32, usize)> = match target_component {
        Some(cid) => ctx
            .order_map
            .get(&(target_mod.to_string(), cid))
            .map(|order| vec![(cid, *order)])
            .unwrap_or_default(),
        None => ctx
            .order_map
            .iter()
            .filter_map(|((mod_name, cid), order)| {
                if mod_name == target_mod {
                    Some((*cid, *order))
                } else {
                    None
                }
            })
            .collect(),
    };

    if matching_orders.is_empty() {
        return;
    }

    let mut earlier: Vec<(u32, usize)> = matching_orders
        .into_iter()
        .filter(|(_, order)| *order < ctx.component.order)
        .collect();
    if earlier.is_empty() {
        return;
    }
    earlier.sort_by_key(|(_, order)| *order);
    let (related_component_id, _related_order) = earlier[0];
    let related_component = target_component.or(Some(related_component_id));
    let related_text = match related_component {
        Some(cid) => format!("{target_mod} #{cid}"),
        None => format!("{target_mod} (any component)"),
    };
    let same_mod_meaning = helpers::classify_same_mod_block(
        ctx.metadata,
        &ctx.component.tp_file,
        target_mod,
        raw_line,
    );
    let (code, severity, reason) = match same_mod_meaning {
        Some(SameModBlockMeaning::Included) => (
            CompatIssueCode::Included,
            Severity::Warning,
            format!("Already provided by {related_text}; that component is currently ordered earlier"),
        ),
        _ => (
            CompatIssueCode::OrderBlock,
            Severity::Error,
            format!("Must be installed before {related_text}; that component is currently ordered earlier"),
        ),
    };
    issues.push(CompatIssue::new(CompatIssueInit {
        code,
        severity,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: target_mod.to_string(),
        related_component,
        reason,
        raw_evidence: Some(raw_line.to_string()),
        component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
    }));
}

pub(super) fn handle_conditional(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
    when_installed: bool,
) {
    let hit = match target_component {
        Some(cid) => ctx.selected_set.contains(&(target_mod.to_string(), cid)),
        None => ctx.selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if hit != when_installed {
        return;
    }

    let description = if when_installed {
        if let Some(cid) = target_component {
            format!("Conditional patch path active because {} #{} is selected", target_mod, cid)
        } else {
            format!("Conditional patch path active because {} is selected", target_mod)
        }
    } else if let Some(cid) = target_component {
        format!(
            "Conditional patch path is active because {} #{} is NOT selected/installed",
            target_mod, cid
        )
    } else {
        format!(
            "Conditional patch path is active because {} is NOT selected/installed",
            target_mod
        )
    };

    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::Conditional,
        severity: Severity::Warning,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: target_mod.to_string(),
        related_component: target_component,
        reason: helpers::resolved_reason_or(ctx.metadata, raw_line, description),
        raw_evidence: Some(raw_line.to_string()),
        component_block: helpers::component_block_for(ctx.metadata, ctx.component.component_id),
    }));
}
