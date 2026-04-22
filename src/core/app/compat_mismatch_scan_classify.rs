// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::compat_mismatch_eval::{
    MismatchContext, RequirementFailureClass, TriState, classify_failed_requirement,
    evaluate_requirement,
};
use super::guards::RequirementGuard;
use crate::parser::compat_dependency_expr::{
    ParsedDependencyTarget, parse_mod_is_installed_dependency_targets,
    parse_negated_mod_is_installed_targets,
};

fn classify_guard_hit(guard: &RequirementGuard, context: &MismatchContext) -> PredicateGuardHit {
    let classification = classify_guard(&guard.eval_text, context);
    if !classification.kind.eq_ignore_ascii_case("mismatch")
        && let Some(target) = first_selected_negated_target(&guard.eval_text, context)
    {
        return PredicateGuardHit {
            kind: "conflict",
            related_mod: Some(target.target_mod.clone()),
            related_component: Some(target.target_component_id.clone()),
            message: format!(
                "Blocked by selected component {} #{}.",
                target.target_mod, target.target_component_id
            ),
            raw_evidence: guard.display_line.clone(),
        };
    }

    PredicateGuardHit {
        kind: classification.kind,
        related_mod: None,
        related_component: None,
        message: classification.message.to_string(),
        raw_evidence: guard.display_line.clone(),
    }
}

fn first_selected_negated_target(
    eval_text: &str,
    context: &MismatchContext,
) -> Option<ParsedDependencyTarget> {
    parse_negated_mod_is_installed_targets(eval_text)?
        .into_iter()
        .find(|target| {
            context.has_checked_component(&target.target_mod, &target.target_component_id)
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GuardClassification {
    pub(super) kind: &'static str,
    pub(super) message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PredicateGuardHit {
    pub(crate) kind: &'static str,
    pub(crate) related_mod: Option<String>,
    pub(crate) related_component: Option<String>,
    pub(crate) message: String,
    pub(crate) raw_evidence: String,
}

pub(super) fn classify_guard(eval_text: &str, context: &MismatchContext) -> GuardClassification {
    match classify_failed_requirement(eval_text, context) {
        RequirementFailureClass::Mismatch => GuardClassification {
            kind: "mismatch",
            message: "TP2 predicate excludes this component for the current game tab.",
        },
        RequirementFailureClass::Conditional => GuardClassification {
            kind: "conditional",
            message: "TP2 predicate excludes this component for the current plan or install state.",
        },
    }
}

pub(super) fn preferred_guard_hit(
    guards: &[RequirementGuard],
    context: &MismatchContext,
) -> Option<PredicateGuardHit> {
    let mut best = None::<(u8, PredicateGuardHit)>;

    for guard in guards {
        if parse_mod_is_installed_dependency_targets(&guard.eval_text).is_some() {
            continue;
        }
        if evaluate_requirement(&guard.eval_text, context) != TriState::False {
            continue;
        }

        let candidate = classify_guard_hit(guard, context);
        let priority = match candidate.kind {
            "mismatch" => 2u8,
            "conflict" => 1u8,
            _ => 0u8,
        };
        if priority == 2 {
            return Some(candidate);
        }
        if best
            .as_ref()
            .map(|(current, _)| priority > *current)
            .unwrap_or(true)
        {
            best = Some((priority, candidate));
        }
    }

    best.map(|(_, hit)| hit)
}

#[cfg(test)]
pub(super) fn preferred_failing_guard(
    guards: &[RequirementGuard],
    context: &MismatchContext,
) -> Option<(String, String, String)> {
    preferred_guard_hit(guards, context)
        .map(|hit| (hit.raw_evidence, hit.kind.to_string(), hit.message))
}
