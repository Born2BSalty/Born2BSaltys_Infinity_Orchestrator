// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::app::scan::ScannedComponent;
use crate::app::scan::cache::{ScanCache, cache_get, cache_put};
use crate::app::scan::parse::{normalize_tp_file, parse_component_line};
use crate::app::state::Step2Tp2ProbeReport;
use crate::install::weidu_scan;
use crate::parser;

use super::language::candidate_language_ids;

pub(super) struct ScanGroupContext<'a> {
    pub group_label: &'a str,
    pub weidu: &'a Path,
    pub game_dir: &'a Path,
    pub mods_root: &'a Path,
    pub cache: &'a Arc<Mutex<ScanCache>>,
    pub ctx: &'a Arc<String>,
    pub preferred_locale: &'a str,
    pub game_install: &'a str,
}

pub(super) fn scan_tp2_group(
    scan_ctx: &ScanGroupContext<'_>,
    tp2_paths: &[std::path::PathBuf],
) -> Result<(Vec<ScannedComponent>, Vec<Step2Tp2ProbeReport>), String> {
    let mut entries = Vec::<ScannedComponent>::new();
    let mut reports = Vec::<Step2Tp2ProbeReport>::new();
    for tp2 in tp2_paths {
        let (tp2_entries, probe) = scan_one_tp2(scan_ctx, tp2)?;
        entries.extend(tp2_entries);
        reports.push(probe);
    }
    Ok((entries, reports))
}

fn scan_one_tp2(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
) -> Result<(Vec<ScannedComponent>, Step2Tp2ProbeReport), String> {
    let work_dir = preferred_scan_work_dir(tp2, scan_ctx.mods_root);
    let mut probe = Step2Tp2ProbeReport {
        group_label: scan_ctx.group_label.to_string(),
        tp2_path: tp2.display().to_string(),
        work_dir: work_dir.display().to_string(),
        ..Step2Tp2ProbeReport::default()
    };
    let prompt_index = parser::collect_prompt_summary_index(
        tp2,
        scan_ctx.mods_root,
        Some(scan_ctx.preferred_locale),
        Some(scan_ctx.game_install),
    );
    apply_parser_probe_meta(&mut probe, &prompt_index);
    if let Some(cached) = scan_cached_tp2(scan_ctx, tp2, &prompt_index, &mut probe) {
        return Ok((cached, probe));
    }
    let components = scan_uncached_tp2(scan_ctx, tp2, &work_dir, &prompt_index, &mut probe)?;
    Ok((components, probe))
}

fn scan_cached_tp2(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
) -> Option<Vec<ScannedComponent>> {
    let cached = cache_get(scan_ctx.cache, scan_ctx.ctx, tp2)?;
    if cached.is_empty() {
        return None;
    }
    probe.used_cache = true;
    probe.selected_from_cache = true;
    let cached = apply_prompt_index(cached, prompt_index);
    probe.parsed_count = cached.len();
    probe.undefined_count = count_undefined_components(&cached);
    Some(cached)
}

fn scan_uncached_tp2(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    work_dir: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
) -> Result<Vec<ScannedComponent>, String> {
    let expected_tp2 = tp2
        .file_name()
        .and_then(|n| n.to_str())
        .map(normalize_tp_file)
        .unwrap_or_default();
    let mut fallback_components = Vec::<ScannedComponent>::new();
    let mut fallback_language = None::<String>;
    let language_ids = candidate_language_ids(
        scan_ctx.weidu,
        tp2,
        scan_ctx.game_dir,
        work_dir,
        scan_ctx.preferred_locale,
    )?;
    probe.language_ids_tried.clone_from(&language_ids);

    for lang_id in language_ids {
        let parsed_for_tp2 =
            scan_language_components(scan_ctx, tp2, work_dir, &lang_id, &expected_tp2)?;
        if parsed_for_tp2.is_empty() {
            continue;
        }
        let undefined = count_undefined_components(&parsed_for_tp2);
        if undefined < parsed_for_tp2.len() {
            return Ok(select_language_components(
                scan_ctx,
                tp2,
                prompt_index,
                probe,
                lang_id,
                parsed_for_tp2,
                undefined,
            ));
        }
        if fallback_components.is_empty() {
            fallback_language = Some(lang_id);
            fallback_components = parsed_for_tp2;
        }
    }
    Ok(select_fallback_components(
        scan_ctx,
        tp2,
        prompt_index,
        probe,
        fallback_language,
        fallback_components,
    ))
}

fn scan_language_components(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    work_dir: &Path,
    lang_id: &str,
    expected_tp2: &str,
) -> Result<Vec<ScannedComponent>, String> {
    let lines = weidu_scan::list_components_lines(
        scan_ctx.weidu,
        scan_ctx.game_dir,
        work_dir,
        tp2,
        lang_id,
    )
    .map_err(|err| {
        format!(
            "failed to list components for {} (language {}): {err}",
            tp2.display(),
            lang_id
        )
    })?;
    Ok(parse_lines_for_tp2(tp2, expected_tp2, lines))
}

