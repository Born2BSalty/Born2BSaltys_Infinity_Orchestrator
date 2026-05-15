// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{r_box, screen_title};
use crate::ui::settings::state_settings::{SettingsScreenState, SettingsTab};
use crate::ui::settings::tab_accounts;
use crate::ui::settings::tab_advanced;
use crate::ui::settings::tab_general;
use crate::ui::settings::tab_paths;
use crate::ui::settings::tab_tools;
use crate::ui::settings::widgets::tab_strip;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_PAGE_PADDING_X_PX, REDESIGN_PAGE_PADDING_Y_PX, REDESIGN_SETTINGS_PAGE_PADDING_TOP_PX,
    ThemePalette, redesign_border_strong, redesign_hover_overlay, redesign_selection_highlight,
    redesign_selection_highlight_hover, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut SettingsScreenState,
    github_login: &str,
) -> Option<SettingsAction> {
    let mut action = None;
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_X_PX),
            right: crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_X_PX),
            top: crate::ui::shared::redesign_tokens::redesign_i8_px(
                REDESIGN_SETTINGS_PAGE_PADDING_TOP_PX,
            ),
            bottom: crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_Y_PX),
        })
        .show(ui, |ui| {
            apply_settings_visuals(ui, palette);
            screen_title::render(ui, palette, "Settings", None);
            tab_strip::render(ui, palette, &mut state.active_tab);
            r_box::redesign_box(ui, palette, None, |ui| {
                ui.set_min_height(ui.available_height());
                match state.active_tab {
                    SettingsTab::General => {
                        if tab_general::render(ui, palette, state) {
                            state.mark_general_changed();
                        }
                    }
                    SettingsTab::Paths => {
                        action = tab_paths::render(ui, palette, state).map(SettingsAction::Paths);
                    }
                    SettingsTab::Tools => tab_tools::render(ui, palette, state),
                    SettingsTab::Accounts => {
                        action = tab_accounts::render(ui, palette, github_login)
                            .map(SettingsAction::Account);
                    }
                    SettingsTab::Advanced => tab_advanced::render(ui, palette, state),
                }
            });
        });
    action
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    Account(tab_accounts::AccountAction),
    Paths(tab_paths::PathsAction),
}

fn apply_settings_visuals(ui: &mut egui::Ui, palette: ThemePalette) {
    let mut style = (**ui.style()).clone();
    style.visuals.widgets.hovered.bg_fill = redesign_hover_overlay(palette);
    style.visuals.widgets.hovered.fg_stroke.color = redesign_text_primary(palette);
    style.visuals.widgets.hovered.bg_stroke.color = redesign_border_strong(palette);
    style.visuals.widgets.active.bg_fill = redesign_selection_highlight_hover(palette);
    style.visuals.widgets.active.fg_stroke.color = redesign_text_primary(palette);
    style.visuals.widgets.active.bg_stroke.color = redesign_border_strong(palette);
    style.visuals.selection.bg_fill = redesign_selection_highlight(palette);
    style.visuals.selection.stroke.color = redesign_text_primary(palette);
    ui.set_style(style);
}
