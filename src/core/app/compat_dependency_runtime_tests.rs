// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use super::super::compat_rule_runtime::CompatActiveItem;
use super::{
    ComponentRequirement, ComponentRequirementCache, ComponentRequirementTarget,
    DependencyEvalMode, scan_dependency_hit,
};

#[test]
fn reports_missing_dep_for_or_requirement_when_none_selected() {
    let mut cache = requirement_cache_with(vec![requirement(
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~)",
        &[("foo", "1"), ("bar", "2")],
    )]);

    let hit = scan_dependency_hit(
        "dummy.tp2",
        "3001",
        Some(2),
        &[],
        &mut cache,
        DependencyEvalMode::MissingOnly,
    )
    .expect("missing dep hit expected");

    assert_eq!(hit.kind, "missing_dep");
    assert_eq!(hit.message, "Requires one of: foo #1 OR bar #2");
    assert_eq!(hit.target_mod, "foo");
    assert_eq!(hit.target_component_id, "1");
}

#[test]
fn reports_order_block_for_or_requirement_when_only_later_target_selected() {
    let mut cache = requirement_cache_with(vec![requirement(
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~)",
        &[("foo", "1"), ("bar", "2")],
    )]);
    let active_items = vec![active_item("bar.tp2", "2", 5)];

    let hit = scan_dependency_hit(
        "dummy.tp2",
        "3001",
        Some(2),
        &active_items,
        &mut cache,
        DependencyEvalMode::MissingThenOrder,
    )
    .expect("order hit expected");

    assert_eq!(hit.kind, "order_block");
    assert_eq!(hit.target_mod, "bar");
    assert_eq!(hit.target_component_id, "2");
}

#[test]
fn ignores_later_or_target_when_an_earlier_target_already_satisfies_requirement() {
    let mut cache = requirement_cache_with(vec![requirement(
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~)",
        &[("foo", "1"), ("bar", "2")],
    )]);
    let active_items = vec![
        active_item("foo.tp2", "1", 1),
        active_item("bar.tp2", "2", 5),
    ];

    let hit = scan_dependency_hit(
        "dummy.tp2",
        "3001",
        Some(2),
        &active_items,
        &mut cache,
        DependencyEvalMode::MissingThenOrder,
    );

    assert!(hit.is_none());
}

fn requirement(raw_line: &str, targets: &[(&str, &str)]) -> ComponentRequirement {
    ComponentRequirement {
        raw_line: raw_line.to_string(),
        targets: targets
            .iter()
            .map(
                |(target_mod, target_component_id)| ComponentRequirementTarget {
                    target_mod: (*target_mod).to_string(),
                    target_component_id: (*target_component_id).to_string(),
                },
            )
            .collect(),
        message: None,
    }
}

fn requirement_cache_with(requirements: Vec<ComponentRequirement>) -> ComponentRequirementCache {
    let mut by_component = HashMap::new();
    by_component.insert("3001".to_string(), requirements);

    let mut cache = HashMap::new();
    cache.insert("dummy.tp2".to_string(), by_component);
    cache
}

fn active_item(tp_file: &str, component_id: &str, order: usize) -> CompatActiveItem {
    CompatActiveItem {
        tp_file: tp_file.to_string(),
        mod_name: String::new(),
        tp2_path: String::new(),
        component_id: component_id.to_string(),
        order: Some(order),
    }
}
