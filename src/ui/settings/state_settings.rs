// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `SettingsScreenState` — per-screen UI state for the Settings destination.
//
// Per Phase 4 P4.T1: this state lives on `OrchestratorApp` (see
// `orchestrator_app::settings_screen_state` field added in this phase). The
// state persists across screen visits within a session but is **not**
// written to disk — `RedesignSettingsStore` + `SettingsStore` hold the
// persisted values.
//
// Fields:
//   - `active_tab`              — currently-active Settings tab.
//   - `name_row_editing`        — General NameRow edit-mode toggle.
//   - `name_row_buffer`         — General NameRow in-progress edit text.
//   - `oauth_popup_open`        — convenience mirror of the WizardState flag
//                                 (lets tests/preview check the toggle without
//                                 needing a WizardState reference). The
//                                 source of truth is still
//                                 `wizard_state.github_auth_popup_open`.
//   - `validate_now_in_flight`  — disables the `Validate now` button while
//                                 a validation pass is running (purely a
//                                 visual lock — validation is synchronous in
//                                 P4.T7).
//   - `path_edit_debounce`      — per-field last-dirty timestamps for the
//                                 per-edit debounce cycle (H11).
//   - `path_validation_results` — last validation report; rendered as the
//                                 per-row hints in `tab_paths` / `tab_tools`.
//
// SPEC: §11.

use std::collections::HashMap;
use std::time::Instant;

use crate::ui::settings::widgets::tab_strip::TabLabel;

/// The five Settings tabs, in fixed render order (SPEC §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    General,
    Paths,
    Tools,
    Accounts,
    Advanced,
}

impl Default for SettingsTab {
    fn default() -> Self {
        SettingsTab::General
    }
}

impl TabLabel for SettingsTab {
    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Paths => "Paths",
            SettingsTab::Tools => "Tools",
            SettingsTab::Accounts => "Accounts",
            SettingsTab::Advanced => "Advanced",
        }
    }
}

impl SettingsTab {
    pub fn all() -> &'static [SettingsTab] {
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

/// Per-field validation result rendered as the row's inline status (border
/// tint + right hint). Each variant carries the message text directly so the
/// renderer doesn't have to know about field roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStatus {
    /// Path is not set yet — no styling, no hint.
    Empty,
    /// Path is valid for its role. `detail` is the optional trailing text
    /// (e.g., a version string for binary paths).
    Ok { detail: Option<String> },
    /// Path is set but suspicious for its role (non-blocking).
    Warning { reason: String },
    /// Path is set but blocking-invalid (doesn't exist or wrong type).
    Error { reason: String },
}

/// Visual tone associated with a `PathStatus`, used by `path_row` to pick the
/// input border color and hint text color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatusTone {
    Neutral,
    Success,
    Warning,
    Error,
}

impl PathStatus {
    /// Hint string for the row's right slot. Empty status returns an empty
    /// string (the row paints nothing).
    pub fn hint_text(&self) -> String {
        match self {
            PathStatus::Empty => String::new(),
            PathStatus::Ok { detail: Some(d) } => format!("ok \u{00B7} {d}"),
            PathStatus::Ok { detail: None } => "ok".to_string(),
            PathStatus::Warning { reason } => format!("! {reason}"),
            PathStatus::Error { reason } => format!("\u{00D7} {reason}"),
        }
    }

    /// Pick the row's visual tone from the status.
    pub fn tone(&self) -> PathStatusTone {
        match self {
            PathStatus::Empty => PathStatusTone::Neutral,
            PathStatus::Ok { .. } => PathStatusTone::Success,
            PathStatus::Warning { .. } => PathStatusTone::Warning,
            PathStatus::Error { .. } => PathStatusTone::Error,
        }
    }
}

/// One full validation pass result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    /// Per-field results keyed by `Step1Settings` field name.
    pub fields: HashMap<&'static str, PathStatus>,
    /// Overall `is_step1_valid` outcome (mirrors `state_validation::is_step1_valid`).
    pub overall_ok: bool,
    /// Aggregate issue count when not OK.
    pub issue_count: usize,
}

#[derive(Debug, Clone)]
pub struct SettingsScreenState {
    pub active_tab: SettingsTab,
    pub name_row_editing: bool,
    pub name_row_buffer: String,
    pub oauth_popup_open: bool,
    pub validate_now_in_flight: bool,
    pub path_edit_debounce: HashMap<&'static str, Instant>,
    pub path_validation_results: ValidationReport,
}

impl Default for SettingsScreenState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::default(),
            name_row_editing: false,
            name_row_buffer: String::new(),
            oauth_popup_open: false,
            validate_now_in_flight: false,
            path_edit_debounce: HashMap::new(),
            path_validation_results: ValidationReport::default(),
        }
    }
}
