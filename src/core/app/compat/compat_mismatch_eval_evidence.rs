// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::parser::{Token, tokenize};

pub(crate) fn render_requirement_evidence(input: &str) -> Option<String> {
    let tokens = tokenize(input);
    let mut parser = EvidenceParser::new(tokens);
    let value = parser.parse_expression();
    if parser.is_at_end() { value } else { None }
}

struct EvidenceParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl EvidenceParser {
    const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_expression(&mut self) -> Option<String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Option<String> {
        let mut value = self.parse_and();
        while self.consume_if(&Token::Or) {
            let rhs = self.parse_and();
            value = combine_rendered("OR", value, rhs);
        }
        value
    }

    fn parse_and(&mut self) -> Option<String> {
        let mut value = self.parse_unary();
        while self.consume_if(&Token::And) {
            let rhs = self.parse_unary();
            value = combine_rendered("AND", value, rhs);
        }
        value
    }

    fn parse_unary(&mut self) -> Option<String> {
        if self.consume_if(&Token::Bang) || self.consume_if(&Token::Not) {
            return self.parse_unary().map(|value| format!("NOT({value})"));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Option<String> {
        let lhs = self.parse_primary()?;
        let Some(op) = self.consume_comparison_op() else {
            return Some(lhs);
        };
        let rhs = self.consume_comparison_value()?;
        Some(format!("{lhs} {} {rhs}", render_comparison_op(&op)))
    }

    fn parse_primary(&mut self) -> Option<String> {
        if self.consume_if(&Token::LParen) {
            let value = self.parse_expression();
            if !self.consume_if(&Token::RParen) {
                return None;
            }
            return value.map(|value| format!("({value})"));
        }

        let Some(Token::Ident(name)) = self.peek().cloned() else {
            return None;
        };
        self.pos += 1;
        let upper = name.to_ascii_uppercase();
        match upper.as_str() {
            "GAME_IS" => self.render_multi_value_call("GAME_IS"),
            "ENGINE_IS" => self.render_multi_value_call("ENGINE_IS"),
            "GAME_INCLUDES" => self.render_multi_value_call("GAME_INCLUDES"),
            "MOD_IS_INSTALLED" => self.render_mod_is_installed(),
            "FILE_EXISTS_IN_GAME" | "FILE_EXISTS" => self.consume_ignored_file_call(),
            "TRUE" => Some("TRUE".to_string()),
            "FALSE" => Some("FALSE".to_string()),
            _ => None,
        }
    }

    fn render_multi_value_call(&mut self, name: &str) -> Option<String> {
        let opened = self.consume_if(&Token::LParen);
        let values = self.consume_call_values();
        if values.is_empty() {
            return None;
        }
        if opened && !self.consume_if(&Token::RParen) {
            return None;
        }
        Some(format!("{name} ~{}~", values.join(" ")))
    }

    fn render_mod_is_installed(&mut self) -> Option<String> {
        let opened = self.consume_if(&Token::LParen);
        let mod_name = self.consume_value()?;
        let component_id = self.consume_value()?;
        if opened && !self.consume_if(&Token::RParen) {
            return None;
        }
        Some(format!(
            "MOD_IS_INSTALLED ~{}~ ~{}~",
            mod_name.trim(),
            component_id.trim()
        ))
    }

    fn consume_ignored_file_call(&mut self) -> Option<String> {
        let opened = self.consume_if(&Token::LParen);
        let _value = self.consume_value()?;
        if opened {
            let _ = self.consume_if(&Token::RParen);
        }
        None
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

    fn consume_comparison_value(&mut self) -> Option<String> {
        self.consume_value()
    }

    const fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

const fn render_comparison_op(op: &Token) -> &'static str {
    match op {
        Token::Eq => "=",
        Token::Gt => ">",
        Token::Lt => "<",
        _ => "?",
    }
}

fn combine_rendered(op: &str, lhs: Option<String>, rhs: Option<String>) -> Option<String> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(format!("({lhs}) {op} ({rhs})")),
        (Some(lhs), None) => Some(lhs),
        (None, Some(rhs)) => Some(rhs),
        (None, None) => None,
    }
}
