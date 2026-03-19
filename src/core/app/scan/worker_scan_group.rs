// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::install::weidu_scan;
use crate::parser;
use crate::ui::scan::ScannedComponent;
use crate::ui::scan::cache::{ScanCache, cache_get, cache_put};
use crate::ui::scan::parse::{normalize_tp_file, parse_component_line};
use crate::ui::state::Step2Tp2ProbeReport;

use super::language::candidate_language_ids;

pub(super) fn scan_tp2_group(
    group_label: &str,
    weidu: &Path,
    game_dir: &Path,
    mods_root: &Path,
    tp2_paths: &[std::path::PathBuf],
    cache: &Arc<Mutex<ScanCache>>,
    ctx: &Arc<String>,
    preferred_locale: &str,
    game_install: &str,
) -> (Vec<ScannedComponent>, Vec<Step2Tp2ProbeReport>) {
    let mut entries = Vec::<ScannedComponent>::new();
    let mut reports = Vec::<Step2Tp2ProbeReport>::new();
    for tp2 in tp2_paths {
        let work_dir = preferred_scan_work_dir(tp2, mods_root);
        let mut probe = Step2Tp2ProbeReport {
            group_label: group_label.to_string(),
            tp2_path: tp2.display().to_string(),
            work_dir: work_dir.display().to_string(),
            ..Step2Tp2ProbeReport::default()
        };
        if let Some(cached) = cache_get(cache, ctx, tp2) {
            if !cached.is_empty() {
                probe.used_cache = true;
                probe.selected_from_cache = true;
                let prompt_index = parser::collect_prompt_summary_index(
                    tp2,
                    mods_root,
                    Some(preferred_locale),
                    Some(game_install),
                );
                apply_parser_probe_meta(&mut probe, &prompt_index);
                let cached = apply_prompt_index(cached, &prompt_index);
                probe.parsed_count = cached.len();
                probe.undefined_count = count_undefined_components(&cached);
                reports.push(probe);
                entries.extend(cached);
                continue;
            }
        }
        let prompt_index = parser::collect_prompt_summary_index(
            tp2,
            mods_root,
            Some(preferred_locale),
            Some(game_install),
        );
        apply_parser_probe_meta(&mut probe, &prompt_index);
        let expected_tp2 = tp2
            .file_name()
            .and_then(|n| n.to_str())
            .map(normalize_tp_file)
            .unwrap_or_default();
        let mut fallback_components = Vec::<ScannedComponent>::new();
        let mut fallback_language = None::<String>;
        let language_ids = candidate_language_ids(weidu, tp2, game_dir, work_dir, preferred_locale);
        probe.language_ids_tried = language_ids.clone();

        for lang_id in language_ids {
            let lines = weidu_scan::list_components_lines(weidu, game_dir, work_dir, tp2, &lang_id)
                .unwrap_or_default();
            let parsed_for_tp2 = parse_lines_for_tp2(tp2, &expected_tp2, lines);
            if parsed_for_tp2.is_empty() {
                continue;
            }
            let undefined = parsed_for_tp2
                .iter()
                .filter(|c| c.display.to_ascii_uppercase().contains("UNDEFINED STRING"))
                .count();
            if undefined < parsed_for_tp2.len() {
                probe.selected_language_id = Some(lang_id.clone());
                let parsed_for_tp2 = apply_prompt_index(parsed_for_tp2, &prompt_index);
                probe.parsed_count = parsed_for_tp2.len();
                probe.undefined_count = undefined;
                cache_put(cache, ctx, tp2, parsed_for_tp2.clone());
                entries.extend(parsed_for_tp2);
                fallback_components.clear();
                break;
            }
            if fallback_components.is_empty() {
                fallback_language = Some(lang_id.clone());
                fallback_components = parsed_for_tp2;
            }
        }
        if !fallback_components.is_empty() {
            probe.selected_language_id = fallback_language;
            let fallback_components = apply_prompt_index(fallback_components, &prompt_index);
            probe.parsed_count = fallback_components.len();
            probe.undefined_count = count_undefined_components(&fallback_components);
            cache_put(cache, ctx, tp2, fallback_components.clone());
            entries.extend(fallback_components);
        }
        reports.push(probe);
    }
    (entries, reports)
}

