// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_accounts` — Accounts sub-tab renderer.
//
// Per SPEC §11.4: three service cards.
//   - GitHub     — `connect` opens BIO's OAuth flow via `oauth_glue::start_github_flow`;
//                  `disconnect` clears the token via `oauth_glue::disconnect_github`.
//   - Nexus Mods — `disabled` (not yet wired). Hover tooltip explains.
//   - Mega       — same as Nexus Mods.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::oauth_glue;
use crate::ui::settings::widgets::account_card::{self, CardState};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;
    let login = orchestrator.wizard_state.github_auth_login.clone();

    // GitHub — fully wired.
    let gh_state = if login.trim().is_empty() {
        CardState::NotConnected
    } else {
        CardState::Connected {
            user_label: login.trim(),
        }
    };
    let gh_clicked = account_card::render(
        ui,
        palette,
        "GH",
        "GitHub",
        gh_state,
        "connect",
        "disconnect",
        false, // not disabled
    );
    if gh_clicked {
        if login.trim().is_empty() {
            oauth_glue::start_github_flow(orchestrator, false);
        } else {
            oauth_glue::disconnect_github(orchestrator);
        }
    }

    // Nexus Mods — coming later; button is non-clickable.
    let _ = account_card::render(
        ui,
        palette,
        "NX",
        "Nexus Mods",
        CardState::NotConnected,
        "connect",
        "disconnect",
        true, // disabled
    );

    // Mega — coming later; button is non-clickable.
    let _ = account_card::render(
        ui,
        palette,
        "M",
        "Mega",
        CardState::NotConnected,
        "connect",
        "disconnect",
        true, // disabled
    );
}
