// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::prompt_eval_expr_tokens::{Token, tokenize};

use super::context::{MismatchContext, TriState};
use super::parser::{ComparisonValue, compare_tristate, parse_comparison_value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequirementFailureClass {
    Mismatch,
    Conditional,
}

pub(crate) fn classify_failed_requirement(
    input: &str,
    context: &MismatchContext,
) -> RequirementFailureClass {
    let tokens = tokenize(input);
    let mut parser = ClassifyingParser::new(tokens, context);
    let result = parser.parse_expression();
    if !parser.is_at_end() {
        return RequirementFailureClass::Conditional;
    }
    if result.value == TriState::False && result.false_game {
        RequirementFailureClass::Mismatch
    } else {
        RequirementFailureClass::Conditional
    }
}

struct ClassifyingParser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    context: &'a MismatchContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamePredicate {
    Game,
    Engine,
    Includes,
}

impl GamePredicate {
    fn is_mismatch(self) -> bool {
        matches!(self, Self::Game | Self::Engine)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClassifiedTri {
    value: TriState,
    true_game: bool,
    true_other: bool,
    false_game: bool,
    false_other: bool,
}

impl ClassifiedTri {
    fn neutral(value: TriState) -> Self {
        Self {
            value,
            true_game: false,
            true_other: false,
            false_game: false,
            false_other: false,
        }
    }

    fn game(value: TriState) -> Self {
        match value {
            TriState::True => Self {
                value,
                true_game: true,
                ..Self::neutral(value)
            },
            TriState::False => Self {
                value,
                false_game: true,
                ..Self::neutral(value)
            },
            _ => Self::neutral(value),
        }
    }

    fn other(value: TriState) -> Self {
        match value {
            TriState::True => Self {
                value,
                true_other: true,
                ..Self::neutral(value)
            },
            TriState::False => Self {
                value,
                false_other: true,
                ..Self::neutral(value)
            },
            _ => Self::neutral(value),
        }
    }

    fn and(self, rhs: Self) -> Self {
        let value = self.value.and(rhs.value);
        let mut out = Self::neutral(value);
        match value {
            TriState::True => {
                if self.value == TriState::True {
                    out.true_game |= self.true_game;
                    out.true_other |= self.true_other;
                }
                if rhs.value == TriState::True {
                    out.true_game |= rhs.true_game;
                    out.true_other |= rhs.true_other;
                }
            }
            TriState::False => {
                if self.value == TriState::False {
                    out.false_game |= self.false_game;
                    out.false_other |= self.false_other;
                }
                if rhs.value == TriState::False {
                    out.false_game |= rhs.false_game;
                    out.false_other |= rhs.false_other;
                }
            }
            _ => {}
        }
        out
    }

    fn or(self, rhs: Self) -> Self {
        let value = self.value.or(rhs.value);
        let mut out = Self::neutral(value);
        match value {
            TriState::True => {
                if self.value == TriState::True {
                    out.true_game |= self.true_game;
                    out.true_other |= self.true_other;
                }
                if rhs.value == TriState::True {
                    out.true_game |= rhs.true_game;
                    out.true_other |= rhs.true_other;
                }
            }
            TriState::False => {
                if self.value == TriState::False {
                    out.false_game |= self.false_game;
                    out.false_other |= self.false_other;
                }
                if rhs.value == TriState::False {
                    out.false_game |= rhs.false_game;
                    out.false_other |= rhs.false_other;
                }
            }
            _ => {}
        }
        out
    }

    fn not(self) -> Self {
        Self {
            value: self.value.not(),
            true_game: self.false_game,
            true_other: self.false_other,
            false_game: self.true_game,
            false_other: self.true_other,
        }
    }
}

impl<'a> ClassifyingParser<'a> {
    fn new(tokens: Vec<Token>, context: &'a MismatchContext) -> Self {
        Self {
            tokens,
            pos: 0,
            context,
        }
    }

    fn parse_expression(&mut self) -> ClassifiedTri {
        self.parse_or()
    }

    fn parse_or(&mut self) -> ClassifiedTri {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            value = value.or(self.parse_and());
        }
        value
    }

    fn parse_and(&mut self) -> ClassifiedTri {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            value = value.and(self.parse_unary());
        }
        value
    }

    fn parse_unary(&mut self) -> ClassifiedTri {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return self.parse_unary().not();
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> ClassifiedTri {
        let lhs = self.parse_primary();
        let Some(op) = self.consume_comparison_op() else {
            return lhs;
        };
        let Some(rhs) = self.consume_comparison_value() else {
            return ClassifiedTri::neutral(TriState::Unknown);
        };
        classify_compared(lhs, op, rhs)
    }

    fn parse_primary(&mut self) -> ClassifiedTri {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            return if self.consume_if(&Token::RParen) {
                value
            } else {
                ClassifiedTri::neutral(TriState::Unknown)
            };
        }

        let Some(Token::Ident(name)) = self.peek().cloned() else {
            return ClassifiedTri::neutral(TriState::Unknown);
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
            "TRUE" => ClassifiedTri::neutral(TriState::True),
            "FALSE" => ClassifiedTri::neutral(TriState::False),
            _ => ClassifiedTri::neutral(TriState::Unknown),
        }
    }

    fn parse_game_call(&mut self, predicate: GamePredicate) -> ClassifiedTri {
        let opened = self.consume_if(&Token::LParen);
        let values = self.consume_call_values();
        let value = match predicate {
            GamePredicate::Game => self.context.eval_game_is(&values),
            GamePredicate::Engine => self.context.eval_engine_is(&values),
            GamePredicate::Includes => self.context.eval_game_includes(&values),
        };
        let value = if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            value
        };
        if predicate.is_mismatch() {
            ClassifiedTri::game(value)
        } else {
            ClassifiedTri::other(value)
        }
    }

    fn parse_mod_is_installed(&mut self) -> ClassifiedTri {
        let opened = self.consume_if(&Token::LParen);
        let Some(mod_name) = self.consume_value() else {
            return ClassifiedTri::neutral(TriState::Unknown);
        };
        let Some(component_id) = self.consume_value() else {
            return ClassifiedTri::neutral(TriState::Unknown);
        };
        let value = self.context.eval_mod_is_installed(&mod_name, &component_id);
        let value = if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            value
        };
        ClassifiedTri::other(value)
    }

    fn parse_ignored_file_call(&mut self) -> ClassifiedTri {
        let opened = self.consume_if(&Token::LParen);
        let Some(_value) = self.consume_value() else {
            return ClassifiedTri::neutral(TriState::Unknown);
        };
        let value = if opened && !self.consume_if(&Token::RParen) {
            TriState::Unknown
        } else {
            TriState::Ignored
        };
        ClassifiedTri::other(value)
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

fn classify_compared(lhs: ClassifiedTri, op: Token, rhs: ComparisonValue) -> ClassifiedTri {
    let value = compare_tristate(lhs.value, op, rhs);
    let mut out = ClassifiedTri::neutral(value);
    let lhs_has_game = lhs.true_game || lhs.false_game;
    let lhs_has_other = lhs.true_other || lhs.false_other;
    match value {
        TriState::True => {
            out.true_game = lhs_has_game;
            out.true_other = lhs_has_other;
        }
        TriState::False => {
            out.false_game = lhs_has_game;
            out.false_other = lhs_has_other;
        }
        TriState::Ignored | TriState::Unknown => {}
    }
    out
}
