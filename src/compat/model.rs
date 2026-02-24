// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatIssueCode {
    ReqMissing,
    ForbidHit,
    GameMismatch,
    Conditional,
    OrderWarn,
    RuleHit,
}

impl CompatIssueCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReqMissing => "REQ_MISSING",
            Self::ForbidHit => "FORBID_HIT",
            Self::GameMismatch => "GAME_MISMATCH",
            Self::Conditional => "CONDITIONAL",
            Self::OrderWarn => "ORDER_WARN",
            Self::RuleHit => "RULE_HIT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IssueSource {
    Tp2 { file: String, line: usize },
    ExternalRule { file: String, line: usize },
}

impl IssueSource {
    pub fn description(&self) -> String {
        match self {
            Self::Tp2 { file, line } => format!("TP2: {file}:{line}"),
            Self::ExternalRule { file, line } => format!("{file}:{line}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatIssue {
    pub issue_id: String,
    pub code: CompatIssueCode,
    pub severity: Severity,
    pub source: IssueSource,
    pub affected_mod: String,
    pub affected_component: Option<u32>,
    pub related_mod: String,
    pub related_component: Option<u32>,
    pub reason: String,
    pub raw_evidence: Option<String>,
}

impl CompatIssue {
    pub fn new(
        code: CompatIssueCode,
        severity: Severity,
        source: IssueSource,
        affected_mod: String,
        affected_component: Option<u32>,
        related_mod: String,
        related_component: Option<u32>,
        reason: String,
        raw_evidence: Option<String>,
    ) -> Self {
        let issue_id = compute_issue_id(
            &code,
            &affected_mod,
            affected_component,
            &related_mod,
            related_component,
        );
        Self {
            issue_id,
            code,
            severity,
            source,
            affected_mod,
            affected_component,
            related_mod,
            related_component,
            reason,
            raw_evidence,
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.severity.is_blocking()
    }
}

fn compute_issue_id(
    code: &CompatIssueCode,
    affected_mod: &str,
    affected_component: Option<u32>,
    related_mod: &str,
    related_component: Option<u32>,
) -> String {
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    affected_mod.to_ascii_uppercase().hash(&mut hasher);
    affected_component.hash(&mut hasher);
    related_mod.to_ascii_uppercase().hash(&mut hasher);
    related_component.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tp2Rule {
    Require {
        target_mod: String,
        target_component: u32,
        raw_line: String,
        line: usize,
    },
    Forbid {
        target_mod: String,
        target_component: u32,
        raw_line: String,
        line: usize,
    },
    RequireGame {
        allowed_games: Vec<String>,
        raw_line: String,
        line: usize,
    },
    RequireGameOrInstalledAny {
        allowed_games: Vec<String>,
        targets: Vec<(String, Option<u32>)>,
        raw_line: String,
        line: usize,
    },
    RequireInstalledMod {
        target_mod: String,
        target_component: Option<u32>,
        raw_line: String,
        line: usize,
    },
    RequireInstalledAny {
        targets: Vec<(String, Option<u32>)>,
        raw_line: String,
        line: usize,
    },
    ForbidInstalledMod {
        target_mod: String,
        target_component: Option<u32>,
        raw_line: String,
        line: usize,
    },
    ConditionalOnInstalled {
        target_mod: String,
        target_component: Option<u32>,
        raw_line: String,
        line: usize,
    },
    ConditionalOnMissing {
        target_mod: String,
        target_component: Option<u32>,
        raw_line: String,
        line: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Tp2Metadata {
    pub tp_file: String,
    pub rules: Vec<(u32, Tp2Rule)>,
}

#[derive(Debug, Clone, Default)]
pub struct CompatValidationResult {
    pub issues: Vec<CompatIssue>,
}

impl CompatValidationResult {
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).count()
    }

    #[allow(dead_code)]
    pub fn has_blocking_errors(&self) -> bool {
        self.issues.iter().any(|i| i.is_blocking())
    }

    #[allow(dead_code)]
    pub fn errors(&self) -> impl Iterator<Item = &CompatIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    #[allow(dead_code)]
    pub fn warnings(&self) -> impl Iterator<Item = &CompatIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Warning)
    }
}
