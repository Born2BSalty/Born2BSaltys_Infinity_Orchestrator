// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, VecDeque};

use super::EmbeddedTerminal;

struct ComponentContext {
    key: String,
    tp_file: String,
    component: String,
    component_name: Option<String>,
}

pub(super) fn set_scripted_inputs(
    term: &mut EmbeddedTerminal,
    entries: HashMap<String, Vec<String>>,
) -> usize {
    term.scripted_inputs_by_component.clear();
    term.current_component_key = None;
    term.current_component_tp2 = None;
    term.current_component_id = None;
    term.current_component_name = None;
    let mut total = 0usize;
    for (k, values) in entries {
        if values.is_empty() {
            continue;
        }
        total = total.saturating_add(values.len());
        term.scripted_inputs_by_component
            .insert(k, VecDeque::from(values));
    }
    term.scripted_inputs_loaded_count = total;
    total
}

pub(super) fn update_current_component_from_output(term: &mut EmbeddedTerminal, output: &str) {
    for line in output.lines() {
        if let Some(info) = parse_component_key_from_installer_line(line) {
            term.current_component_key = Some(info.key);
            term.current_component_tp2 = Some(info.tp_file);
            term.current_component_id = Some(info.component);
            term.current_component_name = info.component_name;
        }
    }
}

pub(super) fn take_next_scripted_input_for_current(term: &mut EmbeddedTerminal) -> Option<String> {
    let current_key = term.current_component_key.as_ref()?;
    let queue = term.scripted_inputs_by_component.get_mut(current_key)?;
    let next = queue.pop_front();
    if queue.is_empty() {
        term.scripted_inputs_by_component.remove(current_key);
    }
    next
}

pub(super) fn peek_next_scripted_input_for_current(term: &EmbeddedTerminal) -> Option<&str> {
    let current_key = term.current_component_key.as_ref()?;
    let queue = term.scripted_inputs_by_component.get(current_key)?;
    queue.front().map(String::as_str)
}

fn parse_component_key_from_installer_line(line: &str) -> Option<ComponentContext> {
    let upper = line.to_ascii_uppercase();
    if !upper.contains("MOD_INSTALLER::INSTALLERS")
        || !(upper.contains("INSTALLING MOD COMPONENT")
            || upper.contains("INSTALLED MOD COMPONENT"))
    {
        return None;
    }
    let tp_file = extract_field(line, "tp_file")?;
    let component = extract_field(line, "component")?;
    let component = component.trim();
    if component.is_empty() {
        return None;
    }
    let component_name = extract_field(line, "component_name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Some(ComponentContext {
        key: component_key(&tp_file, component),
        tp_file: normalize_tp2_filename(&tp_file),
        component: component.to_string(),
        component_name,
    })
}

fn component_key(tp_file: &str, component: &str) -> String {
    format!("{}#{}", normalize_tp2_filename(tp_file), component.trim())
}

fn normalize_tp2_filename(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let filename = replaced
        .rsplit('/')
        .next()
        .unwrap_or(replaced.as_str())
        .trim();
    filename.to_ascii_uppercase()
}

fn extract_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("{field}: \"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
