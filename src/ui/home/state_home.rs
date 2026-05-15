// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `HomeScreenState` — per-screen UI state for the Home destination.
//
// Lives on `OrchestratorApp::home_screen_state`. Persists across screen
// visits within a session; not written to disk (the registry is the source
// of truth for card data).
//
// Run 1 (Phase 5) uses `filter` only. `delete_target` / `reinstall_target` /
// `toast` are declared now (one struct, no point growing it twice) but their
// driving behavior — the confirm dialogs (P5.T7 / P5.T18) and the toast
// surface (P5.T16) — lands in Run 2. They are inert this run.
//
// SPEC: §3.1 (filter chips + default selection + empty-filter copy), §3.4.

/// Which Home filter chip is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFilter {
    /// Finished modlists.
    Installed,
    /// In-progress builds (not yet successfully installed).
    InProgress,
    /// Both lists, installed-first then in-progress-after.
    All,
}

impl Default for HomeFilter {
    /// Steady-state default is `Installed` (SPEC §3.1: "the default selection
    /// when the user lands on Home, since the steady-state experience is
    /// 'play your existing libraries'"). The per-frame
    /// `resolve_default_filter` overrides this when the inferred category is
    /// empty.
    fn default() -> Self {
        HomeFilter::Installed
    }
}

/// SPEC §3.1 default-selection rule, evaluated per render against live
/// registry counts:
///
/// > if installed count > 0 then `Installed`; else if in-progress count > 0
/// > then `In progress`; else `All`.
///
/// "When the inferred default category is empty (e.g. a brand-new install
/// with N = 0), the chip selection falls back to whichever category has
/// content (`In progress`, else `All`)."
pub fn resolve_default_filter(installed_count: usize, in_progress_count: usize) -> HomeFilter {
    if installed_count > 0 {
        HomeFilter::Installed
    } else if in_progress_count > 0 {
        HomeFilter::InProgress
    } else {
        HomeFilter::All
    }
}

/// Faint copy shown when the active chip's filtered list is empty
/// (SPEC §3.1 "Empty states" — wireframe `screens.jsx:313-317`, verbatim).
pub fn empty_filter_message(filter: HomeFilter) -> &'static str {
    match filter {
        HomeFilter::Installed => {
            "No installed modlists yet. Create one or paste an import code to add the first."
        }
        HomeFilter::InProgress => {
            "No in-progress builds. Start a new modlist from \"create your own\"."
        }
        HomeFilter::All => "No modlists yet.",
    }
}

/// Per-screen Home UI state.
#[derive(Debug, Clone, Default)]
pub struct HomeScreenState {
    /// Active filter chip. `None` until the first render resolves the SPEC
    /// §3.1 default from live counts; once the user clicks a chip it holds
    /// their explicit choice.
    pub filter: Option<HomeFilter>,

    // ---- Run 2 fields (declared now; behavior lands in Run 2) ----
    /// Modlist id pending a Delete confirm (P5.T7 — Run 2).
    pub delete_target: Option<String>,
    /// Modlist id pending a Reinstall confirm (P5.T18 — Run 2).
    pub reinstall_target: Option<String>,
    /// Transient toast (P5.T16 — Run 2).
    pub toast: Option<ToastMessage>,
}

/// Visual tone of a toast. The wireframe's only toast is the success-green
/// `✓ <text>` confirmation (`screens.jsx:367-382`, `color: var(--success)`);
/// `Success` reproduces that exactly. `Error` is the orchestrator's
/// bottom-of-screen error surface for SPEC §3.2's "Open install folder"
/// failure path ("surface an error message in the standard status / error
/// message area near the bottom of the screen") — rendered in the danger
/// tone with no `✓` marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastTone {
    /// Green `✓ <text>` confirmation (wireframe default).
    #[default]
    Success,
    /// Coral error line (no leading marker) for surfaced failures.
    Error,
}

/// A transient bottom-center toast. P5.T16 renders + auto-dismisses this
/// (~1.8s). Driven by `HomeScreenState.toast`.
#[derive(Debug, Clone)]
pub struct ToastMessage {
    /// Body text. Does **not** include the leading `✓` — `toast::render`
    /// paints the marker separately in `firacode_nerd` (the shipped Poppins
    /// TTFs are a Latin-only subset and tofu `✓` U+2713; HANDOFF caveat).
    pub text: String,
    /// Instant the toast was shown (drives the ~1.8s auto-dismiss).
    pub shown_at: std::time::Instant,
    /// Visual tone (success = green + ✓, error = coral, no marker).
    pub tone: ToastTone,
}

impl ToastMessage {
    /// A success toast (`✓ <text>`, green) shown "now".
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shown_at: std::time::Instant::now(),
            tone: ToastTone::Success,
        }
    }

    /// An error toast (coral, no marker) shown "now".
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shown_at: std::time::Instant::now(),
            tone: ToastTone::Error,
        }
    }
}

impl HomeScreenState {
    /// The effective filter for this render: the user's explicit choice if
    /// they have clicked a chip, otherwise the SPEC §3.1 default derived from
    /// the live counts.
    pub fn effective_filter(
        &self,
        installed_count: usize,
        in_progress_count: usize,
    ) -> HomeFilter {
        self.filter
            .unwrap_or_else(|| resolve_default_filter(installed_count, in_progress_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_filter_prefers_installed() {
        assert_eq!(resolve_default_filter(3, 2), HomeFilter::Installed);
        assert_eq!(resolve_default_filter(1, 0), HomeFilter::Installed);
    }

    #[test]
    fn default_filter_falls_back_to_in_progress() {
        assert_eq!(resolve_default_filter(0, 2), HomeFilter::InProgress);
    }

    #[test]
    fn default_filter_falls_back_to_all_when_empty() {
        assert_eq!(resolve_default_filter(0, 0), HomeFilter::All);
    }

    #[test]
    fn effective_filter_uses_explicit_choice_over_default() {
        let mut st = HomeScreenState::default();
        // No explicit choice → derived default (installed-first).
        assert_eq!(st.effective_filter(2, 1), HomeFilter::Installed);
        // Explicit choice wins even when counts would imply something else.
        st.filter = Some(HomeFilter::All);
        assert_eq!(st.effective_filter(2, 1), HomeFilter::All);
    }

    #[test]
    fn empty_messages_match_spec() {
        assert_eq!(
            empty_filter_message(HomeFilter::Installed),
            "No installed modlists yet. Create one or paste an import code to add the first."
        );
        assert_eq!(
            empty_filter_message(HomeFilter::InProgress),
            "No in-progress builds. Start a new modlist from \"create your own\"."
        );
        assert_eq!(empty_filter_message(HomeFilter::All), "No modlists yet.");
    }
}
