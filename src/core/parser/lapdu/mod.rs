// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use self::model::ParserFlowNode;
use super::PromptSummaryIndex;

mod map_to_bio;
mod model;
mod runner;

pub fn collect_prompt_summary_index(
    tp2_path: &Path,
    mods_root: &Path,
    preferred_lang: Option<&str>,
    preferred_game: Option<&str>,
) -> PromptSummaryIndex {
    let run = match runner::run_lapdu_parser(tp2_path, mods_root, preferred_lang) {
        Ok(run) => run,
        Err(err) => {
            return PromptSummaryIndex {
                parser_error_count: 1,
                parser_diagnostic_preview: Some(err.message),
                parser_raw_json: err.raw_json,
                ..PromptSummaryIndex::default()
            };
        }
    };
    let mut index = map_to_bio::build_prompt_summary_index(&run.output, preferred_game);
    index.parser_raw_json = Some(run.raw_json);
    index.parser_tra_language_requested = non_empty(&run.output.tra_language_requested);
    index.parser_tra_language_used = non_empty(&run.output.tra_language_used);
    index.parser_flow_node_count = count_flow_nodes(&run.output.flow);
    index.parser_flow_event_ref_count = count_flow_event_refs(&run.output.flow);
    index.parser_event_with_parent_count = run
        .output
        .events
        .iter()
        .filter(|e| e.parent_id.as_deref().is_some_and(|v| !v.trim().is_empty()))
        .count();
    index.parser_event_with_path_count = run
        .output
        .events
        .iter()
        .filter(|e| !e.path_id.trim().is_empty())
        .count();
    index.parser_option_component_binding_count = run
        .output
        .events
        .iter()
        .flat_map(|e| e.options.iter())
        .map(|o| o.component_ids.len())
        .sum();
    index.parser_flow_preview = collect_flow_preview(&run.output.flow, 8);
    index
}

fn collect_flow_preview(nodes: &[ParserFlowNode], limit: usize) -> Vec<(String, String)> {
    let mut out = Vec::<(String, String)>::new();
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().rev().collect();
    while let Some(node) = stack.pop() {
        if out.len() >= limit {
            break;
        }
        out.push((node.id.clone(), node.label.clone()));
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }
    out
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn count_flow_nodes(nodes: &[ParserFlowNode]) -> usize {
    let mut total = 0usize;
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        total += 1;
        for child in &node.children {
            stack.push(child);
        }
    }
    total
}

fn count_flow_event_refs(nodes: &[ParserFlowNode]) -> usize {
    let mut total = 0usize;
    let mut stack: Vec<&ParserFlowNode> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        total += node.event_ids.len();
        for child in &node.children {
            stack.push(child);
        }
    }
    total
}
