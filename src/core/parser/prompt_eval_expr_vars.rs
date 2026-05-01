// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use super::PromptEvalContext;

#[derive(Debug, Clone, Default)]
pub(crate) struct PromptVarContext {
    pub(crate) vars: std::collections::HashMap<String, PromptVarValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptVarValue {
    Int(i64),
    Text(String),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromptComponentInput<'a> {
    pub(crate) raw_line: &'a str,
    pub(crate) component_id: &'a str,
    pub(crate) prompt_events: &'a [crate::parser::PromptSummaryEvent],
}

pub(crate) fn lookup_var<'a>(
    vars: Option<&'a PromptVarContext>,
    name: &str,
) -> Option<&'a PromptVarValue> {
    vars?.vars.get(&normalize_var_name(name))
}

pub fn normalize_tp2_stem(value: &str) -> String {
    let lower = value.replace('\\', "/").to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let no_ext = file.strip_suffix(".tp2").unwrap_or(file);
    no_ext.strip_prefix("setup-").unwrap_or(no_ext).to_string()
}

pub(crate) fn normalize_var_name(input: &str) -> String {
    input
        .trim()
        .trim_matches('%')
        .trim_matches('"')
        .trim_matches('~')
        .to_ascii_lowercase()
}

pub(crate) fn apply_component_block_assignments_text(
    text: &str,
    component_id: &str,
    ctx: &mut PromptVarContext,
) {
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

pub(crate) fn apply_source_file_assignments_text(text: &str, ctx: &mut PromptVarContext) {
    let lines = text.lines().collect::<Vec<_>>();
    for line in &lines {
        if line.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        if let Some((name, value)) = parse_outer_assignment(line) {
            ctx.vars.insert(normalize_var_name(&name), value);
        }
    }
}

pub(crate) fn apply_mod_compat_prompt_value_from_text(
    text: &str,
    prompt_eval: &PromptEvalContext,
    ctx: &mut PromptVarContext,
) {
    if !text.lines().any(|line| line.contains("prompt = 1")) {
        return;
    }
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

pub(crate) fn extract_tp2_path_from_raw_line(raw_line: &str) -> Option<PathBuf> {
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

pub(crate) fn extract_copy_table_path(lines: &[&str], needle: &str) -> Option<String> {
    for line in lines {
        if !line.trim_start().to_ascii_uppercase().starts_with("COPY ") {
            continue;
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
        if let Some(path) = quoted
            .into_iter()
            .find(|path| path.to_ascii_lowercase().contains(needle))
        {
            return Some(path);
        }
    }
    None
}

pub(crate) fn resolve_table_path(source_file: &Path, table_rel: &str) -> Option<PathBuf> {
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

fn trim_wrappers(input: &str) -> String {
    input
        .trim()
        .trim_matches('"')
        .trim_matches('~')
        .trim_matches('%')
        .to_string()
}
