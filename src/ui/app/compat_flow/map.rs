// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::compat::model::CompatValidationResult;
use crate::ui::state::{CompatIssueDisplay, CompatState};

pub(super) fn to_state(result: CompatValidationResult) -> CompatState {
    let issues = map_issues(&result);
    CompatState {
        error_count: result.error_count(),
        warning_count: result.warning_count(),
        issues,
        show_pre_install_modal: false,
    }
}

pub(super) fn combine_states(a: CompatState, b: CompatState) -> CompatState {
    let mut issues = a.issues;
    issues.extend(b.issues);
    CompatState {
        error_count: a.error_count + b.error_count,
        warning_count: a.warning_count + b.warning_count,
        issues,
        show_pre_install_modal: false,
    }
}

fn map_issues(result: &CompatValidationResult) -> Vec<CompatIssueDisplay> {
    result
        .issues
        .iter()
        .map(|i| CompatIssueDisplay {
            issue_id: i.issue_id.clone(),
            code: i.code.as_str().to_string(),
            severity: i.severity.as_str().to_string(),
            is_blocking: i.is_blocking(),
            affected_mod: i.affected_mod.clone(),
            affected_component: i.affected_component,
            related_mod: i.related_mod.clone(),
            related_component: i.related_component,
            reason: i.reason.clone(),
            source: i.source.description(),
            raw_evidence: i.raw_evidence.clone(),
        })
        .collect()
}
