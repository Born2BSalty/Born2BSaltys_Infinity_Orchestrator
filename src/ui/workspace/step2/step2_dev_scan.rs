// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use rfd::FileDialog;
use tracing::warn;

use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step2::step2_rescan_reconcile;
use crate::ui::workspace::workspace_state_loader;

pub fn pick_folder_and_scan(orchestrator: &mut OrchestratorApp) {
    let current = orchestrator.wizard_state.step1.mods_folder.trim();
    let mut dialog = FileDialog::new().set_title("Dev: pick a mods folder to scan");
    if !current.is_empty() {
        dialog = dialog.set_directory(current);
    }
    let Some(picked) = dialog.pick_folder() else {
        return;
    };

    let folder = picked.to_string_lossy().to_string();
    orchestrator
        .wizard_state
        .step1
        .mods_folder
        .clone_from(&folder);

    record_dev_scan_folder(orchestrator, &folder);

    step2_rescan_reconcile::snapshot_current_selection(orchestrator);

    step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);
}

fn record_dev_scan_folder(orchestrator: &mut OrchestratorApp, folder: &str) {
    let id = orchestrator.workspace_view.modlist_id.clone();
    if id.is_empty() {
        return;
    }

    workspace_state_loader::sync_step3_from_step2_if_changed(&mut orchestrator.wizard_state);

    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let mut extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );
    extracted.dev_scanned_mods_folder = Some(folder.to_string());

    let store = orchestrator
        .workspace_stores
        .entry(id.clone())
        .or_insert_with(|| WorkspaceStore::new_for_id(&id));

    match store.save(&extracted) {
        Ok(()) => {
            orchestrator
                .workspace_state
                .insert(id.clone(), extracted.clone());
            orchestrator
                .persistence_cycle
                .last_saved_workspaces
                .insert(id, extracted);
        }
        Err(err) => {
            warn!(
                target = "orchestrator",
                "recording dev-scan folder for the workspace failed: {err} \
                 (this session's scan still runs; cross-restart resume won't)"
            );
        }
    }
}
