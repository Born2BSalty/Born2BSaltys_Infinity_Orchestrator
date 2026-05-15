// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_router` — match on `OrchestratorApp::nav` and dispatch to the active
// destination's renderer (or stub).
//
// Per Phase 2 P2.T4: arms initially rendered Phase-2 stubs. Phase 4 wired the
// real `Settings` screen; Phase 5 P5.T15 wires the real `Home` screen.
// `Install` / `Create` / `Workspace` still render stubs until Phases 5+/6.
// **The `Workspace` arm renders the placeholder stub — NOT the legacy
// `WizardApp::update_loop::run`.** Per H3 / C1 / C4: that path was reverted;
// Phase 6 wires the real workspace view (which calls BIO's per-step page
// renderers directly + an orchestrator-side Step 4 wrapper per C4).
//
// Per Phase 3 P3.T5: when `OrchestratorApp::registry_error` is `Some`, the
// router short-circuits to `registry_error_panel::render_registry_error` —
// the left rail + statusbar still render normally (they live in
// `OrchestratorApp::update`'s shell layout outside this router); only the
// main pane shows the error.
//
// SPEC: §2.1, §13.14.

use eframe::egui;

use crate::ui::home::page_home;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::registry_error_panel;
use crate::ui::orchestrator::stubs;
use crate::ui::settings::page_settings;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // Phase 3 — registry-error short-circuit. Left rail + statusbar continue
    // to render around this; only the main pane is replaced.
    if let Some(err) = orchestrator.registry_error.as_ref() {
        registry_error_panel::render_registry_error(
            ui,
            palette,
            err,
            orchestrator.registry_backup_path.as_ref(),
        );
        return;
    }

    match orchestrator.nav.clone() {
        // Phase 5 P5.T15 — Home stub replaced with the real Home screen.
        NavDestination::Home => page_home::render(ui, orchestrator, ctx),
        NavDestination::Install => stubs::render_install_stub(ui, palette),
        NavDestination::Create => stubs::render_create_stub(ui, palette),
        // Phase 4 P4.T8 — Settings stub replaced with the real 5-tab screen.
        NavDestination::Settings => page_settings::render(ui, orchestrator, ctx),
        NavDestination::Workspace { modlist_id } => {
            stubs::render_workspace_stub(ui, palette, modlist_id.as_deref())
        }
    }
}
