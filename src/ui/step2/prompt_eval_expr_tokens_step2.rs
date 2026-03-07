// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    LParen,
    RParen,
    Bang,
    Eq,
    Gt,
    Lt,
    And,
    Or,
    Not,
    Ident(String),
    Atom(String),
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = Vec::<Token>::new();
    let mut idx = 0usize;

    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() || ch == ',' {
            idx += 1;
            continue;
        }
        match ch {
            '(' => {
                out.push(Token::LParen);
                idx += 1;
            }
            ')' => {
                out.push(Token::RParen);
                idx += 1;
            }
            '!' => {
                out.push(Token::Bang);
                idx += 1;
            }
            '=' => {
                out.push(Token::Eq);
                idx += 1;
            }
            '>' => {
                out.push(Token::Gt);
                idx += 1;
            }
            '<' => {
                out.push(Token::Lt);
                idx += 1;
            }
            '|' if chars.get(idx + 1) == Some(&'|') => {
                out.push(Token::Or);
                idx += 2;
            }
            '&' if chars.get(idx + 1) == Some(&'&') => {
                out.push(Token::And);
                idx += 2;
            }
            '"' | '~' => {
                let quote = ch;
                idx += 1;
                let start = idx;
                while idx < chars.len() && chars[idx] != quote {
                    idx += 1;
                }
                out.push(Token::Atom(chars[start..idx].iter().collect::<String>()));
                if idx < chars.len() {
                    idx += 1;
                }
            }
            _ if is_ident_char(ch) => {
                let start = idx;
                idx += 1;
                while idx < chars.len() && is_ident_char(chars[idx]) {
                    idx += 1;
                }
                let ident = chars[start..idx].iter().collect::<String>();
                let upper = ident.to_ascii_uppercase();
                match upper.as_str() {
                    "AND" => out.push(Token::And),
                    "OR" => out.push(Token::Or),
                    "NOT" => out.push(Token::Not),
                    _ => out.push(Token::Ident(ident)),
                }
            }
            _ => {
                idx += 1;
            }
        }
    }

    out
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '\\' | '%' | '-' | '#')
}
