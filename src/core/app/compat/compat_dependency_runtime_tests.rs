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

#[test]
fn adrian_multiline_optional_components_accept_every_main_alternative() {
    let block = [
        "REQUIRE_PREDICATE ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~)",
        " OR (MOD_IS_INSTALLED ~adrian.tp2~ ~1~)",
        " OR (MOD_IS_INSTALLED ~adrian.tp2~ ~2~)) @130",
    ];
    let parsed = crate::parser::compat_dependency_expr::collect_component_requirements(&block);
    assert_eq!(parsed.len(), 1);
    let rules = parsed
        .into_iter()
        .map(|parsed| ComponentRequirement {
            raw_line: parsed.raw_line,
            targets: parsed
                .targets
                .into_iter()
                .map(|target| ComponentRequirementTarget {
                    target_mod: target.target_mod,
                    target_component_id: target.target_component_id,
                })
                .collect(),
            message: parsed.message,
        })
        .collect::<Vec<_>>();
    for optional in ["10", "20", "30"] {
        let mut cache = HashMap::from([(
            "adrian.tp2".to_string(),
            HashMap::from([(optional.to_string(), rules.clone())]),
        )]);
        for main in ["0", "1", "2"] {
            for mode in [
                DependencyEvalMode::MissingOnly,
                DependencyEvalMode::MissingThenOrder,
            ] {
                assert!(
                    scan_dependency_hit(
                        "adrian.tp2",
                        optional,
                        Some(1),
                        &[active_item("adrian.tp2", main, 0)],
                        &mut cache,
                        mode
                    )
                    .is_none(),
                    "main {main}, optional {optional}"
                );
            }
        }
        let missing = scan_dependency_hit(
            "adrian.tp2",
            optional,
            Some(1),
            &[],
            &mut cache,
            DependencyEvalMode::MissingOnly,
        )
        .expect("main component is required");
        assert_eq!(missing.kind, "missing_dep");
        assert_eq!(
            missing.message,
            "Requires one of: adrian #0 OR adrian #1 OR adrian #2"
        );
    }
}
