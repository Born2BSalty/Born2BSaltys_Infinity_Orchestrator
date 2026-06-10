// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::install_runtime::{destination_prep, fork_pipeline_arm, per_install_dirs};
use crate::registry::model::Game;
use crate::registry::operations;
use crate::registry::operations_create::create_modlist_with_author;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::destination_default::default_destination;
use crate::ui::create::load_draft_dialog::{self, LoadDraftOutcome};
use crate::ui::create::stage_choose::{self, ChooseOutcome};
use crate::ui::create::stage_fork_download::{self, ForkDownloadOutcome};
use crate::ui::create::stage_fork_paste::{self, ForkPasteOutcome};
use crate::ui::create::stage_fork_preview::{self, ForkPreviewOutcome};
use crate::ui::create::state_create::CreateStage;
use crate::ui::home::confirm_delete;
use crate::ui::install::state_install::DestChoice;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::{
    DestinationPrepFlow, OrchestratorApp, PendingCreateStart, PendingFolderDelete,
};
use crate::ui::orchestrator::widgets::clipboard;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::shared::redesign_tokens::ThemePalette;

const COPY_CONFIRM_MS: u64 = 1600;

enum CreateRequest {
    StartScratch,
    GoForkPaste,
    ForkPasteBack,
    ForkPastePreview,
    ForkPreviewBack,
    ForkBeginImport,
    ForkDownloadCancel,
    ForkExtractCompleteRouteToWorkspace(String),
    OpenLoadDraft,
    CloseLoadDraft,
    ResumeWorkspace(String),
    CopyImportCode(String),
    ArmDeleteDraft(String),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    poll_create_destination_prep(orchestrator);

    if let Some(deadline) = orchestrator.create_screen_state.load_draft_copied_until
        && Instant::now() >= deadline
    {
        orchestrator.create_screen_state.load_draft_copied_name = None;
        orchestrator.create_screen_state.load_draft_copied_until = None;
    }

    let mut request = collect_stage_request(ui, palette, ctx, orchestrator);

    if orchestrator.create_screen_state.load_draft_open {
        request = collect_load_draft_request(ctx, palette, orchestrator).or(request);

        render_load_draft_delete_confirm(orchestrator, ctx);
    }

    if let Some(req) = request {
        handle_create_request(orchestrator, ctx, req);
    }
}

fn collect_stage_request(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ctx: &egui::Context,
    orchestrator: &mut OrchestratorApp,
) -> Option<CreateRequest> {
    match orchestrator.create_screen_state.stage {
        CreateStage::Choose => {
            match stage_choose::render(
                ui,
                palette,
                &mut orchestrator.create_screen_state,
                orchestrator.create_destination_prep_rx.is_some(),
            ) {
                ChooseOutcome::StartScratch => Some(CreateRequest::StartScratch),
                ChooseOutcome::GoForkPaste => Some(CreateRequest::GoForkPaste),
                ChooseOutcome::OpenLoadDraft => Some(CreateRequest::OpenLoadDraft),
                ChooseOutcome::Stay => None,
            }
        }
        CreateStage::ForkPaste => {
            match stage_fork_paste::render(ui, palette, &mut orchestrator.create_screen_state) {
                ForkPasteOutcome::Back => Some(CreateRequest::ForkPasteBack),
                ForkPasteOutcome::Preview => Some(CreateRequest::ForkPastePreview),
                ForkPasteOutcome::Stay => None,
            }
        }
        CreateStage::ForkPreview => match stage_fork_preview::render(
            ui,
            palette,
            ctx,
            &mut orchestrator.create_screen_state,
        ) {
            ForkPreviewOutcome::Back => Some(CreateRequest::ForkPreviewBack),
            ForkPreviewOutcome::BeginImport => Some(CreateRequest::ForkBeginImport),
            ForkPreviewOutcome::Stay => None,
        },
        CreateStage::ForkDownload => match stage_fork_download::render_live(ui, orchestrator) {
            ForkDownloadOutcome::Cancel => Some(CreateRequest::ForkDownloadCancel),
            ForkDownloadOutcome::Import => orchestrator
                .active_install_modlist_id
                .clone()
                .map(CreateRequest::ForkExtractCompleteRouteToWorkspace),
            ForkDownloadOutcome::Stay => None,
        },
    }
}

