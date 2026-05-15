// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::settings::model::Step1Settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub ok: bool,
    pub message: String,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStatus {
    Found { version: Option<String> },
    NotFound,
    NotWritable,
}

#[must_use]
pub fn run_now(settings: &Step1Settings) -> ValidationReport {
    let state = Step1State::from(settings.clone());
    let (ok, message) = crate::app::state_validation::run_path_check(&state);
    let issues = if ok {
        Vec::new()
    } else {
        crate::app::state_validation::split_path_check_lines(&message)
    };

    ValidationReport {
        ok,
        message,
        issues,
    }
}
