// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use super::super::compat_mismatch_eval::{
    MismatchContext, TriState, build_mismatch_context, evaluate_requirement,
};
use super::{
    RequirementGuard, classify_guard, collect_requirement_guards, preferred_failing_guard,
    preferred_guard_hit,
};
use crate::app::state::Step1State;

#[test]
fn classifies_positive_game_is_failure_as_mismatch() {
    let context = mismatch_context("BGEE", "BGEE", &[]);
    let classification = classify_guard(r"GAME_IS ~BG2EE EET~", &context);
    assert_eq!(classification.kind, "mismatch");
}

#[test]
fn classifies_negated_game_is_failure_as_mismatch() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    let classification = classify_guard(r"NOT GAME_IS ~BG2EE EET~", &context);
    assert_eq!(classification.kind, "mismatch");
}

#[test]
fn classifies_game_includes_failure_as_conditional() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    let classification = classify_guard(r"GAME_INCLUDES ~SOD~", &context);
    assert_eq!(classification.kind, "conditional");
}

#[test]
fn classifies_mixed_game_and_mod_checks_as_conditional_when_game_matches() {
    let context = mismatch_context("BGEE", "BGEE", &[("foo", "0")]);
    let classification = classify_guard(
        r"GAME_IS ~BGEE~ AND NOT(MOD_IS_INSTALLED ~foo.tp2~ ~0~)",
        &context,
    );
    assert_eq!(classification.kind, "conditional");
}

#[test]
fn classifies_ascension_style_wrong_phase_guard_as_mismatch() {
    let context = mismatch_context("EET", "BGEE", &[]);
    let classification = classify_guard(
        r"GAME_IS ~bg2ee eet~ OR MOD_IS_INSTALLED ~setup-bg2fixpack.tp2~ ~0~",
        &context,
    );
    assert_eq!(classification.kind, "mismatch");
}

#[test]
fn prefers_mismatch_over_earlier_conditional_failure() {
    let context = mismatch_context("EET", "BGEE", &[]);
    let guards = vec![
        RequirementGuard {
            origin: None,
            display_line: "REQUIRE_PREDICATE GAME_INCLUDES ~tob~".to_string(),
            eval_text: "GAME_INCLUDES tob".to_string(),
        },
        RequirementGuard {
            origin: None,
            display_line: "REQUIRE_PREDICATE GAME_IS ~bg2ee eet~ OR MOD_IS_INSTALLED ~setup-bg2fixpack.tp2~ ~0~".to_string(),
            eval_text: "GAME_IS ~bg2ee eet~ OR MOD_IS_INSTALLED ~setup-bg2fixpack.tp2~ ~0~".to_string(),
        },
    ];

    let failing_guard = preferred_failing_guard(&guards, &context).expect("failing guard");
    assert_eq!(failing_guard.1, "mismatch");
}

#[test]
fn treats_eet_as_inactive_until_eet_core_is_checked() {
    let without_core = mismatch_context("EET", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~EET~", &without_core),
        TriState::False
    );

    let with_core = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~EET~", &with_core),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~BG2EE~", &with_core),
        TriState::False
    );
    assert_eq!(
        evaluate_requirement(r"ENGINE_IS ~BG2EE~", &with_core),
        TriState::True
    );
}

#[test]
fn blocks_bg2ee_game_is_component_after_eet_core_is_checked() {
    let context = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    let classification = classify_guard(r"GAME_IS ~bgee bg2ee iwdee~", &context);
    assert_eq!(classification.kind, "mismatch");
}

#[test]
fn brage_file_check_or_game_check_is_not_flagged_on_eet() {
    let guards = collect_requirement_guards(&[
        "BEGIN @0",
        "REQUIRE_PREDICATE (FILE_EXISTS_IN_GAME ~c#q13swd.itm~ OR GAME_IS ~tob bg2ee~ AND NOT GAME_IS ~bgt~) @9031",
        "DESIGNATED 0",
    ]);
    assert_eq!(guards.len(), 1);

    let context = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    assert_eq!(
        evaluate_requirement(&guards[0].eval_text, &context),
        TriState::Unknown
    );
    assert!(preferred_guard_hit(&guards, &context).is_none());
}

