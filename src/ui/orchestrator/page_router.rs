// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_router` — match on `OrchestratorApp::nav` and dispatch to the active
// destination's renderer (or stub).
//
// Per Phase 2 P2.T4: arms initially rendered Phase-2 stubs. Phase 4 wired the
// real `Settings` screen; Phase 5 P5.T15 wires the real `Home` screen and
// P5.T14 wires the real `Install` screen (Run 3: paste stage + stage-4 stub;
// the Preview / Downloading stages render Run-4 / Run-5 placeholders).
// `Create` / `Workspace` still render stubs until Phase 6.
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
use tracing::warn;

use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::home::page_home;
use crate::ui::install::page_install;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::registry_error_panel;
use crate::ui::orchestrator::stubs;
use crate::ui::settings::page_settings;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::{workspace_state_loader, workspace_view};

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
        // Phase 5 P5.T14 — Install stub replaced with the real Install
        // Modlist screen (Run 3: paste stage + stage-4 stub; Preview /
        // Downloading render Run-4 / Run-5 placeholders).
        NavDestination::Install => page_install::render(ui, orchestrator, ctx),
        NavDestination::Create => stubs::render_create_stub(ui, palette),
        // Phase 4 P4.T8 — Settings stub replaced with the real 5-tab screen.
        NavDestination::Settings => page_settings::render(ui, orchestrator, ctx),
        NavDestination::Workspace { modlist_id } => match modlist_id {
            Some(id) => render_workspace(ui, orchestrator, &id, ctx),
            // Phase 6 keeps the dev placeholder for `Workspace { None }` so
            // testing can navigate without a real id (the Phase-2 dev-mode
            // path).
            None => stubs::render_workspace_stub(ui, palette, None),
        },
    }
}

/// Resolve + load a modlist into the orchestrator's owned `WizardState`,
/// then render the workspace shell (P6.T12).
///
/// Steps (per the plan):
///   1. Look the id up in `orchestrator.registry`.
///   2. **C5 gate (safety net):** if a *different* install is running, the
///      rail-nav lock (Phase 7 P7.T9b) should have prevented this
///      transition; as a backstop, re-pin nav to the running modlist's
///      workspace. `install_in_progress` is Phase 7 — stubbed `None` here.
///   3. If `loaded_workspace_id != Some(id)`, lazy-load the modlist's
///      `workspace.json` into the orchestrator maps and
///      `populate_wizard_state_from_workspace`; set `loaded_workspace_id`.
///   4. **Every workspace frame:** `sync_paths_from_settings` (not just on
///      open) so Settings → Paths edits propagate without a close/reopen.
///   5. `workspace_view::render`.
fn render_workspace(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    id: &str,
    ctx: &egui::Context,
) {
    let palette = orchestrator.theme_palette;

    // 1. Registry lookup. Clone the entry out so the immutable
    //    `orchestrator.registry` borrow ends before the `&mut wizard_state`
    //    populate.
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        render_missing_modlist(ui, palette, id);
        return;
    };

    // 2. C5 safety net. `install_runtime::install_concurrency::
    //    install_in_progress(orchestrator)` is Phase 7 P7.T9b; until it
    //    exists there is no running install, so this is `None`.
    let install_in_progress: Option<String> = None; // Phase 7 P7.T9b
    if let Some(running_id) = install_in_progress.as_ref()
        && running_id != id
    {
        // Refuse the swap; re-pin nav to the running install's workspace.
        orchestrator.nav = NavDestination::Workspace {
            modlist_id: Some(running_id.clone()),
        };
        return;
    }

    // 3. Lazy-load + populate on first open / on a modlist swap.
    if orchestrator.workspace_view.loaded_workspace_id.as_deref() != Some(id) {
        // Ensure the workspace state + store are in the orchestrator maps.
        if !orchestrator.workspace_state.contains_key(id) {
            let store = WorkspaceStore::new_for_id(id);
            let loaded = match store.load() {
                Ok(ws) => ws,
                Err(err) => {
                    // `WorkspaceStore::load` maps *both* a missing file and
                    // a corrupt parse to `RegistryError::Corrupt` (a
                    // freshly-created modlist's workspace.json write is
                    // pending until Create's Run-3 path / first save). Run 1
                    // degrades safely to an empty workspace + a warn — the
                    // full SPEC §13.14 per-workspace terminal-error UI is
                    // part of the persistence integration (Run 4 / out of
                    // Run-1 scope per the brief), not a half path shipped
                    // here. (Reported as a deferred item.)
                    warn!(
                        target = "orchestrator",
                        "workspace.json for {id} not loadable ({err}); using empty state \
                         (per-workspace terminal-error UI deferred to the persistence run)"
                    );
                    ModlistWorkspaceState::default()
                }
            };
            orchestrator.workspace_state.insert(id.to_string(), loaded);
            orchestrator.workspace_stores.insert(id.to_string(), store);
        }

        // Clone the workspace state out so the `&orchestrator.workspace_state`
        // borrow ends before the `&mut orchestrator.wizard_state` populate.
        let workspace = orchestrator
            .workspace_state
            .get(id)
            .cloned()
            .unwrap_or_default();

        workspace_state_loader::populate_wizard_state_from_workspace(
            &workspace,
            &entry,
            &orchestrator.settings_store,
            &mut orchestrator.wizard_state,
        );

        // Refresh the workspace view's identity for the freshly-loaded
        // modlist. (The full header/fork population is Run 2; Run 1 sets the
        // name + game + loaded-id so the shell renders correctly and the
        // loader's swap detection works.)
        orchestrator.workspace_view.modlist_id = id.to_string();
        orchestrator.workspace_view.modlist_name = entry.name.clone();
        orchestrator.workspace_view.game = entry.game;
        orchestrator.workspace_view.current_step = WorkspaceStep::Step2;
        orchestrator.workspace_view.completed_steps.clear();
        orchestrator.workspace_view.loaded_workspace_id = Some(id.to_string());
    }

    // 4. Per-frame Settings re-sync (M2) — every workspace frame, not just
    //    on open, so Settings → Paths edits made while away propagate
    //    before the step renderer / install hook reads them.
    workspace_state_loader::sync_paths_from_settings(
        &orchestrator.settings_store,
        &mut orchestrator.wizard_state,
    );

    // 5. Render the workspace shell.
    workspace_view::render(ui, orchestrator, id, ctx);
}

/// Shown when the routed workspace id is not in the registry (e.g. a stale
/// nav after the entry was deleted from Home). Not a terminal error — the
/// registry itself is fine; only this id is gone.
fn render_missing_modlist(ui: &mut egui::Ui, palette: ThemePalette, id: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(format!(
            "Modlist \"{id}\" is no longer in the registry. It may have been deleted.",
        ))
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
