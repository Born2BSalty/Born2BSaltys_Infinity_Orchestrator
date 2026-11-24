use super::{Fixture, GUARD, builtin_metadata, item, named_mod, source_mod, state};
use crate::app::compat_logic::apply_step2_compat_rules_with_load;
use crate::app::compat_mismatch_eval::{TriState, build_mismatch_context, evaluate_requirement};
use crate::app::compat_predicate_constraints::guard_matches;
use crate::app::compat_rules::CompatRulesLoad;
use crate::app::compat_step3_rules::{
    collect_step3_compat_result_with_metadata_and_rules, marker_key,
};

#[test]
fn continuous_regression_unrelated_negative_guards_keep_conflicts_in_click_and_plan_orders() {
    let fixture = Fixture::new();
    for (source, id, target, target_id) in [
        ("cdtweaks", "1035", "bg1npc", "29"),
        ("cdtweaks", "1036", "bg1npc", "29"),
        ("cdtweaks", "70", "iwdification", "10"),
        ("iwdification", "10", "cdtweaks", "70"),
    ] {
        let path = fixture.source(&format!("BEGIN ~Current~ DESIGNATED {id}\nREQUIRE_PREDICATE NOT MOD_IS_INSTALLED ~{target}.tp2~ ~{target_id}~ ~Conflict~\n"));
        for source_first in [false, true] {
            let mut current = source_mod(&path, &[id]);
            current.tp_file = format!("{source}.tp2");
            current.name = current.tp_file.clone();
            let mut mods = vec![current, named_mod(&format!("{target}.tp2"), target_id)];
            let mut rows = vec![
                item(&format!("{source}.tp2"), id),
                item(&format!("{target}.tp2"), target_id),
            ];
            for row in &mut rows {
                row.mod_name = row.tp_file.clone();
            }
            for reverse in [false, true] {
                let mut items = rows.clone();
                if reverse {
                    items.reverse();
                }
                let result = collect_step3_compat_result_with_metadata_and_rules(
                    &state(),
                    "BG2EE",
                    &mods,
                    &items,
                    &[],
                    &builtin_metadata(),
                    &[],
                );
                assert!(result.constraint_outcomes.is_empty());
                assert_eq!(
                    result.markers[&marker_key(&rows[0])].kind,
                    "conflict",
                    "{source}/{id}/{reverse}"
                );
            }
            mods[usize::from(source_first)].components[0].checked = false;
            apply_step2_compat_rules_with_load(
                &state(),
                &mut [],
                &mut mods,
                &builtin_metadata(),
                CompatRulesLoad {
                    rules: Vec::new(),
                    error: None,
                },
            );
            mods[usize::from(source_first)].components[0].checked = true;
            apply_step2_compat_rules_with_load(
                &state(),
                &mut [],
                &mut mods,
                &builtin_metadata(),
                CompatRulesLoad {
                    rules: Vec::new(),
                    error: None,
                },
            );
            assert_eq!(
                mods[0].components[0].compat_kind.as_deref(),
                Some("conflict"),
                "{source}/{id}/{source_first}"
            );
        }
    }
    fixture.finish();
}

