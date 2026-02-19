// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

mod helpers;

use super::model::{
    CompatIssue, CompatIssueCode, CompatValidationResult, IssueSource, Severity, Tp2Metadata,
    Tp2Rule,
};

#[derive(Debug, Clone)]
pub struct SelectedComponent {
    pub mod_name: String,
    pub tp_file: String,
    pub component_id: u32,
    pub order: usize,
}

pub struct CompatValidator {
    tp2_metadata: HashMap<String, Tp2Metadata>,
}

impl CompatValidator {
    pub fn new() -> Self {
        Self {
            tp2_metadata: HashMap::new(),
        }
    }

    pub fn set_tp2_metadata(&mut self, metadata: HashMap<String, Tp2Metadata>) {
        self.tp2_metadata = metadata;
    }

    #[allow(dead_code)]
    pub fn add_tp2_metadata(&mut self, key: String, metadata: Tp2Metadata) {
        self.tp2_metadata.insert(key, metadata);
    }

    pub fn validate(&self, selected: &[SelectedComponent], game_mode: &str) -> CompatValidationResult {
        let mut issues = Vec::new();

        let selected_set = helpers::build_selected_set(selected);
        let order_map = helpers::build_order_map(selected);

        for component in selected {
            let key = helpers::normalize_mod_key(&component.tp_file);
            if let Some(metadata) = self.tp2_metadata.get(&key) {
                for (comp_id, rule) in &metadata.rules {
                    if *comp_id != component.component_id {
                        continue;
                    }

                    match rule {
                        Tp2Rule::Require {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let target_key = (target_mod.clone(), *target_component);
                            let current_key = (
                                helpers::normalize_mod_key(&component.tp_file),
                                component.component_id,
                            );
                            // Ignore self-require entries (same mod/component). They cannot
                            // produce actionable dependency/order information.
                            if target_key == current_key {
                                continue;
                            }
                            if !selected_set.contains(&target_key) {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::ReqMissing,
                                    Severity::Error,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    Some(*target_component),
                                    format!(
                                        "Requires {target_mod} component {target_component} which is not selected"
                                    ),
                                    Some(raw_line.clone()),
                                ));
                            } else if let Some(target_order) = order_map.get(&target_key)
                                && *target_order > component.order
                            {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::OrderWarn,
                                    Severity::Warning,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    Some(*target_component),
                                    format!(
                                        "Requires {target_mod} #{target_component} but it is ordered after this component"
                                    ),
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::Forbid {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let affected_mod_key = helpers::normalize_mod_key(&component.tp_file);
                            let related_mod_key = helpers::normalize_mod_key(target_mod);
                            if affected_mod_key == related_mod_key
                                && component.component_id == *target_component
                            {
                                continue;
                            }
                            let target_key = (target_mod.clone(), *target_component);
                            if selected_set.contains(&target_key) {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::ForbidHit,
                                    Severity::Error,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    Some(*target_component),
                                    format!(
                                        "Incompatible with {target_mod} component {target_component} which is selected"
                                    ),
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::RequireGame {
                            allowed_games,
                            raw_line,
                        } => {
                            let current_game = helpers::normalize_game_mode(game_mode);
                            if !helpers::game_allowed(&current_game, allowed_games) {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::GameMismatch,
                                    Severity::Error,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
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
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::RequireGameOrInstalledAny {
                            allowed_games,
                            targets,
                            raw_line,
                        } => {
                            let current_game = helpers::normalize_game_mode(game_mode);
                            let game_ok = helpers::game_allowed(&current_game, allowed_games);
                            let installed_ok = targets.iter().any(|(target_mod, target_component)| {
                                match target_component {
                                    Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                                    None => selected_set.iter().any(|(m, _)| m == target_mod),
                                }
                            });
                            if !game_ok && !installed_ok {
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
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::RequireInstalledMod {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let hit = match target_component {
                                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
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
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    *target_component,
                                    format!("Requires installed component: {related_text}"),
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::RequireInstalledAny { targets, raw_line } => {
                            let hit = targets.iter().any(|(target_mod, target_component)| match target_component {
                                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                                None => selected_set.iter().any(|(m, _)| m == target_mod),
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
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::ReqMissing,
                                    Severity::Error,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    related_mod,
                                    related_component,
                                    format!("Requires one of: {related_text}"),
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::ForbidInstalledMod {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let hit = match target_component {
                                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                                None => selected_set.iter().any(|(m, _)| m == target_mod),
                            };
                            if hit {
                                let related_text = match target_component {
                                    Some(cid) => format!("{target_mod} #{cid}"),
                                    None => format!("{target_mod} (any component)"),
                                };
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::ForbidHit,
                                    Severity::Error,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    *target_component,
                                    format!(
                                        "Cannot install when {related_text} is currently selected/installed"
                                    ),
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::ConditionalOnInstalled {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let hit = match target_component {
                                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                                None => selected_set.iter().any(|(m, _)| m == target_mod),
                            };
                            if hit {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::Conditional,
                                    Severity::Warning,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    *target_component,
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
                                    },
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                        Tp2Rule::ConditionalOnMissing {
                            target_mod,
                            target_component,
                            raw_line,
                        } => {
                            let hit = match target_component {
                                Some(cid) => selected_set.contains(&(target_mod.clone(), *cid)),
                                None => selected_set.iter().any(|(m, _)| m == target_mod),
                            };
                            if !hit {
                                issues.push(CompatIssue::new(
                                    CompatIssueCode::Conditional,
                                    Severity::Warning,
                                    IssueSource::Tp2 {
                                        file: metadata.tp_file.clone(),
                                    },
                                    component.mod_name.clone(),
                                    Some(component.component_id),
                                    target_mod.clone(),
                                    *target_component,
                                    if let Some(cid) = target_component {
                                        format!(
                                            "Conditional patch path is active because {} #{} is NOT selected/installed",
                                            target_mod, cid
                                        )
                                    } else {
                                        format!(
                                            "Conditional patch path is active because {} is NOT selected/installed",
                                            target_mod
                                        )
                                    },
                                    Some(raw_line.clone()),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Hard blocker: same component selected multiple times in the current set.
        let mut by_key: HashMap<(String, u32), Vec<&SelectedComponent>> = HashMap::new();
        for component in selected {
            by_key
                .entry((helpers::normalize_mod_key(&component.tp_file), component.component_id))
                .or_default()
                .push(component);
        }
        for ((mod_key, component_id), rows) in by_key {
            if rows.len() <= 1 {
                continue;
            }
            let display_mod = rows[0].mod_name.clone();
            issues.push(CompatIssue::new(
                CompatIssueCode::RuleHit,
                Severity::Error,
                IssueSource::ExternalRule {
                    file: "validator".to_string(),
                    line: 0,
                },
                display_mod.clone(),
                Some(component_id),
                display_mod,
                Some(component_id),
                format!(
                    "Component selected multiple times: {mod_key} #{component_id} appears {} times",
                    rows.len()
                ),
                Some("selected_set_duplicate".to_string()),
            ));
        }

        CompatValidationResult { issues }
    }
}

impl Default for CompatValidator {
    fn default() -> Self {
        Self::new()
    }
}
