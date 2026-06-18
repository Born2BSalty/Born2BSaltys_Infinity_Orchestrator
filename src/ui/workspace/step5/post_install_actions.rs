// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::registry::model::ModlistEntry;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::workspace::step5::state_workspace_step5::PostInstallAction;
use crate::ui::workspace::step5::success_banner;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &WizardState,
    _entry: &ModlistEntry,
) -> Option<PostInstallAction> {
    if !success_banner::clean_exit(state) {
        return None;
    }

    let mut action: Option<PostInstallAction> = None;

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
        let s = WizardState::default();
        assert!(!success_banner::clean_exit(&s));
    }
}
