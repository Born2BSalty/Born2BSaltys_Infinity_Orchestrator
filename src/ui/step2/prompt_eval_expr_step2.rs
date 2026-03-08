// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::step2::prompt_eval_expr_engine_step2::{EvalState, Parser};
use crate::ui::step2::prompt_eval_expr_tokens_step2::tokenize;
use crate::ui::step2::prompt_eval_vars_step2::PromptVarContext;
use crate::ui::step2::state_step2::PromptEvalContext;

pub(crate) fn evaluate_condition_clause(
    condition_text: &str,
    prompt_eval: &PromptEvalContext,
    prompt_vars: Option<&PromptVarContext>,
) -> bool {
    evaluate_condition_clause_state(condition_text, prompt_eval, prompt_vars).is_not_false()
}

pub(crate) fn evaluate_condition_clause_state(
    condition_text: &str,
    prompt_eval: &PromptEvalContext,
    prompt_vars: Option<&PromptVarContext>,
) -> EvalState {
    let normalized = normalize_condition_text(condition_text);
    if normalized.is_empty() {
        return EvalState::True;
    }
    let tokens = tokenize(&normalized);
    if tokens.is_empty() {
        return EvalState::True;
    }
    let mut parser = Parser::new(tokens, prompt_eval, prompt_vars);
    let value = parser.parse_expression();
    if parser.is_at_end() {
        value
    } else {
        EvalState::Unknown
    }
}

fn normalize_condition_text(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut normalized = trimmed.replace("&&", " AND ");
    if normalized.to_ascii_uppercase().contains("ACTION_IF") {
        normalized = replace_ascii_ci(&normalized, "if:", "");
        normalized = replace_ascii_ci(&normalized, "else:", "");
        normalized = replace_ascii_ci(&normalized, "ACTION_IF", "");
        normalized = replace_ascii_ci(&normalized, "BEGIN", "");
        return normalized.trim().to_string();
    }
    strip_trailing_begin(&normalized).trim().to_string()
}

fn strip_trailing_begin(input: &str) -> &str {
    let upper = input.to_ascii_uppercase();
    if let Some(idx) = upper.rfind("BEGIN") {
        return &input[..idx];
    }
    input
}

fn replace_ascii_ci(input: &str, needle: &str, replacement: &str) -> String {
    let upper_input = input.to_ascii_uppercase();
    let upper_needle = needle.to_ascii_uppercase();
    let mut out = String::with_capacity(input.len());
    let mut start = 0usize;
    while let Some(rel_idx) = upper_input[start..].find(&upper_needle) {
        let idx = start + rel_idx;
        out.push_str(&input[start..idx]);
        out.push_str(replacement);
        start = idx + needle.len();
    }
    out.push_str(&input[start..]);
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{evaluate_condition_clause, evaluate_condition_clause_state};
    use crate::ui::step2::prompt_eval_expr_engine_step2::EvalState;
    use crate::ui::step2::state_step2::PromptEvalContext;

    fn test_context() -> PromptEvalContext {
        let mut active_games = HashSet::new();
        active_games.insert("eet".to_string());
        active_games.insert("bg2ee".to_string());

        let mut checked_components = HashSet::new();
        checked_components.insert(("randomiser".to_string(), "1300".to_string()));
        checked_components.insert(("rr".to_string(), "12".to_string()));

        PromptEvalContext {
            active_games,
            game_dir: None,
            checked_components,
        }
    }

    #[test]
    fn evaluates_negated_mod_call_forms() {
        let ctx = test_context();
        assert!(evaluate_condition_clause("!MOD_IS_INSTALLED ~randomiser~ ~1200~", &ctx, None));
        assert!(evaluate_condition_clause("!(MOD_IS_INSTALLED ~randomiser~ ~1200~)", &ctx, None));
        assert!(evaluate_condition_clause("! (MOD_IS_INSTALLED ~randomiser~ ~1200~)", &ctx, None));
        assert!(evaluate_condition_clause("NOT MOD_IS_INSTALLED ~randomiser~ ~1200~", &ctx, None));
        assert!(!evaluate_condition_clause("!MOD_IS_INSTALLED ~rr~ ~12~", &ctx, None));
    }

    #[test]
    fn evaluates_game_is_with_negation() {
        let ctx = test_context();
        assert!(evaluate_condition_clause("GAME_IS ~bg2ee eet~", &ctx, None));
        assert!(!evaluate_condition_clause("!GAME_IS ~bg2ee~", &ctx, None));
        assert!(evaluate_condition_clause("NOT GAME_IS ~bgee~", &ctx, None));
    }

    #[test]
    fn evaluates_and_or_with_parentheses() {
        let ctx = test_context();
        assert!(evaluate_condition_clause("(MOD_IS_INSTALLED ~rr~ ~12~ AND GAME_IS ~eet~) OR MOD_IS_INSTALLED ~foo~ ~1~", &ctx, None));
        assert!(!evaluate_condition_clause("MOD_IS_INSTALLED ~foo~ ~1~ OR (GAME_IS ~bgee~ AND MOD_IS_INSTALLED ~foo~ ~2~)", &ctx, None));
    }

    #[test]
    fn evaluates_action_if_wrappers() {
        let ctx = test_context();
        assert!(evaluate_condition_clause("if:ACTION_IF ! ( MOD_IS_INSTALLED ~foo~ ~1~ ) BEGIN", &ctx, None));
    }

    #[test]
    fn evaluates_file_exists_in_game() {
        let mut ctx = test_context();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("bio_prompt_eval_{stamp}"));
        let child = root.join("override").join("test.itm");
        fs::create_dir_all(child.parent().expect("parent")).expect("mkdir");
        fs::write(&child, b"ok").expect("write");
        ctx.game_dir = Some(root.to_string_lossy().to_string());
        assert!(evaluate_condition_clause("FILE_EXISTS_IN_GAME ~override/test.itm~", &ctx, None));
        assert!(!evaluate_condition_clause("!FILE_EXISTS_IN_GAME ~override/test.itm~", &ctx, None));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unresolved_variable_conditions_stay_unknown() {
        let ctx = test_context();
        assert_eq!(
            evaluate_condition_clause_state(
                "ACTION_IF (num_selected > 0) BEGIN",
                &ctx,
                None,
            ),
            EvalState::Unknown
        );
    }
}
