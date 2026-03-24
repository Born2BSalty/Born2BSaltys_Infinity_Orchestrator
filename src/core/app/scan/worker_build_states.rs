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
            let tp2_text = if tp2_path.trim().is_empty() {
                None
            } else {
                fs::read_to_string(&tp2_path).ok()
            };
            let mut deduped_components = dedup_components(comps);
            if let Some(tp2_text) = tp2_text.as_deref() {
                reorder_components_by_tp2_order(&mut deduped_components, tp2_text);
            }
            let hidden_prompt_like_component_ids =
                detect_hidden_prompt_like_component_ids(tp2_text.as_deref(), &deduped_components);
            deduped_components.retain(|component| {
                !hidden_prompt_like_component_ids.contains(component.component_id.trim())
            });
            let mod_prompt_summary = deduped_components
                .iter()
                .filter_map(|c| c.mod_prompt_summary.as_deref())
                .map(str::trim)
                .find(|s| !s.is_empty())
                .map(ToString::to_string);
            let mod_prompt_events = deduped_components
                .iter()
                .find_map(|c| {
                    (!c.mod_prompt_events.is_empty()).then_some(c.mod_prompt_events.clone())
                })
                .unwrap_or_default();
            let meta_mode_component_ids =
                detect_meta_mode_component_ids(&tp2_path, mods_root, tp2_text.as_deref());
            Step2ModState {
                tp_file,
                tp2_path,
                readme_path,
                web_url: None,
                mod_prompt_summary,
                mod_prompt_events,
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
                        prompt_summary: component.prompt_summary,
                        prompt_events: component.prompt_events,
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

fn reorder_components_by_tp2_order(components: &mut [ScannedComponent], tp2_text: &str) {
    let order = parse_tp2_component_order(tp2_text);
    if order.is_empty() {
        return;
    }

    components.sort_by_key(|component| {
        order
            .get(component.component_id.trim())
            .copied()
            .unwrap_or(usize::MAX)
    });
}

fn detect_hidden_prompt_like_component_ids(
    tp2_text: Option<&str>,
    components: &[ScannedComponent],
) -> std::collections::HashSet<String> {
    let mut hidden = std::collections::HashSet::<String>::new();
    let Some(tp2_text) = tp2_text else {
        return hidden;
    };

    let blocks = parse_tp2_component_blocks(tp2_text);
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if blocks.is_empty() && ordered_blocks.is_empty() {
        return hidden;
    }

    let mut families = Vec::<(String, Vec<String>)>::new();
    for component in components {
        let Some((header, _choice)) = split_subcomponent_display_label(&component.display) else {
            continue;
        };
        let header_key = header.to_ascii_lowercase();
        if let Some((_, ids)) = families.iter_mut().find(|(key, _)| *key == header_key) {
            ids.push(component.component_id.trim().to_string());
        } else {
            families.push((header_key, vec![component.component_id.trim().to_string()]));
        }
    }

    let mut family_size_counts = HashMap::<usize, usize>::new();
    for (_, component_ids) in &families {
        *family_size_counts.entry(component_ids.len()).or_insert(0) += 1;
    }

    let asset_only_cluster_counts =
        asset_only_subcomponent_cluster_size_counts(&ordered_blocks);

    for (_, component_ids) in families {
        if component_ids.len() < 2 {
            continue;
        }
        let mut family_blocks = Vec::<&Tp2ComponentBlock>::new();
        for id in &component_ids {
            let Some(block) = blocks.get(id) else {
                family_blocks.clear();
                break;
            };
            family_blocks.push(block);
        }
        if family_blocks.is_empty() {
            let size = component_ids.len();
            let family_size_is_unique = family_size_counts.get(&size).copied().unwrap_or(0) == 1;
            let asset_cluster_size_is_unique =
                asset_only_cluster_counts.get(&size).copied().unwrap_or(0) == 1;
            if family_size_is_unique && asset_cluster_size_is_unique {
                hidden.extend(component_ids);
            }
            continue;
        }
        if family_blocks.iter().all(|block| block_is_asset_choice_only(block)) {
            hidden.extend(component_ids);
        }
    }

    hidden
}

fn parse_tp2_component_blocks(tp2_text: &str) -> HashMap<String, Tp2ComponentBlock> {
    let mut out = HashMap::<String, Tp2ComponentBlock>::new();
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
        let component_id = block
            .iter()
            .find_map(|bl| parse_designated_id(&bl.to_ascii_uppercase()));
        if let Some(id) = component_id {
            out.insert(
                id.clone(),
                Tp2ComponentBlock {
                    subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
                    body_lines: block.iter().map(|line| (*line).to_string()).collect(),
                },
            );
        }
        i = j;
    }
    out
}

