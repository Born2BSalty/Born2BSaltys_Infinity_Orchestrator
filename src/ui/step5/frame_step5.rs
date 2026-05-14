// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::page_step5::{Step5RenderOptions, Step5RenderRuntime};

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: Step5RenderRuntime<'_>,
    options: Step5RenderOptions<'_>,
) -> Option<Step5Action> {
    crate::ui::step5::content_step5::render(ui, state, runtime, options)
}
