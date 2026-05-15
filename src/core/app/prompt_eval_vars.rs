// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::parser::{
    PromptComponentInput, PromptEvalContext, PromptVarContext,
    apply_component_block_assignments_text, apply_mod_compat_prompt_value_from_text,
    apply_source_file_assignments_text, extract_copy_table_path, extract_tp2_path_from_raw_line,
    resolve_table_path,
};

pub(crate) fn build_prompt_var_context(
    component: PromptComponentInput<'_>,
    prompt_eval: &PromptEvalContext,
) -> PromptVarContext {
    let mut ctx = PromptVarContext::default();
    if let Some(tp2_path) = extract_tp2_path_from_raw_line(component.raw_line)
        && let Ok(text) = fs::read_to_string(&tp2_path)
    {
        apply_component_block_assignments_text(&text, component.component_id, &mut ctx);
    }

    let source_files = component
        .prompt_events
        .iter()
        .map(|event| event.source_file.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect::<HashSet<_>>();
    for source_file in source_files {
        let path = PathBuf::from(&source_file);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        apply_source_file_assignments_text(&text, &mut ctx);

        let lines = text.lines().collect::<Vec<_>>();
        let Some(table_rel) = extract_copy_table_path(&lines, "mod_compat.2da") else {
            continue;
        };
        let Some(table_path) = resolve_table_path(&path, &table_rel) else {
            continue;
        };
        let Ok(table_text) = fs::read_to_string(table_path) else {
            continue;
        };
        apply_mod_compat_prompt_value_from_text(&table_text, prompt_eval, &mut ctx);
    }
    ctx
}
