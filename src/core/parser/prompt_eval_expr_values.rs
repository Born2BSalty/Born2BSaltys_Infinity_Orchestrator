// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::PromptVarValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvalState {
    True,
    False,
    Unknown,
}

impl EvalState {
    pub(crate) fn from_bool(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }

    pub(crate) fn is_not_false(self) -> bool {
        !matches!(self, Self::False)
    }

    pub(crate) fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum ScalarValue {
    State(EvalState),
    Int(i64),
    Text(String),
    Unknown(String),
}

impl ScalarValue {
    pub(super) fn from_var(value: &PromptVarValue) -> Self {
        match value {
            PromptVarValue::Int(v) => Self::Int(*v),
            PromptVarValue::Text(v) => Self::Text(v.clone()),
        }
    }

    pub(super) fn truth_state(&self) -> EvalState {
        match self {
            Self::State(v) => *v,
            Self::Int(v) => EvalState::from_bool(*v != 0),
            Self::Text(v) => {
                EvalState::from_bool(!v.trim().is_empty() && !v.eq_ignore_ascii_case("0"))
            }
            Self::Unknown(_) => EvalState::Unknown,
        }
    }

    pub(super) fn eq_value(&self, rhs: &Self) -> EvalState {
        match (self, rhs) {
            (Self::State(a), Self::State(b)) => EvalState::from_bool(a == b),
            (Self::Int(a), Self::Int(b)) => EvalState::from_bool(a == b),
            (Self::Unknown(_), _) | (_, Self::Unknown(_)) => EvalState::Unknown,
            _ => EvalState::from_bool(self.as_text().eq_ignore_ascii_case(&rhs.as_text())),
        }
    }

    pub(super) fn as_i64(&self) -> Option<i64> {
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
            Self::Text(v) => v
                .trim()
                .trim_matches('%')
                .trim_matches('"')
                .trim_matches('~')
                .to_string(),
            Self::Unknown(v) => v
                .trim()
                .trim_matches('%')
                .trim_matches('"')
                .trim_matches('~')
                .to_string(),
        }
    }
}

pub(super) fn normalize_game_token(input: String) -> String {
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
