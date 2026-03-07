// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use super::super::model::{CompatIssue, CompatIssueCode, IssueSource, Severity, Tp2Metadata};
use super::SelectedComponent;

pub(super) fn handle_require_installed_any(
    issues: &mut Vec<CompatIssue>,
    metadata: &Tp2Metadata,
    component: &SelectedComponent,
    targets: &[(String, Option<u32>)],
    raw_line: &str,
    line: usize,
    selected_set: &HashSet<(String, u32)>,
    order_map: &HashMap<(String, u32), usize>,
) {
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
                line,
            },
            component.mod_name.clone(),
            Some(component.component_id),
            related_mod,
            related_component,
            format!("Requires one of: {related_text}"),
            Some(raw_line.to_string()),
        ));
        return;
    }

    let mut matched_orders: Vec<usize> = Vec::new();
    for (target_mod, target_component) in targets {
        match target_component {
            Some(cid) => {
                if selected_set.contains(&(target_mod.clone(), *cid))
                    && let Some(order) = order_map.get(&(target_mod.clone(), *cid))
                {
                    matched_orders.push(*order);
                }
            }
            None => {
                for ((mod_key, _), order) in order_map {
                    if mod_key == target_mod {
                        matched_orders.push(*order);
                    }
                }
            }
        }
    }
    matched_orders.sort_unstable();
    matched_orders.dedup();

    if matched_orders.is_empty() || matched_orders.iter().any(|order| *order <= component.order) {
        return;
    }

    let (related_mod, related_component) = targets
        .first()
        .cloned()
        .unwrap_or_else(|| ("unknown".to_string(), None));
    issues.push(CompatIssue::new(
        CompatIssueCode::OrderWarn,
        Severity::Warning,
        IssueSource::Tp2 {
            file: metadata.tp_file.clone(),
            line,
        },
        component.mod_name.clone(),
        Some(component.component_id),
        related_mod,
        related_component,
        format!(
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
        Some(raw_line.to_string()),
    ));
}
