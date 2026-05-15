// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use super::super::Token;

use super::values::{EvalState, ScalarValue, normalize_game_token};
use super::{PromptEvalContext, PromptVarContext, lookup_var, normalize_tp2_stem};

pub(crate) struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    prompt_eval: &'a PromptEvalContext,
    prompt_vars: Option<&'a PromptVarContext>,
}

impl<'a> Parser<'a> {
    pub(crate) const fn new(
        tokens: Vec<Token>,
        prompt_eval: &'a PromptEvalContext,
        prompt_vars: Option<&'a PromptVarContext>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            prompt_eval,
            prompt_vars,
        }
    }

    pub(crate) fn parse_expression(&mut self) -> EvalState {
        self.parse_or()
    }

    pub(crate) const fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn parse_or(&mut self) -> EvalState {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            let rhs = self.parse_and();
            value = value.or(rhs);
        }
        value
    }

    fn parse_and(&mut self) -> EvalState {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            let rhs = self.parse_unary();
            value = value.and(rhs);
        }
        value
    }

    fn parse_unary(&mut self) -> EvalState {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return self.parse_unary().not();
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> EvalState {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            return if self.consume_if(&Token::RParen) {
                value
            } else {
                EvalState::Unknown
            };
        }

        let Some(lhs) = self.consume_scalar() else {
            return EvalState::False;
        };
        if self.consume_if(&Token::Eq) {
            let Some(rhs) = self.consume_scalar() else {
                return EvalState::Unknown;
            };
            return lhs.eq_value(&rhs);
        }
        if self.consume_if_ident("STRING_EQUAL_CASE")
            || self.consume_if_ident("STRING_COMPARE_CASE")
        {
            let Some(rhs) = self.consume_scalar() else {
                return EvalState::Unknown;
            };
            return lhs.eq_value(&rhs);
        }
        if self.consume_if(&Token::Gt) {
            let Some(rhs) = self.consume_scalar() else {
                return EvalState::Unknown;
            };
            return match (lhs.as_i64(), rhs.as_i64()) {
                (Some(a), Some(b)) => EvalState::from_bool(a > b),
                _ => EvalState::Unknown,
            };
        }
        if self.consume_if(&Token::Lt) {
            let Some(rhs) = self.consume_scalar() else {
                return EvalState::Unknown;
            };
            return match (lhs.as_i64(), rhs.as_i64()) {
                (Some(a), Some(b)) => EvalState::from_bool(a < b),
                _ => EvalState::Unknown,
            };
        }
        lhs.truth_state()
    }

    fn parse_call(&mut self, name: &str) -> EvalState {
        let opened = self.consume_if(&Token::LParen);
        let value = match name.to_ascii_uppercase().as_str() {
            "GAME_IS" => self.eval_game_is(opened),
            "ENGINE_IS" => self.eval_engine_is(opened),
            "MOD_IS_INSTALLED" => self.eval_mod_is_installed(opened),
            "FILE_EXISTS_IN_GAME" => self.eval_file_exists_in_game(opened),
            "TRUE" => EvalState::True,
            "FALSE" => EvalState::False,
            _ => EvalState::Unknown,
        };
        if opened {
            if self.consume_if(&Token::RParen) {
                value
            } else {
                EvalState::Unknown
            }
        } else {
            value
        }
    }

    fn consume_scalar(&mut self) -> Option<ScalarValue> {
        match self.peek().cloned()? {
            Token::Ident(name) => {
                self.pos += 1;
                let upper = name.to_ascii_uppercase();
                if matches!(self.peek(), Some(Token::LParen))
                    || matches!(
                        upper.as_str(),
                        "GAME_IS" | "ENGINE_IS" | "MOD_IS_INSTALLED" | "FILE_EXISTS_IN_GAME"
                    )
                {
                    return Some(ScalarValue::State(self.parse_call(&name)));
                }
                if upper == "TRUE" {
                    return Some(ScalarValue::State(EvalState::True));
                }
                if upper == "FALSE" {
                    return Some(ScalarValue::State(EvalState::False));
                }
                if let Ok(value) = name.parse::<i64>() {
                    return Some(ScalarValue::Int(value));
                }
                if let Some(value) = lookup_var(self.prompt_vars, &name) {
                    return Some(ScalarValue::from_var(value));
                }
                Some(ScalarValue::Unknown(name))
            }
            Token::Atom(value) => {
                self.pos += 1;
                if let Some(var) = lookup_var(self.prompt_vars, &value) {
                    return Some(ScalarValue::from_var(var));
                }
                if let Ok(parsed) = value.parse::<i64>() {
                    return Some(ScalarValue::Int(parsed));
                }
                if value.contains('%') {
                    Some(ScalarValue::Unknown(value))
                } else {
                    Some(ScalarValue::Text(value))
                }
            }
            _ => None,
        }
    }

    fn eval_game_is(&mut self, opened: bool) -> EvalState {
        let values = self.collect_call_values(opened);
        if values.is_empty() {
            return EvalState::Unknown;
        }
        EvalState::from_bool(
            values
                .into_iter()
                .map(|value| normalize_game_token(&value))
                .any(|game| self.prompt_eval.active_games.contains(&game)),
        )
    }

    fn eval_engine_is(&mut self, opened: bool) -> EvalState {
        let values = self.collect_call_values(opened);
        if values.is_empty() {
            return EvalState::Unknown;
        }
        EvalState::from_bool(
            values
                .into_iter()
                .map(|value| normalize_game_token(&value))
                .any(|game| self.prompt_eval.active_engines.contains(&game)),
        )
    }

    fn eval_mod_is_installed(&mut self, opened: bool) -> EvalState {
        let Some(mod_name) = self.consume_value() else {
            return EvalState::Unknown;
        };
        let Some(component_id) = self.consume_value() else {
            return EvalState::Unknown;
        };
        if opened {
            self.skip_extra_values_until_rparen();
        }
        EvalState::from_bool(self.prompt_eval.checked_components.contains(&(
            normalize_tp2_stem(&mod_name),
            component_id.trim().to_string(),
        )))
    }

    fn eval_file_exists_in_game(&mut self, opened: bool) -> EvalState {
        let Some(rel_path) = self.consume_value() else {
            return EvalState::Unknown;
        };
        if opened {
            self.skip_extra_values_until_rparen();
        }
        let Some(game_dir) = self.prompt_eval.game_dir.as_deref() else {
            return EvalState::Unknown;
        };
        let joined = Path::new(game_dir).join(rel_path.replace('\\', "/"));
        EvalState::from_bool(joined.exists())
    }

    fn collect_call_values(&mut self, opened: bool) -> Vec<String> {
        let mut values = Vec::<String>::new();
        while let Some(value) = self.consume_value() {
            if value.contains(char::is_whitespace) {
                for part in value.split_whitespace() {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        values.push(trimmed.to_string());
                    }
                }
            } else {
                values.push(value);
            }
        }
        if opened {
            self.skip_extra_values_until_rparen();
        }
        values
    }

    fn skip_extra_values_until_rparen(&mut self) {
        while matches!(self.peek(), Some(Token::Atom(_) | Token::Ident(_))) {
            self.pos += 1;
        }
    }

    fn consume_value(&mut self) -> Option<String> {
        match self.peek().cloned() {
            Some(Token::Atom(value) | Token::Ident(value)) => {
                self.pos += 1;
                Some(value)
            }
            _ => None,
        }
    }

    fn consume_if(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume_if_ident(&mut self, expected: &str) -> bool {
        let Some(Token::Ident(value)) = self.peek() else {
            return false;
        };
        if value.eq_ignore_ascii_case(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}
