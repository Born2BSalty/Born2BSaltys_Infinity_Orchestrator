// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

#[path = "prompt_eval_expr_conditions.rs"]
mod conditions;
#[path = "prompt_eval_expr_parser.rs"]
mod parser;
#[path = "prompt_eval_expr_values.rs"]
mod values;
#[path = "prompt_eval_expr_vars.rs"]
mod vars;

#[derive(Debug, Clone, Default)]
pub struct PromptEvalContext {
    pub active_games: HashSet<String>,
    pub active_engines: HashSet<String>,
    pub game_dir: Option<String>,
    pub checked_components: HashSet<(String, String)>,
    pub signature: String,
}
pub(crate) use conditions::evaluate_condition_clause;
pub(crate) use parser::Parser;
pub(crate) use values::EvalState;
pub use vars::normalize_tp2_stem;
pub(crate) use vars::{
    PromptComponentInput, PromptVarContext, PromptVarValue,
    apply_component_block_assignments_text, apply_mod_compat_prompt_value_from_text,
    apply_source_file_assignments_text, extract_copy_table_path, extract_tp2_path_from_raw_line,
    lookup_var, resolve_table_path,
};
