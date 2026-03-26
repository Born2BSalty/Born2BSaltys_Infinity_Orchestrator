// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatIssueCode {
    ReqMissing,
    GameMismatch,
    Conditional,
    Included,
    OrderBlock,
    RuleHit,
}

impl CompatIssueCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReqMissing => "REQ_MISSING",
            Self::GameMismatch => "GAME_MISMATCH",
            Self::Conditional => "CONDITIONAL",
            Self::Included => "INCLUDED",
            Self::OrderBlock => "ORDER_BLOCK",
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
    pub component_block: Option<String>,
}

pub struct CompatIssueInit {
    pub code: CompatIssueCode,
    pub severity: Severity,
    pub source: IssueSource,
    pub affected_mod: String,
    pub affected_component: Option<u32>,
    pub related_mod: String,
    pub related_component: Option<u32>,
    pub reason: String,
    pub raw_evidence: Option<String>,
    pub component_block: Option<String>,
}

impl CompatIssue {
    pub fn new(init: CompatIssueInit) -> Self {
        let issue_id = compute_issue_id(
            &init.code,
            &init.affected_mod,
            init.affected_component,
            &init.related_mod,
            init.related_component,
        );
        Self {
            issue_id,
            code: init.code,
            severity: init.severity,
            source: init.source,
            affected_mod: init.affected_mod,
            affected_component: init.affected_component,
            related_mod: init.related_mod,
            related_component: init.related_component,
            reason: init.reason,
            raw_evidence: init.raw_evidence,
            component_block: init.component_block,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRequirementKind {
    Directory,
    File,
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
    ForbidGame {
        blocked_games: Vec<String>,
        raw_line: String,
        line: usize,
    },
    RequireGameIncludes {
        required_games: Vec<String>,
        raw_line: String,
        line: usize,
    },
    Deprecated {
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
    RequirePath {
        kind: PathRequirementKind,
        path: String,
        must_exist: bool,
        message: Option<String>,
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
    pub setup_tra: std::collections::HashMap<String, String>,
    pub component_blocks: std::collections::HashMap<u32, String>,
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
}