#[test]
fn upgrades_selected_negated_mod_guard_to_conflict() {
    let context = mismatch_context("BG2EE", "BG2EE", &[("foo", "1")]);
    let guards = vec![RequirementGuard {
        origin: None,
        display_line: "REQUIRE_PREDICATE NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~)".to_string(),
        eval_text: "NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~)".to_string(),
    }];

    let hit = preferred_guard_hit(&guards, &context).expect("guard hit");
    assert_eq!(hit.kind, "conflict");
    assert_eq!(hit.related_mod.as_deref(), Some("foo"));
    assert_eq!(hit.related_component.as_deref(), Some("1"));
}

#[test]
fn keeps_mismatch_precedence_over_selected_negated_mod_guard() {
    let context = mismatch_context("BGEE", "BGEE", &[("foo", "1")]);
    let guards = vec![RequirementGuard {
        origin: None,
        display_line: "REQUIRE_PREDICATE GAME_IS ~BG2EE~ AND NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~)"
            .to_string(),
        eval_text: "GAME_IS ~BG2EE~ AND NOT(MOD_IS_INSTALLED ~foo.tp2~ ~1~)".to_string(),
    }];

    let hit = preferred_guard_hit(&guards, &context).expect("guard hit");
    assert_eq!(hit.kind, "mismatch");
}

#[test]
fn strips_trailing_tra_reference_from_requirement_predicate() {
    let guards = collect_requirement_guards(&[
        "BEGIN @900000 //EET - resource importation",
        "REQUIRE_PREDICATE ENGINE_IS ~bg2ee~ @900001 //Please install it on BG2:EE engine",
        "DESIGNATED 0",
    ]);

    assert_eq!(guards.len(), 1);
    assert_eq!(guards[0].eval_text, "ENGINE_IS ~bg2ee~");
    assert!(!guards[0].display_line.contains("@900001"));

    let context = mismatch_context("EET", "BGEE", &[]);
    let failing_guard = preferred_failing_guard(&guards, &context).expect("failing guard");
    assert_eq!(failing_guard.1, "mismatch");
}

#[test]
fn strips_trailing_literal_message_from_requirement_predicate() {
    let guards = collect_requirement_guards(&[
        "BEGIN @32002",
        "REQUIRE_PREDICATE GAME_IS ~bgee bg2ee iwdee~ ~Game not supported.~",
        "DESIGNATED 32002",
    ]);

    assert_eq!(guards.len(), 1);
    assert_eq!(guards[0].eval_text, "GAME_IS ~bgee bg2ee iwdee~");
    assert!(!guards[0].display_line.contains("Game not supported"));

    let context = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    let failing_guard = preferred_failing_guard(&guards, &context).expect("failing guard");
    assert_eq!(failing_guard.1, "mismatch");
}

#[test]
fn keeps_required_game_value_when_requirement_has_no_message() {
    let guards = collect_requirement_guards(&[
        "BEGIN @900000",
        "REQUIRE_PREDICATE ENGINE_IS ~bg2ee~",
        "DESIGNATED 0",
    ]);

    assert_eq!(guards.len(), 1);
    assert_eq!(guards[0].eval_text, "ENGINE_IS ~bg2ee~");
}

fn mismatch_context(mode: &str, tab: &str, checked: &[(&str, &str)]) -> MismatchContext {
    let step1 = Step1State {
        game_install: mode.to_string(),
        ..Step1State::default()
    };
    let checked_components = checked
        .iter()
        .map(|(mod_name, component_id)| ((*mod_name).to_string(), (*component_id).to_string()))
        .collect::<HashSet<_>>();
    build_mismatch_context(&step1, tab, checked_components)
}

fn scs_fixture(classic_restriction: bool) -> String {
    let restriction = if classic_restriction {
        "REQUIRE_PREDICATE (GAME_IS ~bgt tutu tutu_totsc~) @15999\n"
    } else {
        ""
    };
    format!(
        "BACKUP ~backup~\n\nBEGIN @20074 DESIGNATED 3500 GROUP @4 SUBCOMPONENT @20076 (GAME_IS ~tutu tutu_totsc~)\n\nREQUIRE_PREDICATE (GAME_IS ~tutu tutu_totsc bgee bg2ee eet iwdee~ || MOD_IS_INSTALLED ~setup-bg2fixpack.tp2~ ~0~) @24124\n\nREQUIRE_PREDICATE !GAME_IS ~iwdee~ @50009\n\nREQUIRE_PREDICATE GAME_IS ~bgee eet bgt tutu tutu_totsc~ @16001\n{restriction}\nLAF run END\n\nBEGIN @20077 DESIGNATED 3501 GROUP @4 SUBCOMPONENT @20076\nLAF run END\n"
    )
}

