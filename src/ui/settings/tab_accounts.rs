// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_accounts` — Accounts sub-tab renderer.
//
// Per Phase 4 P4.T5: three service cards.
//   - GitHub        — `connect` opens BIO's OAuth flow via `oauth_glue::start_github_flow`.
//                     `disconnect` clears the token via `oauth_glue::disconnect_github`.
//   - Nexus Mods    — `connect` shows a "not yet implemented" hint inline.
//   - Mega          — same as Nexus Mods.

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::oauth_glue;
use crate::ui::settings::widgets::account_card::{self, CardState};
use crate::ui::shared::redesign_tokens::redesign_text_faint;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;
    let login = orchestrator.wizard_state.github_auth_login.clone();

    // GitHub.
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
    );
    if gh_clicked {
        if login.trim().is_empty() {
            oauth_glue::start_github_flow(orchestrator, false);
        } else {
            oauth_glue::disconnect_github(orchestrator);
        }
    }

    // Nexus Mods (stub).
    let nx_clicked = account_card::render(
        ui,
        palette,
        "NX",
        "Nexus Mods",
        CardState::NotConnected,
        "connect",
        "disconnect",
    );
    if nx_clicked {
        orchestrator.accounts_stub_hint =
            Some("Nexus Mods OAuth is coming in a future release.".to_string());
    }

    // Mega (stub).
    let mg_clicked = account_card::render(
        ui,
        palette,
        "M",
        "Mega",
        CardState::NotConnected,
        "connect",
        "disconnect",
    );
    if mg_clicked {
        orchestrator.accounts_stub_hint =
            Some("Mega OAuth is coming in a future release.".to_string());
    }

    if let Some(hint) = orchestrator.accounts_stub_hint.clone() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(hint)
                .size(12.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_faint(palette)),
        );
    }
}
