// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatIssue {
    pub kind: String,
    pub related_mod: String,
    pub related_component: Option<u32>,
    pub reason: String,
    pub source: String,
    pub raw_evidence: Option<String>,
}
