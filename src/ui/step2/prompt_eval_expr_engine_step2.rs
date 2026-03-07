// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::ui::step2::prompt_eval_expr_tokens_step2::Token;
use crate::ui::step2::prompt_eval_vars_step2::{lookup_var, PromptVarContext, PromptVarValue};
use crate::ui::step2::state_step2::{normalize_tp2_stem, PromptEvalContext};

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

    pub(crate) fn parse_expression(&mut self) -> bool {
        self.parse_or()
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn parse_or(&mut self) -> bool {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            let rhs = self.parse_and();
            value = value || rhs;
        }
        value
    }

    fn parse_and(&mut self) -> bool {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            let rhs = self.parse_unary();
            value = value && rhs;
        }
        value
    }

    fn parse_unary(&mut self) -> bool {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return !self.parse_unary();
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> bool {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            return self.consume_if(&Token::RParen) && value;
        }

        let Some(lhs) = self.consume_scalar() else {
            return false;
        };
        if self.consume_if(&Token::Eq) {
            let Some(rhs) = self.consume_scalar() else {
                return false;
            };
            return lhs.eq_value(&rhs);
        }
        if self.consume_if_ident("STRING_EQUAL_CASE") || self.consume_if_ident("STRING_COMPARE_CASE") {
            let Some(rhs) = self.consume_scalar() else {
                return false;
            };
            return lhs.eq_value(&rhs);
        }
        if self.consume_if(&Token::Gt) {
            let Some(rhs) = self.consume_scalar() else {
                return false;
            };
            return lhs.as_i64().zip(rhs.as_i64()).is_some_and(|(a, b)| a > b);
        }
        if self.consume_if(&Token::Lt) {
            let Some(rhs) = self.consume_scalar() else {
                return false;
            };
            return lhs.as_i64().zip(rhs.as_i64()).is_some_and(|(a, b)| a < b);
        }
        lhs.truthy()
    }

    fn parse_call(&mut self, name: &str) -> bool {
        let opened = self.consume_if(&Token::LParen);
        let value = match name.to_ascii_uppercase().as_str() {
            "GAME_IS" => self.eval_game_is(opened),
            "MOD_IS_INSTALLED" => self.eval_mod_is_installed(opened),
            "FILE_EXISTS_IN_GAME" => self.eval_file_exists_in_game(opened),
            "TRUE" => true,
            "FALSE" => false,
            _ => false,
        };
        if opened {
            self.consume_if(&Token::RParen) && value
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
                    || matches!(upper.as_str(), "GAME_IS" | "MOD_IS_INSTALLED" | "FILE_EXISTS_IN_GAME")
                {
                    return Some(ScalarValue::Bool(self.parse_call(&name)));
                }
                if upper == "TRUE" {
                    return Some(ScalarValue::Bool(true));
                }
                if upper == "FALSE" {
                    return Some(ScalarValue::Bool(false));
                }
                if let Ok(value) = name.parse::<i64>() {
                    return Some(ScalarValue::Int(value));
                }
                if let Some(value) = lookup_var(self.prompt_vars, &name) {
                    return Some(ScalarValue::from_var(value));
                }
                Some(ScalarValue::Text(name))
            }
            Token::Atom(value) => {
                self.pos += 1;
                if let Some(var) = lookup_var(self.prompt_vars, &value) {
                    return Some(ScalarValue::from_var(var));
                }
                if let Ok(parsed) = value.parse::<i64>() {
                    return Some(ScalarValue::Int(parsed));
                }
                Some(ScalarValue::Text(value))
            }
            _ => None,
        }
    }

    fn eval_game_is(&mut self, opened: bool) -> bool {
        let values = self.collect_call_values(opened);
        if values.is_empty() {
            return false;
        }
        values
            .into_iter()
            .map(normalize_game_token)
            .any(|game| self.prompt_eval.active_games.contains(&game))
    }

    fn eval_mod_is_installed(&mut self, opened: bool) -> bool {
        let Some(mod_name) = self.consume_value() else {
            return false;
        };
        let Some(component_id) = self.consume_value() else {
            return false;
        };
        if opened {
            self.skip_extra_values_until_rparen();
        }
        self.prompt_eval
            .checked_components
            .contains(&(normalize_tp2_stem(&mod_name), component_id.trim().to_string()))
    }

    fn eval_file_exists_in_game(&mut self, opened: bool) -> bool {
        let Some(rel_path) = self.consume_value() else {
            return false;
        };
        if opened {
            self.skip_extra_values_until_rparen();
        }
        let Some(game_dir) = self.prompt_eval.game_dir.as_deref() else {
            return false;
        };
        let joined = Path::new(game_dir).join(rel_path.replace('\\', "/"));
        joined.exists()
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
    Bool(bool),
    Int(i64),
    Text(String),
}

impl ScalarValue {
    fn from_var(value: &PromptVarValue) -> Self {
        match value {
            PromptVarValue::Int(v) => Self::Int(*v),
            PromptVarValue::Text(v) => Self::Text(v.clone()),
        }
    }

    fn truthy(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            Self::Int(v) => *v != 0,
            Self::Text(v) => !v.trim().is_empty() && !v.eq_ignore_ascii_case("0"),
        }
    }

    fn eq_value(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            _ => self.as_text().eq_ignore_ascii_case(&rhs.as_text()),
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Bool(v) => Some(i64::from(*v)),
            Self::Int(v) => Some(*v),
            Self::Text(v) => v.parse::<i64>().ok(),
        }
    }

    fn as_text(&self) -> String {
        match self {
            Self::Bool(v) => {
                if *v {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            Self::Int(v) => v.to_string(),
            Self::Text(v) => v.trim().trim_matches('%').trim_matches('"').trim_matches('~').to_string(),
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
