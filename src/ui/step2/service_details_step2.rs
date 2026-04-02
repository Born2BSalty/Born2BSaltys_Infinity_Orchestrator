// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{Step2Selection, WizardState};
use crate::ui::step2::service_component_parse_step2::{parse_lang, parse_version};
use crate::ui::step2::state_step2::Step2Details;

const DETAILS_COMPONENT_PREVIEW_MAX_LINES: usize = 48;
const DETAILS_COMPONENT_PREVIEW_MAX_CHARS: usize = 3_000;
const DETAILS_COMPONENT_PREVIEW_SOFT_EXTEND_LINES: usize = 12;

pub fn selected_details(state: &WizardState) -> Step2Details {
    let Some(selection) = &state.step2.selected else {
        return Step2Details::default();
    };
    let mods = match selection {
        Step2Selection::Mod { game_tab, .. } | Step2Selection::Component { game_tab, .. } => {
            if game_tab == "BGEE" {
                &state.step2.bgee_mods
            } else {
                &state.step2.bg2ee_mods
            }
        }
    };
    match selection {
        Step2Selection::Mod { tp_file, .. } => mods
            .iter()
            .find(|m| &m.tp_file == tp_file)
            .map(|mod_state| Step2Details {
                mod_name: Some(mod_state.name.clone()),
                component_label: None,
                component_id: None,
                shown_component_count: Some(mod_state.components.len()),
                hidden_component_count: Some(mod_state.hidden_components.len()),
                raw_component_count: Some(
                    mod_state.components.len() + mod_state.hidden_components.len(),
                ),
                component_lang: None,
                component_version: None,
                selected_order: None,
                is_checked: None,
                is_disabled: None,
                compat_kind: None,
                compat_role: None,
                compat_code: None,
                disabled_reason: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_related_target: None,
                compat_graph: None,
                compat_evidence: None,
                compat_component_block: None,
                raw_line: None,
                tp_file: Some(details_tp2_file_name(&mod_state.tp_file)),
                tp2_folder: details_parent_folder(&mod_state.tp2_path),
                tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
                readme_path: mod_state.readme_path.clone(),
                web_url: mod_state.web_url.clone(),
            })
            .unwrap_or_default(),
        Step2Selection::Component {
            tp_file,
            component_id,
            component_key,
            ..
        } => mods
            .iter()
            .find(|m| &m.tp_file == tp_file)
            .and_then(|mod_state| {
                mod_state
                    .components
                    .iter()
                    .find(|c| {
                        &c.component_id == component_id
                            && (component_key.is_empty() || c.raw_line == *component_key)
                    })
                    .map(|component| {
                        let component_tp2 = parse_component_tp2_from_raw(&component.raw_line)
                            .unwrap_or_else(|| mod_state.tp_file.clone());
                        let compat_kind = component.compat_kind.clone();
                        let compat_related_target =
                            component.compat_related_mod.as_deref().map(|related_mod| {
                                format!(
                                    "{}{}",
                                    related_mod,
                                    component
                                        .compat_related_component
                                        .as_deref()
                                        .map(|component_id| format!(" #{component_id}"))
                                        .unwrap_or_default()
                                )
                            });
                        let compat_component_block =
                            details_component_block_from_tp2(&mod_state.tp2_path, &component.component_id);

                        Step2Details {
                            mod_name: Some(details_display_name_from_tp2(&component_tp2)),
                            component_label: Some(component.label.clone()),
                            component_id: Some(component.component_id.clone()),
                            shown_component_count: Some(mod_state.components.len()),
                            hidden_component_count: Some(mod_state.hidden_components.len()),
                            raw_component_count: Some(
                                mod_state.components.len() + mod_state.hidden_components.len(),
                            ),
                            component_lang: parse_lang(&component.raw_line),
                            component_version: parse_version(&component.raw_line),
                            selected_order: component.selected_order,
                            is_checked: Some(component.checked),
                            is_disabled: Some(component.disabled),
                            compat_kind: compat_kind.clone(),
                            compat_role: compat_kind.as_ref().map(|kind| {
                                details_compat_role(kind, component.compat_source.as_deref())
                            }),
                            compat_code: compat_kind.as_deref().map(details_compat_code_from_kind),
                            disabled_reason: component.disabled_reason.clone(),
                            compat_source: component.compat_source.clone(),
                            compat_related_mod: component.compat_related_mod.clone(),
                            compat_related_component: component.compat_related_component.clone(),
                            compat_related_target,
                            compat_graph: component.compat_graph.clone(),
                            compat_evidence: component.compat_evidence.clone(),
                            compat_component_block,
                            raw_line: Some(component.raw_line.clone()),
                            tp_file: Some(details_tp2_file_name(&component_tp2)),
                            tp2_folder: details_parent_folder(&mod_state.tp2_path),
                            tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
                            readme_path: mod_state.readme_path.clone(),
                            web_url: mod_state.web_url.clone(),
                        }
                    })
            })
            .unwrap_or_default(),
    }
}

