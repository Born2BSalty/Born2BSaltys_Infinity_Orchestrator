// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod lapdu;

#[derive(Debug, Clone, Default)]
pub struct PromptSummaryIndex {
    pub by_component_id: HashMap<String, String>,
    pub by_component_id_events: HashMap<String, Vec<PromptSummaryEvent>>,
    pub mod_summary: Option<String>,
    pub mod_events: Vec<PromptSummaryEvent>,
    pub parser_source_file: Option<String>,
    pub parser_event_count: usize,
    pub parser_warning_count: usize,
    pub parser_error_count: usize,
    pub parser_diagnostic_preview: Option<String>,
    pub parser_raw_json: Option<String>,
    pub parser_tra_language_requested: Option<String>,
    pub parser_tra_language_used: Option<String>,
    pub parser_flow_node_count: usize,
    pub parser_flow_event_ref_count: usize,
    pub parser_event_with_parent_count: usize,
    pub parser_event_with_path_count: usize,
    pub parser_option_component_binding_count: usize,
    pub parser_flow_preview: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptSummaryEvent {
    pub kind: String,
    pub node_id: String,
    pub text: String,
    pub summary_line: String,
    pub source_file: String,
    pub line: Option<u32>,
    pub branch_path: Vec<String>,
    pub condition: Option<String>,
    pub condition_id: Option<String>,
    pub game_allow: Vec<String>,
    pub game_deny: Vec<String>,
}

pub fn collect_prompt_summary_index(
    tp2_path: &Path,
    mods_root: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) -> PromptSummaryIndex {
    match lapdu::collect_prompt_summary_index(tp2_path, mods_root, preferred_lang, preferred_game) {
        Ok(index) => index,
        Err(err) => PromptSummaryIndex {
            parser_error_count: 1,
            parser_diagnostic_preview: Some(err.clone()),
            ..PromptSummaryIndex::default()
        },
    }
}