fn parse_tp2_component_blocks_in_order(tp2_text: &str) -> Vec<Tp2ComponentBlock> {
    let mut out = Vec::<Tp2ComponentBlock>::new();
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
        out.push(Tp2ComponentBlock {
            subcomponent_key: block.iter().find_map(|line| parse_subcomponent_key(line)),
            body_lines: block.iter().map(|line| (*line).to_string()).collect(),
        });
        i = j;
    }
    out
}

fn split_subcomponent_display_label(label: &str) -> Option<(String, String)> {
    let (base, choice) = label.split_once("->")?;
    let base = base.trim();
    let choice = choice.trim();
    if base.is_empty() || choice.is_empty() {
        None
    } else {
        Some((base.to_string(), choice.to_string()))
    }
}

fn block_is_asset_choice_only(block: &Tp2ComponentBlock) -> bool {
    let mut saw_copy = false;
    for line in &block.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("BEGIN ")
            || upper.starts_with("LABEL ")
            || upper.starts_with("SUBCOMPONENT ")
            || upper.starts_with("GROUP ")
            || upper.starts_with("REQUIRE_")
            || upper.starts_with("DESIGNATED ")
        {
            continue;
        }
        if (upper.starts_with("COPY ") || upper.starts_with("COPY_LARGE "))
            && copy_line_looks_like_cosmetic_asset(trimmed)
        {
            saw_copy = true;
            continue;
        }
        return false;
    }
    saw_copy
}

fn asset_only_subcomponent_cluster_size_counts(
    ordered_blocks: &[Tp2ComponentBlock],
) -> HashMap<usize, usize> {
    let mut out = HashMap::<usize, usize>::new();
    let mut i = 0usize;
    while i < ordered_blocks.len() {
        let Some(key) = ordered_blocks[i].subcomponent_key.as_deref() else {
            i += 1;
            continue;
        };
        let mut j = i + 1;
        while j < ordered_blocks.len()
            && ordered_blocks[j].subcomponent_key.as_deref() == Some(key)
        {
            j += 1;
        }

        let cluster = &ordered_blocks[i..j];
        if cluster.len() >= 2 && cluster.iter().all(block_is_asset_choice_only) {
            *out.entry(cluster.len()).or_insert(0) += 1;
        }
        i = j;
    }
    out
}

fn copy_line_looks_like_cosmetic_asset(line: &str) -> bool {
    let mut paths = extract_tilde_or_quote_paths(line);
    if paths.is_empty() {
        return false;
    }
    let source = paths.remove(0);
    let lower = source.replace('\\', "/").to_ascii_lowercase();
    let has_cosmetic_dir = lower.contains("/portrait")
        || lower.contains("/portraits/")
        || lower.contains("/art/")
        || lower.contains("/graphics/")
        || lower.contains("/sound/")
        || lower.contains("/sounds/")
        || lower.contains("/voice/")
        || lower.contains("/voices/");
    let cosmetic_ext = ["bmp", "png", "jpg", "jpeg", "bam", "mos", "pvrz", "wav", "ogg", "wbm"];
    let ext_ok = Path::new(&lower)
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|ext| cosmetic_ext.iter().any(|allowed| ext.eq_ignore_ascii_case(allowed)));
    has_cosmetic_dir || ext_ok
}

fn extract_tilde_or_quote_paths(line: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote != b'~' && quote != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        if i <= bytes.len() {
            let value = line[start..i].trim();
            if !value.is_empty() {
                out.push(value.to_string());
            }
        }
        i += 1;
    }
    out
}

fn parse_subcomponent_key(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.to_ascii_uppercase().starts_with("SUBCOMPONENT ") {
        return None;
    }
    let tail = trimmed["SUBCOMPONENT".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }
    if let Some(rest) = tail.strip_prefix('~') {
        let end = rest.find('~')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| value.to_string());
    }
    if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| value.to_string());
    }
    let value: String = tail
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    (!value.is_empty()).then_some(value)
}


fn parse_tp2_component_order(tp2_text: &str) -> HashMap<String, usize> {
    let mut out = HashMap::<String, usize>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut i = 0usize;
    let mut begin_index = 0usize;

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
        for bl in block {
            if let Some(id) = parse_designated_id(&bl.to_ascii_uppercase()) {
                out.entry(id).or_insert(begin_index);
                break;
            }
        }

        begin_index += 1;
        i = j;
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
struct Tp2ComponentBlock {
    subcomponent_key: Option<String>,
    body_lines: Vec<String>,
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
