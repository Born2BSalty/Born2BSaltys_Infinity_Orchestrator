// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::ui::step2::prompt_eval_expr_tokens_step2::Token;
use crate::ui::step2::prompt_eval_vars_step2::{lookup_var, PromptVarContext, PromptVarValue};
use crate::ui::step2::state_step2::{normalize_tp2_stem, PromptEvalContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvalState {
    True,
    False,
    Unknown,
}

impl EvalState {
    pub(crate) fn from_bool(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    pub(crate) fn is_not_false(self) -> bool {
        !matches!(self, Self::False)
    }

    fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

pub(crate) struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    prompt_eval: &'a PromptEvalContext,
    prompt_vars: Option<&'a PromptVarContext>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(
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

    pub(crate) fn is_at_end(&self) -> bool {
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
        if self.consume_if_ident("STRING_EQUAL_CASE") || self.consume_if_ident("STRING_COMPARE_CASE") {
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
                    || matches!(upper.as_str(), "GAME_IS" | "ENGINE_IS" | "MOD_IS_INSTALLED" | "FILE_EXISTS_IN_GAME")
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
        EvalState::from_bool(values
            .into_iter()
            .map(normalize_game_token)
            .any(|game| self.prompt_eval.active_games.contains(&game)))
    }

    fn eval_engine_is(&mut self, opened: bool) -> EvalState {
        let values = self.collect_call_values(opened);
        if values.is_empty() {
            return EvalState::Unknown;
        }
        EvalState::from_bool(values
            .into_iter()
            .map(normalize_game_token)
            .any(|game| self.prompt_eval.active_engines.contains(&game)))
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
        EvalState::from_bool(self.prompt_eval
            .checked_components
            .contains(&(normalize_tp2_stem(&mod_name), component_id.trim().to_string())))
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
        loop {
            match self.peek() {
                Some(Token::RParen) | None => break,
                Some(Token::Atom(_)) | Some(Token::Ident(_)) => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    fn consume_value(&mut self) -> Option<String> {
        match self.peek().cloned() {
            Some(Token::Atom(value)) | Some(Token::Ident(value)) => {
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

#[derive(Debug, Clone)]
enum ScalarValue {
    State(EvalState),
    Int(i64),
    Text(String),
    Unknown(String),
}

impl ScalarValue {
    fn from_var(value: &PromptVarValue) -> Self {
        match value {
            PromptVarValue::Int(v) => Self::Int(*v),
            PromptVarValue::Text(v) => Self::Text(v.clone()),
        }
    }

    fn truth_state(&self) -> EvalState {
        match self {
            Self::State(v) => *v,
            Self::Int(v) => EvalState::from_bool(*v != 0),
            Self::Text(v) => EvalState::from_bool(!v.trim().is_empty() && !v.eq_ignore_ascii_case("0")),
            Self::Unknown(_) => EvalState::Unknown,
        }
    }

    fn eq_value(&self, rhs: &Self) -> EvalState {
        match (self, rhs) {
            (Self::State(a), Self::State(b)) => EvalState::from_bool(a == b),
            (Self::Int(a), Self::Int(b)) => EvalState::from_bool(a == b),
            (Self::Unknown(_), _) | (_, Self::Unknown(_)) => EvalState::Unknown,
            _ => EvalState::from_bool(self.as_text().eq_ignore_ascii_case(&rhs.as_text())),
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::State(v) => Some(i64::from(matches!(v, EvalState::True))),
            Self::Int(v) => Some(*v),
            Self::Text(v) => v.parse::<i64>().ok(),
            Self::Unknown(_) => None,
        }
    }

    fn as_text(&self) -> String {
        match self {
            Self::State(v) => {
                if matches!(v, EvalState::True) {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            Self::Int(v) => v.to_string(),
            Self::Text(v) => v.trim().trim_matches('%').trim_matches('"').trim_matches('~').to_string(),
            Self::Unknown(v) => v.trim().trim_matches('%').trim_matches('"').trim_matches('~').to_string(),
        }
    }
}

fn normalize_game_token(input: String) -> String {
    let lower = input.trim().to_ascii_lowercase();
    if lower.contains("iwd") {
        return "iwdee".to_string();
    }
    if lower.contains("eet") {
        return "eet".to_string();
    }
    if lower.contains("bg2") {
        return "bg2ee".to_string();
    }
    if lower.contains("bg1") || lower.contains("bgee") {
        return "bgee".to_string();
    }
    lower
}
