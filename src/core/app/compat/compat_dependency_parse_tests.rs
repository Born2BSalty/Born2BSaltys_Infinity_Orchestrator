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

#[test]
fn loads_all_adrian_multiline_alternatives_for_each_optional_component() {
    let root = test_root();
    let tp2_path = root.0.join("adrian.tp2");
    let text = [10, 20, 30].map(|component| format!(
        "BEGIN @111\nDESIGNATED {component}\nREQUIRE_PREDICATE ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~)\n OR (MOD_IS_INSTALLED ~adrian.tp2~ ~1~)\n OR (MOD_IS_INSTALLED ~adrian.tp2~ ~2~)) @130\nINCLUDE ~adrian/lib/content.tpa~\n"
    )).join("\n");
    std::fs::write(&tp2_path, text).expect("write Adrian fixture");
    let requirements = load_component_requirements(&tp2_path.to_string_lossy());
    assert_eq!(requirements.len(), 3);
    for component in ["10", "20", "30"] {
        let rules = requirements
            .get(component)
            .expect("optional component rule");
        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0]
                .targets
                .iter()
                .map(|target| (
                    target.target_mod.as_str(),
                    target.target_component_id.as_str()
                ))
                .collect::<Vec<_>>(),
            [("adrian", "0"), ("adrian", "1"), ("adrian", "2")]
        );
    }
}

#[test]
fn multiline_predicates_preserve_comments_messages_and_neighboring_requirements() {
    let block = [
        "REQUIRE_PREDICATE ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~) /* ignored ) OR",
        "MOD_IS_INSTALLED ~wrong.tp2~ ~99~ */",
        " OR (MOD_IS_INSTALLED ~adrian.tp2~ ~1~)) // ignored (",
        " ~Message with (parentheses) and // literal",
        " continued text~",
        "REQUIRE_COMPONENT ~other.tp2~ 4 @10",
        "COPY ~input~ ~override~",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~third.tp2~ ~5~) @20",
    ];
    let parsed = crate::parser::compat_dependency_expr::collect_component_requirements(&block);
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].targets.len(), 2);
    assert_eq!(parsed[0].targets[1].target_component_id, "1");
    assert_eq!(parsed[1].targets[0].target_mod, "other");
    assert_eq!(parsed[2].targets[0].target_mod, "third");
}

#[test]
fn malformed_or_mixed_multiline_predicates_never_become_partial_dependencies() {
    for predicate in [
        "REQUIRE_PREDICATE ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~)",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~)) @130",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~)\n AND GAME_IS ~BG2EE~ @130",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~)\n OR NOT (MOD_IS_INSTALLED ~adrian.tp2~ ~1~) @130",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~)\n OR (GAME_IS ~BG2EE~) @130",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~) OR",
        "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~0~) ~unfinished",
    ] {
        let mut block = predicate.lines().collect::<Vec<_>>();
        block.extend([
            "COPY ~input~ ~override~",
            "REQUIRE_COMPONENT ~other.tp2~ 4 @10",
        ]);
        let parsed = crate::parser::compat_dependency_expr::collect_component_requirements(&block);
        assert_eq!(parsed.len(), 1, "{predicate}");
        assert_eq!(parsed[0].targets[0].target_mod, "other", "{predicate}");
    }
}

#[test]
fn supports_separate_operator_and_nested_or_continuations() {
    let block = [
        "REQUIRE_PREDICATE",
        " ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~)",
        " OR",
        " (MOD_IS_INSTALLED ~adrian.tp2~ ~1~))",
        " OR (MOD_IS_INSTALLED ~adrian.tp2~ ~2~)",
        " @130",
    ];
    let parsed = crate::parser::compat_dependency_expr::collect_component_requirements(&block);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].targets.len(), 3);
}

#[test]
fn multiline_messages_treat_statement_keywords_as_quoted_text() {
    for keyword in [
        "COPY",
        "INCLUDE",
        "BEGIN",
        "REQUIRE_COMPONENT",
        "REQUIRE_PREDICATE",
    ] {
        let text = format!(
            "REQUIRE_PREDICATE (MOD_IS_INSTALLED ~adrian.tp2~ ~1~) ~Required files:\n{keyword} the main component first.\nThen continue.~\nREQUIRE_COMPONENT ~other.tp2~ 4 @10"
        );
        let block = text.lines().collect::<Vec<_>>();
        let parsed = crate::parser::compat_dependency_expr::collect_component_requirements(&block);
        assert_eq!(parsed.len(), 2, "{keyword}");
        assert_eq!(parsed[0].targets[0].target_mod, "adrian", "{keyword}");
        assert_eq!(parsed[0].targets[0].target_component_id, "1");
        assert_eq!(parsed[1].targets[0].target_mod, "other", "{keyword}");
    }
}

