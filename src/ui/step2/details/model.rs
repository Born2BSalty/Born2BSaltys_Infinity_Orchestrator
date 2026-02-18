// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Default)]
pub struct Step2Details {
    pub mod_name: Option<String>,
    pub component_label: Option<String>,
    pub component_id: Option<String>,
    pub component_lang: Option<String>,
    pub component_version: Option<String>,
    pub selected_order: Option<usize>,
    pub is_checked: Option<bool>,
    pub is_disabled: Option<bool>,
    pub compat_kind: Option<String>,
    pub compat_role: Option<String>,
    pub compat_code: Option<String>,
    pub disabled_reason: Option<String>,
    pub compat_source: Option<String>,
    pub compat_related_target: Option<String>,
    pub compat_graph: Option<String>,
    pub compat_evidence: Option<String>,
    pub raw_line: Option<String>,
    pub tp_file: Option<String>,
    pub tp2_path: Option<String>,
    pub readme_path: Option<String>,
    pub web_url: Option<String>,
}
