// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::model::{
    CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity, Tp2Rule,
};
use super::validator_helpers as helpers;
use super::validator_rule_handlers_any::handle_require_installed_any;
use super::validator_rule_handlers_misc::{
    handle_conditional, handle_forbid_installed, handle_require_game_or_installed_any,
    handle_require_installed_mod,
};
use super::{RuleEvalContext, SelectedComponent};

pub(super) fn apply_rule(
    issues: &mut Vec<CompatIssue>,
    metadata: &super::super::model::Tp2Metadata,
    component: &SelectedComponent,
    rule: &Tp2Rule,
    game_mode: &str,
    selected_set: &std::collections::HashSet<(String, u32)>,
    order_map: &std::collections::HashMap<(String, u32), usize>,
) {
    let ctx = RuleEvalContext {
        metadata,
        component,
        game_mode,
        selected_set,
        order_map,
    };

    match rule {
        Tp2Rule::Require {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_require(issues, &ctx, target_mod, *target_component, raw_line, *line),
        Tp2Rule::Forbid {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_forbid(issues, &ctx, target_mod, *target_component, raw_line, *line),
        Tp2Rule::RequireGame {
            allowed_games,
            raw_line,
            line,
        } => handle_require_game(issues, &ctx, allowed_games, raw_line, *line),
        Tp2Rule::ForbidGame {
            blocked_games,
            raw_line,
            line,
        } => handle_forbid_game(issues, &ctx, blocked_games, raw_line, *line),
        Tp2Rule::RequireGameIncludes {
            required_games,
            raw_line,
            line,
        } => handle_require_game_includes(issues, &ctx, required_games, raw_line, *line),
        Tp2Rule::Deprecated { .. } => {},
        Tp2Rule::RequireGameOrInstalledAny {
            allowed_games,
            targets,
            raw_line,
            line,
        } => handle_require_game_or_installed_any(issues, &ctx, allowed_games, targets, raw_line, *line),
        Tp2Rule::RequireInstalledMod {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_require_installed_mod(issues, &ctx, target_mod, *target_component, raw_line, *line),
        Tp2Rule::RequireInstalledAny {
            targets,
            raw_line,
            line,
        } => handle_require_installed_any(issues, &ctx, targets, raw_line, *line),
        Tp2Rule::RequirePath { .. } => {}
        Tp2Rule::ForbidInstalledMod {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_forbid_installed(issues, &ctx, target_mod, *target_component, raw_line, *line),
        Tp2Rule::ConditionalOnInstalled {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_conditional(issues, &ctx, target_mod, *target_component, raw_line, *line, true),
        Tp2Rule::ConditionalOnMissing {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_conditional(issues, &ctx, target_mod, *target_component, raw_line, *line, false),
    }
}

fn handle_require(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: u32,
    raw_line: &str,
    line: usize,
) {
    let target_key = (target_mod.to_string(), target_component);
    let current_key = (
        helpers::normalize_mod_key(&ctx.component.tp_file),
        ctx.component.component_id,
    );
    if target_key == current_key {
        return;
    }

    if !ctx.selected_set.contains(&target_key) {
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
            related_component: Some(target_component),
            reason: format!("Requires {target_mod} component {target_component} which is not selected"),
            raw_evidence: Some(raw_line.to_string()),
        }));
        return;
    }

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
            related_component: Some(target_component),
            reason: format!(
                "Requires {target_mod} #{target_component} but it is ordered after this component"
            ),
            raw_evidence: Some(raw_line.to_string()),
        }));
    }
}

fn handle_forbid(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    target_mod: &str,
    target_component: u32,
    raw_line: &str,
    line: usize,
) {
    let affected_mod_key = helpers::normalize_mod_key(&ctx.component.tp_file);
    let related_mod_key = helpers::normalize_mod_key(target_mod);
    if affected_mod_key == related_mod_key && ctx.component.component_id == target_component {
        return;
    }
    let target_key = (target_mod.to_string(), target_component);
    if ctx.selected_set.contains(&target_key) {
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
            related_component: Some(target_component),
            reason: format!("Incompatible with {target_mod} component {target_component} which is selected"),
            raw_evidence: Some(raw_line.to_string()),
        }));
    }
}

fn handle_forbid_game(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    blocked_games: &[String],
    raw_line: &str,
    line: usize,
) {
    let current_game = helpers::normalize_game_mode(ctx.game_mode);
    if !blocked_games
        .iter()
        .any(|game| game.eq_ignore_ascii_case(&current_game))
    {
        return;
    }
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::GameMismatch,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: blocked_games.join("|"),
        related_component: None,
        reason: format!(
            "Blocked on GAME_IS {} and current mode is {}",
            blocked_games.join(","),
            current_game
        ),
        raw_evidence: Some(raw_line.to_string()),
    }));
}

fn handle_require_game_includes(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    required_games: &[String],
    raw_line: &str,
    line: usize,
) {
    let current_game = helpers::normalize_game_mode(ctx.game_mode);
    if helpers::game_includes(&current_game, required_games) {
        return;
    }
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::GameMismatch,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: required_games.join("|"),
        related_component: None,
        reason: format!(
            "Requires GAME_INCLUDES {} but current mode is {}",
            required_games.join(","),
            current_game
        ),
        raw_evidence: Some(raw_line.to_string()),
    }));
}

fn handle_require_game(
    issues: &mut Vec<CompatIssue>,
    ctx: &RuleEvalContext<'_>,
    allowed_games: &[String],
    raw_line: &str,
    line: usize,
) {
    let current_game = helpers::normalize_game_mode(ctx.game_mode);
    if helpers::game_allowed(&current_game, allowed_games) {
        return;
    }
    issues.push(CompatIssue::new(CompatIssueInit {
        code: CompatIssueCode::GameMismatch,
        severity: Severity::Error,
        source: IssueSource::Tp2 {
            file: ctx.metadata.tp_file.clone(),
            line,
        },
        affected_mod: ctx.component.mod_name.clone(),
        affected_component: Some(ctx.component.component_id),
        related_mod: allowed_games.join("|"),
        related_component: None,
        reason: format!(
            "Requires GAME_IS {} but current mode is {}",
            allowed_games.join(","),
            current_game
        ),
        raw_evidence: Some(raw_line.to_string()),
    }));
}
