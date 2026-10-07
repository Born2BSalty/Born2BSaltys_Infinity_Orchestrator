// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use super::{
    RequirementFailureClass, TriState, build_mismatch_context, classify_failed_requirement,
    evaluate_requirement, render_requirement_evidence,
};
use crate::app::state::Step1State;

#[test]
fn evaluates_mod_is_installed_equals_zero_as_true_when_missing() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(r"(MOD_IS_INSTALLED ~item_rev/item_rev.tp2~ 0)=0", &context),
        TriState::True
    );
}

#[test]
fn evaluates_mod_is_installed_equals_zero_as_false_when_selected() {
    let context = mismatch_context("BG2EE", "BG2EE", &[("item_rev", "0")]);
    assert_eq!(
        evaluate_requirement(r"(MOD_IS_INSTALLED ~item_rev/item_rev.tp2~ 0)=0", &context),
        TriState::False
    );
    assert_eq!(
        classify_failed_requirement(r"(MOD_IS_INSTALLED ~item_rev/item_rev.tp2~ 0)=0", &context),
        RequirementFailureClass::Conditional
    );
}

#[test]
fn renders_comparison_evidence() {
    assert_eq!(
        render_requirement_evidence(r"(MOD_IS_INSTALLED ~item_rev/item_rev.tp2~ 0)=0").as_deref(),
        Some("(MOD_IS_INSTALLED ~item_rev/item_rev.tp2~ ~0~) = 0")
    );
}

#[test]
fn keeps_eet_game_identity_separate_from_bg2ee_engine() {
    let context = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bgee bg2ee iwdee~", &context),
        TriState::False
    );
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~eet~", &context),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"ENGINE_IS ~bg2ee~", &context),
        TriState::True
    );
}

#[test]
fn iwdee_tab_evaluates_as_iwdee_with_how_and_totlm() {
    for tab in ["IWDEE", "BGEE"] {
        let context = mismatch_context("IWDEE", tab, &[]);
        assert_eq!(
            evaluate_requirement(r"GAME_IS ~iwdee~", &context),
            TriState::True
        );
        assert_eq!(
            evaluate_requirement(r"ENGINE_IS ~iwdee~", &context),
            TriState::True
        );
        assert_eq!(
            evaluate_requirement(r"GAME_INCLUDES ~iwd how totlm~", &context),
            TriState::True
        );
        assert_eq!(
            evaluate_requirement(r"GAME_IS ~bgee bg2ee~", &context),
            TriState::False
        );
    }
}

#[test]
fn game_is_iwdee_predicate_matches_on_an_iwdee_tab() {
    let context = mismatch_context("IWDEE", "IWDEE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~iwdee~", &context),
        TriState::True
    );
}

#[test]
fn bgee_only_predicate_mismatches_on_an_iwdee_tab() {
    let context = mismatch_context("IWDEE", "IWDEE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bgee~", &context),
        TriState::False
    );
}

#[test]
fn bgee_bg2ee_eet_contexts_unchanged() {
    let baldurs_gate_1 = mismatch_context("BGEE", "BGEE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bgee~", &baldurs_gate_1),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"GAME_INCLUDES ~bg1 totsc~", &baldurs_gate_1),
        TriState::True
    );

    let baldurs_gate_2 = mismatch_context("BG2EE", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bg2ee~", &baldurs_gate_2),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"GAME_INCLUDES ~bg2 soa tob~", &baldurs_gate_2),
        TriState::True
    );

    let eet_first = mismatch_context("EET", "BGEE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bgee~", &eet_first),
        TriState::True
    );

    let eet_second_no_core = mismatch_context("EET", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~bg2ee~", &eet_second_no_core),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~eet~", &eet_second_no_core),
        TriState::False
    );

    let eet_second_with_core = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    assert_eq!(
        evaluate_requirement(r"GAME_IS ~eet~", &eet_second_with_core),
        TriState::True
    );
    assert_eq!(
        evaluate_requirement(r"GAME_INCLUDES ~bg1 totsc sod~", &eet_second_with_core),
        TriState::True
    );
}

const BRAGE_PREDICATE: &str =
    r"(FILE_EXISTS_IN_GAME ~c#q13swd.itm~ OR GAME_IS ~tob bg2ee~ AND NOT GAME_IS ~bgt~)";

#[test]
fn brage_file_check_or_game_check_is_unknown_on_eet_with_core() {
    let context = mismatch_context("EET", "BG2EE", &[("eet", "0")]);
    assert_eq!(
        evaluate_requirement(BRAGE_PREDICATE, &context),
        TriState::Unknown
    );
    assert_eq!(
        classify_failed_requirement(BRAGE_PREDICATE, &context),
        RequirementFailureClass::Conditional
    );
}

#[test]
fn brage_file_check_or_game_check_is_unknown_on_bgee() {
    let context = mismatch_context("BGEE", "BGEE", &[]);
    assert_eq!(
        evaluate_requirement(BRAGE_PREDICATE, &context),
        TriState::Unknown
    );
    assert_eq!(
        classify_failed_requirement(BRAGE_PREDICATE, &context),
        RequirementFailureClass::Conditional
    );
}

#[test]
fn brage_file_check_or_game_check_is_true_on_bg2ee() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(BRAGE_PREDICATE, &context),
        TriState::True
    );
}

#[test]
fn file_check_and_wrong_game_still_mismatches() {
    let context = mismatch_context("BG2EE", "BG2EE", &[]);
    let predicate = r"GAME_IS ~bgee~ AND FILE_EXISTS ~x~";
    assert_eq!(evaluate_requirement(predicate, &context), TriState::False);
    assert_eq!(
        classify_failed_requirement(predicate, &context),
        RequirementFailureClass::Mismatch
    );
}

#[test]
fn wrong_game_and_file_check_under_or_mismatches_off_game_and_is_true_on_game() {
    let predicate = r"((GAME_IS ~bgee~) AND FILE_EXISTS ~x~) OR GAME_IS ~bg2ee~";

    let icewind_dale = mismatch_context("IWDEE", "IWDEE", &[]);
    assert_eq!(
        evaluate_requirement(predicate, &icewind_dale),
        TriState::False
    );
    assert_eq!(
        classify_failed_requirement(predicate, &icewind_dale),
        RequirementFailureClass::Mismatch
    );

    let baldurs_gate_2 = mismatch_context("BG2EE", "BG2EE", &[]);
    assert_eq!(
        evaluate_requirement(predicate, &baldurs_gate_2),
        TriState::True
    );
}

#[test]
fn or_truth_table_treats_file_check_like_unknown_unless_true() {
    use TriState::{False, Ignored, True, Unknown};
    let expected = [
        (True, True, True),
        (True, False, True),
        (True, Ignored, True),
        (True, Unknown, True),
        (False, True, True),
        (False, False, False),
        (False, Ignored, Unknown),
        (False, Unknown, Unknown),
        (Ignored, True, True),
        (Ignored, False, Unknown),
        (Ignored, Ignored, Unknown),
        (Ignored, Unknown, Unknown),
        (Unknown, True, True),
        (Unknown, False, Unknown),
        (Unknown, Ignored, Unknown),
        (Unknown, Unknown, Unknown),
    ];
    for (lhs, rhs, result) in expected {
        assert_eq!(lhs.or(rhs), result, "{lhs:?} OR {rhs:?}");
    }
}

fn mismatch_context(mode: &str, tab: &str, checked: &[(&str, &str)]) -> super::MismatchContext {
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
