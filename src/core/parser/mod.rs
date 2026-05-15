// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod compat_dependency_expr;
pub mod lapdu;
pub mod prompt_eval_expr;
pub mod prompt_eval_expr_tokens;
pub mod weidu_component_line;
pub mod weidu_version;

pub(crate) use compat_dependency_expr::{
    ParsedComponentRequirement, ParsedDependencyTarget, collect_component_requirements,
    parse_mod_is_installed_dependency_targets, parse_negated_mod_is_installed_targets,
};
#[cfg(test)]
pub(crate) use compat_dependency_expr::{
    normalize_component_id, parse_predicate_requirement_line, parse_requirement_line,
    parse_simple_mod_is_installed_predicate,
};
pub(crate) use prompt_eval_expr::{
    PromptComponentInput, PromptEvalContext, PromptVarContext,
    apply_component_block_assignments_text, apply_mod_compat_prompt_value_from_text,
    apply_source_file_assignments_text, evaluate_condition_clause, extract_copy_table_path,
    extract_tp2_path_from_raw_line, normalize_tp2_stem, resolve_table_path,
};
pub(crate) use prompt_eval_expr_tokens::{Token, tokenize};
pub(crate) use weidu_component_line::parse_lang;
pub(crate) use weidu_version::{normalize_version_text, parse_version};

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

#[must_use]
pub fn collect_prompt_summary_index(
    tp2_path: &Path,
    mods_root: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) -> PromptSummaryIndex {
    lapdu::collect_prompt_summary_index(tp2_path, mods_root, preferred_lang, preferred_game)
}

pub(crate) fn collect_tp2_component_blocks<'a>(tp2_text: &'a str) -> Vec<(String, Vec<&'a str>)> {
    let lines: Vec<&'a str> = tp2_text.lines().collect();
    let mut out = Vec::<(String, Vec<&'a str>)>::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            index += 1;
        }

        let block = lines[start..index].to_vec();
        let Some(component_id) = block.iter().find_map(|entry| {
            let upper_line = entry.to_ascii_uppercase();
            if upper_line.trim_start().starts_with("//") {
                return None;
            }
            let index = upper_line.find("DESIGNATED")?;
            let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                return None;
            }
            let normalized = digits.trim_start_matches('0');
            Some(if normalized.is_empty() {
                "0".to_string()
            } else {
                normalized.to_string()
            })
        }) else {
            continue;
        };

        out.push((component_id, block));
    }

    out
}
