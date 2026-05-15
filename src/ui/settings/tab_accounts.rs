// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::settings::widgets::account_card;
use crate::ui::shared::redesign_tokens::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAction {
    ConnectGitHub,
    DisconnectGitHub,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    github_login: &str,
) -> Option<AccountAction> {
    let trimmed_login = github_login.trim();
    let connected_user = (!trimmed_login.is_empty()).then_some(trimmed_login);
    let github_action_label = if connected_user.is_some() {
        "disconnect"
    } else {
        "connect"
    };

    let action = if account_card::render(
        ui,
        palette,
        "GitHub",
        "GH",
        connected_user,
        github_action_label,
    )
    .clicked()
    {
        Some(if connected_user.is_some() {
            AccountAction::DisconnectGitHub
        } else {
            AccountAction::ConnectGitHub
        })
    } else {
        None
    };
    ui.add_space(10.0);
    let _ = account_card::render(ui, palette, "Nexus Mods", "NX", None, "connect");
    ui.add_space(10.0);
    let _ = account_card::render(ui, palette, "Mega", "M", None, "connect");

    action
}
