// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::Deserialize;

pub(crate) const COMPAT_RULES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct CompatRulesFile {
    #[serde(default)]
    pub(crate) schema_version: Option<u32>,
    #[serde(default)]
    pub(crate) rules: Vec<CompatRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CompatRule {
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default, alias = "mod_name")]
    pub(crate) r#mod: StringOrMany,
    #[serde(default)]
    pub(crate) component: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) component_id: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) mode: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) tab: Option<StringOrMany>,
    #[serde(default, alias = "issue")]
    pub(crate) kind: String,
    #[serde(default, alias = "when_kind")]
    pub(crate) match_kind: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) clear_kinds: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) position: Option<String>,
    #[serde(default)]
    pub(crate) path_field: Option<String>,
    #[serde(default)]
    pub(crate) path_check: Option<String>,
    #[serde(default)]
    pub(crate) game_file: Option<String>,
    #[serde(default)]
    pub(crate) game_file_check: Option<String>,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) related_mod: Option<StringOrMany>,
    #[serde(default)]
    pub(crate) related_component: Option<StringOrMany>,
    #[serde(skip)]
    pub(crate) loaded_from: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum StringOrMany {
    One(String),
    Many(Vec<String>),
}

impl Default for StringOrMany {
    fn default() -> Self {
        Self::One(String::new())
    }
}

impl StringOrMany {
    pub(crate) fn trimmed_items(&self) -> Vec<String> {
        match self {
            Self::One(value) => {
                let value = value.trim();
                if value.is_empty() {
                    Vec::new()
                } else {
                    vec![value.to_string()]
                }
            }
            Self::Many(values) => values
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect(),
        }
    }

    pub(crate) fn normalized_items(&self) -> Vec<String> {
        self.trimmed_items()
            .into_iter()
            .map(|value| value.to_ascii_uppercase())
            .collect()
    }
}

const fn default_true() -> bool {
    true
}
