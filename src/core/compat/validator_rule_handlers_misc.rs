// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use super::super::model::{CompatIssue, CompatIssueCode, IssueSource, Severity, Tp2Metadata};
use super::SelectedComponent;
use super::validator_helpers as helpers;

pub(super) fn handle_require_game_or_installed_any(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    allowed_games: &[String],
    targets: &[(String, Option<u32>)],
    raw_line: &str,
    line: usize,
    game_mode: &str,
    selected_set: &HashSet<(String, u32)>,
) {
    let current_game = helpers::normalize_game_mode(game_mode);
    let game_ok = helpers::game_allowed(&current_game, allowed_games);
    let installed_ok =
        targets
            .iter()
            .any(|(target_mod, target_component)| match target_component {
                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                None => selected_set.iter().any(|(m, _)| m == target_mod),
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
    issues.push(CompatIssue::new(
        CompatIssueCode::ReqMissing,
        Severity::Error,
        IssueSource::Tp2 {
            file: metadata.tp_file.clone(),
            line,
        },
        component.mod_name.clone(),
        Some(component.component_id),
        related_mod,
        related_component,
        format!(
            "Requires GAME_IS {} OR one of: {}",
            allowed_games.join(","),
            related_text
        ),
        Some(raw_line.to_string()),
    ));
}

pub(super) fn handle_require_installed_mod(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) {
    let hit = match target_component {
        Some(cid) => selected_set.contains(&(target_mod.to_string(), cid)),
        None => selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if !hit {
        let related_text = match target_component {
            Some(cid) => format!("{target_mod} #{cid}"),
            None => format!("{target_mod} (any component)"),
        };
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
            target_component,
            format!("Requires installed component: {related_text}"),
            Some(raw_line.to_string()),
        ));
        return;
    }

    let Some(cid) = target_component else {
        return;
    };
    let target_key = (target_mod.to_string(), cid);
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
            Some(cid),
            format!(
                "Requires installed component: {} #{} but it is ordered after this component",
                target_mod, cid
            ),
            Some(raw_line.to_string()),
        ));
    }
}

pub(super) fn handle_forbid_installed(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
) {
    let hit = match target_component {
        Some(cid) => selected_set.contains(&(target_mod.to_string(), cid)),
        None => selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if !hit {
        return;
    }
    let related_text = match target_component {
        Some(cid) => format!("{target_mod} #{cid}"),
        None => format!("{target_mod} (any component)"),
    };
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
        target_component,
        format!("Cannot install when {related_text} is currently selected/installed"),
        Some(raw_line.to_string()),
    ));
}

pub(super) fn handle_conditional(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    target_mod: &str,
    target_component: Option<u32>,
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
    when_installed: bool,
) {
    let hit = match target_component {
        Some(cid) => selected_set.contains(&(target_mod.to_string(), cid)),
        None => selected_set.iter().any(|(m, _)| m == target_mod),
    };
    if hit != when_installed {
        return;
    }

    let description = if when_installed {
        if let Some(cid) = target_component {
            format!(
                "Conditional patch path active because {} #{} is selected",
                target_mod, cid
            )
        } else {
            format!(
                "Conditional patch path active because {} is selected",
                target_mod
            )
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

    issues.push(CompatIssue::new(
        CompatIssueCode::Conditional,
        Severity::Warning,
        IssueSource::Tp2 {
            file: metadata.tp_file.clone(),
            line,
        },
        component.mod_name.clone(),
        Some(component.component_id),
        target_mod.to_string(),
        target_component,
        description,
        Some(raw_line.to_string()),
    ));
}