#[test]
fn compat_three_fixes_legacy_bytes_retain_native_dependencies() {
    let root = test_root();
    let path = root.0.join("legacy.tp2");
    std::fs::write(
        &path,
        b"BEGIN ~Caf\xe9~\nREQUIRE_COMPONENT ~base.tp2~ 7 ~Required~\n",
    )
    .expect("legacy fixture");
    let requirements = load_component_requirements(path.to_str().expect("path"));
    assert_eq!(
        requirements.get("0").expect("legacy dependency")[0].targets[0].target_mod,
        "base"
    );
}

#[test]
fn compat_three_fixes_label_preserves_header_and_designated_ids() {
    let flags = crate::app::compat_tp2_blocks::collect_component_flags(
        "BEGIN ~First~\nLABEL Ascension-RewrittenFinalChapterOfToB\nDESIGNATED 60\nSUBCOMPONENT @100\nREQUIRE_PREDICATE GAME_IS ~bg2ee eet~ @1\nCOPY ~a~ ~b~\nBEGIN ~Second~\nSUBCOMPONENT @100\n",
    );
    assert_eq!(
        flags
            .iter()
            .map(|flag| flag.component_id.as_str())
            .collect::<Vec<_>>(),
        ["60", "61"]
    );
    assert_eq!(flags[0].predicates.len(), 1);
    assert!(flags[0].family.is_some());
}

#[test]
fn compat_three_fixes_shared_forbid_reaches_selected_order_runtime() {
    use crate::app::compat_conflict_parse::load_component_conflicts;
    use crate::app::compat_conflict_runtime::{
        build_conflict_scan_context, scan_conflict_hit_with_context,
    };
    use crate::app::compat_rule_runtime::CompatActiveItem;
    let root = test_root();
    let path = root.0.join("ascension.tp2");
    std::fs::write(&path, "BEGIN ~First~ DESIGNATED 60 SUBCOMPONENT @100\nFORBID_COMPONENT ~stratagems/setup-stratagems.tp2~ 5900 ~Install BEFORE SCS~\nCOPY ~a~ ~b~\nBEGIN ~Second~ DESIGNATED 61 SUBCOMPONENT @100\nCOPY ~a~ ~b~\n").expect("family fixture");
    let path = path.to_str().expect("path");
    let make_item = |tp_file: &str, component_id: &str, order| CompatActiveItem {
        tp_file: tp_file.to_string(),
        mod_name: tp_file.to_string(),
        tp2_path: if tp_file == "ascension" {
            path.to_string()
        } else {
            String::new()
        },
        component_id: component_id.to_string(),
        order: Some(order),
    };
    let context = build_conflict_scan_context(
        &[
            make_item("stratagems", "5900", 0),
            make_item("ascension", "61", 1),
        ],
        &mut std::collections::HashMap::new(),
    );
    let hit = scan_conflict_hit_with_context("ascension", "61", Some(1), &context)
        .expect("second family member must warn when selected after SCS");
    assert_eq!(hit.kind, "order_block");
    assert_eq!(hit.target_component_id, "5900");
    assert!(hit.message.contains("BEFORE"));
    let conflicts = load_component_conflicts(path);
    assert_eq!(conflicts.get("60").expect("direct control").len(), 1);
    assert_eq!(conflicts.get("61").expect("inherited rule").len(), 1);
    let context = build_conflict_scan_context(
        &[
            make_item("ascension", "61", 0),
            make_item("stratagems", "5900", 1),
        ],
        &mut std::collections::HashMap::new(),
    );
    assert!(scan_conflict_hit_with_context("ascension", "61", Some(0), &context).is_none());
}