fn collect_load_draft_request(
    ctx: &egui::Context,
    palette: ThemePalette,
    orchestrator: &OrchestratorApp,
) -> Option<CreateRequest> {
    let copied = orchestrator
        .create_screen_state
        .load_draft_copied_name
        .clone();
    match load_draft_dialog::render(ctx, palette, &orchestrator.registry, copied.as_deref()) {
        LoadDraftOutcome::Cancelled => Some(CreateRequest::CloseLoadDraft),
        LoadDraftOutcome::Resume(id) => Some(CreateRequest::ResumeWorkspace(id)),
        LoadDraftOutcome::CopyImportCode(id) => Some(CreateRequest::CopyImportCode(id)),
        LoadDraftOutcome::Delete(id) => Some(CreateRequest::ArmDeleteDraft(id)),
        LoadDraftOutcome::Pending => None,
    }
}

fn handle_create_request(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
    request: CreateRequest,
) {
    match request {
        CreateRequest::StartScratch => start_scratch(orchestrator),
        CreateRequest::GoForkPaste => {
            orchestrator.create_screen_state.fork_code.clear();
            orchestrator.create_screen_state.clear_fork_preview();
            orchestrator.create_screen_state.stage = CreateStage::ForkPaste;
        }
        CreateRequest::ForkPasteBack => {
            orchestrator.create_screen_state.clear_fork_preview();
            orchestrator.create_screen_state.stage = CreateStage::Choose;
        }
        CreateRequest::ForkPastePreview => {
            run_fork_preview_parse(&mut orchestrator.create_screen_state);
            orchestrator.create_screen_state.stage = CreateStage::ForkPreview;
        }
        CreateRequest::ForkPreviewBack => {
            orchestrator.create_screen_state.clear_fork_preview();
            orchestrator.create_screen_state.stage = CreateStage::ForkPaste;
        }
        CreateRequest::ForkBeginImport => match fork_pipeline_arm::mint_and_arm(orchestrator) {
            Ok(_) => {
                orchestrator.create_screen_state.stage = CreateStage::ForkDownload;
            }
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "Create fork: mint_and_arm failed: {err}"
                );
            }
        },
        CreateRequest::ForkDownloadCancel => fork_download_cancel(orchestrator),
        CreateRequest::ForkExtractCompleteRouteToWorkspace(id) => {
            fork_extract_complete_route_to_workspace(orchestrator, id);
        }
        CreateRequest::OpenLoadDraft => {
            orchestrator.create_screen_state.load_draft_open = true;
        }
        CreateRequest::CloseLoadDraft => {
            orchestrator.create_screen_state.load_draft_open = false;
            orchestrator.create_screen_state.load_draft_copied_name = None;
            orchestrator.create_screen_state.load_draft_copied_until = None;
            orchestrator.create_screen_state.load_draft_delete_target = None;
        }
        CreateRequest::ResumeWorkspace(id) => {
            orchestrator.create_screen_state.load_draft_open = false;
            orchestrator.create_screen_state.load_draft_copied_name = None;
            orchestrator.create_screen_state.load_draft_copied_until = None;
            orchestrator.create_screen_state.resumed_build_id = Some(id.clone());
            orchestrator.nav = NavDestination::Workspace {
                modlist_id: Some(id),
            };
        }
        CreateRequest::CopyImportCode(id) => copy_import_code(orchestrator, ctx, &id),
        CreateRequest::ArmDeleteDraft(id) => {
            orchestrator.create_screen_state.load_draft_delete_target = Some(id);
        }
    }
}

