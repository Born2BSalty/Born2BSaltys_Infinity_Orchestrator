// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use super::super::model::{
    CompatIssue, CompatIssueCode, CompatIssueInit, IssueSource, Severity, Tp2Metadata,
};
use super::validator_helpers as helpers;
use super::validator_rule_handlers::{apply_step2_selection_rule, apply_step3_selection_rule};
use super::{RuleEvalContext, SelectedComponent};

pub(super) fn validate_duplicates(selected: &[SelectedComponent]) -> Vec<CompatIssue> {
    let mut issues = Vec::new();
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
        issues.push(CompatIssue::new(CompatIssueInit {
            code: CompatIssueCode::RuleHit,
            severity: Severity::Error,
            source: IssueSource::ExternalRule {
                file: "validator".to_string(),
                line: 0,
            },
            affected_mod: display_mod.clone(),
            affected_component: Some(component_id),
            related_mod: display_mod,
            related_component: Some(component_id),
            reason: format!(
                "Component selected multiple times: {mod_key} #{component_id} appears {} times",
                rows.len()
            ),
            raw_evidence: Some("selected_set_duplicate".to_string()),
            component_block: None,
        }));
    }

    issues
}

pub(super) fn validate_step2_selection_rules(
    selected: &[SelectedComponent],
    game_mode: &str,
    tp2_metadata: &HashMap<String, Tp2Metadata>,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) -> Vec<CompatIssue> {
    let mut issues = Vec::new();

    for component in selected {
        let key = helpers::normalize_mod_key(&component.tp_file);
        let Some(metadata) = tp2_metadata.get(&key) else {
            continue;
        };

        for (comp_id, rule) in &metadata.rules {
            if *comp_id != component.component_id {
                continue;
            }
            let ctx = RuleEvalContext {
                metadata,
                component,
                game_mode,
                selected_set,
                order_map,
            };
            apply_step2_selection_rule(&mut issues, &ctx, rule);
        }
    }

    issues
}

pub(super) fn validate_step3_selection_rules(
    selected: &[SelectedComponent],
    game_mode: &str,
    tp2_metadata: &HashMap<String, Tp2Metadata>,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) -> Vec<CompatIssue> {
    let mut issues = Vec::new();

    for component in selected {
        let key = helpers::normalize_mod_key(&component.tp_file);
        let Some(metadata) = tp2_metadata.get(&key) else {
            continue;
        };

        for (comp_id, rule) in &metadata.rules {
            if *comp_id != component.component_id {
                continue;
            }
            let ctx = RuleEvalContext {
                metadata,
                component,
                game_mode,
                selected_set,
                order_map,
            };
            apply_step3_selection_rule(&mut issues, &ctx, rule);
        }
    }

    issues
}
