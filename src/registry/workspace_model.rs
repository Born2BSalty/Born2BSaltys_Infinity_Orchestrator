// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModlistWorkspaceState {
    #[serde(default)]
    pub order_bgee: Vec<ComponentRef>,
    #[serde(default)]
    pub order_bg2ee: Vec<ComponentRef>,
    #[serde(default)]
    pub order_iwdee: Vec<ComponentRef>,
    #[serde(default)]
    pub expand_state: HashMap<String, bool>,
    #[serde(default)]
    pub step3_group_collapse: HashMap<String, bool>,
    #[serde(default)]
    pub prompt_overrides: HashMap<String, PromptOverride>,
    #[serde(default)]
    pub last_share_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRef {
    #[serde(default)]
    pub tp2: String,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub language: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptOverride {
    #[serde(default)]
    pub answer: String,
}
