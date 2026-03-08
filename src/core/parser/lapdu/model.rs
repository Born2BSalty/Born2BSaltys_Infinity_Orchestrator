// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserOutput {
    pub schema_version: u32,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub tra_language_requested: String,
    #[serde(default)]
    pub tra_language_used: String,
    #[serde(default)]
    pub events: Vec<ParserEvent>,
    #[serde(default)]
    pub flow: Vec<ParserFlowNode>,
    #[serde(default)]
    pub warnings: Vec<ParserDiagnostic>,
    #[serde(default)]
    pub errors: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserEvent {
    #[serde(default)]
    pub kind: String,
    pub interactive: Option<bool>,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub path_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub options: Vec<ParserOption>,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub branch_path: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub condition_id: Option<String>,
    #[serde(default)]
    pub game_allow: Vec<String>,
    #[serde(default)]
    pub game_deny: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserOption {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub component_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserFlowNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub event_ids: Vec<String>,
    #[serde(default)]
    pub children: Vec<ParserFlowNode>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserDiagnostic {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
}
