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
                reorder_components_by_tp2_order(&mut deduped_components, &tp2_path, tp2_text);
            }
            let derived_weidu_groups = tp2_text
                .as_deref()
                .map(|tp2_text| detect_weidu_groups(&tp2_path, tp2_text, &deduped_components))
                .unwrap_or_default();
            let derived_collapsible_groups = tp2_text
                .as_deref()
                .map(|tp2_text| detect_derived_collapsible_groups(&tp_file, tp2_text, &deduped_components))
                .unwrap_or_default();
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
                    .map(|component| {
                        let derived_group = derived_collapsible_groups
                            .get(component.component_id.trim())
                            .cloned();
                        Step2ComponentState {
                            is_meta_mode_component: meta_mode_component_ids
                                .contains(component.component_id.trim()),
                            component_id: component.component_id.clone(),
                            label: component.display,
                            weidu_group: derived_weidu_groups
                                .get(component.component_id.trim())
                                .cloned(),
                            collapsible_group: derived_group.as_ref().map(|group| group.header.clone()),
                            collapsible_group_is_umbrella: derived_group
                                .as_ref()
                                .is_some_and(|group| group.is_umbrella),
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
                        }
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

fn detect_weidu_groups(
    tp2_path: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, String> {
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if ordered_blocks.len() < 2 {
        return HashMap::new();
    }

    let tra_map = load_tp2_setup_tra_map(Path::new(tp2_path));
    let component_ids = components
        .iter()
        .map(|component| component.component_id.trim().to_string())
        .collect::<std::collections::HashSet<_>>();

    let mut out = HashMap::<String, String>::new();
    let mut distinct = std::collections::HashSet::<String>::new();
    for block in &ordered_blocks {
        let component_id = block.component_id.trim();
        if !component_ids.contains(component_id) {
            continue;
        }
        let Some(group_token) = block.group_key.as_deref() else {
            continue;
        };
        let Some(group_label) = resolve_group_token_label(group_token, &tra_map) else {
            continue;
        };
        let cleaned = group_label.trim();
        if cleaned.is_empty() {
            continue;
        }
        out.insert(component_id.to_string(), cleaned.to_string());
        distinct.insert(cleaned.to_ascii_lowercase());
    }

    if distinct.len() < 2 {
        HashMap::new()
    } else {
        out
    }
}

fn load_tp2_setup_tra_map(tp2_path: &Path) -> HashMap<String, String> {
    let Some(base) = tp2_path.parent() else {
        return HashMap::new();
    };

    let mut candidates = Vec::<std::path::PathBuf>::new();
    let tp2_stem = tp2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let mod_key = tp2_stem.strip_prefix("setup-").unwrap_or(tp2_stem);
    let custom_setup_name = if mod_key.is_empty() {
        None
    } else {
        Some(format!("{mod_key}setup.tra"))
    };
    let preferred = [
        base.join("lang/english/setup.tra"),
        base.join("lang/english").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/english").join(name))
            .unwrap_or_default(),
        base.join("lang/en_us/setup.tra"),
        base.join("lang/en_us").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/en_us").join(name))
            .unwrap_or_default(),
        base.join("lang/en_US/setup.tra"),
        base.join("lang/en_US").join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join("lang/en_US").join(name))
            .unwrap_or_default(),
        base.join("setup.tra"),
        base.join(format!("{tp2_stem}.tra")),
        custom_setup_name
            .as_ref()
            .map(|name| base.join(name))
            .unwrap_or_default(),
    ];
    for path in preferred {
        if path.is_file() && !candidates.iter().any(|existing| existing == &path) {
            candidates.push(path);
        }
    }
    if candidates.is_empty() {
        for path in walk_setup_tra_files(base) {
            if !candidates.iter().any(|existing| existing == &path) {
                candidates.push(path);
            }
        }
    }

    for path in candidates {
        if let Ok(text) = fs::read_to_string(&path) {
            let map = parse_tra_string_map(&text);
            if !map.is_empty() {
                return map;
            }
        }
    }
    HashMap::new()
}

fn walk_setup_tra_files(base: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::<std::path::PathBuf>::new();
    let Ok(read_dir) = fs::read_dir(base) else {
        return out;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_setup_tra_files(&path));
            continue;
        }
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| {
                name.ends_with(".tra") && name.to_ascii_lowercase().contains("setup")
            })
        {
            out.push(path);
        }
    }
    out
}

fn parse_tra_string_map(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::<String, String>::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            continue;
        }
        let Some((key, rhs)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let rhs = rhs.trim();
        let value = if let Some(rest) = rhs.strip_prefix('~') {
            rest.split('~').next().unwrap_or_default().trim().to_string()
        } else if let Some(rest) = rhs.strip_prefix('"') {
            rest.split('"').next().unwrap_or_default().trim().to_string()
        } else {
            continue;
        };
        if !value.is_empty() {
            out.insert(key, value);
        }
    }
    out
}