fn details_parent_folder(tp2_path: &str) -> Option<String> {
    let tp2_path = tp2_path.trim();
    if tp2_path.is_empty() {
        return None;
    }
    std::path::Path::new(tp2_path)
        .parent()
        .map(|path| path.display().to_string())
        .filter(|path| !path.trim().is_empty())
}

fn details_component_block_from_tp2(tp2_path: &str, component_id: &str) -> Option<String> {
    if tp2_path.trim().is_empty() || component_id.trim().is_empty() {
        return None;
    }
    let tp2_text = fs::read_to_string(tp2_path).ok()?;
    let lines: Vec<&str> = tp2_text.lines().collect();
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

        let block = &lines[start..index];
        let Some(block_id) = block
            .iter()
            .find_map(|entry| details_parse_designated_id(&entry.to_ascii_uppercase()))
        else {
            continue;
        };

        if block_id.eq_ignore_ascii_case(component_id.trim()) {
            let display_start = details_component_block_start(&lines, start);
            let display_end = details_component_block_end(&lines, display_start, index);
            let mut preview = lines[display_start..display_end].join("\n");
            if display_end < index {
                preview.push_str("\n...");
            }
            return Some(preview);
        }
    }

    None
}

fn details_component_block_start(lines: &[&str], block_start: usize) -> usize {
    let mut start = block_start;
    let mut saw_comment = false;

    while start > 0 {
        let prev = lines[start - 1].trim();
        if prev.is_empty() {
            start -= 1;
            continue;
        }
        if details_is_component_header_comment(prev) {
            saw_comment = true;
            start -= 1;
            continue;
        }
        break;
    }

    if saw_comment { start } else { block_start }
}

fn details_component_block_end(lines: &[&str], block_start: usize, block_end: usize) -> usize {
    let mut end = block_start;
    let mut char_count = 0usize;

    while end < block_end {
        char_count += lines[end].len() + 1;
        end += 1;
        if end - block_start >= DETAILS_COMPONENT_PREVIEW_MAX_LINES
            || char_count >= DETAILS_COMPONENT_PREVIEW_MAX_CHARS
        {
            break;
        }
    }

    if end >= block_end {
        while end > block_start && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        return end;
    }

    let mut extended = end;
    let soft_limit = (end + DETAILS_COMPONENT_PREVIEW_SOFT_EXTEND_LINES).min(block_end);
    while extended < soft_limit {
        let line = lines[extended].trim();
        extended += 1;
        if line.is_empty() {
            end = extended;
            break;
        }
        if details_is_component_section_banner_line(line) {
            end = extended.saturating_sub(1);
            break;
        }
    }

    while end > block_start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if end == block_start {
        let mut fallback = block_start + 1;
        while fallback < block_end {
            let line = lines[fallback].trim();
            if line.is_empty()
                || details_is_component_header_comment(line)
                || details_is_component_header_line(line)
            {
                fallback += 1;
                continue;
            }
            break;
        }
        while fallback > block_start && lines[fallback - 1].trim().is_empty() {
            fallback -= 1;
        }
        return fallback.max((block_start + 1).min(block_end));
    }

    end
}

