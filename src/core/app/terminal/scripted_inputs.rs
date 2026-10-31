// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::step5::answer_capture::ReplayPlan;
use std::collections::VecDeque;

use crate::platform_defaults::{compose_component_key, normalize_tp2_filename};

use super::EmbeddedTerminal;

struct ComponentContext {
    key: String,
    tp_file: String,
    component: String,
    component_name: Option<String>,
}

pub(super) fn set_scripted_inputs(term: &mut EmbeddedTerminal, entries: ReplayPlan) -> usize {
    term.scripted_inputs_by_component.clear();
    term.typed_scripted_inputs = entries
        .entries
        .into_iter()
        .map(|entry| (entry.component, VecDeque::from(entry.answers)))
        .collect();
    term.current_component_key = None;
    term.current_component_tp2 = None;
    term.current_component_id = None;
    term.current_component_name = None;
    term.current_batch.clear();
    term.pending_display_patches.clear();
    let mut total = term
        .typed_scripted_inputs
        .iter()
        .map(|(_, values)| values.len())
        .sum::<usize>();
    for (k, values) in entries.legacy_fallback {
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
    if term
        .capture_session
        .as_ref()
        .is_some_and(|session| session.phase().is_none())
    {
        return None;
    }
    if let Some(key) = term.capture_session.as_ref().and_then(|session| {
        session
            .active_component()
            .or_else(|| session.replay_component(term.current_component_key.as_deref()?))
    }) && let Some((_, queue)) = term
        .typed_scripted_inputs
        .iter_mut()
        .find(|(component, _)| component == key)
    {
        return queue.pop_front();
    }
    let current_key = term.current_component_key.as_ref()?;
    if term
        .capture_session
        .as_ref()
        .is_some_and(|session| !session.legacy_fallback_allowed(current_key))
    {
        return None;
    }
    let queue = term.scripted_inputs_by_component.get_mut(current_key)?;
    let next = queue.pop_front();
    if queue.is_empty() {
        term.scripted_inputs_by_component.remove(current_key);
    }
    next
}

pub(super) fn peek_next_scripted_input_for_current(term: &EmbeddedTerminal) -> Option<&str> {
    if term
        .capture_session
        .as_ref()
        .is_some_and(|session| session.phase().is_none())
    {
        return None;
    }
    if let Some(key) = term.capture_session.as_ref().and_then(|session| {
        session
            .active_component()
            .or_else(|| session.replay_component(term.current_component_key.as_deref()?))
    }) && let Some((_, queue)) = term
        .typed_scripted_inputs
        .iter()
        .find(|(component, _)| component == key)
    {
        return queue.front().map(String::as_str);
    }
    let current_key = term.current_component_key.as_ref()?;
    if term
        .capture_session
        .as_ref()
        .is_some_and(|session| !session.legacy_fallback_allowed(current_key))
    {
        return None;
    }
    let queue = term.scripted_inputs_by_component.get(current_key)?;
    queue.front().map(String::as_str)
}

fn parse_component_key_from_installer_line(line: &str) -> Option<ComponentContext> {
    let upper = line.to_ascii_uppercase();
    let is_install_component_line = upper.contains("INSTALLING MOD COMPONENT")
        || upper.contains("INSTALLED MOD COMPONENT")
        || upper.contains("INSTALLING MOD WEIDUBATCHEDCOMPONENTS")
        || upper.contains("INSTALLED MOD WEIDUBATCHEDCOMPONENTS");
    if !upper.contains("MOD_INSTALLER::INSTALLERS") || !is_install_component_line {
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
        key: compose_component_key(&tp_file, component),
        tp_file: normalize_tp2_filename(&tp_file),
        component: component.to_string(),
        component_name,
    })
}

pub(super) fn extract_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("{field}: \"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