fn resolve_group_token_label(token: &str, tra_map: &HashMap<String, String>) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('@') {
        return tra_map.get(trimmed).cloned();
    }
    if let Some(rest) = trimmed.strip_prefix('~') {
        return Some(rest.split('~').next().unwrap_or_default().trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        return Some(rest.split('"').next().unwrap_or_default().trim().to_string());
    }
    Some(trimmed.to_string())
}

fn detect_derived_collapsible_groups(
    tp_file: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, DerivedCollapsibleGroup> {
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if ordered_blocks.len() < 3 {
        return HashMap::new();
    }

    let display_by_id: HashMap<String, String> = components
        .iter()
        .map(|component| {
            (
                component.component_id.trim().to_string(),
                component.display.trim().to_string(),
            )
        })
        .collect();

    let mut same_mod_installed_guards = HashMap::<String, String>::new();
    for block in &ordered_blocks {
        let mut targets = block
            .body_lines
            .iter()
            .filter_map(|line| parse_same_mod_installed_guard_target(tp_file, line))
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        if targets.len() == 1 {
            same_mod_installed_guards.insert(block.component_id.clone(), targets[0].clone());
        }
    }

    let mut out = HashMap::<String, DerivedCollapsibleGroup>::new();
    let mut idx = 0usize;
    while idx < ordered_blocks.len() {
        let umbrella = &ordered_blocks[idx];
        let umbrella_id = umbrella.component_id.trim();
        let Some(umbrella_display) = display_by_id.get(umbrella_id) else {
            idx += 1;
            continue;
        };
        if split_subcomponent_display_label(umbrella_display).is_some() {
            idx += 1;
            continue;
        }

        let mut child_ids = Vec::<String>::new();
        let mut j = idx + 1;
        while j < ordered_blocks.len() {
            let child = &ordered_blocks[j];
            if same_mod_installed_guards
                .get(child.component_id.trim())
                .is_some_and(|target| target == umbrella_id)
            {
                child_ids.push(child.component_id.clone());
                j += 1;
            } else {
                break;
            }
        }

        if child_ids.len() >= 2 {
            let header = derive_collapsible_group_header(umbrella_display);
            out.insert(
                umbrella_id.to_string(),
                DerivedCollapsibleGroup {
                    header: header.clone(),
                    is_umbrella: true,
                },
            );
            for child_id in child_ids {
                if display_by_id.contains_key(child_id.trim()) {
                    out.insert(
                        child_id,
                        DerivedCollapsibleGroup {
                            header: header.clone(),
                            is_umbrella: false,
                        },
                    );
                }
            }
            idx = j;
            continue;
        }
        idx += 1;
    }

    out
}

fn derive_collapsible_group_header(umbrella_display: &str) -> String {
    let trimmed = umbrella_display.trim();
    let without_parenthetical = if trimmed.ends_with(')') {
        trimmed
            .rsplit_once('(')
            .map(|(head, _)| head.trim_end())
            .filter(|head| !head.is_empty())
            .unwrap_or(trimmed)
    } else {
        trimmed
    };
    let lower = without_parenthetical.to_ascii_lowercase();
    if lower.starts_with("install all spell tweaks") {
        return "Spell Tweaks".to_string();
    }
    let derived = lower
        .strip_prefix("install all ")
        .map(str::trim)
        .or_else(|| lower.strip_prefix("all ").map(str::trim));
    if let Some(rest) = derived
        && !rest.is_empty()
    {
        return title_case_words(rest);
    }
    without_parenthetical.to_string()
}

fn title_case_words(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_ascii_lowercase());
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_same_mod_installed_guard_target(tp_file: &str, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_PREDICATE") || !upper.contains("!MOD_IS_INSTALLED") {
        return None;
    }

    let guard_idx = upper.find("!MOD_IS_INSTALLED")?;
    let after = trimmed[guard_idx + "!MOD_IS_INSTALLED".len()..].trim_start();
    let quote = after.chars().next()?;
    if quote != '~' && quote != '"' {
        return None;
    }

    let rest = &after[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let raw_path = rest[..end].trim();
    if raw_path.is_empty() {
        return None;
    }

    let normalized_path = raw_path.replace('\\', "/");
    let raw_file = Path::new(&normalized_path)
        .file_name()
        .and_then(|v| v.to_str())?
        .to_ascii_lowercase();
    if raw_file != tp_file.to_ascii_lowercase() {
        return None;
    }

    let tail = rest[end + quote.len_utf8()..].trim_start();
    let component_id: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if component_id.is_empty() {
        None
    } else {
        Some(component_id)
    }
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

fn reorder_components_by_tp2_order(
    components: &mut [ScannedComponent],
    tp2_path: &str,
    tp2_text: &str,
) {
    let tra_map = load_tp2_setup_tra_map(Path::new(tp2_path));
    let (order_by_id, order_by_label) = parse_tp2_component_order(tp2_text, &tra_map);
    if order_by_id.is_empty() && order_by_label.is_empty() {
        return;
    }

    components.sort_by_key(|component| {
        order_by_id
            .get(component.component_id.trim())
            .copied()
            .or_else(|| {
                order_by_label
                    .get(&normalize_component_order_label(&component.display))
                    .copied()
            })
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
        for id in &component_ids {
            if let Some(component) = components.iter().find(|component| component.component_id.trim() == id)
                && let Some((_header, choice)) = split_subcomponent_display_label(&component.display)
                && choice.eq_ignore_ascii_case("skip")
            {
                hidden.insert(id.clone());
            }
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
                    component_id: id.clone(),
                    group_key: block.iter().find_map(|line| parse_group_key(line)),
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
        let component_id = block
            .iter()
            .find_map(|bl| parse_designated_id(&bl.to_ascii_uppercase()))
            .unwrap_or_default();
        out.push(Tp2ComponentBlock {
            component_id,
            group_key: block.iter().find_map(|line| parse_group_key(line)),
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

fn parse_group_key(line: &str) -> Option<String> {
    if line.trim_start().starts_with("//") {
        return None;
    }
    let upper = line.to_ascii_uppercase();
    let idx = upper.find("GROUP")?;
    let tail = line[idx + "GROUP".len()..].trim_start();
    if tail.is_empty() {
        return None;
    }
    if let Some(rest) = tail.strip_prefix('~') {
        let end = rest.find('~')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| format!("~{}~", value));
    }
    if let Some(rest) = tail.strip_prefix('"') {
        let end = rest.find('"')?;
        let value = rest[..end].trim();
        return (!value.is_empty()).then(|| format!("\"{}\"", value));
    }
    let value: String = tail
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    (!value.is_empty()).then_some(value)
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
    if trimmed.starts_with("//") {
        return None;
    }
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


fn parse_tp2_component_order(
    tp2_text: &str,
    tra_map: &HashMap<String, String>,
) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let mut out_by_id = HashMap::<String, usize>::new();
    let mut out_by_label = HashMap::<String, usize>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut i = 0usize;
    let mut begin_index = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if !trimmed.to_ascii_uppercase().starts_with("BEGIN ") {
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
        if let Some(label) = parse_begin_label(trimmed, tra_map) {
            out_by_label
                .entry(normalize_component_order_label(&label))
                .or_insert(begin_index);
        }
        for bl in block {
            if let Some(id) = parse_designated_id(&bl.to_ascii_uppercase()) {
                out_by_id.entry(id).or_insert(begin_index);
                break;
            }
        }

        begin_index += 1;
        i = j;
    }

    (out_by_id, out_by_label)
}

fn parse_begin_label(line: &str, tra_map: &HashMap<String, String>) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("BEGIN ") {
        return None;
    }
    let tail = trimmed["BEGIN".len()..].trim_start();
    if let Some(rest) = tail.strip_prefix('@') {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        let token = format!("@{digits}");
        return resolve_group_token_label(&token, tra_map);
    }
    let quote = tail.chars().next()?;
    if quote != '~' && quote != '"' {
        return None;
    }
    let rest = &tail[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn normalize_component_order_label(value: &str) -> String {
    let trimmed = value.trim();
    let base = trimmed
        .rsplit_once(':')
        .and_then(|(head, tail)| {
            let versionish = tail.trim();
            let normalized = versionish.strip_prefix('v').unwrap_or(versionish);
            (!normalized.is_empty()
                && normalized
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == '_' || c == '-'))
            .then_some(head.trim())
        })
        .unwrap_or(trimmed);
    base.to_ascii_lowercase()
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
    component_id: String,
    group_key: Option<String>,
    subcomponent_key: Option<String>,
    body_lines: Vec<String>,
}

#[derive(Debug, Clone)]
struct DerivedCollapsibleGroup {
    header: String,
    is_umbrella: bool,
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
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let idx = upper_line.find("DESIGNATED")?;
    let tail = upper_line[idx + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        let normalized = digits.trim_start_matches('0');
        if normalized.is_empty() {
            Some("0".to_string())
        } else {
            Some(normalized.to_string())
        }
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
