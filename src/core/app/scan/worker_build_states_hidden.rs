// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ui::scan::ScannedComponent;

use super::tp2_blocks::{
    extract_tilde_or_quote_paths, parse_tp2_component_blocks, parse_tp2_component_blocks_in_order,
    split_subcomponent_display_label, Tp2ComponentBlock,
};

pub(super) fn detect_hidden_prompt_like_component_ids(
    tp2_text: Option<&str>,
    components: &[ScannedComponent],
) -> HashSet<String> {
    let mut hidden = HashSet::<String>::new();
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

    let asset_only_cluster_counts = asset_only_subcomponent_cluster_size_counts(&ordered_blocks);

    for (_, component_ids) in families {
        if component_ids.len() < 2 {
            continue;
        }
        for id in &component_ids {
            if let Some(component) = components
                .iter()
                .find(|component| component.component_id.trim() == id)
                && let Some((_header, choice)) =
                    split_subcomponent_display_label(&component.display)
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
    let mut index = 0usize;
    while index < ordered_blocks.len() {
        let Some(key) = ordered_blocks[index].subcomponent_key.as_deref() else {
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < ordered_blocks.len()
            && ordered_blocks[end].subcomponent_key.as_deref() == Some(key)
        {
            end += 1;
        }

        let cluster = &ordered_blocks[index..end];
        if cluster.len() >= 2 && cluster.iter().all(block_is_asset_choice_only) {
            *out.entry(cluster.len()).or_insert(0) += 1;
        }
        index = end;
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
    let cosmetic_ext = [
        "bmp", "png", "jpg", "jpeg", "bam", "mos", "pvrz", "wav", "ogg", "wbm",
    ];
    let ext_ok = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            cosmetic_ext
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        });
    has_cosmetic_dir || ext_ok
}