fn with_scs_tp2<T>(text: &str, check: impl FnOnce(&str) -> T) -> T {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "bio-scs-requirements-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let path = directory.join("fixture.tp2");
    std::fs::write(&path, text).expect("fixture TP2");
    let result = check(path.to_str().expect("fixture path"));
    std::fs::remove_file(&path).expect("remove fixture");
    std::fs::remove_dir(&directory).expect("remove fixture directory");
    result
}

#[test]
fn scs_shared_first_requirement_blocks_second_option_with_origin() {
    with_scs_tp2(&scs_fixture(true), |path| {
        for context in [
            mismatch_context("BG2EE", "BG2EE", &[]),
            mismatch_context("BGEE", "BGEE", &[]),
            mismatch_context("EET", "BG2EE", &[("eet", "0")]),
        ] {
            let hit = super::scan_predicate_guard_hit(path, "3501", &context)
                .expect("3501 inherits first subcomponent requirements");
            assert_eq!(hit.kind, "mismatch");
            assert!(hit.raw_evidence.starts_with("REQUIRE_PREDICATE"));
            assert!(hit.raw_evidence.contains("3500"));
            assert!(hit.message.contains("3500"));
            assert!(hit.related_mod.is_none());
            assert!(hit.related_component.is_none());
        }
        let context = mismatch_context("BGEE", "BGEE", &[]);
        let hit = super::scan_predicate_guard_hit(path, "3501", &context).expect("shared guard");
        assert!(hit.raw_evidence.contains("line 10"));
        assert!(hit.message.contains("line 10"));
    });
}

#[test]
fn scs_inline_first_option_predicate_stays_local() {
    with_scs_tp2(&scs_fixture(false), |path| {
        for context in [
            mismatch_context("BGEE", "BGEE", &[]),
            mismatch_context("EET", "BG2EE", &[("eet", "0")]),
        ] {
            assert!(super::scan_predicate_guard_hit(path, "3501", &context).is_none());
            let hit = super::scan_predicate_guard_hit(path, "3500", &context)
                .expect("first option is Tutu only");
            assert_eq!(hit.kind, "mismatch");
            assert!(hit.raw_evidence.starts_with("SUBCOMPONENT"));
        }
    });
}

#[test]
fn scs_forced_option_condition_is_local() {
    with_scs_tp2(
        &scs_fixture(false).replace("SUBCOMPONENT", "FORCED_SUBCOMPONENT"),
        |path| {
            let context = mismatch_context("BGEE", "BGEE", &[]);
            assert!(super::scan_predicate_guard_hit(path, "3501", &context).is_none());
            assert!(super::scan_predicate_guard_hit(path, "3500", &context).is_some());
        },
    );
}

#[test]
fn scs_family_origin_is_source_order_across_nonconsecutive_and_decreasing_ids() {
    let text = "BEGIN ~First~ DESIGNATED 50 SUBCOMPONENT @0004\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nCOPY ~a~ ~b~\nBEGIN ~Group~ DESIGNATED 3 GROUP @4\nBEGIN ~Literal~ SUBCOMPONENT ~4~\nBEGIN ~Other~ SUBCOMPONENT @5\nBEGIN ~Last~ SUBCOMPONENT @4\nBEGIN ~Next~\n";
    with_scs_tp2(text, |path| {
        let guards = super::load_component_guards(path);
        assert_eq!(guards.get("50").expect("first guard").len(), 1);
        let context = mismatch_context("BG2EE", "BG2EE", &[]);
        for id in ["3", "4", "5", "7"] {
            assert!(
                super::scan_predicate_guard_hit(path, id, &context).is_none(),
                "{id}"
            );
        }
        let hit = super::scan_predicate_guard_hit(path, "6", &context).expect("family member");
        assert!(hit.message.contains("50"));
        assert!(hit.message.contains("line 2"));
    });
}

