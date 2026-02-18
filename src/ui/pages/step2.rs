// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

mod action;
mod actions_tabs;
mod compat_popup;
mod details_pane;
mod header;
mod layout;
mod list_pane;

pub use action::Step2Action;

pub fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<Step2Action> {
    layout::render(ui, state)
}
