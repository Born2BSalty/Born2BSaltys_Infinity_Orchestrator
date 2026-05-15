// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use super::compat_rule_runtime::game_dir_for_tab as shared_game_dir_for_tab;
use crate::app::state::Step1State;
use crate::parser::{Token, tokenize};

#[derive(Debug, Clone, Default)]
pub(crate) struct PathRequirementContext {
    game_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathEvalOutcome {
    pub(crate) value: PathTriState,
    pub(crate) used_supported_predicate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathTriState {
    True,
    False,
    Ignored,
    Unknown,
}

impl PathTriState {
    const fn from_bool(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }

    const fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    const fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    const fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Ignored => Self::Ignored,
            Self::Unknown => Self::Unknown,
        }
    }
}

impl PathRequirementContext {
    pub(crate) fn for_tab(step1: &Step1State, tab: &str) -> Self {
        Self {
            game_dir: shared_game_dir_for_tab(step1, tab).map(ToString::to_string),
        }
    }

    fn eval_directory_exists(&self, value: &str) -> PathTriState {
        self.eval_path_exists(value, true)
    }

    fn eval_file_exists(&self, value: &str) -> PathTriState {
        self.eval_path_exists(value, false)
    }

    fn eval_path_exists(&self, value: &str, expect_directory: bool) -> PathTriState {
        let Some(path) = resolve_requirement_path(value, self.game_dir.as_deref()) else {
            return PathTriState::Unknown;
        };
        if expect_directory {
            PathTriState::from_bool(path.is_dir())
        } else {
            PathTriState::from_bool(path.is_file())
        }
    }
}

pub(crate) fn evaluate_path_requirement(
    input: &str,
    context: &PathRequirementContext,
) -> PathEvalOutcome {
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens, context);
    let value = parser.parse_expression();
    PathEvalOutcome {
        value,
        used_supported_predicate: parser.used_supported_predicate,
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    context: &'a PathRequirementContext,
    used_supported_predicate: bool,
}

impl<'a> Parser<'a> {
    const fn new(tokens: Vec<Token>, context: &'a PathRequirementContext) -> Self {
        Self {
            tokens,
            pos: 0,
            context,
            used_supported_predicate: false,
        }
    }

    fn parse_expression(&mut self) -> PathTriState {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PathTriState {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            value = value.or(self.parse_and());
        }
        value
    }

    fn parse_and(&mut self) -> PathTriState {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            value = value.and(self.parse_unary());
        }
        value
    }

    fn parse_unary(&mut self) -> PathTriState {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return self.parse_unary().not();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> PathTriState {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            return if self.consume_if(&Token::RParen) {
                value
            } else {
                PathTriState::Unknown
            };
        }

        let Some(Token::Ident(name)) = self.peek().cloned() else {
            return PathTriState::Unknown;
        };
        self.pos += 1;
        match name.to_ascii_uppercase().as_str() {
            "DIRECTORY_EXISTS" => self.parse_path_call(true),
            "FILE_EXISTS_IN_GAME" | "FILE_EXISTS" => self.parse_ignored_file_call(),
            "TRUE" => PathTriState::True,
            "FALSE" => PathTriState::False,
            _ => PathTriState::Unknown,
        }
    }

    fn parse_path_call(&mut self, expect_directory: bool) -> PathTriState {
        let opened = self.consume_if(&Token::LParen);
        let value = self.consume_call_value();
        let outcome = if let Some(value) = value {
            self.used_supported_predicate = true;
            if expect_directory {
                self.context.eval_directory_exists(&value)
            } else {
                self.context.eval_file_exists(&value)
            }
        } else {
            PathTriState::Unknown
        };
        if opened && !self.consume_if(&Token::RParen) {
            PathTriState::Unknown
        } else {
            outcome
        }
    }

    fn parse_ignored_file_call(&mut self) -> PathTriState {
        let opened = self.consume_if(&Token::LParen);
        let value = self.consume_call_value();
        if value.is_none() {
            return PathTriState::Unknown;
        }
        if opened && !self.consume_if(&Token::RParen) {
            PathTriState::Unknown
        } else {
            PathTriState::Ignored
        }
    }

    fn consume_call_value(&mut self) -> Option<String> {
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
}

fn resolve_requirement_path(value: &str, game_dir: Option<&str>) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('%') {
        return None;
    }

    let path = Path::new(trimmed);
    if path.is_absolute() || looks_like_windows_absolute(trimmed) {
        return Some(PathBuf::from(trimmed));
    }

    let game_dir = game_dir?.trim();
    if game_dir.is_empty() {
        return None;
    }
    Some(Path::new(game_dir).join(trimmed))
}

fn looks_like_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::PathTriState;
    use super::{PathRequirementContext, evaluate_path_requirement};

    #[test]
    fn skips_variable_paths() {
        let context = PathRequirementContext::default();
        let outcome = evaluate_path_requirement(r"DIRECTORY_EXISTS ~%MOD_FOLDER%/base~", &context);
        assert_eq!(outcome.value, PathTriState::Unknown);
        assert!(outcome.used_supported_predicate);
    }

    #[test]
    fn parses_boolean_directory_requirement() {
        let context = PathRequirementContext::default();
        let outcome =
            evaluate_path_requirement(r"!(DIRECTORY_EXISTS ~foo~) OR FILE_EXISTS ~bar~", &context);
        assert!(outcome.used_supported_predicate);
    }
}