#[test]
fn scs_only_first_requirements_inherit_and_later_requirements_remain_local() {
    let text = "BEGIN ~A~ DESIGNATED 10 SUBCOMPONENT ~family~\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nBEGIN ~B~ SUBCOMPONENT \"family\"\nREQUIRE_PREDICATE GAME_IS ~BG2EE~ @2\nBEGIN ~C~ FORCED_SUBCOMPONENT ~family~\nBEGIN ~D~ SUBCOMPONENT ~Family~\nBEGIN ~E~ SUBCOMPONENT ~family ~\n";
    with_scs_tp2(text, |path| {
        let context = mismatch_context("BGEE", "BGEE", &[]);
        assert!(super::scan_predicate_guard_hit(path, "10", &context).is_none());
        let hit = super::scan_predicate_guard_hit(path, "11", &context).expect("own failure");
        assert!(!hit.message.contains("inherited"));
        for id in ["12", "13", "14"] {
            assert!(super::scan_predicate_guard_hit(path, id, &context).is_none());
        }
        let wrong_game = mismatch_context("BG2EE", "BG2EE", &[]);
        assert!(super::scan_predicate_guard_hit(path, "12", &wrong_game).is_some());
        for id in ["13", "14"] {
            assert!(super::scan_predicate_guard_hit(path, id, &wrong_game).is_none());
        }
    });
}

#[test]
fn scs_flag_lexing_accepts_multiline_labels_flags_and_balanced_expressions() {
    let text = "BEGIN\n~First~\nDESIGNATED\n20\nGROUP\n@9\nLABEL ~marker~\nSuBcOmPoNeNt\n@7\n( GAME_IS\n~tutu~ )\nREQUIRE_PREDICATE\n( GAME_IS ~BGEE~\n OR GAME_IS ~BG2EE~ )\n@1\nCOPY ~a~ ~b~\nBEGIN ~Second~ DESIGNATED 21 SUBCOMPONENT @7\n";
    with_scs_tp2(text, |path| {
        let bgee = mismatch_context("BGEE", "BGEE", &[]);
        assert!(super::scan_predicate_guard_hit(path, "20", &bgee).is_some());
        assert!(super::scan_predicate_guard_hit(path, "21", &bgee).is_none());
        let iwdee = mismatch_context("IWDEE", "IWDEE", &[]);
        let hit =
            super::scan_predicate_guard_hit(path, "21", &iwdee).expect("multiline shared guard");
        assert!(hit.raw_evidence.contains("line 12"));
    });
}

#[test]
fn scs_flag_lexing_ignores_comments_quoted_keywords_inline_files_and_actions() {
    let cases = [
        "BEGIN ~A~ DESIGNATED 1\n/* SUBCOMPONENT @7 */\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n",
        "BEGIN ~A SUBCOMPONENT @7~ DESIGNATED 1\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n",
        "BEGIN ~A~ DESIGNATED 1\n// SUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n",
        "BEGIN ~A~ DESIGNATED 1\nCOPY ~a~ ~b~\nSUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n",
        "<<<<<<<< .../payload\nBEGIN ~A~ DESIGNATED 1 SUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n>>>>>>>>\n",
        "BEGIN ~A~ DESIGNATED 1 UNKNOWN_COMPONENT_FLAG @7 SUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\n",
    ];
    for prefix in cases {
        let text = format!("{prefix}BEGIN ~B~ DESIGNATED 2 SUBCOMPONENT @7\n");
        with_scs_tp2(&text, |path| {
            let context = mismatch_context("BG2EE", "BG2EE", &[]);
            assert!(
                super::scan_predicate_guard_hit(path, "2", &context).is_none(),
                "{prefix}"
            );
        });
    }
}

#[test]
fn scs_unsupported_and_runtime_requirements_do_not_become_false_prefixes() {
    for expression in [
        "GAME_IS ~BGEE~ UNKNOWN_PREDICATE ~x~",
        "FILE_EXISTS_IN_GAME ~runtime.itm~",
        "GAME_IS ~BGEE~ OR FILE_EXISTS_IN_GAME ~runtime.itm~",
    ] {
        let text = format!(
            "BEGIN ~A~ DESIGNATED 1 SUBCOMPONENT @7\nREQUIRE_PREDICATE {expression} @1\nBEGIN ~B~ SUBCOMPONENT @7\n"
        );
        with_scs_tp2(&text, |path| {
            let context = mismatch_context("BG2EE", "BG2EE", &[]);
            assert!(
                super::scan_predicate_guard_hit(path, "2", &context).is_none(),
                "{expression}"
            );
        });
    }
}

