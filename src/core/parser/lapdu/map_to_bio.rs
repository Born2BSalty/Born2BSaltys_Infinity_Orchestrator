// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeSet, HashMap};

use super::model::{ParserEvent, ParserOutput};
use crate::parser::{PromptSummaryEvent, PromptSummaryIndex};

pub(super) fn build_prompt_summary_index(
    output: &ParserOutput,
    preferred_game: Option<&str>,
) -> PromptSummaryIndex {
    if output.schema_version != 2 {
        return PromptSummaryIndex::default();
    }

    let target_game = normalize_game_id(preferred_game);
    let mut by_component_id: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_component_id_events: HashMap<String, Vec<PromptSummaryEvent>> = HashMap::new();
    let mut mod_lines: Vec<String> = Vec::new();
    let mut mod_events = Vec::<PromptSummaryEvent>::new();

    for event in &output.events {
        if event.kind.eq_ignore_ascii_case("subcomponent_prompt") {
            continue;
        }
        if !event_matches_game(event, target_game.as_deref()) {
            continue;
        }
        if !should_include_event(event) {
            continue;
        }
        let line = event_summary_line(event);
        if line.is_empty() {
            continue;
        }
        let summary_event = PromptSummaryEvent {
            kind: event.kind.clone(),
            node_id: event.node_id.clone(),
            text: event.text.clone(),
            summary_line: line.clone(),
            source_file: event.source_file.clone(),
            line: event.line,
            branch_path: event.branch_path.clone(),
            condition: event.condition.clone(),
            condition_id: event.condition_id.clone(),
            game_allow: event.game_allow.clone(),
            game_deny: event.game_deny.clone(),
        };
        mod_lines.push(line.clone());
        mod_events.push(summary_event.clone());

        if let Some(component_id) = extract_component_id_from_node_id(&event.node_id) {
            by_component_id
                .entry(component_id.clone())
                .or_default()
                .push(line);
            by_component_id_events
                .entry(component_id)
                .or_default()
                .push(summary_event);
        }
    }

    let by_component_id = by_component_id
        .into_iter()
        .map(|(k, v)| (k, dedupe_and_join(v, 4)))
        .collect::<HashMap<_, _>>();

    let mod_summary = if mod_lines.is_empty() {
        None
    } else {
        Some(dedupe_and_join(mod_lines, 6))
    };

    PromptSummaryIndex {
        by_component_id,
        by_component_id_events,
        mod_summary,
        mod_events,
        parser_source_file: (!output.source_file.trim().is_empty())
            .then_some(output.source_file.clone()),
        parser_event_count: output.events.len(),
        parser_warning_count: output.warnings.len(),
        parser_error_count: output.errors.len(),
        parser_diagnostic_preview: diagnostic_preview(output),
    }
}

fn normalize_game_id(input: Option<&str>) -> Option<String> {
    let lower = input?.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    if lower.contains("iwd") {
        return Some("iwdee".to_string());
    }
    if lower.contains("eet") {
        return Some("eet".to_string());
    }
    if lower.contains("bg2") {
        return Some("bg2ee".to_string());
    }
    if lower.contains("bg1") || lower.contains("bgee") {
        return Some("bgee".to_string());
    }
    Some(lower)
}

fn event_matches_game(event: &ParserEvent, target_game: Option<&str>) -> bool {
    let Some(target_game) = target_game else {
        return true;
    };
    let deny_match = event
        .game_deny
        .iter()
        .any(|g| g.trim().eq_ignore_ascii_case(target_game));
    if deny_match {
        return false;
    }
    if event.game_allow.is_empty() {
        return true;
    }
    event
        .game_allow
        .iter()
        .any(|g| g.trim().eq_ignore_ascii_case(target_game))
}

fn event_summary_line(event: &ParserEvent) -> String {
    let text = event.text.trim();
    if text.is_empty() {
        return String::new();
    }

    let mut out = text.to_string();
    if !event.options.is_empty() {
        let mut options = Vec::<String>::new();
        for o in &event.options {
            let value = o.value.trim();
            let label = o.label.trim();
            if value.is_empty() && label.is_empty() {
                continue;
            }
            if value == "0"
                && (label.is_empty() || label.eq_ignore_ascii_case("option 0"))
                && event.options.len() == 1
            {
                continue;
            }
            if value.is_empty() {
                options.push(label.to_string());
            } else if label.is_empty() {
                options.push(value.to_string());
            } else {
                options.push(format!("{value} = {label}"));
            }
            if options.len() >= 4 {
                break;
            }
        }
        if !options.is_empty() {
            for option in options {
                out.push('\n');
                out.push_str("- ");
                out.push_str(&option);
            }
        }
    }
    out
}

fn should_include_event(event: &ParserEvent) -> bool {
    if event.interactive == Some(false) {
        return false;
    }
    let text = event.text.trim();
    if text.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("add_spell_ex:") {
        return false;
    }
    if lower.contains("press a key to continue") {
        return false;
    }
    true
}

fn dedupe_and_join(lines: Vec<String>, max_items: usize) -> String {
    let mut seen = BTreeSet::<String>::new();
    let mut out = Vec::<String>::new();
    for line in lines {
        let normalized = line.trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.clone()) {
            out.push(normalized);
            if out.len() >= max_items {
                break;
            }
        }
    }
    out.join("\n")
}

fn extract_component_id_from_node_id(node_id: &str) -> Option<String> {
    let marker = ":@";
    let pos = node_id.find(marker)?;
    let rest = &node_id[pos + marker.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn diagnostic_preview(output: &ParserOutput) -> Option<String> {
    let first = output
        .errors
        .first()
        .or_else(|| output.warnings.first())?;
    let code = first.code.trim();
    let msg = first.message.trim();
    if code.is_empty() && msg.is_empty() {
        None
    } else if code.is_empty() {
        Some(msg.to_string())
    } else if msg.is_empty() {
        Some(code.to_string())
    } else {
        Some(format!("{code}: {msg}"))
    }
}
