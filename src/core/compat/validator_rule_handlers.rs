// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use super::super::model::{CompatIssue, CompatIssueCode, IssueSource, Severity, Tp2Metadata, Tp2Rule};
use super::validator_helpers as helpers;
use super::validator_rule_handlers_any::handle_require_installed_any;
use super::validator_rule_handlers_misc::{
    handle_conditional, handle_forbid_installed, handle_require_game_or_installed_any,
    handle_require_installed_mod,
};
use super::SelectedComponent;

pub(super) fn apply_rule(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    rule: &Tp2Rule,
    game_mode: &str,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) {
    match rule {
        Tp2Rule::Require {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_require(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
            order_map,
        ),
        Tp2Rule::Forbid {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_forbid(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
        ),
        Tp2Rule::RequireGame {
            allowed_games,
            raw_line,
            line,
        } => handle_require_game(
            issues,
            metadata,
            component,
            allowed_games,
            raw_line,
            *line,
            game_mode,
        ),
        Tp2Rule::RequireGameOrInstalledAny {
            allowed_games,
            targets,
            raw_line,
            line,
        } => handle_require_game_or_installed_any(
            issues,
            metadata,
            component,
            allowed_games,
            targets,
            raw_line,
            *line,
            game_mode,
            selected_set,
        ),
        Tp2Rule::RequireInstalledMod {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_require_installed_mod(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
            order_map,
        ),
        Tp2Rule::RequireInstalledAny {
            targets,
            raw_line,
            line,
        } => handle_require_installed_any(
            issues,
            metadata,
            component,
            targets,
            raw_line,
            *line,
            selected_set,
            order_map,
        ),
        Tp2Rule::RequirePath { .. } => {},
        Tp2Rule::ForbidInstalledMod {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_forbid_installed(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
        ),
        Tp2Rule::ConditionalOnInstalled {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_conditional(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
            true,
        ),
        Tp2Rule::ConditionalOnMissing {
            target_mod,
            target_component,
            raw_line,
            line,
        } => handle_conditional(
            issues,
            metadata,
            component,
            target_mod,
            *target_component,
            raw_line,
            *line,
            selected_set,
            false,
        ),
    }
}

fn handle_require(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    target_mod: &str,
    target_component: u32,
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) {
    let target_key = (target_mod.to_string(), target_component);
    let current_key = (
        helpers::normalize_mod_key(&component.tp_file),
        component.component_id,
    );
    if target_key == current_key {
        return;
    }

    if !selected_set.contains(&target_key) {
        issues.push(CompatIssue::new(
            CompatIssueCode::ReqMissing,
            Severity::Error,
            IssueSource::Tp2 {
                file: metadata.tp_file.clone(),
                line,
            },
            component.mod_name.clone(),
            Some(component.component_id),
            target_mod.to_string(),
            Some(target_component),
            format!("Requires {target_mod} component {target_component} which is not selected"),
            Some(raw_line.to_string()),
        ));
        return;
    }

    if let Some(target_order) = order_map.get(&target_key)
        && *target_order > component.order
    {
        issues.push(CompatIssue::new(
            CompatIssueCode::OrderWarn,
            Severity::Warning,
            IssueSource::Tp2 {
                file: metadata.tp_file.clone(),
                line,
            },
            component.mod_name.clone(),
            Some(component.component_id),
            target_mod.to_string(),
            Some(target_component),
            format!(
                "Requires {target_mod} #{target_component} but it is ordered after this component"
            ),
            Some(raw_line.to_string()),
        ));
    }
}

fn handle_forbid(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    target_mod: &str,
    target_component: u32,
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
) {
    let affected_mod_key = helpers::normalize_mod_key(&component.tp_file);
    let related_mod_key = helpers::normalize_mod_key(target_mod);
    if affected_mod_key == related_mod_key && component.component_id == target_component {
        return;
    }
    let target_key = (target_mod.to_string(), target_component);
    if selected_set.contains(&target_key) {
        issues.push(CompatIssue::new(
            CompatIssueCode::ForbidHit,
            Severity::Error,
            IssueSource::Tp2 {
                file: metadata.tp_file.clone(),
                line,
            },
            component.mod_name.clone(),
            Some(component.component_id),
            target_mod.to_string(),
            Some(target_component),
            format!("Incompatible with {target_mod} component {target_component} which is selected"),
            Some(raw_line.to_string()),
        ));
    }
}

fn handle_require_game(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    allowed_games: &[String],
    raw_line: &str,
    line: usize,
    game_mode: &str,
) {
    let current_game = helpers::normalize_game_mode(game_mode);
    if helpers::game_allowed(&current_game, allowed_games) {
        return;
    }
    issues.push(CompatIssue::new(
        CompatIssueCode::GameMismatch,
        Severity::Error,
        IssueSource::Tp2 {
            file: metadata.tp_file.clone(),
            line,
        },
        component.mod_name.clone(),
        Some(component.component_id),
        allowed_games.join("|"),
        None,
        format!(
            "Requires GAME_IS {} but current mode is {}",
            allowed_games.join(","),
            current_game
        ),
        Some(raw_line.to_string()),
    ));
}
