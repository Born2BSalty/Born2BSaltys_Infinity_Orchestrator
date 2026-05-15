// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Home stub destination.
//
// Per Phase 2 P2.T5: title "Welcome back, adventurer", sub line, plus a
// dev-mode-only `Open workspace stub (dev)` button that flips `nav` to
// `NavDestination::Workspace { modlist_id: None }`. Phase 5 removes the
// button.
//
// Per Phase 3 P3.T8 + P3.T9: a second dev-mode-only `Seed test modlist (dev)`
// button is added that calls `bio::registry::dev_seed::seed_demo_entry` and
// renders a transient confirmation line on success. The statusbar's modlist
// count reads from `OrchestratorApp::registry.entries.len()` in
// `OrchestratorApp::update`, so it bumps automatically after a click.
//
// SPEC: §3, §13.1.

use std::time::Instant;

use eframe::egui;

use crate::registry::dev_seed;
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{
    redesign_btn, redesign_label, render_screen_title, BtnOpts,
};
use crate::ui::shared::redesign_tokens::{redesign_text_faint, ThemePalette};

pub fn render_home_stub(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    render_screen_title(
        ui,
        palette,
        "Welcome back, adventurer",
        Some("Coming in Phase 5 \u{2014} SPEC \u{00A7}3"),
    );

    ui.add_space(8.0);
    faint_phase_note(ui, palette, "Home will list installed + in-progress modlists, an Add-a-modlist box, and the first-launch empty state.");

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

        // Phase 3 dev-only seed button. Adjacent to the workspace stub
        // button — both will be replaced in Phase 5 by the real Home content.
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

        if let Some(toast) = orchestrator
            .home_stub_state
            .seed_toast_text
            .as_ref()
            .cloned()
        {
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
            // Mark the persistence cycle aware — the registry was saved
            // synchronously by `seed_demo_entry`, but the cycle's snapshot
            // needs to refresh so the next debounce tick doesn't re-write.
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

/// Per-screen state for the home stub. Phase 3 only carries a transient
/// toast string. Phase 5 replaces this with the real Home screen state.
#[derive(Debug, Clone, Default)]
pub struct HomeStubState {
    pub seed_toast_text: Option<String>,
}
