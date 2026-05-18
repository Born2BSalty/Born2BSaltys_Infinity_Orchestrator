// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `success_banner` — the post-install success banner row, rendered ABOVE
// BIO's embedded Step-5 panel (per H9, in the chrome row immediately above
// the panel; visually it replaces the empty pre-install banner slot).
//
// **Visibility gate — the C3 clean-exit triple.** Per the Phase-7 plan
// (the "Clean-exit definition (per C3)" section) the banner is visible
// **only** when:
//
//   state.step5.install_running == false
//       && state.step5.last_exit_code == Some(0)
//       && state.step5.last_install_failed == false
//
// (Each field is `pub` per `state_step5.rs:17-19`; the triple supersedes
// every `errors_detected` reference — BIO's `Step5State` has no such
// field.) Pre-install and during-install the triple is false ⇒ the row is
// **empty** (the embedded panel below shows BIO's pre-install Command /
// Summary cards). When it holds, the banner is the wireframe
// `FinalPlanPanel` `installComplete` row (`screens.jsx:3230-3235`): a
// success-green `Installed` Pill + `<N> mods · <C> components · no errors`
// + a right-aligned `ran <MM:SS> · finished <relative>`.
//
// **Run 1 scope.** Run 1 ships the C3-gated component but it renders
// **nothing** this run: no install has run, so the C3 triple is
// structurally false (a fresh `Step5State` has `install_running == false`
// but `last_exit_code == None`, so the triple is false). The render body —
// the Pill + counts + duration line — is Run 3 (P7.T4), which also adds
// `format_install_duration` to `src/ui/shared/format_relative.rs`. The C3
// predicate itself is implemented + unit-tested now so the gate is settled
// and Run 3 only fills the visible body.
//
// SPEC: §9.2.

use crate::app::state::WizardState;

/// The C3 clean-exit triple (the Phase-7 plan's canonical success gate).
/// `true` iff an install has completed cleanly: not running, last exit
/// code 0, and BIO did not flag a likely failure on exit
/// (`step5_runtime_status::process_exit_event` sets `last_install_failed`
/// from `term.likely_failure_visible()` and `last_exit_code` on every
/// exit). The success banner, the post-install action row, and the P7.T6
/// registry transition all gate on this exact predicate — defined once
/// here so they can never drift.
#[must_use]
pub fn clean_exit(state: &WizardState) -> bool {
    !state.step5.install_running
        && state.step5.last_exit_code == Some(0)
        && !state.step5.last_install_failed
}

/// Render the success-banner row.
///
/// **Run 1:** returns immediately unless the C3 triple holds. The C3
/// triple is structurally false until an install has completed cleanly
/// (Run 2 starts installs; the success body is Run 3 / P7.T4), so this is
/// a no-op slot this run — exactly the empty pre-install banner the
/// Run-1 breakpoint requires. `_ui` is the chrome row's `Ui`; Run 3 paints
/// the Pill + counts + duration into it when `clean_exit` is true.
pub fn render(_ui: &mut eframe::egui::Ui, state: &WizardState) {
    if !clean_exit(state) {
        // Pre-install / during-install / failed install ⇒ empty slot. The
        // embedded `page_step5::render` panel below shows BIO's pre-install
        // Command/Summary cards (or the live console).
        return;
    }

    // P7.T4 (Run 3): the success banner body lands here —
    //   Box(borderColor: success) {
    //     Pill(success) "Installed"
    //     Label "<N> mods · <C> components · no errors"
    //     Label(right, faint) "ran <MM:SS> · finished <relative>"
    //   }
    // gated on this `clean_exit` predicate, with `format_install_duration`
    // (added to `src/ui/shared/format_relative.rs` per L9/P7.T4) for the
    // `<MM:SS>` / `<H:MM:SS>` duration. Run 1 intentionally renders
    // nothing here — no install has completed.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_is_not_clean_exit() {
        // A fresh `Step5State` has `install_running == false` but
        // `last_exit_code == None` — the C3 triple must be false (the
        // Run-1 "empty banner pre-install" property).
        let s = WizardState::default();
        assert!(!clean_exit(&s), "no install has run ⇒ the banner is hidden");
    }

    #[test]
    fn running_install_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = true;
        s.step5.last_exit_code = Some(0);
        assert!(
            !clean_exit(&s),
            "an install still running is not a clean exit"
        );
    }

    #[test]
    fn nonzero_exit_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(1);
        assert!(!clean_exit(&s), "a nonzero exit code is not clean");
    }

    #[test]
    fn flagged_failure_is_not_clean_exit() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = true;
        assert!(
            !clean_exit(&s),
            "a BIO-flagged likely-failure is not a clean exit even at \
             exit code 0"
        );
    }

    #[test]
    fn clean_triple_holds() {
        let mut s = WizardState::default();
        s.step5.install_running = false;
        s.step5.last_exit_code = Some(0);
        s.step5.last_install_failed = false;
        assert!(
            clean_exit(&s),
            "not running + exit 0 + not flagged ⇒ clean exit (the C3 \
             triple the banner/post-install/registry-flip gate on)"
        );
    }
}