#[test]
fn scs_family_scope_is_tp2_local_and_selected_block_remains_truthful() {
    with_scs_tp2(&scs_fixture(true), |first| {
        with_scs_tp2(
            "BEGIN ~Other~ DESIGNATED 3501 SUBCOMPONENT @20076\nCOPY ~a~ ~b~\n",
            |second| {
                let context = mismatch_context("BGEE", "BGEE", &[]);
                assert!(super::scan_predicate_guard_hit(first, "3501", &context).is_some());
                assert!(super::scan_predicate_guard_hit(second, "3501", &context).is_none());
            },
        );
    });
    let text = scs_fixture(true);
    let blocks = crate::app::compat_tp2_blocks::collect_tp2_component_blocks(&text);
    let selected = &blocks
        .iter()
        .find(|(id, _)| id == "3501")
        .expect("selected block")
        .1;
    assert!(selected[0].contains("3501"));
    assert!(
        selected
            .iter()
            .all(|line| !line.contains("REQUIRE_PREDICATE"))
    );
}

#[test]
fn scs_opt_in_actual_tp2_read_only_proof() {
    let Some(path) = std::env::var_os("BIO_SCS_READONLY_TP2") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    assert!(path.is_absolute());
    let before = std::fs::read(&path).expect("read opted-in TP2");
    for (context, expected_line) in [
        (mismatch_context("BG2EE", "BG2EE", &[]), "line 518"),
        (mismatch_context("BGEE", "BGEE", &[]), "line 519"),
        (
            mismatch_context("EET", "BG2EE", &[("eet", "0")]),
            "line 519",
        ),
    ] {
        let hit =
            super::scan_predicate_guard_hit(path.to_str().expect("TP2 path"), "3501", &context)
                .expect("actual SCS shared guard");
        assert_eq!(hit.kind, "mismatch");
        assert!(hit.message.contains("3500"));
        assert!(hit.raw_evidence.contains(expected_line));
        println!(
            "Expected {expected_line}: {}\n{}",
            hit.message, hit.raw_evidence
        );
        assert!(hit.related_mod.is_none());
        assert!(hit.related_component.is_none());
    }
    assert_eq!(std::fs::read(&path).expect("reread opted-in TP2"), before);
}

fn scs_component(id: &str, checked: bool) -> crate::app::state::Step2ComponentState {
    crate::app::state::Step2ComponentState {
        component_id: id.to_string(),
        label: id.to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        is_meta_mode_component: false,
        disabled: false,
        compat_kind: None,
        compat_source: None,
        compat_related_mod: None,
        compat_related_component: None,
        compat_graph: None,
        compat_evidence: None,
        disabled_reason: None,
        checked,
        selected_order: checked.then_some(1),
    }
}

fn scs_mod_state(tp_file: &str, components: &[(&str, bool)]) -> crate::app::state::Step2ModState {
    crate::app::state::Step2ModState {
        name: tp_file.to_string(),
        tp_file: tp_file.to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: components.iter().any(|(_, checked)| *checked),
        hidden_components: Vec::new(),
        components: components
            .iter()
            .map(|(id, checked)| scs_component(id, *checked))
            .collect(),
    }
}

#[test]
fn scs_step2_and_single_hit_have_matching_disabled_state_and_provenance() {
    with_scs_tp2(&scs_fixture(true), |path| {
        let step1 = Step1State {
            game_install: "BGEE".to_string(),
            ..Step1State::default()
        };
        let context = mismatch_context("BGEE", "BGEE", &[]);
        let expected = super::scan_predicate_guard_hit(path, "3501", &context).expect("shared hit");
        let mut mods = vec![scs_mod_state("fixture", &[("3501", false)])];
        mods[0].tp2_path = path.to_string();
        super::apply_step2_scan_mismatch(&step1, "BGEE", &mut mods);
        let component = &mods[0].components[0];
        assert!(component.disabled);
        assert_eq!(component.compat_kind.as_deref(), Some("mismatch"));
        assert_eq!(component.compat_source.as_deref(), Some(path));
        assert_eq!(
            component.compat_evidence.as_deref(),
            Some(expected.raw_evidence.as_str())
        );
        assert_eq!(
            component.disabled_reason.as_deref(),
            Some(expected.message.as_str())
        );
        assert!(component.compat_related_mod.is_none());
        assert!(component.compat_related_component.is_none());
    });
}

