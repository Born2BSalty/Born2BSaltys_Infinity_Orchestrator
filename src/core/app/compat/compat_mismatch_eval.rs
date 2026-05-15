// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "compat_mismatch_eval_classify.rs"]
pub mod classify;
#[path = "compat_mismatch_eval_context.rs"]
pub mod context;
#[path = "compat_mismatch_eval_evidence.rs"]
pub mod evidence;
#[path = "compat_mismatch_eval_parser.rs"]
pub mod parser;

pub(crate) use classify::{RequirementFailureClass, classify_failed_requirement};
pub(crate) use context::{MismatchContext, TriState, build_mismatch_context};
pub(crate) use evidence::render_requirement_evidence;
pub(crate) use parser::evaluate_requirement;

#[cfg(test)]
#[path = "compat_mismatch_eval_tests.rs"]
mod tests;
