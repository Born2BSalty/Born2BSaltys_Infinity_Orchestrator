// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Step2CompatRulesFile {
    #[serde(default)]
    pub rules: Vec<Step2CompatRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Step2CompatRule {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, alias = "mod_name")]
    pub r#mod: String,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub component_id: Option<String>,
    #[serde(default)]
    pub mode: Option<StringOrMany>,
    #[serde(default)]
    pub tab: Option<StringOrMany>,
    #[serde(default, alias = "issue")]
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub related_mod: Option<String>,
    #[serde(default)]
    pub related_component: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrMany {
    One(String),
    Many(Vec<String>),
}

impl StringOrMany {
    pub fn normalized_items(&self) -> Vec<String> {
        match self {
            Self::One(v) => vec![v.to_ascii_uppercase()],
            Self::Many(v) => v.iter().map(|s| s.to_ascii_uppercase()).collect(),
        }
    }
}

fn default_true() -> bool {
    true
}
