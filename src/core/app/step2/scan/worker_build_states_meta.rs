// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use super::tp2_blocks::parse_designated_id;

#[derive(Debug, Clone)]
struct CandidateMetaComponent {
    id: String,
    label: String,
}

pub(super) fn detect_meta_mode_component_ids(
    tp2_path: &str,
    mods_root: &Path,
    tp2_text: Option<&str>,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::<String>::new();
    let tp2 = Path::new(tp2_path);
    let Some(tp2_text) = tp2_text else {
        return out;
    };

    let candidates = parse_no_log_record_candidates(tp2_text);
    if candidates.is_empty() {
        return out;
    }

    let mut context = String::new();
    context.push_str(&tp2_text.to_ascii_lowercase());
    for include in include_paths_from_tp2(tp2, mods_root, tp2_text) {
        if let Ok(text) = fs::read_to_string(&include) {
            context.push('\n');
            context.push_str(&text.to_ascii_lowercase());
        }
    }
    let has_batch_behavior = context.contains("--force-install-list") && context.contains("abort");
    if !has_batch_behavior {
        return out;
    }

    for candidate in &candidates {
        let id_pattern = format!("component_number={}", candidate.id);
        let id_pattern_spaced = format!("component_number = {}", candidate.id);
        let has_direct_id_branch =
            context.contains(&id_pattern) || context.contains(&id_pattern_spaced);
        let has_batch_label = candidate.label.to_ascii_lowercase().contains("batch");
        if has_direct_id_branch || has_batch_label {
            out.insert(candidate.id.clone());
        }
    }

    if out.is_empty() && candidates.len() == 1 {
        out.insert(candidates[0].id.clone());
    }
    out
}

fn include_paths_from_tp2(
    tp2_path: &Path,
    mods_root: &Path,
    tp2_text: &str,
) -> Vec<std::path::PathBuf> {
    let mut out = Vec::<std::path::PathBuf>::new();
    let Some(base) = tp2_path.parent() else {
        return out;
    };
    for line in tp2_text.lines() {
        let upper = line.to_ascii_uppercase();
        if !upper.contains("INCLUDE") {
            continue;
        }
        let Some(start) = line.find('~') else {
            continue;
        };
        let Some(end_rel) = line[start + 1..].find('~') else {
            continue;
        };
        let raw = line[start + 1..start + 1 + end_rel].trim();
        if raw.is_empty() || raw.starts_with(".../") {
            continue;
        }
        let rel = raw.replace('\\', "/");
        let candidates = [mods_root.join(&rel), base.join(&rel)];
        for full in candidates {
            if full.is_file() && !out.iter().any(|path| path == &full) {
                out.push(full);
            }
        }
    }
    out
}

fn parse_no_log_record_candidates(tp2_text: &str) -> Vec<CandidateMetaComponent> {
    let mut out = Vec::<CandidateMetaComponent>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        if !line.trim_start().to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < lines.len() {
            let next = lines[end].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            end += 1;
        }
        let block = &lines[index..end];
        let mut no_log_record = false;
        let mut component_id: Option<String> = None;
        let mut label = String::new();
        for line in block {
            let upper = line.to_ascii_uppercase();
            if upper.contains("NO_LOG_RECORD") {
                no_log_record = true;
            }
            if component_id.is_none() {
                component_id = parse_designated_id(&upper);
            }
            if label.is_empty() {
                label = parse_label_value(line).unwrap_or_default();
            }
        }
        if no_log_record && let Some(id) = component_id {
            out.push(CandidateMetaComponent { id, label });
        }
        index = end;
    }
    out
}

fn parse_label_value(line: &str) -> Option<String> {
    if !line.to_ascii_uppercase().contains("LABEL") {
        return None;
    }
    let start = line.find('~')?;
    let end_rel = line[start + 1..].find('~')?;
    Some(line[start + 1..start + 1 + end_rel].to_string())
}
