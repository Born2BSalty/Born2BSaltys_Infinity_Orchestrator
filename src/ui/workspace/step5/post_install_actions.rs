// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `post_install_actions` — the post-install action row, rendered
// **immediately ABOVE** BIO's embedded Step-5 panel (per H9).
//
// **H9 positioning (the plan's pragmatic resolution of SPEC §9.2's "next
// to the disabled Install button").** BIO's `page_step5::render` places
// the Install button at the **top** of its panel (the install row is the
// panel's first content row — verified, `content_install_row_step5.rs`).
// Rendering this row *below* the panel would put the buttons hundreds of
// pixels from the Install button, contradicting SPEC §9.2's "next to"
// directive; rendering it **immediately above** the embedded panel places
// `Return to Home` + `Open install folder` visually adjacent to BIO's
// now-disabled `✓ Installed` button — without the heavy C4-style rewrite
// of BIO's Step-5 body (rejected for v1 alpha). The chrome row order from
// the top is therefore: success-banner row, then this post-install action
// row, then the embedded `page_step5::render` panel.
//
// **Visibility gate — the C3 clean-exit triple.** Visible only when
// `success_banner::clean_exit(state)` holds (the single shared C3
// predicate — `install_running == false && last_exit_code == Some(0) &&
// last_install_failed == false`). Pre-install and during-install the row
// is empty/hidden.
//
// **Run 1 scope.** Run 1 ships the C3-gated component but it renders
// **nothing** this run (the C3 triple is structurally false — no install
// has run). The buttons + their handlers (`Return to Home` →
// `orchestrator.nav = NavDestination::Home`; `Open install folder` →
// platform open of the registry entry's `destination_folder`) are Run 3
// (P7.T5); the `Share import code` button is flipped to primary teal by
// `workspace_header.rs` separately (Run 3 — not this row).
//
// SPEC: §9.2 (H9 positioning).

use crate::app::state::WizardState;
use crate::ui::workspace::step5::success_banner;

/// Render the post-install action row.
///
/// **Run 1:** returns immediately unless the C3 triple holds. It is
/// structurally false until an install has completed cleanly (Run 2 starts
/// installs; the buttons are Run 3 / P7.T5), so this is a no-op slot this
/// run — exactly the hidden pre-install post-install row the Run-1
/// breakpoint requires. `_ui` is the chrome row's `Ui` (immediately above
/// the embedded panel per H9); Run 3 paints the two primary buttons into
/// it when `clean_exit` is true.
pub fn render(_ui: &mut eframe::egui::Ui, state: &WizardState) {
    if !success_banner::clean_exit(state) {
        // Pre-install / during-install / failed install ⇒ empty/hidden
        // row. The embedded `page_step5::render` panel renders directly
        // below this slot.
        return;
    }

    // P7.T5 (Run 3): the two primary buttons land here, immediately above
    // BIO's panel (per H9 — visually adjacent to BIO's now-disabled
    // `✓ Installed` button at the top of that panel):
    //   - `Return to Home`      → orchestrator.nav = NavDestination::Home
    //   - `Open install folder` → platform-open the registry entry's
    //                             `destination_folder`
    // gated on `success_banner::clean_exit`. Run 1 renders nothing here —
    // no install has completed.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_until_clean_exit() {
        // Run-1 property: with no install run, the post-install row is
        // hidden (the C3 triple is false). Mirrors the success-banner gate
        // — they share the one `clean_exit` predicate so they can never
        // disagree about when the post-install chrome appears.
        let s = WizardState::default();
        assert!(!success_banner::clean_exit(&s));
    }
}
