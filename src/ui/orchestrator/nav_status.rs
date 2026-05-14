// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `nav_status` — read-only formatter that turns BIO's existing path-validation
// state into the rail's bottom status indicator (SPEC §2.1).
//
// Per Phase 2 P2.T8: this module only **reads** state. The actual validation
// logic stays in BIO's existing `state_validation*.rs` files (unmodified).
// `compute_path_validation_summary` calls `is_step1_valid` and
// `step1_validation_messages` (both `pub` per `state_validation.rs`) to
// produce the rail label.
//
// Output format mirrors the wireframe (`app.jsx:122-131`):
//   - Ok  → green dot + "weidu v<detected> · all paths ok"
//   - Err → red dot   + "× <N> path issues"
//
// The "weidu v<detected>" segment is left as "weidu vX" placeholder in
// Phase 2 — Phase 4 wires real WeiDU version detection via a tool-version
// cache (per phase 4 P4.T?). This is documented as a known stub.

use crate::app::state::WizardState;

/// Whether all configured paths validate cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathValidationKind {
    /// All paths valid; show the green status dot.
    Ok,
    /// One or more paths invalid; show the red status dot.
    Err(usize),
}

/// Computed summary suitable for the left-rail's bottom indicator.
#[derive(Debug, Clone)]
pub struct PathValidationSummary {
    pub kind: PathValidationKind,
    /// The one-line text shown next to the dot.
    pub text: String,
}

impl Default for PathValidationSummary {
    fn default() -> Self {
        Self {
            kind: PathValidationKind::Err(0),
            text: String::from("paths not configured"),
        }
    }
}

/// Build the rail's path-validation summary from BIO's existing read-only
/// validation entries.
///
/// **No BIO files are modified.** This function only **calls** the public
/// `is_step1_valid` entry (and `cached_path_check` if available). The `step1`
/// portion of `WizardState` is the same data BIO inspects in its own Step 1
/// page.
///
/// Note: `state_validation::step1_validation_messages` is `pub(super)` and
/// not reachable from this module. We approximate the "path issue count"
/// from `step1_path_check` (cached `(ok, message)` tuple) when present;
/// otherwise we just report a generic "× paths not configured" string.
///
/// Phase 4 will additionally consume a tool-version cache to fill in
/// `weidu v<N>`; Phase 2 emits a placeholder "weidu v…" suffix.
pub fn compute_path_validation_summary(state: &WizardState) -> PathValidationSummary {
    use crate::app::state_validation;

    let step1 = &state.step1;
    if state_validation::is_step1_valid(step1) {
        return PathValidationSummary {
            kind: PathValidationKind::Ok,
            text: String::from("weidu v\u{2026} \u{00B7} all paths ok"),
        };
    }

    // Try to derive a count from the cached path-check message if a recent
    // path-check has been run. Otherwise fall back to a generic message.
    let issue_count = state
        .step1_path_check
        .as_ref()
        .filter(|(ok, _)| !*ok)
        .map(|(_, msg)| state_validation::split_path_check_lines(msg).len())
        .unwrap_or(0);

    let text = if issue_count == 0 {
        String::from("\u{00D7} paths not configured")
    } else if issue_count == 1 {
        String::from("\u{00D7} 1 path issue")
    } else {
        format!("\u{00D7} {issue_count} path issues")
    };

    PathValidationSummary {
        kind: PathValidationKind::Err(issue_count),
        text,
    }
}