fn apply_prompt_index(
    mut components: Vec<ScannedComponent>,
    prompt_index: &parser::PromptSummaryIndex,
) -> Vec<ScannedComponent> {
    if components.is_empty() {
        return components;
    }

    let mut has_component_prompt = false;
    for component in &mut components {
        if let Some(summary) = prompt_index
            .by_component_id
            .get(component.component_id.trim())
        {
            component.prompt_summary = Some(summary.clone());
            has_component_prompt = true;
        }
        component.prompt_events = prompt_index
            .by_component_id_events
            .get(component.component_id.trim())
            .cloned()
            .unwrap_or_default();
    }

    if has_component_prompt {
        let mut lines = Vec::<String>::new();
        for component in &components {
            let Some(summary) = component.prompt_summary.as_deref() else {
                continue;
            };
            let summary = summary.trim();
            if summary.is_empty() {
                continue;
            }
            lines.push(format!("{}:\n{}", component.display.trim(), summary));
        }
        let mod_summary = if lines.is_empty() {
            prompt_index.mod_summary.clone()
        } else {
            Some(lines.join("\n\n"))
        };
        for component in &mut components {
            component.mod_prompt_summary = mod_summary.clone();
            component.mod_prompt_events = prompt_index.mod_events.clone();
        }
    } else {
        for component in &mut components {
            component.mod_prompt_summary = prompt_index.mod_summary.clone();
            component.mod_prompt_events = prompt_index.mod_events.clone();
        }
    }

    components
}

fn apply_parser_probe_meta(
    probe: &mut Step2Tp2ProbeReport,
    prompt_index: &parser::PromptSummaryIndex,
) {
    probe.parser_source_file = prompt_index.parser_source_file.clone();
    probe.parser_event_count = prompt_index.parser_event_count;
    probe.parser_warning_count = prompt_index.parser_warning_count;
    probe.parser_error_count = prompt_index.parser_error_count;
    probe.parser_diagnostic_preview = prompt_index.parser_diagnostic_preview.clone();
    probe.parser_raw_json = prompt_index.parser_raw_json.clone();
    probe.parser_tra_language_requested = prompt_index.parser_tra_language_requested.clone();
    probe.parser_tra_language_used = prompt_index.parser_tra_language_used.clone();
    probe.parser_flow_node_count = prompt_index.parser_flow_node_count;
    probe.parser_flow_event_ref_count = prompt_index.parser_flow_event_ref_count;
    probe.parser_event_with_parent_count = prompt_index.parser_event_with_parent_count;
    probe.parser_event_with_path_count = prompt_index.parser_event_with_path_count;
    probe.parser_option_component_binding_count =
        prompt_index.parser_option_component_binding_count;
    probe.parser_flow_preview = prompt_index.parser_flow_preview.clone();
}

fn preferred_scan_work_dir<'a>(tp2: &'a Path, mods_root: &'a Path) -> &'a Path {
    if let Some(parent) = tp2.parent()
        && let Some(parent_of_parent) = parent.parent()
    {
        return parent_of_parent;
    }
    if let Some(parent) = tp2.parent() {
        return parent;
    }
    mods_root
}

fn parse_lines_for_tp2(
    tp2: &Path,
    expected_tp2: &str,
    lines: Vec<String>,
) -> Vec<ScannedComponent> {
    let mut out = Vec::<ScannedComponent>::new();
    for line in lines {
        if let Some(mut parsed) = parse_component_line(&line) {
            if let Some(found_tp2) = parsed.tp_file.as_deref()
                && normalize_tp_file(found_tp2) != expected_tp2
            {
                continue;
            }
            if parsed.tp_file.is_none() {
                parsed.tp_file = tp2.file_name().map(|n| n.to_string_lossy().to_string());
            }
            out.push(parsed);
        }
    }
    out
}

fn count_undefined_components(components: &[ScannedComponent]) -> usize {
    components
        .iter()
        .filter(|c| c.display.to_ascii_uppercase().contains("UNDEFINED STRING"))
        .count()
}
