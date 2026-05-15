// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_issue::CompatIssue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompatIssueDisplay {
    pub kind: String,
    pub code: String,
    pub status_label: String,
    pub status_tone: CompatIssueStatusTone,
    pub related_mod: String,
    pub related_component: Option<u32>,
    pub reason: String,
    pub source: String,
    pub raw_evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatIssueStatusTone {
    Neutral,
    Blocking,
    Warning,
}

pub(crate) fn display_issue(issue: &CompatIssue) -> CompatIssueDisplay {
    let (status_label, status_tone) = popup_issue_status(issue.kind.as_str());
    CompatIssueDisplay {
        kind: issue.kind.clone(),
        code: issue_code(issue.kind.as_str()).to_string(),
        status_label: status_label.to_string(),
        status_tone,
        related_mod: issue.related_mod.clone(),
        related_component: issue.related_component,
        reason: issue.reason.clone(),
        source: issue.source.clone(),
        raw_evidence: issue.raw_evidence.clone(),
    }
}

fn issue_code(kind: &str) -> &'static str {
    match kind {
        "mismatch" | "game_mismatch" => "MISMATCH",
        "missing_dep" => "REQ_MISSING",
        "included" => "INCLUDED",
        "order_block" => "ORDER_BLOCK",
        "conditional" => "CONDITIONAL",
        "path_requirement" => "PATH_REQUIREMENT",
        "deprecated" => "DEPRECATED",
        _ => "RULE_HIT",
    }
}

pub(crate) fn popup_issue_status(kind: &str) -> (&'static str, CompatIssueStatusTone) {
    match kind {
        "included" => ("Already included", CompatIssueStatusTone::Neutral),
        "not_needed" => ("Not needed", CompatIssueStatusTone::Neutral),
        "missing_dep" | "order_block" | "warning" | "deprecated" | "conditional" => {
            ("Warning only", CompatIssueStatusTone::Warning)
        }
        _ => ("Resolve before continuing", CompatIssueStatusTone::Blocking),
    }
}