fn render_load_draft_delete_confirm(orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let Some(id) = orchestrator
        .create_screen_state
        .load_draft_delete_target
        .clone()
    else {
        return;
    };
    let Some(entry) = orchestrator.registry.find(&id).cloned() else {
        orchestrator.create_screen_state.load_draft_delete_target = None;
        return;
    };

    let (title, body) = confirm_delete::delete_dialog_text(&entry);
    let dialog = confirm_delete::delete_confirm("load_draft", &title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.create_screen_state.load_draft_delete_target = None;
            let name = entry.name;
            match operations::remove_entry_and_save(
                &id,
                &orchestrator.registry_store,
                &mut orchestrator.registry,
            ) {
                Ok(Some(target)) => {
                    orchestrator.persistence_cycle.last_saved_registry =
                        orchestrator.registry.clone();
                    orchestrator
                        .notification_manager
                        .info(format!("Deleting \"{}\"\u{2026}", target.name));
                    let rx = operations::spawn_delete_folder_worker(target.dest);
                    orchestrator
                        .pending_folder_deletes
                        .push(PendingFolderDelete {
                            modlist_name: target.name,
                            rx,
                        });
                }
                Ok(None) => {
                    orchestrator.persistence_cycle.last_saved_registry =
                        orchestrator.registry.clone();
                    orchestrator
                        .notification_manager
                        .success(format!("Deleted \"{name}\""));
                }
                Err(err) => {
                    orchestrator
                        .notification_manager
                        .error(format!("Couldn't delete \"{name}\": {err}"));
                }
            }
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.create_screen_state.load_draft_delete_target = None;
        }
        ConfirmOutcome::Pending => {}
    }
}

fn start_scratch(orchestrator: &mut OrchestratorApp) {
    if orchestrator.create_destination_prep_rx.is_some() {
        warn!(
            target = "orchestrator",
            "Create: scratch start ignored because destination prep is already running"
        );
        return;
    }

    let name = orchestrator
        .create_screen_state
        .modlist_name
        .trim()
        .to_string();
    if name.is_empty() {
        warn!(
            target = "orchestrator",
            "Create: `start \u{2192}` with an empty modlist name — ignored (name is required)"
        );
        return;
    }
    let game = orchestrator.create_screen_state.game;
    let dest = {
        let d = orchestrator.create_screen_state.destination.trim();
        if d.is_empty() {
            default_destination(&name)
        } else {
            d.to_string()
        }
    };

    let choice = orchestrator.create_screen_state.destination_choice;
    if destination_choice_requires_worker(choice) {
        let token = orchestrator.next_destination_prep_token(
            DestinationPrepFlow::CreateScratch,
            &dest,
            None,
        );
        orchestrator.create_destination_prep_rx = Some(PendingCreateStart {
            token,
            name,
            destination: dest.clone(),
            game,
            worker: destination_prep::spawn_prepare_destination_worker(PathBuf::from(dest), choice),
        });
        return;
    }

    finish_start_scratch(orchestrator, &name, game, &dest);
}

fn poll_create_destination_prep(orchestrator: &mut OrchestratorApp) {
    let is_current = orchestrator
        .create_destination_prep_rx
        .as_ref()
        .is_some_and(|pending| pending_create_matches_current(orchestrator, pending));
    if !is_current {
        orchestrator.abandon_create_destination_prep();
        return;
    }

    let result = match orchestrator.create_destination_prep_rx.as_ref() {
        Some(pending) => pending.worker.rx.try_recv(),
        None => return,
    };

    match result {
        Ok(Ok(_report)) => {
            if let Some(pending) = orchestrator.create_destination_prep_rx.take() {
                let still_current = pending_create_matches_current(orchestrator, &pending);
                let name = pending.name.clone();
                let game = pending.game;
                let destination = pending.destination.clone();
                orchestrator.complete_destination_prep_worker(pending.worker);
                if still_current {
                    finish_start_scratch(orchestrator, &name, game, &destination);
                }
            }
        }
        Ok(Err(err)) => {
            let pending = orchestrator.create_destination_prep_rx.take();
            if let Some(pending) = pending {
                warn!(
                    target = "orchestrator",
                    "Create: preparing destination {} for {} failed: {err}",
                    pending.destination,
                    pending.name
                );
                orchestrator.complete_destination_prep_worker(pending.worker);
            } else {
                warn!(
                    target = "orchestrator",
                    "Create: preparing destination failed: {err}"
                );
            }
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            let pending = orchestrator.create_destination_prep_rx.take();
            if let Some(pending) = pending {
                warn!(
                    target = "orchestrator",
                    "Create: destination prep worker disconnected for {}", pending.destination
                );
                orchestrator.complete_destination_prep_worker(pending.worker);
            } else {
                warn!(
                    target = "orchestrator",
                    "Create: destination prep worker disconnected"
                );
            }
        }
    }
}

