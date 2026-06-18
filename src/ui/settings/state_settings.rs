// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::time::Instant;

use crate::ui::settings::widgets::tab_strip::TabLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Paths,
    Tools,
    Accounts,
    Advanced,
}

impl TabLabel for SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Paths => "Paths",
            Self::Tools => "Tools",
            Self::Accounts => "Accounts",
            Self::Advanced => "Advanced",
        }
    }
}

impl SettingsTab {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        const ALL: [SettingsTab; 5] = [
            SettingsTab::General,
            SettingsTab::Paths,
            SettingsTab::Tools,
            SettingsTab::Accounts,
            SettingsTab::Advanced,
        ];
        &ALL
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStatus {
    Empty,
    Ok { detail: Option<String> },
    Warning { reason: String },
    Error { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatusTone {
    Neutral,
    Success,
    Warning,
    Error,
}

impl PathStatus {
    #[must_use]
    pub fn hint_text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Ok { detail: Some(d) } => format!("ok \u{00B7} {d}"),
            Self::Ok { detail: None } => "ok".to_string(),
            Self::Warning { reason } => format!("! {reason}"),
            Self::Error { reason } => format!("\u{00D7} {reason}"),
        }
    }

    #[must_use]
    pub const fn tone(&self) -> PathStatusTone {
        match self {
            Self::Empty => PathStatusTone::Neutral,
            Self::Ok { .. } => PathStatusTone::Success,
            Self::Warning { .. } => PathStatusTone::Warning,
            Self::Error { .. } => PathStatusTone::Error,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub fields: HashMap<&'static str, PathStatus>,
    pub overall_ok: bool,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsScreenState {
    pub active_tab: SettingsTab,
    pub name_row_editing: bool,
    pub name_row_buffer: String,
    pub oauth_popup_open: bool,
    pub validate_now_in_flight: bool,
    pub path_edit_debounce: HashMap<&'static str, Instant>,
    pub path_validation_results: ValidationReport,
}
