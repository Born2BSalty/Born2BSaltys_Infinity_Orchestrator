// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct PromptAnswersDisk {
    #[serde(default)]
    pub(super) entries: Vec<PromptAnswerDiskEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct PromptAnswerDiskEntry {
    #[serde(default)]
    pub(super) key: String,
    #[serde(default)]
    pub(super) alias: String,
    #[serde(default)]
    pub(super) answer: String,
    #[serde(default = "default_enabled")]
    pub(super) enabled: bool,
    #[serde(default)]
    pub(super) preview: String,
    #[serde(default)]
    pub(super) component_key: String,
    #[serde(default)]
    pub(super) tp2_file: String,
    #[serde(default)]
    pub(super) component_id: String,
    #[serde(default)]
    pub(super) component_name: String,
    #[serde(default)]
    pub(super) prompt_kind: String,
    #[serde(default)]
    pub(super) source: String,
    #[serde(default)]
    pub(super) captured_at: u64,
    #[serde(default)]
    pub(super) last_used_at: u64,
    #[serde(default)]
    pub(super) hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PromptAnswerEntry {
    #[serde(default)]
    pub(crate) alias: String,
    pub(crate) answer: String,
    #[serde(default = "default_enabled")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) preview: String,
    #[serde(default)]
    pub(crate) component_key: String,
    #[serde(default)]
    pub(crate) tp2_file: String,
    #[serde(default)]
    pub(crate) component_id: String,
    #[serde(default)]
    pub(crate) component_name: String,
    #[serde(default)]
    pub(crate) prompt_kind: String,
    #[serde(default)]
    pub(crate) source: String,
    #[serde(default)]
    pub(crate) captured_at: u64,
    #[serde(default)]
    pub(crate) last_used_at: u64,
    #[serde(default)]
    pub(crate) hit_count: u64,
}

pub(super) const fn default_enabled() -> bool {
    false
}
