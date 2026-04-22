// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserOutput {
    pub schema_version: u32,
    #[serde(default, deserialize_with = "string_or_default")]
    pub source_file: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub tra_language_requested: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub tra_language_used: String,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub events: Vec<ParserEvent>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub flow: Vec<ParserFlowNode>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub warnings: Vec<ParserDiagnostic>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub errors: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserEvent {
    #[serde(default, deserialize_with = "string_or_default")]
    pub kind: String,
    #[serde(default)]
    pub interactive: Option<bool>,
    #[serde(default, deserialize_with = "string_or_default")]
    pub node_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default, deserialize_with = "string_or_default")]
    pub path_id: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub text: String,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub options: Vec<ParserOption>,
    #[serde(default, deserialize_with = "string_or_default")]
    pub source_file: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub branch_path: Vec<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub condition_id: Option<String>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub game_allow: Vec<String>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub game_deny: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserOption {
    #[serde(default, deserialize_with = "string_or_default")]
    pub label: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub value: String,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub component_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserFlowNode {
    #[serde(default, deserialize_with = "string_or_default")]
    pub id: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub label: String,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub event_ids: Vec<String>,
    #[serde(default, deserialize_with = "vec_or_default")]
    pub children: Vec<ParserFlowNode>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParserDiagnostic {
    #[serde(default, deserialize_with = "string_or_default")]
    pub code: String,
    #[serde(default, deserialize_with = "string_or_default")]
    pub message: String,
}

fn string_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn vec_or_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}