#[test]
fn scs_inherited_positive_mod_or_requirement_keeps_dependency_bypass() {
    let text = "BEGIN ~A~ DESIGNATED 1 SUBCOMPONENT @7\nREQUIRE_PREDICATE ((MOD_IS_INSTALLED ~adrian.tp2~ ~0~)\n OR (MOD_IS_INSTALLED ~adrian.tp2~ ~1~)\n OR (MOD_IS_INSTALLED ~adrian.tp2~ ~2~)) @130\nBEGIN ~B~ SUBCOMPONENT @7\n";
    with_scs_tp2(text, |path| {
        for checked in [
            vec![],
            vec![("adrian", "0")],
            vec![("adrian", "1")],
            vec![("adrian", "2")],
        ] {
            let context = mismatch_context("BG2EE", "BG2EE", &checked);
            for id in ["1", "2"] {
                assert!(super::scan_predicate_guard_hit(path, id, &context).is_none());
            }
        }
    });
}

#[test]
fn scs_begin_after_multiline_comment_closure_keeps_component_and_physical_origin() {
    let text = "BACKUP ~backup~ /* comment starts\nstill comment\n*/ BEGIN ~First~ DESIGNATED 1 SUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nBEGIN ~Second~ SUBCOMPONENT @7\n";
    with_scs_tp2(text, |path| {
        let context = mismatch_context("BG2EE", "BG2EE", &[]);
        let hit =
            super::scan_predicate_guard_hit(path, "2", &context).expect("comment-closure BEGIN");
        assert!(hit.message.contains("#1"));
        assert!(hit.raw_evidence.contains("line 4"));
    });
}

#[test]
fn scs_percent_begin_label_preserves_following_implicit_component_ids() {
    let text = "BEGIN ~First~ DESIGNATED 5 SUBCOMPONENT @7\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nBEGIN %Runtime label%\nCOPY ~a~ ~b~\nBEGIN ~Last~ SUBCOMPONENT @7\n";
    with_scs_tp2(text, |path| {
        let context = mismatch_context("BG2EE", "BG2EE", &[]);
        assert!(super::scan_predicate_guard_hit(path, "6", &context).is_none());
        let hit =
            super::scan_predicate_guard_hit(path, "7", &context).expect("third declaration is #7");
        assert!(hit.message.contains("#5"));
    });
}

#[test]
fn scs_malformed_option_condition_does_not_guess_inherited_family() {
    for text in [
        "BEGIN ~A~ DESIGNATED 1 SUBCOMPONENT ~family~\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nBEGIN ~B~ SUBCOMPONENT ~family~ (\n",
        "BEGIN ~A~ DESIGNATED 1 SUBCOMPONENT ~family~ (\nREQUIRE_PREDICATE GAME_IS ~BGEE~ @1\nBEGIN ~B~ SUBCOMPONENT ~family~\n",
    ] {
        with_scs_tp2(text, |path| {
            let context = mismatch_context("BG2EE", "BG2EE", &[]);
            assert!(super::scan_predicate_guard_hit(path, "2", &context).is_none());
        });
    }
}

#[test]
fn compat_three_fixes_legacy_guard_is_evaluated_in_genuine_context() {
    with_scs_tp2(
        "BEGIN ~Legacy~ DESIGNATED 27\nREQUIRE_PREDICATE GAME_IS ~bgt~ ~Classic only~\n",
        |path| {
            let mut bytes = std::fs::read(path).expect("fixture bytes");
            bytes[12] = 0xe9;
            std::fs::write(path, bytes).expect("legacy fixture");
            let context = mismatch_context("BG2EE", "BG2EE", &[]);
            let hit = super::scan_predicate_guard_hit(path, "27", &context).expect("legacy guard");
            assert_eq!(hit.kind, "mismatch");
            assert!(hit.raw_evidence.contains("bgt"));
        },
    );
}