fn pending_create_matches_current(
    orchestrator: &OrchestratorApp,
    pending: &PendingCreateStart,
) -> bool {
    if !matches!(orchestrator.nav, NavDestination::Create)
        || orchestrator.create_screen_state.stage != CreateStage::Choose
    {
        return false;
    }

    let name = orchestrator.create_screen_state.modlist_name.trim();
    if name != pending.name {
        return false;
    }
    let destination = {
        let d = orchestrator.create_screen_state.destination.trim();
        if d.is_empty() {
            default_destination(name)
        } else {
            d.to_string()
        }
    };

    pending.token.matches_context(
        orchestrator.destination_prep_generation,
        DestinationPrepFlow::CreateScratch,
        &destination,
        None,
    ) && pending.game == orchestrator.create_screen_state.game
}

fn finish_start_scratch(orchestrator: &mut OrchestratorApp, name: &str, game: Game, dest: &str) {
    let scratch_mods_folder = match create_scratch_mods_folder(dest, game) {
        Ok(path) => path,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Create: creating scratch mods folder failed: {err}"
            );
            return;
        }
    };

    let author = orchestrator.redesign_settings.user_name.clone();
    let entry = match create_modlist_with_author(
        name,
        game,
        dest,
        Some(author.as_str()),
        &mut orchestrator.registry,
    ) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Create: create_modlist failed: {err}"
            );
            return;
        }
    };

    orchestrator
        .notification_manager
        .success(format!("Created \"{}\"", entry.name));

    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let workspace_state = ModlistWorkspaceState {
        scratch_mods_folder: Some(scratch_mods_folder),
        ..Default::default()
    };
    if let Err(err) = canonical_store.save(&workspace_state) {
        warn!(
            target = "orchestrator",
            "Create: writing canonical workspace.json for {} failed: {err} \
             (the router degrades to an empty workspace)",
            entry.id
        );
    }
    orchestrator
        .workspace_state
        .insert(entry.id.clone(), workspace_state);
    orchestrator
        .workspace_stores
        .insert(entry.id.clone(), canonical_store);

    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "Create: atomic registry persist failed: {err} \
             (entry is in memory + workspace.json is on disk; recoverable)"
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(Instant::now());

    let new_id = entry.id;
    orchestrator.create_screen_state.modlist_name.clear();
    orchestrator.create_screen_state.destination.clear();
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.create_screen_state.resumed_build_id = Some(new_id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(new_id),
    };
}

fn create_scratch_mods_folder(destination: &str, game: Game) -> Result<String, String> {
    let dirs = per_install_dirs::resolve(destination, game);
    std::fs::create_dir_all(&dirs.mods_folder)
        .map_err(|err| format!("create mods folder {}: {err}", dirs.mods_folder.display()))?;
    Ok(dirs.mods_folder.to_string_lossy().to_string())
}

const fn destination_choice_requires_worker(choice: Option<DestChoice>) -> bool {
    matches!(choice, Some(DestChoice::Clear | DestChoice::Backup))
}