fn details_is_component_header_comment(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("*/")
}

fn details_is_component_section_banner_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let slash_count = trimmed.chars().filter(|&ch| ch == '/').count();
    let backslash_count = trimmed.chars().filter(|&ch| ch == '\\').count();
    slash_count >= 6 && backslash_count >= 6
}

fn details_is_component_header_line(line: &str) -> bool {
    let upper = line.trim_start().to_ascii_uppercase();
    upper.starts_with("REQUIRE_")
        || upper.starts_with("FORBID_")
        || upper.starts_with("DESIGNATED")
        || upper.starts_with("LABEL")
        || upper.starts_with("GROUP")
        || upper.starts_with("SUBCOMPONENT")
        || upper.starts_with("VERSION")
}

fn details_compat_code_from_kind(kind: &str) -> String {
    if kind.eq_ignore_ascii_case("mismatch") || kind.eq_ignore_ascii_case("game_mismatch") {
        "MISMATCH".to_string()
    } else if kind.eq_ignore_ascii_case("missing_dep") {
        "REQ_MISSING".to_string()
    } else if kind.eq_ignore_ascii_case("conflict") || kind.eq_ignore_ascii_case("not_compatible") {
        "RULE_HIT".to_string()
    } else if kind.eq_ignore_ascii_case("included") {
        "INCLUDED".to_string()
    } else if kind.eq_ignore_ascii_case("order_block") {
        "ORDER_BLOCK".to_string()
    } else if kind.eq_ignore_ascii_case("conditional") {
        "CONDITIONAL".to_string()
    } else if kind.eq_ignore_ascii_case("path_requirement") {
        "PATH_REQUIREMENT".to_string()
    } else if kind.eq_ignore_ascii_case("deprecated") {
        "DEPRECATED".to_string()
    } else {
        kind.to_ascii_uppercase()
    }
}

fn details_compat_role(kind: &str, source: Option<&str>) -> String {
    let Some(source) = source.map(str::trim) else {
        return "Compatibility rule".to_string();
    };
    let lower = source.to_ascii_lowercase();
    if lower.ends_with(".toml") {
        "Rule file".to_string()
    } else if lower.ends_with(".tp2") {
        if kind.eq_ignore_ascii_case("mismatch") {
            "TP2 guard".to_string()
        } else if kind.eq_ignore_ascii_case("missing_dep") {
            "TP2 dependency check".to_string()
        } else if kind.eq_ignore_ascii_case("path_requirement") {
            "TP2 path check".to_string()
        } else {
            "TP2 relation".to_string()
        }
    } else {
        "Compatibility rule".to_string()
    }
}

fn details_parse_designated_id(upper_line: &str) -> Option<String> {
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let index = upper_line.find("DESIGNATED")?;
    let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(|ch| ch.is_ascii_digit()).collect();
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

fn details_display_name_from_tp2(tp2_ref: &str) -> String {
    let file = details_tp2_file_name(tp2_ref);
    let lower = file.to_ascii_lowercase();
    let stem = if lower.ends_with(".tp2") {
        &file[..file.len().saturating_sub(4)]
    } else {
        file.as_str()
    };
    let stem = if stem.to_ascii_lowercase().starts_with("setup-") {
        &stem[6..]
    } else {
        stem
    };
    if stem.is_empty() {
        return tp2_ref.to_string();
    }
    stem.to_string()
}

fn details_tp2_file_name(tp2_ref: &str) -> String {
    let trimmed = tp2_ref.trim().trim_matches(['~', '"']);
    let file = if let Some(idx) = trimmed.rfind(['/', '\\']) {
        &trimmed[idx + 1..]
    } else {
        trimmed
    };
    if file.is_empty() {
        tp2_ref.to_string()
    } else {
        file.to_string()
    }
}
