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
// **The buttons (SPEC §9.2 / wireframe `screens.jsx:3258-3262` action
// row).** Two primary CTAs:
//   - `Return to Home`      → the caller sets `orchestrator.nav =
//                             NavDestination::Home` (the freshly-installed
//                             modlist now appears under Home's Installed
//                             chip).
//   - `Open install folder` → the caller reveals the registry entry's
//                             `destination_folder` via the existing
//                             `registry::operations::open_install_folder`
//                             helper (the SAME platform-open the Home Kebab
//                             uses — `rfd`/`explorer`/`open`/`xdg-open`).
// The `Share import code` button is rendered separately by
// `workspace_header.rs` (Phase-6 file — Phase 7 just flips it
// enabled/primary-teal when C3 holds); it is NOT in this row.
//
// **Why a returned action enum, not a `&mut OrchestratorApp`.** The
// render-gate (`tests/ui_snapshot_*`) is DATA-LOSS-safe — it must NOT
// construct an `OrchestratorApp`/store. Returning
// `Option<PostInstallAction>` lets the gate render the row with just
// `(palette, state, entry)` while the chrome dispatcher applies the action
// against the live orchestrator (the same split-render pattern Step 2/4
// use with their action enums).
//
// SPEC: §9.2 (H9 positioning).

use eframe::egui;

use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::workspace::step5::state_workspace_step5::PostInstallAction;
use crate::ui::workspace::step5::success_banner;

/// Render the post-install action row.
///
/// Returns `None` (renders nothing) unless the C3 triple holds — the empty
/// pre-install / during-install / failed-install slot (the embedded
/// `page_step5::render` panel renders directly below). When `clean_exit`
/// holds, paints `Return to Home` + `Open install folder` as primary CTAs
/// (per H9, immediately above BIO's now-disabled `✓ Installed` button) and
/// returns the chosen [`PostInstallAction`] for the caller to apply.
///
/// `entry` is the routed modlist's registry entry — only its identity is
/// used here (the open-folder target is resolved by the caller via
/// `operations::open_install_folder(entry)`); it is taken so the row's
/// gating/identity stays symmetric with `success_banner::render`.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &WizardState,
    _entry: &ModlistEntry,
) -> Option<PostInstallAction> {
    if !success_banner::clean_exit(state) {
        // Pre-install / during-install / failed install ⇒ empty/hidden
        // row. The embedded `page_step5::render` panel renders directly
        // below this slot.
        return None;
    }

    let mut action: Option<PostInstallAction> = None;

    // Wireframe action row (`screens.jsx:3257`): `display:flex; gap:8;
    // marginBottom:8; flexWrap:wrap; alignItems:center`. Both are primary
    // (teal fill + 2×2 shadow) — SPEC §9.2 "Two new primary actions".
    let row_margin_bottom = 8.0;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if redesign_btn(
            ui,
            palette,
            "Return to Home",
            BtnOpts {
                primary: true,
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            action = Some(PostInstallAction::ReturnToHome);
        }

        if redesign_btn(
            ui,
            palette,
            "Open install folder",
            BtnOpts {
                primary: true,
                small: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            action = Some(PostInstallAction::OpenInstallFolder);
        }
    });

    ui.add_space(row_margin_bottom);
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_until_clean_exit() {
        // With no install run, the post-install row is hidden (the C3
        // triple is false). Mirrors the success-banner gate — they share
        // the one `clean_exit` predicate so they can never disagree about
        // when the post-install chrome appears.
        let s = WizardState::default();
        assert!(!success_banner::clean_exit(&s));
    }
}
