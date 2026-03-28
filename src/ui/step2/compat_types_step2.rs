// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
