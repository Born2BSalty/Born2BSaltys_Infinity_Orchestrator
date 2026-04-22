// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::compat_dependency_expr::{
    normalize_component_id, parse_mod_is_installed_dependency_targets,
    parse_negated_mod_is_installed_targets, parse_predicate_requirement_line,
    parse_requirement_line, parse_simple_mod_is_installed_predicate,
};

#[test]
fn parses_tilde_requirement_component() {
    let parsed =
        parse_requirement_line(r#"REQUIRE_COMPONENT ~bg1npc/bg1npc.tp2~ 0 @1004 /* comment */"#)
            .expect("requirement should parse");
    assert_eq!(parsed.targets.len(), 1);
    assert_eq!(parsed.targets[0].target_mod, "bg1npc");
    assert_eq!(parsed.targets[0].target_component_id, "0");
    assert_eq!(parsed.message, None);
}

#[test]
fn parses_quoted_requirement_component() {
    let parsed = parse_requirement_line(
        r#"REQUIRE_COMPONENT "setup-arestorationp.tp2" "11" ~Requires the previous component.~"#,
    )
    .expect("requirement should parse");
    assert_eq!(parsed.targets.len(), 1);
    assert_eq!(parsed.targets[0].target_mod, "arestorationp");
    assert_eq!(parsed.targets[0].target_component_id, "11");
    assert_eq!(
        parsed.message.as_deref(),
        Some("Requires the previous component.")
    );
}

#[test]
fn ignores_commented_requirement_component() {
    assert!(parse_requirement_line(r#"// REQUIRE_COMPONENT ~foo.tp2~ 0 @1"#).is_none());
    assert!(parse_requirement_line(r#"/* REQUIRE_COMPONENT ~foo.tp2~ 0 @1 */"#).is_none());
}

#[test]
fn normalizes_component_ids() {
    assert_eq!(normalize_component_id("007").as_deref(), Some("7"));
    assert_eq!(normalize_component_id("~000~").as_deref(), Some("0"));
    assert_eq!(normalize_component_id("abc"), None);
}

#[test]
fn parses_simple_predicate_requirement_component() {
    let parsed = parse_predicate_requirement_line(
        r#"REQUIRE_PREDICATE (MOD_IS_INSTALLED ~EEEX/EEEX.TP2~ ~0~) ~This component requires EEEx.~"#,
    )
    .expect("predicate requirement should parse");
    assert_eq!(parsed.targets.len(), 1);
    assert_eq!(parsed.targets[0].target_mod, "eeex");
    assert_eq!(parsed.targets[0].target_component_id, "0");
    assert_eq!(parsed.message, None);
}

#[test]
fn parses_or_predicate_requirement_components() {
    let parsed = parse_predicate_requirement_line(
        r#"REQUIRE_PREDICATE (MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~) ~Needs one of them.~"#,
    )
    .expect("predicate requirement should parse");
    assert_eq!(parsed.targets.len(), 2);
    assert_eq!(parsed.targets[0].target_mod, "foo");
    assert_eq!(parsed.targets[0].target_component_id, "1");
    assert_eq!(parsed.targets[1].target_mod, "bar");
    assert_eq!(parsed.targets[1].target_component_id, "2");
}

#[test]
fn ignores_negated_predicate_requirement_component() {
    assert!(
        parse_predicate_requirement_line(
            r#"REQUIRE_PREDICATE (!MOD_IS_INSTALLED ~foo.tp2~ ~1~) ~Only when missing.~"#
        )
        .is_none()
    );
}

#[test]
fn ignores_compound_predicate_requirement_component() {
    assert!(
        parse_mod_is_installed_dependency_targets(
            r#"(MOD_IS_INSTALLED ~foo.tp2~ ~1~ AND GAME_IS ~BG2EE~) ~Mixed predicate.~"#
        )
        .is_none()
    );
}

#[test]
fn ignores_mixed_or_predicate_requirement_component() {
    assert!(
        parse_mod_is_installed_dependency_targets(
            r#"(GAME_IS ~BG2EE~) OR (MOD_IS_INSTALLED ~foo.tp2~ ~1~) ~Mixed predicate.~"#
        )
        .is_none()
    );
}

#[test]
fn ignores_multi_target_predicate_for_simple_parser() {
    assert!(
        parse_simple_mod_is_installed_predicate(
            r#"(MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~) ~Mixed predicate.~"#
        )
        .is_none()
    );
}

#[test]
fn parses_negated_mod_is_installed_targets() {
    let parsed = parse_negated_mod_is_installed_targets(
        r#"NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~) AND !MOD_IS_INSTALLED ~bar.tp2~ ~2~"#,
    )
    .expect("negated targets should parse");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].target_mod, "foo");
    assert_eq!(parsed[0].target_component_id, "1");
    assert_eq!(parsed[1].target_mod, "bar");
    assert_eq!(parsed[1].target_component_id, "2");
}

#[test]
fn ignores_broad_negated_group_for_negated_target_parser() {
    assert!(
        parse_negated_mod_is_installed_targets(
            r#"NOT((MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~))"#
        )
        .is_none()
    );
}
