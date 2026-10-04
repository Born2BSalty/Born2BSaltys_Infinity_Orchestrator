// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::parser::prompt_eval_expr::{
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
        if !text.lines().any(|line| line.contains("prompt = 1")) {
            continue;
        }

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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::parser::PromptSummaryEvent;
    use crate::parser::prompt_eval_expr::PromptVarValue;

    static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let n = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            Self(
                std::env::temp_dir()
                    .join(format!("bio_prompt_vars_test_{}_{n}", std::process::id())),
            )
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        fs::write(path, text).expect("write file");
    }

    fn write_randomiser(root: &Path, script: &str) -> PathBuf {
        let script_path = root.join("randomiser").join("lib").join("arrays.tpa");
        write_file(&script_path, script);
        write_file(
            &root.join("randomiser").join("lists").join("mod_compat.2da"),
            "Mod Component Ident Item\n\nrr.tp2 12 w20 amul21\n",
        );
        script_path
    }

    fn prompt_after(script_path: &Path, ticked: &[(&str, &str)]) -> Option<PromptVarValue> {
        let events = [PromptSummaryEvent {
            source_file: script_path.to_string_lossy().into_owned(),
            ..PromptSummaryEvent::default()
        }];
        let component = PromptComponentInput {
            raw_line: "~x/y.tp2~ #0 #1300",
            component_id: "1300",
            prompt_events: &events,
        };
        let prompt_eval = PromptEvalContext {
            checked_components: ticked
                .iter()
                .map(|(stem, id)| ((*stem).to_string(), (*id).to_string()))
                .collect(),
            ..PromptEvalContext::default()
        };
        build_prompt_var_context(component, &prompt_eval)
            .vars
            .get("prompt")
            .cloned()
    }

    #[test]
    fn a_script_that_reads_mod_compat_lights_prompt_for_a_ticked_row() {
        let root = TempRoot::new();
        let script_path = write_randomiser(
            &root.0,
            "OUTER_SET prompt = 0\nCOPY - \"randomiser/lists/mod_compat.2da\"\n      prompt = 1\nACTION_IF prompt = 1 BEGIN\n",
        );
        assert_eq!(
            prompt_after(&script_path, &[("rr", "12")]),
            Some(PromptVarValue::Int(1))
        );
        assert_eq!(
            prompt_after(&script_path, &[]),
            Some(PromptVarValue::Int(0))
        );
    }

    #[test]
    fn a_script_without_the_prompt_marker_leaves_prompt_unset() {
        let root = TempRoot::new();
        let script_path = write_randomiser(
            &root.0,
            "COPY - \"randomiser/lists/mod_compat.2da\"\nACTION_IF has_items BEGIN\n",
        );
        assert_eq!(prompt_after(&script_path, &[("rr", "12")]), None);
    }
}
