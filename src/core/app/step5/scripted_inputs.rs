// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::app::state::Step1State;
use crate::app::step5::command_config::build_install_command_config;
use crate::install::step5_command_log_paths::{resolve_bg2_log_file, resolve_bgee_log_file};
use crate::platform_defaults::compose_component_key;

use super::prompt_memory;

pub(crate) fn load_from_step1(step1: &Step1State) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for path in source_log_paths(step1) {
        merge_from_path(&mut out, Path::new(&path));
    }
    merge_from_prompt_memory(&mut out);
    out
}

fn merge_from_prompt_memory(dst: &mut HashMap<String, Vec<String>>) {
    let memory_entries = prompt_memory::list_component_sequences();
    for (component_key, inputs) in memory_entries {
        if inputs.is_empty() {
            continue;
        }
        dst.entry(component_key).or_insert(inputs);
    }
}

fn source_log_paths(step1: &Step1State) -> Vec<String> {
    let install_config = build_install_command_config(step1);
    if install_config.game_install == "EET" {
        vec![
            resolve_bgee_log_file(&install_config),
            resolve_bg2_log_file(&install_config),
        ]
    } else if install_config.game_install == "BG2EE" {
        vec![resolve_bg2_log_file(&install_config)]
    } else {
        vec![resolve_bgee_log_file(&install_config)]
    }
}

fn merge_from_path(dst: &mut HashMap<String, Vec<String>>, path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    for line in raw.lines() {
        if let Some((component, inputs)) = parse_line(line) {
            if inputs.is_empty() {
                continue;
            }
            dst.entry(component).or_default().extend(inputs);
        }
    }
}

fn parse_line(line: &str) -> Option<(String, Vec<String>)> {
    let marker = "@wlb-inputs:";
    let marker_pos = line.to_ascii_lowercase().find(marker)?;
    let spec = line[marker_pos + marker.len()..].trim();
    if spec.is_empty() {
        return None;
    }

    let trimmed = line.trim();
    if !trimmed.starts_with('~') {
        return None;
    }
    let first = trimmed.find('~')?;
    let rest = &trimmed[first + 1..];
    let second_rel = rest.find('~')?;
    let tp2 = &rest[..second_rel];
    let after = rest[second_rel + 1..].trim_start();
    let mut parts = after.split_whitespace();
    let _lang = parts.next()?;
    let comp = parts.next()?;
    let comp = comp.trim_start_matches('#');
    if comp.is_empty() {
        return None;
    }

    let key = compose_component_key(tp2, comp);
    let inputs = parse_inputs(spec);
    Some((key, inputs))
}

fn parse_inputs(spec: &str) -> Vec<String> {
    spec.split(',').map(|p| p.trim().to_string()).collect()
}
