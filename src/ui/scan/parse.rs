// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::path::Path;

use super::ScannedComponent;

pub fn dedup_components(comps: Vec<ScannedComponent>) -> Vec<ScannedComponent> {
    let mut out_by_key = BTreeMap::<(String, String), ScannedComponent>::new();
    for component in comps {
        let tp_file_key = component
            .tp_file
            .as_deref()
            .map(normalize_tp_file)
            .unwrap_or_default();
        let key = (tp_file_key, component.component_id.clone());
        match out_by_key.get(&key) {
            Some(existing) => {
                if is_better_component_label(&existing.display, &component.display) {
                    out_by_key.insert(key, component);
                }
            }
            None => {
                out_by_key.insert(key, component);
            }
        }
    }
    out_by_key.into_values().collect()
}

pub fn parse_component_line(line: &str) -> Option<ScannedComponent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('~') {
        let install_path = trimmed.split('~').nth(1).unwrap_or_default().trim();
        let after_tp2 = trimmed.split('~').nth(2)?.trim_start();
        let mut tokens = after_tp2.split_whitespace();
        let _lang = tokens.next()?;
        let component_token = tokens.next()?;
        if !component_token.starts_with('#') {
            return None;
        }
        let component_id = component_token.trim_start_matches('#').to_string();
        if component_id.is_empty() || !component_id.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let display = trimmed
            .split_once("//")
            .map(|(_, rhs)| rhs.trim().to_string())
            .unwrap_or_else(|| trimmed.to_string());
        return Some(ScannedComponent {
            tp_file: Path::new(install_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string()),
            component_id,
            display,
            raw_line: trimmed.to_string(),
        });
    }

    let mut digits_end = 0usize;
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() {
            digits_end += 1;
        } else {
            break;
        }
    }
    if digits_end == 0 {
        return None;
    }
    let component_id = trimmed[..digits_end].to_string();
    let rest = trimmed[digits_end..].trim_start();
    let display = rest
        .strip_prefix(':')
        .or_else(|| rest.strip_prefix('-'))
        .unwrap_or(rest)
        .trim()
        .to_string();
    if display.is_empty() {
        None
    } else {
        Some(ScannedComponent {
            tp_file: None,
            component_id,
            display,
            raw_line: trimmed.to_string(),
        })
    }
}

pub fn normalize_tp_file(value: &str) -> String {
    let file = Path::new(value)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase();
    file.strip_prefix("setup-").unwrap_or(&file).to_string()
}

fn is_better_component_label(current: &str, candidate: &str) -> bool {
    let current_has_version = looks_like_versioned_label(current);
    let candidate_has_version = looks_like_versioned_label(candidate);
    if candidate_has_version && !current_has_version {
        return true;
    }
    if candidate_has_version == current_has_version {
        return candidate.len() > current.len();
    }
    false
}

fn looks_like_versioned_label(value: &str) -> bool {
    let trimmed = value.trim();
    if let Some((_, tail)) = trimmed.rsplit_once(':') {
        let ver = tail.trim();
        return !ver.is_empty() && ver.chars().any(|c| c.is_ascii_digit());
    }
    false
}
