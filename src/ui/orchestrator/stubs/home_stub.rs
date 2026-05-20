// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use eframe::egui;

use crate::registry::dev_seed;
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{
    BtnOpts, redesign_btn, redesign_label, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

pub fn render_home_stub(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    render_screen_title(
        ui,
        palette,
        "Welcome back, adventurer",
        Some("Coming in Phase 5 \u{2014} SPEC \u{00A7}3"),
    );

    ui.add_space(8.0);
    faint_phase_note(
        ui,
        palette,
        "Home will list installed + in-progress modlists, an Add-a-modlist box, and the first-launch empty state.",
    );

    if orchestrator.dev_mode {
        ui.add_space(20.0);
        let resp = redesign_btn(
            ui,
            palette,
            "Open workspace stub (dev)",
            BtnOpts {
                small: true,
                primary: true,
                ..Default::default()
            },
        );
        if resp.clicked() {
            orchestrator.nav = NavDestination::Workspace { modlist_id: None };
        }

        ui.add_space(8.0);
        let seed_resp = redesign_btn(
            ui,
            palette,
            "Seed test modlist (dev)",
            BtnOpts {
                small: true,
                primary: false,
                ..Default::default()
            },
        );
        if seed_resp.clicked() {
            handle_seed_click(orchestrator);
        }

        if let Some(toast) = orchestrator.home_stub_state.seed_toast_text.clone() {
            ui.add_space(8.0);
            let _ = redesign_label(ui, palette, &toast);
        }

        ui.add_space(6.0);
        let _ = redesign_label(
            ui,
            palette,
            "dev-only \u{2014} both buttons are removed in Phase 5 when Home gains real content",
        );
    }
}

fn handle_seed_click(orchestrator: &mut OrchestratorApp) {
    let registry = &mut orchestrator.registry;
    let registry_store = &orchestrator.registry_store;
    let result = dev_seed::seed_demo_entry(registry, registry_store, |id| {
        WorkspaceStore::new_for_id(id)
    });
    match result {
        Ok(entry) => {
            orchestrator.persistence_cycle.last_saved_registry = orchestrator.registry.clone();
            orchestrator
                .persistence_cycle
                .mark_registry_dirty(Instant::now());
            orchestrator.home_stub_state.seed_toast_text =
                Some(format!("Seeded \"{}\" (id {}).", entry.name, entry.id));
        }
        Err(err) => {
            orchestrator.home_stub_state.seed_toast_text = Some(format!("Seed failed: {err}"));
        }
    }
}

fn faint_phase_note(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(13.0)
            .family(egui::FontFamily::Proportional)
            .color(redesign_text_faint(palette)),
    );
}

#[derive(Debug, Clone, Default)]
pub struct HomeStubState {
    pub seed_toast_text: Option<String>,
}
