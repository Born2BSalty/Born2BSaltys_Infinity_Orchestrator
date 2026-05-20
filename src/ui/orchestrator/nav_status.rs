// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathValidationKind {
    Ok,
    Err(usize),
}

#[derive(Debug, Clone)]
pub struct PathValidationSummary {
    pub kind: PathValidationKind,
    pub text: String,
}

impl Default for PathValidationSummary {
    fn default() -> Self {
        Self {
            kind: PathValidationKind::Err(0),
            text: String::from("paths not configured"),
        }
    }
}

#[must_use]
pub fn compute_path_validation_summary(state: &WizardState) -> PathValidationSummary {
    use crate::app::state_validation;

    let step1 = &state.step1;
    if state_validation::is_step1_valid(step1) {
        return PathValidationSummary {
            kind: PathValidationKind::Ok,
            text: String::from("weidu v\u{2026} \u{00B7} all paths ok"),
        };
    }

    let issue_count = state
        .step1_path_check
        .as_ref()
        .filter(|(ok, _)| !*ok)
        .map_or(0, |(_, msg)| {
            state_validation::split_path_check_lines(msg).len()
        });

    let text = if issue_count == 0 {
        String::from("\u{00D7} paths not configured")
    } else if issue_count == 1 {
        String::from("\u{00D7} 1 path issue")
    } else {
        format!("\u{00D7} {issue_count} path issues")
    };

    PathValidationSummary {
        kind: PathValidationKind::Err(issue_count),
        text,
    }
}
