// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavStatusKind {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathValidationSummary {
    pub kind: NavStatusKind,
    pub text: String,
}

#[must_use]
pub fn compute_path_validation_summary(state: &WizardState) -> PathValidationSummary {
    match &state.step1_path_check {
        Some((true, _)) => PathValidationSummary {
            kind: NavStatusKind::Ok,
            text: "weidu v249 · all paths ok".to_string(),
        },
        Some((false, message)) => {
            let count = crate::app::state_validation::split_path_check_lines(message)
                .len()
                .max(1);
            PathValidationSummary {
                kind: NavStatusKind::Error,
                text: format!("× {count} path issues"),
            }
        }
        None => PathValidationSummary {
            kind: NavStatusKind::Error,
            text: "× paths not checked".to_string(),
        },
    }
}
