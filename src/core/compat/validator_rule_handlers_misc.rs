// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::model::{CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity};
use super::validator_helpers as helpers;
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
    let installed_ok = targets
        .iter()
        .any(|(target_mod, target_component)| match target_component {
            Some(cid) => ctx.selected_set.contains(&(target_mod.clone(), *cid)),
            None => ctx.selected_set.iter().any(|(m, _)| m == target_mod),
        });
    if game_ok || installed_ok {
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
        reason: format!(
            "Requires GAME_IS {} OR one of: {}",
            allowed_games.join(","),
            related_text
        ),
        raw_evidence: Some(raw_line.to_string()),
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
    let hit = match target_component {
        Some(cid) => ctx.selected_set.contains(&(target_mod.to_string(), cid)),
        None => ctx.selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if !hit {
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
            reason: format!("Requires installed component: {related_text}"),
            raw_evidence: Some(raw_line.to_string()),
        }));
        return;
    }

    let Some(cid) = target_component else {
        return;
    };
    let target_key = (target_mod.to_string(), cid);
    if let Some(target_order) = ctx.order_map.get(&target_key)
        && *target_order > ctx.component.order
    {
        issues.push(CompatIssue::new(CompatIssueInit {
            code: CompatIssueCode::OrderWarn,
            severity: Severity::Warning,
            source: IssueSource::Tp2 {
                file: ctx.metadata.tp_file.clone(),
                line,
            },
            affected_mod: ctx.component.mod_name.clone(),
            affected_component: Some(ctx.component.component_id),
            related_mod: target_mod.to_string(),
            related_component: Some(cid),
            reason: format!(
                "Requires installed component: {} #{} but it is ordered after this component",
                target_mod, cid
            ),
            raw_evidence: Some(raw_line.to_string()),
        }));
    }
}

pub(super) fn handle_forbid_installed(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
) {
    let hit = match target_component {
        Some(cid) => ctx.selected_set.contains(&(target_mod.to_string(), cid)),
        None => ctx.selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if !hit {
        return;
    }
    let related_text = match target_component {
        Some(cid) => format!("{target_mod} #{cid}"),
        None => format!("{target_mod} (any component)"),
    };
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::ForbidHit,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: target_mod.to_string(),
        related_component: target_component,
        reason: format!("Cannot install when {related_text} is currently selected/installed"),
        raw_evidence: Some(raw_line.to_string()),
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
        reason: description,
        raw_evidence: Some(raw_line.to_string()),
    }));
}