fn guards_before_next_line(next_line: &str) -> Vec<RequirementGuard> {
    collect_requirement_guards(&[
        "BEGIN ~Wrong game~ DESIGNATED 1",
        "REQUIRE_PREDICATE GAME_IS ~bg2ee eet~",
        next_line,
    ])
}

#[test]
fn require_predicate_followed_by_unlisted_action_keeps_guard() {
    let expected = guards_before_next_line("COPY ~a~ ~b~");
    assert_eq!(expected.len(), 1);
    let context = mismatch_context("BGEE", "BGEE", &[]);
    let expected_hit = preferred_guard_hit(&expected, &context).expect("COPY guard hit");
    assert_eq!(expected_hit.kind, "mismatch");
    for next_line in [
        "ACTION_DEFINE_ARRAY names BEGIN ~a~ END",
        "ACTION_MATCH ~%x%~ WITH ~a~ BEGIN END DEFAULT END",
        "OUTER_FOR (i = 0; i < 2; ++i) BEGIN END",
        "DEFINE_ACTION_FUNCTION helper BEGIN END",
        "CLEAR_MEMORY",
    ] {
        let guards = guards_before_next_line(next_line);
        assert_eq!(guards.len(), 1, "{next_line}");
        assert_eq!(guards[0].eval_text, expected[0].eval_text, "{next_line}");
        assert_eq!(
            guards[0].display_line, expected[0].display_line,
            "{next_line}"
        );
        let hit = preferred_guard_hit(&guards, &context).expect(next_line);
        assert_eq!(hit.kind, expected_hit.kind, "{next_line}");
        assert_eq!(hit.message, expected_hit.message, "{next_line}");
    }
}

#[test]
fn require_predicate_dangling_operator_before_unknown_line_is_dropped() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    for ending in ["OR", "AND", "NOT", "(", "||", "&&", "!"] {
        let predicate = format!("REQUIRE_PREDICATE GAME_IS ~bgee~ {ending}");
        let guards = collect_requirement_guards(&[
            "BEGIN ~Dangling~ DESIGNATED 1",
            predicate.as_str(),
            "VARIABLE_IS_SET x",
        ]);
        assert!(guards.is_empty(), "{ending}");
        assert!(preferred_guard_hit(&guards, &context).is_none(), "{ending}");
    }
}

#[test]
fn subcomponent_condition_followed_by_unlisted_action_keeps_family() {
    use crate::app::compat_tp2_blocks::{
        FamilyIdentity, PredicateFlagKind, collect_component_flags,
    };
    let flags = collect_component_flags(
        "BEGIN ~A~ DESIGNATED 1 SUBCOMPONENT ~family~ (GAME_IS ~bg2ee~)\nACTION_DEFINE_ARRAY names BEGIN ~a~ END\n",
    );
    assert_eq!(flags.len(), 1);
    assert_eq!(
        flags[0].family,
        Some(FamilyIdentity::Literal("family".to_string()))
    );
    assert_eq!(flags[0].predicates.len(), 1);
    assert_eq!(flags[0].predicates[0].kind, PredicateFlagKind::Option);
    assert_eq!(flags[0].predicates[0].expression, "( GAME_IS ~bg2ee~ )");
}

#[test]
fn subcomponent_without_condition_followed_by_unlisted_action_keeps_family() {
    use crate::app::compat_tp2_blocks::{FamilyIdentity, collect_component_flags};
    for action in ["ACTION_DEFINE_ARRAY names BEGIN ~a~ END", "COPY ~a~ ~b~"] {
        let flags = collect_component_flags(&format!(
            "BEGIN ~A~ DESIGNATED 2 SUBCOMPONENT ~family~\n{action}\n"
        ));
        assert_eq!(flags.len(), 1, "{action}");
        assert_eq!(
            flags[0].family,
            Some(FamilyIdentity::Literal("family".to_string())),
            "{action}"
        );
        assert!(flags[0].predicates.is_empty(), "{action}");
    }
}

#[path = "compat_three_fixes_corpus_tests.rs"]
mod corpus;
