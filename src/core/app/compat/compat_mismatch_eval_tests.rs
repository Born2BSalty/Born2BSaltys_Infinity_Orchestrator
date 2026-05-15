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
