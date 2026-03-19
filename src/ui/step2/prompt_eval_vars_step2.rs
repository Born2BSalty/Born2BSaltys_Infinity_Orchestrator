// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ui::state::Step2ComponentState;
use crate::ui::step2::state_step2::{PromptEvalContext, normalize_tp2_stem};

#[derive(Debug, Clone, Default)]
pub(crate) struct PromptVarContext {
    vars: HashMap<String, PromptVarValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptVarValue {
    Int(i64),
    Text(String),
}

pub(crate) fn build_prompt_var_context(
    component: &Step2ComponentState,
    prompt_eval: &PromptEvalContext,
) -> PromptVarContext {
    let mut ctx = PromptVarContext::default();
    if let Some(tp2_path) = extract_tp2_path_from_raw_line(&component.raw_line) {
        apply_component_block_assignments(&tp2_path, &component.component_id, &mut ctx);
    }

    let source_files = component
        .prompt_events
        .iter()
        .map(|event| event.source_file.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect::<HashSet<_>>();
    for source_file in source_files {
        apply_source_file_assignments(&source_file, prompt_eval, &mut ctx);
    }
    ctx
}

pub(crate) fn lookup_var<'a>(
    vars: Option<&'a PromptVarContext>,
    name: &str,
) -> Option<&'a PromptVarValue> {
    vars?.vars.get(&normalize_var_name(name))
}

fn apply_component_block_assignments(
    tp2_path: &Path,
    component_id: &str,
    ctx: &mut PromptVarContext,
) {
    let Ok(text) = fs::read_to_string(tp2_path) else {
        return;
    };
    let lines = text.lines().collect::<Vec<_>>();
    let Some(start) = find_component_block_start(&lines, component_id) else {
        return;
    };
    for line in &lines[start + 1..] {
        if line.trim_start().to_ascii_uppercase().starts_with("BEGIN ") {
            break;
        }
        if let Some((name, value)) = parse_outer_assignment(line) {
            ctx.vars.insert(normalize_var_name(&name), value);
        }
    }
}

fn apply_source_file_assignments(
    source_file: &str,
    prompt_eval: &PromptEvalContext,
    ctx: &mut PromptVarContext,
) {
    let path = PathBuf::from(source_file);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let lines = text.lines().collect::<Vec<_>>();

    for line in &lines {
        if line.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        if let Some((name, value)) = parse_outer_assignment(line) {
            ctx.vars.insert(normalize_var_name(&name), value);
        }
    }

    apply_mod_compat_prompt_value(&path, &lines, prompt_eval, ctx);
}

fn apply_mod_compat_prompt_value(
    source_file: &Path,
    lines: &[&str],
    prompt_eval: &PromptEvalContext,
    ctx: &mut PromptVarContext,
) {
    if !lines.iter().any(|line| line.contains("prompt = 1")) {
        return;
    }
    let Some(table_rel) = lines
        .iter()
        .find_map(|line| extract_copy_table_path(line, "mod_compat.2da"))
    else {
        return;
    };
    let Some(table_path) = resolve_table_path(source_file, &table_rel) else {
        return;
    };
    let Ok(text) = fs::read_to_string(table_path) else {
        return;
    };

    let mut prompt = matches!(ctx.vars.get("prompt"), Some(PromptVarValue::Int(v)) if *v != 0);
    for line in text.lines().skip(1) {
        let cols = line.split_whitespace().collect::<Vec<_>>();
        if cols.len() < 2 {
            continue;
        }
        let mod_key = normalize_tp2_stem(cols[0]);
        let component_id = cols[1].trim().to_string();
        if prompt_eval
            .checked_components
            .contains(&(mod_key, component_id))
        {
            prompt = true;
            break;
        }
    }
    ctx.vars
        .insert("prompt".to_string(), PromptVarValue::Int(i64::from(prompt)));
}

fn extract_tp2_path_from_raw_line(raw_line: &str) -> Option<PathBuf> {
    let start = raw_line.find('~')?;
    let end_rel = raw_line[start + 1..].find('~')?;
    let path = raw_line[start + 1..start + 1 + end_rel].trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

fn find_component_block_start(lines: &[&str], component_id: &str) -> Option<usize> {
    for (idx, line) in lines.iter().enumerate() {
        let upper = line.trim_start().to_ascii_uppercase();
        if !upper.starts_with("BEGIN ") || !upper.contains("DESIGNATED") {
            continue;
        }
        if upper.contains(&format!("DESIGNATED {component_id}")) {
            return Some(idx);
        }
    }
    None
}

fn parse_outer_assignment(line: &str) -> Option<(String, PromptVarValue)> {
    let trimmed = line.trim();
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("OUTER_SET ") {
        let rest = trimmed["OUTER_SET ".len()..].trim();
        let (name, value) = rest.split_once('=')?;
        let parsed = value.trim().parse::<i64>().ok()?;
        return Some((name.trim().to_string(), PromptVarValue::Int(parsed)));
    }
    if upper.starts_with("OUTER_SPRINT ") {
        let rest = trimmed["OUTER_SPRINT ".len()..].trim();
        let mut parts = rest.split_whitespace();
        let name = parts.next()?.trim().to_string();
        let value = parts.collect::<Vec<_>>().join(" ");
        if value.is_empty() {
            return None;
        }
        let clean = trim_wrappers(&value);
        return Some((name, PromptVarValue::Text(clean)));
    }
    None
}

fn extract_copy_table_path(line: &str, needle: &str) -> Option<String> {
    if !line.trim_start().to_ascii_uppercase().starts_with("COPY ") {
        return None;
    }
    let mut quoted = Vec::<String>::new();
    let chars = line.chars().collect::<Vec<_>>();
    let mut idx = 0usize;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch == '"' || ch == '~' {
            let quote = ch;
            idx += 1;
            let start = idx;
            while idx < chars.len() && chars[idx] != quote {
                idx += 1;
            }
            quoted.push(chars[start..idx].iter().collect::<String>());
        }
        idx += 1;
    }
    quoted
        .into_iter()
        .find(|path| path.to_ascii_lowercase().contains(needle))
}

fn resolve_table_path(source_file: &Path, table_rel: &str) -> Option<PathBuf> {
    let rel = table_rel.replace('\\', "/");
    let source_dir = source_file.parent()?;
    let mod_root = source_dir.parent().unwrap_or(source_dir);
    let mods_root = mod_root.parent().unwrap_or(mod_root);
    let candidates = [
        PathBuf::from(&rel),
        mods_root.join(&rel),
        mod_root.join(&rel),
        source_dir.join(&rel),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

fn normalize_var_name(input: &str) -> String {
    input
        .trim()
        .trim_matches('%')
        .trim_matches('"')
        .trim_matches('~')
        .to_ascii_lowercase()
}

fn trim_wrappers(input: &str) -> String {
    input
        .trim()
        .trim_matches('"')
        .trim_matches('~')
        .trim_matches('%')
        .to_string()
}
