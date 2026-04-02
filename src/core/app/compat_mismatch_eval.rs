// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::Path;

use crate::ui::state::Step1State;
use crate::ui::step2::prompt_eval_expr_tokens_step2::{tokenize, Token};

use super::compat_rule_runtime::normalize_mod_key;

pub(crate) fn build_mismatch_context(
    step1: &Step1State,
    tab: &str,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    if tab.eq_ignore_ascii_case("BGEE") {
        return build_bgee_context(step1, tab, checked_components);
    }

    build_bg2ee_context(step1.game_install.eq_ignore_ascii_case("EET"), checked_components)
}

fn build_bgee_context(
    step1: &Step1State,
    tab: &str,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    let mut active_games = HashSet::<String>::new();
    let mut active_includes = HashSet::<String>::new();
    let mut uncertain_includes = HashSet::<String>::new();

    active_games.insert("bgee".to_string());
    active_includes.insert("bg1".to_string());
    active_includes.insert("totsc".to_string());

    if let Some(game_dir) = game_dir_for_tab(step1, tab) {
        if detect_sod(game_dir) {
            active_includes.insert("sod".to_string());
        } else {
            uncertain_includes.insert("sod".to_string());
        }
    } else {
        uncertain_includes.insert("sod".to_string());
    }

    MismatchContext {
        active_games,
        active_includes,
        uncertain_includes,
        checked_components,
    }
}

fn build_bg2ee_context(
    include_eet: bool,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    let mut active_games = HashSet::<String>::new();
    let mut active_includes = HashSet::<String>::new();

    active_games.insert("bg2ee".to_string());
    active_includes.insert("bg2".to_string());
    active_includes.insert("soa".to_string());
    active_includes.insert("tob".to_string());

    if include_eet {
        active_games.insert("eet".to_string());
        active_includes.insert("bg1".to_string());
        active_includes.insert("totsc".to_string());
        active_includes.insert("sod".to_string());
    }

    MismatchContext {
        active_games,
        active_includes,
        uncertain_includes: HashSet::new(),
        checked_components,
    }
}

pub(crate) fn evaluate_requirement(input: &str, context: &MismatchContext) -> TriState {
    let tokens = tokenize(input);
    let mut parser = Parser::new(tokens, context);
    parser.parse_expression()
}

pub(crate) fn render_requirement_evidence(input: &str) -> Option<String> {
    let tokens = tokenize(input);
    let mut parser = EvidenceParser::new(tokens);
    parser.parse_expression()
}

#[derive(Debug, Default)]
pub(crate) struct MismatchContext {
    active_games: HashSet<String>,
    active_includes: HashSet<String>,
    uncertain_includes: HashSet<String>,
    checked_components: HashSet<(String, String)>,
}

impl MismatchContext {
    fn eval_game_is(&self, values: &[String]) -> TriState {
        if values.is_empty() {
            return TriState::Unknown;
        }
        TriState::from_bool(
            values
                .iter()
                .map(|value| normalize_game_token(value))
                .any(|value| self.active_games.contains(&value)),
        )
    }

    fn eval_game_includes(&self, values: &[String]) -> TriState {
        if values.is_empty() {
            return TriState::Unknown;
        }
        if values
            .iter()
            .map(|value| normalize_include_token(value))
            .any(|value| self.active_includes.contains(&value))
        {
            TriState::True
        } else if values
            .iter()
            .map(|value| normalize_include_token(value))
            .any(|value| self.uncertain_includes.contains(&value))
        {
            TriState::Unknown
        } else {
            TriState::False
        }
    }

    fn eval_mod_is_installed(&self, mod_name: &str, component_id: &str) -> TriState {
        let component_id = component_id.trim();
        if component_id.is_empty() {
            return TriState::Unknown;
        }
        TriState::from_bool(
            self.checked_components
                .contains(&(normalize_mod_key(mod_name), component_id.to_string())),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriState {
    True,
    False,
    Ignored,
    Unknown,
}

impl TriState {
    fn from_bool(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Ignored => Self::Ignored,
            Self::Unknown => Self::Unknown,
        }
    }
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    context: &'a MismatchContext,
}

struct EvidenceParser {
    tokens: Vec<Token>,
    pos: usize,
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
        self.parse_primary()
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
            "GAME_IS" => self.parse_game_call(true),
            "ENGINE_IS" => self.parse_game_call(true),
            "GAME_INCLUDES" => self.parse_game_call(false),
            "MOD_IS_INSTALLED" => self.parse_mod_is_installed(),
            "FILE_EXISTS_IN_GAME" => self.parse_ignored_file_call(),
            "FILE_EXISTS" => self.parse_ignored_file_call(),
            "TRUE" => TriState::True,
            "FALSE" => TriState::False,
            _ => TriState::Unknown,
        }
    }

    fn parse_game_call(&mut self, game_is: bool) -> TriState {
        let opened = self.consume_if(&Token::LParen);
        let values = self.consume_call_values();
        let value = if game_is {
            self.context.eval_game_is(&values)
        } else {
            self.context.eval_game_includes(&values)
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

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

impl EvidenceParser {
    fn new(tokens: Vec<Token>) -> Self {
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
        self.parse_primary()
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
            "FILE_EXISTS_IN_GAME" => self.consume_ignored_file_call(),
            "FILE_EXISTS" => self.consume_ignored_file_call(),
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
}

fn combine_rendered(op: &str, lhs: Option<String>, rhs: Option<String>) -> Option<String> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(format!("({lhs}) {op} ({rhs})")),
        (Some(lhs), None) => Some(lhs),
        (None, Some(rhs)) => Some(rhs),
        (None, None) => None,
    }
}

fn game_dir_for_tab<'a>(step1: &'a Step1State, tab: &str) -> Option<&'a str> {
    let value = if tab.eq_ignore_ascii_case("BGEE") {
        if step1.game_install.eq_ignore_ascii_case("EET") {
            if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                step1.eet_pre_dir.trim()
            } else {
                step1.eet_bgee_game_folder.trim()
            }
        } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
            step1.generate_directory.trim()
        } else {
            step1.bgee_game_folder.trim()
        }
    } else if step1.game_install.eq_ignore_ascii_case("EET") {
        if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
            step1.eet_new_dir.trim()
        } else {
            step1.eet_bg2ee_game_folder.trim()
        }
    } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
        step1.generate_directory.trim()
    } else {
        step1.bg2ee_game_folder.trim()
    };

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn detect_sod(game_dir: &str) -> bool {
    let root = Path::new(game_dir);
    [
        root.join("dlc").join("sod-dlc.zip"),
        root.join("DLC").join("sod-dlc.zip"),
        root.join("movies").join("sodcin01.wbm"),
        root.join("Movies").join("sodcin01.wbm"),
    ]
    .into_iter()
    .any(|path| path.exists())
}

fn normalize_game_token(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '~' | '\'' | '.' | ',' | ';'))
        .to_ascii_lowercase();
    match normalized.as_str() {
        "bg:ee" | "bg-ee" | "bg1ee" => "bgee".to_string(),
        "bgii:ee" | "bgii-ee" | "bg2:ee" => "bg2ee".to_string(),
        "iwd:ee" => "iwdee".to_string(),
        _ => normalized,
    }
}

fn normalize_include_token(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '~' | '\'' | '.' | ',' | ';'))
        .to_ascii_lowercase();
    match normalized.as_str() {
        "soa" => "bg2".to_string(),
        "totsc" => "bg1".to_string(),
        _ => normalized,
    }
}
