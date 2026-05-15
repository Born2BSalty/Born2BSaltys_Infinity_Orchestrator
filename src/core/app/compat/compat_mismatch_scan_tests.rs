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
            display_line: "REQUIRE_PREDICATE GAME_INCLUDES ~tob~".to_string(),
            eval_text: "GAME_INCLUDES tob".to_string(),
        },
        RequirementGuard {
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
fn upgrades_selected_negated_mod_guard_to_conflict() {
    let context = mismatch_context("BG2EE", "BG2EE", &[("foo", "1")]);
    let guards = vec![RequirementGuard {
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
