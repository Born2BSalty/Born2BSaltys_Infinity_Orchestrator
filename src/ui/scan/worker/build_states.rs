// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::ui::scan::ScannedComponent;
use crate::ui::scan::discovery::display_name_from_group_key;
use crate::ui::scan::parse::dedup_components;
use crate::ui::scan::readme::find_best_readme;
use crate::ui::state::{Step2ComponentState, Step2ModState};

pub(super) fn to_mod_states(
    map: BTreeMap<String, Vec<ScannedComponent>>,
    tp2_map: BTreeMap<String, String>,
    mods_root: &Path,
) -> Vec<Step2ModState> {
    let mut mods: Vec<Step2ModState> = map
        .into_iter()
        .map(|(group_key, comps)| {
            let tp2_path = tp2_map.get(&group_key).cloned().unwrap_or_default();
            let display_name = display_name_from_group_key(&group_key);
            let readme_path = find_best_readme(mods_root, &tp2_path, &display_name);
            let tp_file = Path::new(&tp2_path)
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| display_name.clone());
            let deduped_components = dedup_components(comps);
            let meta_mode_component_ids = detect_meta_mode_component_ids(&tp2_path, mods_root);
            Step2ModState {
                tp_file,
                tp2_path,
                readme_path,
                web_url: None,
                name: display_name,
                checked: false,
                components: deduped_components
                    .into_iter()
                    .map(|component| Step2ComponentState {
                        is_meta_mode_component: meta_mode_component_ids
                            .contains(component.component_id.trim()),
                        component_id: component.component_id,
                        label: component.display,
                        raw_line: component.raw_line,
                        disabled: false,
                        compat_kind: None,
                        compat_source: None,
                        compat_related_mod: None,
                        compat_related_component: None,
                        compat_graph: None,
                        compat_evidence: None,
                        disabled_reason: None,
                        checked: false,
                        selected_order: None,
                    })
                    .collect(),
            }
        })
        .collect();

    // If duplicate names still exist (same mod name in different folders),
    // append relative folder path to disambiguate in UI.
    let mut counts: HashMap<String, usize> = HashMap::new();
    for m in &mods {
        *counts.entry(m.name.to_ascii_lowercase()).or_insert(0) += 1;
    }
    for m in &mut mods {
        if counts.get(&m.name.to_ascii_lowercase()).copied().unwrap_or(0) > 1
            && let Ok(rel) = Path::new(&m.tp2_path).strip_prefix(mods_root)
            && let Some(parent) = rel.parent()
        {
            let rel_parent = parent.to_string_lossy().replace('\\', "/");
            m.name = format!("{} ({})", m.name, rel_parent);
        }
    }

    mods
}

fn detect_meta_mode_component_ids(
    tp2_path: &str,
    mods_root: &Path,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::<String>::new();
    let tp2 = Path::new(tp2_path);
    let Ok(tp2_text) = fs::read_to_string(tp2) else {
        return out;
    };

    let candidates = parse_no_log_record_candidates(&tp2_text);
    if candidates.is_empty() {
        return out;
    }

    let mut context = String::new();
    context.push_str(&tp2_text.to_ascii_lowercase());
    for include in include_paths_from_tp2(tp2, mods_root, &tp2_text) {
        if let Ok(text) = fs::read_to_string(&include) {
            context.push('\n');
            context.push_str(&text.to_ascii_lowercase());
        }
    }
    let has_batch_behavior = context.contains("--force-install-list")
        && context.contains("abort");
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
            if full.is_file() && !out.iter().any(|p| p == &full) {
                out.push(full);
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
struct CandidateMetaComponent {
    id: String,
    label: String,
}

fn parse_no_log_record_candidates(tp2_text: &str) -> Vec<CandidateMetaComponent> {
    let mut out = Vec::<CandidateMetaComponent>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if !line.trim_start().to_ascii_uppercase().starts_with("BEGIN ") {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len() {
            let next = lines[j].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            j += 1;
        }
        let block = &lines[i..j];
        let mut no_log_record = false;
        let mut component_id: Option<String> = None;
        let mut label = String::new();
        for bl in block {
            let upper = bl.to_ascii_uppercase();
            if upper.contains("NO_LOG_RECORD") {
                no_log_record = true;
            }
            if component_id.is_none() {
                component_id = parse_designated_id(&upper);
            }
            if label.is_empty() {
                label = parse_label_value(bl).unwrap_or_default();
            }
        }
        if no_log_record && let Some(id) = component_id {
            out.push(CandidateMetaComponent { id, label });
        }
        i = j;
    }
    out
}

fn parse_designated_id(upper_line: &str) -> Option<String> {
    let idx = upper_line.find("DESIGNATED")?;
    let tail = upper_line[idx + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn parse_label_value(line: &str) -> Option<String> {
    if !line.to_ascii_uppercase().contains("LABEL") {
        return None;
    }
    let start = line.find('~')?;
    let end_rel = line[start + 1..].find('~')?;
    Some(line[start + 1..start + 1 + end_rel].to_string())
}
