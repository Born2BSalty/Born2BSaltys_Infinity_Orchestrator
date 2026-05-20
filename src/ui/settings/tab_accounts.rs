// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::oauth_glue;
use crate::ui::settings::widgets::account_card::{self, AccountCard, CardState};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;
    let login = orchestrator.wizard_state.github_auth_login.clone();

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
        AccountCard {
            initials: "GH",
            service_name: "GitHub",
            state: gh_state,
            connect_label: "connect",
            disconnect_label: "disconnect",
            disabled: false,
        },
    );
    if gh_clicked {
        if login.trim().is_empty() {
            oauth_glue::start_github_flow(orchestrator, false);
        } else {
            oauth_glue::disconnect_github(orchestrator);
        }
    }

    let _ = account_card::render(
        ui,
        palette,
        AccountCard {
            initials: "NX",
            service_name: "Nexus Mods",
            state: CardState::NotConnected,
            connect_label: "connect",
            disconnect_label: "disconnect",
            disabled: true,
        },
    );

    let _ = account_card::render(
        ui,
        palette,
        AccountCard {
            initials: "M",
            service_name: "Mega",
            state: CardState::NotConnected,
            connect_label: "connect",
            disconnect_label: "disconnect",
            disabled: true,
        },
    );
}