fn select_language_components(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
    lang_id: String,
    parsed_for_tp2: Vec<ScannedComponent>,
    undefined: usize,
) -> Vec<ScannedComponent> {
    probe.selected_language_id = Some(lang_id);
    let parsed_for_tp2 = apply_prompt_index(parsed_for_tp2, prompt_index);
    probe.parsed_count = parsed_for_tp2.len();
    probe.undefined_count = undefined;
    cache_put(scan_ctx.cache, scan_ctx.ctx, tp2, parsed_for_tp2.clone());
    parsed_for_tp2
}

fn select_fallback_components(
    scan_ctx: &ScanGroupContext<'_>,
    tp2: &Path,
    prompt_index: &parser::PromptSummaryIndex,
    probe: &mut Step2Tp2ProbeReport,
    fallback_language: Option<String>,
    fallback_components: Vec<ScannedComponent>,
) -> Vec<ScannedComponent> {
    if fallback_components.is_empty() {
        return fallback_components;
    }
    probe.selected_language_id = fallback_language;
    let fallback_components = apply_prompt_index(fallback_components, prompt_index);
    probe.parsed_count = fallback_components.len();
    probe.undefined_count = count_undefined_components(&fallback_components);
    cache_put(
        scan_ctx.cache,
        scan_ctx.ctx,
        tp2,
        fallback_components.clone(),
    );
    fallback_components
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
            component.mod_prompt_summary.clone_from(&mod_summary);
            component
                .mod_prompt_events
                .clone_from(&prompt_index.mod_events);
        }
    } else {
        for component in &mut components {
            component
                .mod_prompt_summary
                .clone_from(&prompt_index.mod_summary);
            component
                .mod_prompt_events
                .clone_from(&prompt_index.mod_events);
        }
    }

    components
}

fn apply_parser_probe_meta(
    probe: &mut Step2Tp2ProbeReport,
    prompt_index: &parser::PromptSummaryIndex,
) {
    probe
        .parser_source_file
        .clone_from(&prompt_index.parser_source_file);
    probe.parser_event_count = prompt_index.parser_event_count;
    probe.parser_warning_count = prompt_index.parser_warning_count;
    probe.parser_error_count = prompt_index.parser_error_count;
    probe
        .parser_diagnostic_preview
        .clone_from(&prompt_index.parser_diagnostic_preview);
    probe
        .parser_raw_json
        .clone_from(&prompt_index.parser_raw_json);
    probe
        .parser_tra_language_requested
        .clone_from(&prompt_index.parser_tra_language_requested);
    probe
        .parser_tra_language_used
        .clone_from(&prompt_index.parser_tra_language_used);
    probe.parser_flow_node_count = prompt_index.parser_flow_node_count;
    probe.parser_flow_event_ref_count = prompt_index.parser_flow_event_ref_count;
    probe.parser_event_with_parent_count = prompt_index.parser_event_with_parent_count;
    probe.parser_event_with_path_count = prompt_index.parser_event_with_path_count;
    probe.parser_option_component_binding_count =
        prompt_index.parser_option_component_binding_count;
    probe
        .parser_flow_preview
        .clone_from(&prompt_index.parser_flow_preview);
}

fn preferred_scan_work_dir(tp2: &Path, mods_root: &Path) -> PathBuf {
    if let Some(shared_work_dir) = shared_package_scan_work_dir(tp2, mods_root) {
        return shared_work_dir;
    }
    if let Some(parent) = tp2.parent()
        && let Some(parent_of_parent) = parent.parent()
    {
        return parent_of_parent.to_path_buf();
    }
    if let Some(parent) = tp2.parent() {
        return parent.to_path_buf();
    }
    mods_root.to_path_buf()
}

fn shared_package_scan_work_dir(tp2: &Path, mods_root: &Path) -> Option<PathBuf> {
    let source = fs::read_to_string(tp2).ok()?;
    let source_lower = source.to_ascii_lowercase();
    let tp2_parent = tp2.parent()?;
    let mut current = tp2_parent.parent()?;

    loop {
        if !current.starts_with(mods_root) {
            break;
        }
        let folder_name = current.file_name().and_then(|s| s.to_str())?;
        if source_references_shared_root(&source_lower, folder_name) {
            let work_dir = current
                .parent()
                .filter(|p| p.starts_with(mods_root))
                .unwrap_or(mods_root);
            return Some(work_dir.to_path_buf());
        }
        if current == mods_root {
            break;
        }
        let next = current.parent()?;
        if next == current {
            break;
        }
        current = next;
    }

    None
}

fn source_references_shared_root(source_lower: &str, folder_name: &str) -> bool {
    let folder_name = folder_name.to_ascii_lowercase();
    let tilde = format!("~{folder_name}/");
    let quote = format!("\"{folder_name}/");
    source_lower.contains(&tilde) || source_lower.contains(&quote)
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