fn copy_import_code(orchestrator: &mut OrchestratorApp, ctx: &egui::Context, id: &str) {
    let name = orchestrator
        .registry
        .find(id)
        .map_or_else(|| "modlist".to_string(), |e| e.name.clone());
    if let Some(code) = operations::share_code_for(id, &orchestrator.registry) {
        clipboard::copy(ctx, code);
        orchestrator.create_screen_state.load_draft_copied_name = Some(name);
    } else {
        orchestrator.create_screen_state.load_draft_copied_name = Some(format!(
            "{name}\u{201D} \u{2014} no import code yet \u{201C}"
        ));
    }
    orchestrator.create_screen_state.load_draft_copied_until =
        Some(Instant::now() + Duration::from_millis(COPY_CONFIRM_MS));
}

fn run_fork_preview_parse(state: &mut crate::ui::create::state_create::CreateScreenState) {
    state.clear_fork_preview();
    match preview_modlist_share_code(state.fork_code.trim()) {
        Ok(preview) => {
            state.fork_preview = Some(preview);
            state.fork_active_preview_tab =
                crate::ui::install::state_install::PreviewTab::default();
        }
        Err(msg) => {
            state.fork_preview_parse_error = Some(msg);
        }
    }
}

fn fork_download_cancel(orchestrator: &mut OrchestratorApp) {
    orchestrator.reset_install_screen_to_paste();
    orchestrator.create_screen_state.fork_download_progress =
        crate::ui::install::stage_downloading::DownloadProgress::default();
    orchestrator.create_screen_state.stage = CreateStage::ForkPreview;
}

fn fork_extract_complete_route_to_workspace(orchestrator: &mut OrchestratorApp, id: String) {
    let name = orchestrator
        .registry
        .find(&id)
        .map_or_else(|| "modlist".to_string(), |e| e.name.clone());
    orchestrator
        .notification_manager
        .success(format!("Imported \"{name}\" \u{2014} ready to edit"));

    orchestrator.reset_install_screen_to_paste();
    orchestrator.create_screen_state.fork_code.clear();
    orchestrator.create_screen_state.clear_fork_preview();
    orchestrator.create_screen_state.fork_download_progress =
        crate::ui::install::stage_downloading::DownloadProgress::default();
    orchestrator.create_screen_state.stage = CreateStage::Choose;
    orchestrator.create_screen_state.modlist_name.clear();
    orchestrator.create_screen_state.destination.clear();
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.create_screen_state.resumed_build_id = Some(id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(id),
    };
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
    use crate::registry::store::RegistryStore;
    use egui_toast::ToastKind;

    static CREATETEST_TMP: AtomicU64 = AtomicU64::new(0);

    fn orch_for_create_test() -> OrchestratorApp {
        let mut app = OrchestratorApp::new(false);
        let tmp = std::env::temp_dir().join(format!(
            "bio_createtest_{}_{}.json",
            std::process::id(),
            CREATETEST_TMP.fetch_add(1, Ordering::Relaxed)
        ));
        app.registry_store = RegistryStore::new_with_path(tmp);
        app.registry = ModlistRegistry::default();
        app
    }

    #[test]
    fn destination_choices_that_touch_disk_run_on_worker() {
        assert!(destination_choice_requires_worker(Some(DestChoice::Clear)));
        assert!(destination_choice_requires_worker(Some(DestChoice::Backup)));
        assert!(!destination_choice_requires_worker(Some(
            DestChoice::Continue
        )));
        assert!(!destination_choice_requires_worker(None));
    }

    #[test]
    fn fork_import_complete_pushes_success_toast() {
        let mut app = orch_for_create_test();
        app.registry.entries.push(ModlistEntry {
            id: "FORKTEST00000".to_string(),
            name: "Imported Fork".to_string(),
            game: Game::EET,
            state: ModlistState::InProgress,
            ..Default::default()
        });

        fork_extract_complete_route_to_workspace(&mut app, "FORKTEST00000".to_string());

        let history = app.notification_manager.history();
        assert_eq!(
            history.len(),
            1,
            "exactly one notification must be enqueued"
        );
        let record = history.back().unwrap();
        assert_eq!(
            record.kind,
            ToastKind::Success,
            "fork import complete must be a success toast"
        );
        assert_eq!(
            record.text, "Imported \"Imported Fork\" \u{2014} ready to edit",
            "toast text must include the imported modlist name"
        );
    }
}