#[test]
fn compat_three_fixes_label_matrix_preserves_predicates_and_expression_minus() {
    use crate::app::compat_tp2_blocks::collect_component_flags;
    for label in [
        "LABEL Ascension-ExpandedEpilogues",
        "LABEL AscalonsQuestpack-JobWellPaid",
        "LABEL Repeated--Hyphens",
        "LABEL ~Quoted-with-space label~",
        "LABEL \"Quoted-label\"",
    ] {
        for separator in [" ", "\n", " /* boundary */ "] {
            let text = format!(
                "BEGIN ~One~\n{label}{separator}DESIGNATED 40\nREQUIRE_PREDICATE (1 - 2) < -1 ~Message~\nCOPY ~a~ ~b~\nREQUIRE_PREDICATE FALSE ~Action~\nBEGIN ~Two~\n"
            );
            let flags = collect_component_flags(&text);
            assert_eq!(
                flags
                    .iter()
                    .map(|flag| flag.component_id.as_str())
                    .collect::<Vec<_>>(),
                ["40", "41"],
                "{text}"
            );
            assert_eq!(flags[0].predicates.len(), 1, "{text}");
            assert!(flags[0].predicates[0].expression.contains('-'));
        }
    }
}

#[test]
fn compat_three_fixes_malformed_labels_do_not_consume_following_flags_or_actions() {
    use crate::app::compat_tp2_blocks::collect_component_flags;
    for label in [
        "LABEL",
        "LABEL -bad",
        "LABEL bad-",
        "LABEL bad - atom",
        "LABEL bad/*gap*/-atom",
        "LABEL bad/atom",
    ] {
        let flags = collect_component_flags(&format!(
            "BEGIN ~One~\n{label}\nDESIGNATED 40\nREQUIRE_PREDICATE FALSE ~No leak~\n"
        ));
        assert_eq!(flags[0].component_id, "0", "{label}");
        assert!(flags[0].predicates.is_empty(), "{label}");
    }
    let flags = collect_component_flags(
        "BEGIN ~One~\nLABEL Good-Atom\nDESIGNATED 90\nCOPY ~a~ ~b~\nLABEL Action-Label\nREQUIRE_PREDICATE FALSE ~Action~\nBEGIN ~Two~ DESIGNATED 3\nLABEL Next-Atom REQUIRE_PREDICATE TRUE ~Good~\nBEGIN ~Third~\n",
    );
    assert_eq!(
        flags
            .iter()
            .map(|flag| flag.component_id.as_str())
            .collect::<Vec<_>>(),
        ["90", "3", "4"]
    );
    assert!(flags[0].predicates.is_empty());
    assert_eq!(flags[1].predicates.len(), 1);
}

#[test]
fn compat_three_fixes_binary_literal_does_not_hide_header_or_fabricate_flags() {
    use crate::app::compat_tp2_blocks::collect_component_flags;
    let root = test_root();
    let path = root.0.join("binary-literal.tp2");
    let bytes=b"BEGIN ~Real~\nREQUIRE_PREDICATE GAME_IS ~bgee bg2ee eet iwdee~ ~Supported~\nREQUIRE_COMPONENT ~base.tp2~ 7 ~Required~\nCOPY ~game.exe~ ~game.exe~\nOUTER_SPRINT binary ~f9\x83\x10G\0\0t% BEGIN REQUIRE_PREDICATE FORBID_COMPONENT~\n/* BEGIN ~Comment~ REQUIRE_PREDICATE FALSE ~No~ */\n<<<<<<<< payload\nBEGIN ~Inline~\nREQUIRE_PREDICATE FALSE ~No~\nFORBID_COMPONENT ~fake.tp2~ 0 ~No~\n>>>>>>>>\n";
    std::fs::write(&path, bytes).expect("binary literal fixture");
    let text = crate::app::compat_text::read_compat_text(&path).expect("literal bytes decode");
    let flags = collect_component_flags(&text);
    assert_eq!(flags.len(), 1);
    assert_eq!(flags[0].predicates.len(), 1);
    assert!(flags[0].forbids.is_empty());
    let rules = load_component_requirements(path.to_str().expect("path"));
    assert_eq!(rules["0"].len(), 1);
    assert_eq!(rules["0"][0].targets[0].target_mod, "base");
    let blob = root.0.join("blob.tp2");
    std::fs::write(&blob, [0x83, 0, 0x10, 0xff, 0x7f]).expect("blob");
    assert!(load_component_requirements(blob.to_str().expect("path")).is_empty());
    assert!(
        crate::app::compat_conflict_parse::load_component_conflicts(blob.to_str().expect("path"))
            .is_empty()
    );
}
