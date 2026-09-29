// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{ComponentRequirementTarget, load_component_requirements};
use crate::parser::compat_dependency_expr::{
    normalize_component_id, parse_mod_is_installed_dependency_targets,
    parse_negated_mod_is_installed_targets, parse_predicate_requirement_line,
    parse_requirement_line, parse_simple_mod_is_installed_predicate,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn test_root() -> TestRoot {
    let root = TestRoot(std::env::temp_dir().join(format!(
        "bio_dependency_parse_{}_{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&root.0).expect("create test root");
    root
}

#[test]
fn a_kale_shaped_file_keys_its_requirement_under_weidus_number() {
    let root = test_root();
    let tp2_path = root.0.join("setup-kale.tp2");
    let tp2_text = [
        "BACKUP ~kale/backup~",
        "AUTHOR ~x~",
        "BEGIN @10",
        "COPY ~kale/a~ ~override~",
        "BEGIN @20",
        "COPY ~kale/b~ ~override~",
        "BEGIN @30",
        "SUBCOMPONENT @54",
        "REQUIRE_COMPONENT ~OFPATHSANDWAYS.TP2~ ~12~ ~The 3-Foot-Tall-Fury Kit must be installed.~",
        "COPY ~kale/c~ ~override~",
        "BEGIN @40",
        "COPY ~kale/d~ ~override~",
    ]
    .join("\n");
    std::fs::write(&tp2_path, tp2_text).expect("write tp2");

    let requirements = load_component_requirements(&tp2_path.to_string_lossy());

    assert_eq!(requirements.len(), 1);
    let fury = requirements.get("2").expect("requirement keyed under 2");
    assert_eq!(fury.len(), 1);
    assert_eq!(
        fury[0].targets,
        [ComponentRequirementTarget {
            target_mod: "ofpathsandways".to_string(),
            target_component_id: "12".to_string(),
        }]
    );
    assert_eq!(
        fury[0].message.as_deref(),
        Some("The 3-Foot-Tall-Fury Kit must be installed.")
    );
}

#[test]
fn parses_tilde_requirement_component() {
    let parsed =
        parse_requirement_line(r"REQUIRE_COMPONENT ~bg1npc/bg1npc.tp2~ 0 @1004 /* comment */")
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
    assert!(parse_requirement_line(r"// REQUIRE_COMPONENT ~foo.tp2~ 0 @1").is_none());
    assert!(parse_requirement_line(r"/* REQUIRE_COMPONENT ~foo.tp2~ 0 @1 */").is_none());
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
        r"REQUIRE_PREDICATE (MOD_IS_INSTALLED ~EEEX/EEEX.TP2~ ~0~) ~This component requires EEEx.~",
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
        r"REQUIRE_PREDICATE (MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~) ~Needs one of them.~",
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
            r"REQUIRE_PREDICATE (!MOD_IS_INSTALLED ~foo.tp2~ ~1~) ~Only when missing.~"
        )
        .is_none()
    );
}

#[test]
fn ignores_compound_predicate_requirement_component() {
    assert!(
        parse_mod_is_installed_dependency_targets(
            r"(MOD_IS_INSTALLED ~foo.tp2~ ~1~ AND GAME_IS ~BG2EE~) ~Mixed predicate.~"
        )
        .is_none()
    );
}

#[test]
fn ignores_mixed_or_predicate_requirement_component() {
    assert!(
        parse_mod_is_installed_dependency_targets(
            r"(GAME_IS ~BG2EE~) OR (MOD_IS_INSTALLED ~foo.tp2~ ~1~) ~Mixed predicate.~"
        )
        .is_none()
    );
}

#[test]
fn parses_mod_is_installed_with_int_marker_predicate_component() {
    let parsed = parse_mod_is_installed_dependency_targets(
        r#"MOD_IS_INSTALLED "setup-stratagems.tp2" 5900 || IS_AN_INT stratagems_component_5900_installed @16025"#,
    )
    .expect("SCS install-session marker predicate should parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].target_mod, "stratagems");
    assert_eq!(parsed[0].target_component_id, "5900");
}

#[test]
fn ignores_multi_target_predicate_for_simple_parser() {
    assert!(
        parse_simple_mod_is_installed_predicate(
            r"(MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~) ~Mixed predicate.~"
        )
        .is_none()
    );
}

#[test]
fn parses_negated_mod_is_installed_targets() {
    let parsed = parse_negated_mod_is_installed_targets(
        r"NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~) AND !MOD_IS_INSTALLED ~bar.tp2~ ~2~",
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
            r"NOT((MOD_IS_INSTALLED ~foo.tp2~ ~1~) OR (MOD_IS_INSTALLED ~bar.tp2~ ~2~))"
        )
        .is_none()
    );
}
