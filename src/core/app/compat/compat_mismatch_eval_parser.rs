// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::prompt_eval_expr_tokens::{Token, tokenize};

use super::context::{MismatchContext, TriState};

pub(crate) fn evaluate_requirement(input: &str, context: &MismatchContext) -> TriState {
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens, context);
    let value = parser.parse_expression();
    if parser.is_at_end() {
        value
    } else {
        TriState::Unknown
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    context: &'a MismatchContext,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, context: &'a MismatchContext) -> Self {
        Self {
            tokens,
            pos: 0,
            context,
        }
    }

    fn parse_expression(&mut self) -> TriState {
        self.parse_or()
    }

    fn parse_or(&mut self) -> TriState {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            value = value.or(self.parse_and());
        }
        value
    }

    fn parse_and(&mut self) -> TriState {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            value = value.and(self.parse_unary());
        }
        value
    }

    fn parse_unary(&mut self) -> TriState {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return self.parse_unary().not();
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> TriState {
        let lhs = self.parse_primary();
        let Some(op) = self.consume_comparison_op() else {
            return lhs;
        };
        let Some(rhs) = self.consume_comparison_value() else {
            return TriState::Unknown;
        };
        compare_tristate(lhs, op, rhs)
    }

    fn parse_primary(&mut self) -> TriState {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            return if self.consume_if(&Token::RParen) {
                value
            } else {
                TriState::Unknown
            };
        }

        let Some(Token::Ident(name)) = self.peek().cloned() else {
            return TriState::Unknown;
        };
        self.pos += 1;
        let upper = name.to_ascii_uppercase();
        match upper.as_str() {
            "GAME_IS" => self.parse_game_call(GamePredicate::Game),
            "ENGINE_IS" => self.parse_game_call(GamePredicate::Engine),
            "GAME_INCLUDES" => self.parse_game_call(GamePredicate::Includes),
            "MOD_IS_INSTALLED" => self.parse_mod_is_installed(),
            "FILE_EXISTS_IN_GAME" => self.parse_ignored_file_call(),
            "FILE_EXISTS" => self.parse_ignored_file_call(),
            "TRUE" => TriState::True,
            "FALSE" => TriState::False,
            _ => TriState::Unknown,
        }
    }

    fn parse_game_call(&mut self, predicate: GamePredicate) -> TriState {
        let opened = self.consume_if(&Token::LParen);
        let values = self.consume_call_values();
        let value = match predicate {
            GamePredicate::Game => self.context.eval_game_is(&values),
            GamePredicate::Engine => self.context.eval_engine_is(&values),
            GamePredicate::Includes => self.context.eval_game_includes(&values),
        };
        if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            value
        }
    }

    fn parse_mod_is_installed(&mut self) -> TriState {
        let opened = self.consume_if(&Token::LParen);
        let Some(mod_name) = self.consume_value() else {
            return TriState::Unknown;
        };
        let Some(component_id) = self.consume_value() else {
            return TriState::Unknown;
        };
        let value = self.context.eval_mod_is_installed(&mod_name, &component_id);
        if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            value
        }
    }

    fn parse_ignored_file_call(&mut self) -> TriState {
        let opened = self.consume_if(&Token::LParen);
        let Some(_value) = self.consume_value() else {
            return TriState::Unknown;
        };
        if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            TriState::Ignored
        }
    }

    fn consume_call_values(&mut self) -> Vec<String> {
        match self.peek().cloned() {
            Some(Token::Atom(value)) => {
                self.pos += 1;
                value
                    .split_whitespace()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect()
            }
            Some(Token::Ident(_)) => {
                let mut values = Vec::<String>::new();
                while let Some(Token::Ident(value)) = self.peek().cloned() {
                    if value.chars().all(|ch| ch.is_ascii_digit()) {
                        break;
                    }
                    self.pos += 1;
                    values.push(value);
                }
                values
            }
            _ => Vec::new(),
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

    fn consume_comparison_op(&mut self) -> Option<Token> {
        match self.peek().cloned() {
            Some(Token::Eq | Token::Gt | Token::Lt) => {
                let token = self.peek().cloned()?;
                self.pos += 1;
                Some(token)
            }
            _ => None,
        }
    }

    fn consume_comparison_value(&mut self) -> Option<ComparisonValue> {
        let value = self.consume_value()?;
        parse_comparison_value(&value)
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamePredicate {
    Game,
    Engine,
    Includes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ComparisonValue {
    Int(i64),
}

pub(super) fn parse_comparison_value(value: &str) -> Option<ComparisonValue> {
    let trimmed = value.trim();
    if let Ok(int) = trimmed.parse::<i64>() {
        Some(ComparisonValue::Int(int))
    } else if trimmed.eq_ignore_ascii_case("TRUE") {
        Some(ComparisonValue::Int(1))
    } else if trimmed.eq_ignore_ascii_case("FALSE") {
        Some(ComparisonValue::Int(0))
    } else {
        None
    }
}

pub(super) fn compare_tristate(lhs: TriState, op: Token, rhs: ComparisonValue) -> TriState {
    let Some(lhs) = tristate_to_int(lhs) else {
        return TriState::Unknown;
    };
    let rhs = comparison_value_to_int(rhs);
    let value = match op {
        Token::Eq => lhs == rhs,
        Token::Gt => lhs > rhs,
        Token::Lt => lhs < rhs,
        _ => return TriState::Unknown,
    };
    TriState::from_bool(value)
}

fn tristate_to_int(value: TriState) -> Option<i64> {
    match value {
        TriState::True => Some(1),
        TriState::False => Some(0),
        TriState::Ignored | TriState::Unknown => None,
    }
}

fn comparison_value_to_int(value: ComparisonValue) -> i64 {
    match value {
        ComparisonValue::Int(value) => value,
    }
}