#[test]
fn continuous_regression_positive_or_dependencies_and_and_limitation_are_preserved() {
    let fixture = Fixture::new();
    for (id, tp, alternatives) in [
        ("1", "Alora.tp2", vec!["0", "1"]),
        ("9", "YESLICKNPC.tp2", vec!["0", "1"]),
        ("10", "Kivan.tp2", vec!["100"]),
        ("1", "EEex.tp2", vec!["0", "1"]),
    ] {
        let expression = alternatives
            .iter()
            .map(|target_id| format!("MOD_IS_INSTALLED ~{tp}~ ~{target_id}~"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let path = fixture.source(&format!("BEGIN ~Current~ DESIGNATED {id}\nREQUIRE_PREDICATE ({expression}) ~Required~\nREQUIRE_PREDICATE {GUARD} ~Before~\n"));
        let mut mods = vec![source_mod(&path, &[id])];
        let current = item("Continuous_End.tp2", id);
        let missing = collect_step3_compat_result_with_metadata_and_rules(
            &state(),
            "BG2EE",
            &mods,
            std::slice::from_ref(&current),
            &[],
            &builtin_metadata(),
            &[],
        );
        assert_eq!(
            missing.markers[&marker_key(&current)].kind,
            "missing_dep",
            "{tp}"
        );
        for target_id in alternatives {
            mods.truncate(1);
            mods.push(named_mod(tp, target_id));
            for earlier in [false, true] {
                let target = item(tp, target_id);
                let items = if earlier {
                    vec![target, current.clone()]
                } else {
                    vec![current.clone(), target]
                };
                let result = collect_step3_compat_result_with_metadata_and_rules(
                    &state(),
                    "BG2EE",
                    &mods,
                    &items,
                    &[],
                    &builtin_metadata(),
                    &[],
                );
                assert_eq!(
                    result
                        .markers
                        .get(&marker_key(&current))
                        .map(|marker| marker.kind.as_str()),
                    if earlier { None } else { Some("order_block") },
                    "{tp}/{target_id}/{earlier}"
                );
            }
        }
    }
    let expression = "MOD_IS_INSTALLED ~ISRA.TP2~ ~0~ AND MOD_IS_INSTALLED ~ISRA_BG2.TP2~ ~0~";
    assert!(
        crate::parser::compat_dependency_expr::parse_mod_is_installed_dependency_targets(
            expression
        )
        .is_none(),
        "existing AND is not flattened into alternatives"
    );
    for (selected, expected) in [
        (vec![], TriState::False),
        (vec![("isra".to_string(), "0".to_string())], TriState::False),
        (
            vec![
                ("isra".to_string(), "0".to_string()),
                ("isra_bg2".to_string(), "0".to_string()),
            ],
            TriState::True,
        ),
    ] {
        let context = build_mismatch_context(&state(), "BG2EE", selected.into_iter().collect());
        assert_eq!(evaluate_requirement(expression, &context), expected);
    }
    fixture.finish();
}

#[test]
fn continuous_regression_boolean_comparison_and_mixed_guards_keep_legacy_results() {
    use TriState::{False, Ignored, True, Unknown};
    let context = build_mismatch_context(
        &state(),
        "BG2EE",
        [
            ("eet".to_string(), "0".to_string()),
            ("x".to_string(), "0".to_string()),
        ]
        .into_iter()
        .collect(),
    );
    let metadata = builtin_metadata();
    for (name, expression, expected) in [
        (
            "true-or-negative",
            "TRUE OR NOT MOD_IS_INSTALLED ~x.tp2~ ~0~",
            True,
        ),
        (
            "not-or",
            "NOT (MOD_IS_INSTALLED ~x.tp2~ ~0~ OR MOD_IS_INSTALLED ~y.tp2~ ~1~)",
            False,
        ),
        (
            "dlcmerger-comparison",
            "NOT ~%WEIDU_ARCH%~ STR_EQ ~x86~",
            Unknown,
        ),
        ("eet-comparison", "(MOD_IS_INSTALLED ~x.tp2~ ~0~)=0", False),
        (
            "skills-label-lookup",
            "MOD_IS_INSTALLED ~EEex.tp2~ (ID_OF_LABEL ~EEex.tp2~ ~B3-EEex-Main~)",
            Unknown,
        ),
        (
            "eefixpack-nested",
            "((GAME_IS ~bgee~ AND FILE_EXISTS ~bgee.tph~) OR (GAME_IS ~bg2ee~ AND FILE_EXISTS ~bg2ee.tph~))",
            False,
        ),
        (
            "itemupgrade-language",
            "((\"%LANGUAGE%\" STRING_COMPARE_CASE \"american\" = 0) OR (\"%LANGUAGE%\" STRING_COMPARE_CASE \"french\" = 0))",
            Unknown,
        ),
        (
            "endless-game-file",
            "((GAME_IS ~eet~) OR (GAME_IS ~bgee~ AND FILE_EXISTS_IN_GAME ~bd0103.are~))",
            True,
        ),
        ("ignored-file", "FILE_EXISTS ~unresolved.tph~", Ignored),
        (
            "atweaks-mixed",
            "!MOD_IS_INSTALLED item_rev.tp2 0 AND !MOD_IS_INSTALLED bgttweak.tp2 2204 AND !GAME_IS ca",
            True,
        ),
    ] {
        assert!(!guard_matches(&metadata.records[0], expression), "{name}");
        assert_eq!(
            evaluate_requirement(expression, &context),
            expected,
            "{name}"
        );
    }
}

#[test]
fn continuous_regression_same_mod_click_orders_still_reject_conflicting_coselection() {
    let fixture = Fixture::new();
    for id in ["1", "2", "3", "4", "5", "7", "8", "9", "10"] {
        for deferred_first in [false, true] {
            let order = format!("REQUIRE_PREDICATE {GUARD} ~Before~");
            let exclusion = format!(
                "REQUIRE_PREDICATE NOT MOD_IS_INSTALLED ~Continuous_End.tp2~ ~{id}~ ~Exclusive~"
            );
            let guards = if deferred_first {
                format!("{order}\n{exclusion}")
            } else {
                format!("{exclusion}\n{order}")
            };
            let path = fixture.source(&format!(
                "BEGIN ~All~ DESIGNATED 0\n{guards}\nBEGIN ~Individual~ DESIGNATED {id}\n{order}\n"
            ));
            for all_first in [false, true] {
                let mut mods = vec![source_mod(&path, &["0", id])];
                let second = usize::from(all_first);
                mods[0].components[second].checked = false;
                apply_step2_compat_rules_with_load(
                    &state(),
                    &mut [],
                    &mut mods,
                    &builtin_metadata(),
                    CompatRulesLoad {
                        rules: Vec::new(),
                        error: None,
                    },
                );
                mods[0].components[second].checked = true;
                apply_step2_compat_rules_with_load(
                    &state(),
                    &mut [],
                    &mut mods,
                    &builtin_metadata(),
                    CompatRulesLoad {
                        rules: Vec::new(),
                        error: None,
                    },
                );
                assert_eq!(
                    mods[0].components[0].compat_kind.as_deref(),
                    Some("conflict"),
                    "{id}/{deferred_first}/{all_first}"
                );
                assert!(!mods[0].components[0].disabled);
                assert!(mods[0].components[0].checked);
            }
        }
    }
    fixture.finish();
}

#[test]
fn continuous_regression_forbid_order_and_conflict_survive_deferred_guard() {
    let fixture = Fixture::new();
    for reciprocal in [false, true] {
        for deferred_first in [false, true] {
            let before = format!("REQUIRE_PREDICATE {GUARD} ~Before~");
            let independent = "FORBID_COMPONENT ~target.tp2~ 0 ~Before target~";
            let guards = if deferred_first {
                format!("{before}\n{independent}")
            } else {
                format!("{independent}\n{before}")
            };
            let path = fixture.source(&format!("BEGIN ~C~ DESIGNATED 0\n{guards}\n"));
            let target_path = fixture.0.join("target.tp2");
            std::fs::write(&target_path, if reciprocal { "BEGIN ~Target~ DESIGNATED 0\nFORBID_COMPONENT ~Continuous_End.tp2~ 0 ~Conflict~\n" } else { "BEGIN ~Target~ DESIGNATED 0\n" }).expect("target source");
            let mut target = named_mod("target.tp2", "0");
            target.tp2_path = target_path.to_string_lossy().into_owned();
            let mut mods = vec![source_mod(&path, &["0"]), target];
            for target_before in [false, true] {
                let current = item("Continuous_End.tp2", "0");
                let mut target = item("target.tp2", "0");
                target.mod_name = "target.tp2".to_string();
                let mut items = vec![item("EET_End.tp2", "0")];
                if target_before {
                    items.extend([target, current.clone()]);
                } else {
                    items.extend([current.clone(), target]);
                }
                let result = collect_step3_compat_result_with_metadata_and_rules(
                    &state(),
                    "BG2EE",
                    &mods,
                    &items,
                    &[],
                    &builtin_metadata(),
                    &[],
                );
                assert_eq!(result.constraint_outcomes.len(), 1);
                assert_eq!(
                    result.markers[&marker_key(&current)].kind,
                    if reciprocal {
                        "conflict"
                    } else {
                        "order_block"
                    },
                    "{reciprocal}/{deferred_first}/{target_before}"
                );
            }
            apply_step2_compat_rules_with_load(
                &state(),
                &mut [],
                &mut mods,
                &builtin_metadata(),
                CompatRulesLoad {
                    rules: Vec::new(),
                    error: None,
                },
            );
            assert_eq!(
                mods[0].components[0].compat_kind.as_deref(),
                reciprocal.then_some("conflict")
            );
        }
    }
    fixture.finish();
}

#[test]
fn continuous_regression_require_order_survives_deferred_guard() {
    let fixture = Fixture::new();
    for deferred_first in [false, true] {
        let before = format!("REQUIRE_PREDICATE {GUARD} ~Before~");
        let required = "REQUIRE_COMPONENT ~target.tp2~ 0 ~Required earlier~";
        let guards = if deferred_first {
            format!("{before}\n{required}")
        } else {
            format!("{required}\n{before}")
        };
        let path = fixture.source(&format!("BEGIN ~C~ DESIGNATED 0\n{guards}\n"));
        let mut mods = vec![source_mod(&path, &["0"]), named_mod("target.tp2", "0")];
        let current = item("Continuous_End.tp2", "0");
        let items = [
            item("EET_End.tp2", "0"),
            current.clone(),
            item("target.tp2", "0"),
        ];
        let result = collect_step3_compat_result_with_metadata_and_rules(
            &state(),
            "BG2EE",
            &mods,
            &items,
            &[],
            &builtin_metadata(),
            &[],
        );
        assert_eq!(result.constraint_outcomes.len(), 1);
        assert_eq!(result.markers[&marker_key(&current)].kind, "order_block");
        assert!(
            result.markers[&marker_key(&current)]
                .raw_evidence
                .as_deref()
                .is_some_and(|evidence| evidence.contains("REQUIRE_COMPONENT"))
        );
        apply_step2_compat_rules_with_load(
            &state(),
            &mut [],
            &mut mods,
            &builtin_metadata(),
            CompatRulesLoad {
                rules: Vec::new(),
                error: None,
            },
        );
        assert_eq!(mods[0].components[0].compat_kind, None);
    }
    fixture.finish();
}

#[test]
fn continuous_regression_planning_preserves_noncopy_and_resume_skip_installed_arguments() {
    use crate::app::state::ResumeTargets;
    use crate::install::step5_command_config::{InstallCommandConfig, SafetyOptions};
    use crate::install::step5_command_install::build_install_invocation;
    use crate::install::step5_command_resume::build_resume_invocation;
    let fixture = Fixture::new();
    let path = fixture.source(&format!(
        "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
    ));
    let mods = [source_mod(&path, &["0"])];
    let items = [item("Continuous_End.tp2", "0"), item("EET_End.tp2", "0")];
    for skip_installed in [false, true] {
        let config = InstallCommandConfig {
            game_install: "EET".to_string(),
            safety: SafetyOptions {
                skip_installed,
                ..SafetyOptions::default()
            },
            ..InstallCommandConfig::default()
        };
        let targets = ResumeTargets::default();
        let before = [
            build_install_invocation(&config),
            build_resume_invocation(&config, &targets),
        ];
        let result = collect_step3_compat_result_with_metadata_and_rules(
            &state(),
            "BG2EE",
            &mods,
            &items,
            &[],
            &builtin_metadata(),
            &[],
        );
        assert!(result.constraint_outcomes.is_empty());
        let after = [
            build_install_invocation(&config),
            build_resume_invocation(&config, &targets),
        ];
        assert_eq!(before, after);
        for (_, args) in after {
            assert!(
                args.windows(2)
                    .any(|pair| pair[0] == "--skip-installed"
                        && pair[1] == skip_installed.to_string())
            );
            assert!(
                !args
                    .iter()
                    .any(|arg| arg == "--pre-eet-directory" || arg == "--eet-directory")
            );
        }
    }
    fixture.finish();
}
