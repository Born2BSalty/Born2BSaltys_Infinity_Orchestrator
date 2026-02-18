// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CompatState {
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<CompatIssueDisplay>,
    pub show_pre_install_modal: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompatIssueDisplay {
    pub issue_id: String,
    pub code: String,
    pub severity: String,
    pub is_blocking: bool,
    pub affected_mod: String,
    pub affected_component: Option<u32>,
    pub related_mod: String,
    pub related_component: Option<u32>,
    pub reason: String,
    pub source: String,
    pub raw_evidence: Option<String>,
}
