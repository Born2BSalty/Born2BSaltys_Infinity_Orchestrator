// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use eframe::egui;
use tracing::warn;

use crate::install_runtime::replaced_owners::{self, HeldOwners};
use crate::install_runtime::{destination_prep, per_install_dirs};
use crate::registry::destination_claim::{
    ClaimContext, DestinationClaim, resolve_destination_claim,
};
use crate::registry::model::Game;
use crate::registry::operations_create::create_modlist_with_author;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::{ModlistWorkspaceState, ModsSource};
use crate::ui::create::create_log_import::CreateLogImport;
use crate::ui::create::destination_default::default_destination;
use crate::ui::create::load_draft_dialog::{self, LoadDraftOutcome};
use crate::ui::create::stage_choose::{self, ChooseOutcome};
use crate::ui::create::state_create::{CreateMode, CreateScreenState};
use crate::ui::home::confirm_delete;
use crate::ui::install::state_install::DestChoice;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::{
    DestinationPrepFlow, OrchestratorApp, PendingCreateStart,
};
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::shared::redesign_tokens::ThemePalette;

enum CreateRequest {
    StartScratch,
    OpenLoadDraft,
    CloseLoadDraft,
    ResumeWorkspace(String),
    ArmDeleteDraft(String),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    poll_create_destination_prep(orchestrator);

    let mut request = collect_stage_request(ui, palette, orchestrator);

    if orchestrator.create_screen_state.load_draft_open {
        request = collect_load_draft_request(ctx, palette, orchestrator).or(request);

        render_load_draft_delete_confirm(orchestrator, ctx);
    }

    if let Some(req) = request {
        handle_create_request(orchestrator, req);
    }
}

fn collect_stage_request(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) -> Option<CreateRequest> {
    let destination_prep_running = orchestrator.create_destination_prep_rx.is_some();
    let ctx = stage_choose::ChooseCtx {
        registry: &orchestrator.registry,
        active_install_id: orchestrator.active_install_modlist_id.as_deref(),
        step1: &orchestrator.wizard_state.step1,
        saved_step1: &orchestrator.bio_settings_last_saved.step1,
    };
    match stage_choose::render(
        ui,
        palette,
        &mut orchestrator.create_screen_state,
        destination_prep_running,
        &ctx,
    ) {
        ChooseOutcome::StartScratch => Some(CreateRequest::StartScratch),
        ChooseOutcome::OpenLoadDraft => Some(CreateRequest::OpenLoadDraft),
        ChooseOutcome::Stay => None,
    }
}

fn collect_load_draft_request(
    ctx: &egui::Context,
    palette: ThemePalette,
    orchestrator: &OrchestratorApp,
) -> Option<CreateRequest> {
    match load_draft_dialog::render(ctx, palette, &orchestrator.registry) {
        LoadDraftOutcome::Cancelled => Some(CreateRequest::CloseLoadDraft),
        LoadDraftOutcome::Resume(id) => Some(CreateRequest::ResumeWorkspace(id)),
        LoadDraftOutcome::Delete(id) => Some(CreateRequest::ArmDeleteDraft(id)),
        LoadDraftOutcome::Pending => None,
    }
}

fn handle_create_request(orchestrator: &mut OrchestratorApp, request: CreateRequest) {
    match request {
        CreateRequest::StartScratch => start_scratch(orchestrator),
        CreateRequest::OpenLoadDraft => {
            orchestrator.create_screen_state.load_draft_open = true;
        }
        CreateRequest::CloseLoadDraft => {
            orchestrator.create_screen_state.load_draft_open = false;
            orchestrator.create_screen_state.load_draft_delete_target = None;
        }
        CreateRequest::ResumeWorkspace(id) => {
            orchestrator.create_screen_state.load_draft_open = false;
            orchestrator.create_screen_state.resumed_build_id = Some(id.clone());
            orchestrator.nav = NavDestination::Workspace {
                modlist_id: Some(id),
            };
        }
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
            crate::ui::home::delete_modlist::delete_confirmed_modlist(orchestrator, &entry);
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

    let log_import = click_time_log_import(&orchestrator.create_screen_state);
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
            log_import,
            worker: destination_prep::spawn_prepare_destination_worker(PathBuf::from(dest), choice),
        });
        return;
    }

    finish_start_scratch(orchestrator, &name, game, &dest, log_import);
}

fn click_time_log_import(state: &CreateScreenState) -> bool {
    state.mode == CreateMode::FromLogs
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
                let log_import = pending.log_import;
                orchestrator.complete_destination_prep_worker(pending.worker);
                if still_current {
                    finish_start_scratch(orchestrator, &name, game, &destination, log_import);
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
    if !matches!(orchestrator.nav, NavDestination::Create) {
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

fn scratch_claim(
    orchestrator: &mut OrchestratorApp,
    dest: &str,
) -> Result<Option<HeldOwners>, String> {
    let claim = resolve_destination_claim(
        &orchestrator.registry,
        &ClaimContext {
            destination: dest,
            held_id: None,
            installing_id: orchestrator.active_install_modlist_id.as_deref(),
        },
    );
    match claim {
        DestinationClaim::Free | DestinationClaim::Adopt(_) => Ok(None),
        DestinationClaim::Replace(ids) => replaced_owners::hold_owners(orchestrator, &ids)
            .map(Some)
            .map_err(|err| format!("the modlist at {dest} could not be replaced: {err}")),
        DestinationClaim::Refused(refusal) => Err(refusal.message(&orchestrator.registry)),
    }
}

fn finish_start_scratch(
    orchestrator: &mut OrchestratorApp,
    name: &str,
    game: Game,
    dest: &str,
    log_import: bool,
) {
    let mut held = match scratch_claim(orchestrator, dest) {
        Ok(h) => h,
        Err(msg) => {
            warn!(target = "orchestrator", "Create scratch: refused — {msg}");
            return;
        }
    };

    let scratch_mods_folder = match create_scratch_mods_folder(dest, game) {
        Ok(path) => path,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Create: creating scratch mods folder failed: {err}"
            );
            if let Some(h) = held.take() {
                replaced_owners::restore_owners(orchestrator, h);
            }
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
            if let Some(h) = held.take() {
                replaced_owners::restore_owners(orchestrator, h);
            }
            return;
        }
    };

    orchestrator
        .notification_manager
        .success(format!("Created \"{}\"", entry.name));

    let source = ModsSource::GlobalModsFolder;

    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let workspace_state = ModlistWorkspaceState {
        scratch_mods_folder: Some(scratch_mods_folder),
        mods_source: source,
        last_rescanned_mods_source: source,
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

    if let Some(h) = held.take() {
        replaced_owners::finalize_owners(h, &entry.id);
    }

    let new_id = entry.id;
    orchestrator.create_log_import = log_import.then(|| CreateLogImport {
        modlist_id: new_id.clone(),
    });
    clear_create_form(&mut orchestrator.create_screen_state);
    orchestrator.create_screen_state.resumed_build_id = Some(new_id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(new_id),
    };
}

fn clear_create_form(state: &mut CreateScreenState) {
    state.modlist_name.clear();
    state.destination.clear();
    state.destination_choice = None;
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use egui_toast::ToastKind;

    fn orch_for_create_test() -> OrchestratorApp {
        OrchestratorApp::new_isolated_for_test("createtest")
    }

    #[test]
    fn created_list_starts_on_the_global_mods_folder_even_when_none_is_set() {
        let dest = TempDestGuard::new("globaldefault");
        let mut app = orch_for_create_test();
        app.redesign_settings.user_name = "@tester".to_string();
        let global = app
            .settings_store
            .load()
            .map(|s| s.step1.effective_global_mods_folder().to_string())
            .unwrap_or_default();
        assert_eq!(global.trim().len(), 0, "isolated app has no mods folder");

        finish_start_scratch(&mut app, "Fresh", Game::BGEE, &dest.as_string(), false);

        let fresh = app
            .registry
            .entries
            .iter()
            .find(|e| e.name == "Fresh")
            .expect("the list is created");
        let workspace = app
            .workspace_state
            .get(&fresh.id)
            .expect("the workspace state is seeded");
        assert_eq!(workspace.mods_source, ModsSource::GlobalModsFolder);
        assert_eq!(
            workspace.last_rescanned_mods_source,
            ModsSource::GlobalModsFolder
        );
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

        crate::install_runtime::fork_route::extract_complete_route_to_workspace(
            &mut app,
            "FORKTEST00000".to_string(),
        );

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

    struct TempDestGuard(PathBuf);

    static TEMP_DEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    impl TempDestGuard {
        fn new(tag: &str) -> Self {
            Self(std::env::temp_dir().join(format!(
                "bio_create_scratch_test_{tag}_{}_{}",
                std::process::id(),
                TEMP_DEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )))
        }

        fn as_string(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempDestGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn scratch_take_over_holds_the_old_owner_and_removes_its_data_dir() {
        let dest = TempDestGuard::new("takeover");
        let mut app = orch_for_create_test();
        app.registry.entries.push(ModlistEntry {
            id: "OLD000000001".to_string(),
            name: "Old scratch".to_string(),
            game: Game::EET,
            destination_folder: dest.as_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let old_data_dir = crate::registry::store_workspace::modlist_data_dir("OLD000000001");
        std::fs::create_dir_all(&old_data_dir).expect("seed the old data dir");
        app.redesign_settings.user_name = "@tester".to_string();

        finish_start_scratch(&mut app, "Fresh", Game::BGEE, &dest.as_string(), false);

        assert!(app.registry.find("OLD000000001").is_none());
        assert!(!old_data_dir.exists());
        let fresh: Vec<_> = app
            .registry
            .entries
            .iter()
            .filter(|e| e.name == "Fresh")
            .collect();
        assert_eq!(fresh.len(), 1);
        assert_eq!(
            app.nav,
            NavDestination::Workspace {
                modlist_id: Some(fresh[0].id.clone())
            }
        );
    }

    #[test]
    fn scratch_mint_failure_restores_the_old_owner() {
        let dest = TempDestGuard::new("mint-failure");
        std::fs::write(&dest.0, b"blocker").expect("seed a blocking file");
        let mut app = orch_for_create_test();
        app.registry.entries.push(ModlistEntry {
            id: "OLD000000002".to_string(),
            name: "Old scratch 2".to_string(),
            game: Game::EET,
            destination_folder: dest.as_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let old_data_dir = crate::registry::store_workspace::modlist_data_dir("OLD000000002");
        std::fs::create_dir_all(&old_data_dir).expect("seed the old data dir");
        app.redesign_settings.user_name = "@tester".to_string();

        finish_start_scratch(&mut app, "Fresh 2", Game::BGEE, &dest.as_string(), false);

        let restored = app
            .registry
            .entries
            .iter()
            .position(|e| e.id == "OLD000000002")
            .expect("the old owner is restored");
        assert_eq!(restored, 0);
        assert!(old_data_dir.exists());
    }

    fn app_ready_to_start(mode: CreateMode, game: Game, dest: &TempDestGuard) -> OrchestratorApp {
        let mut app = orch_for_create_test();
        app.redesign_settings.user_name = "@tester".to_string();
        let state = &mut app.create_screen_state;
        state.modlist_name = "From Logs".to_string();
        state.game = game;
        state.destination = dest.as_string();
        state.mode = mode;
        app
    }

    fn new_list_id(app: &OrchestratorApp) -> String {
        match &app.nav {
            NavDestination::Workspace {
                modlist_id: Some(id),
            } => id.clone(),
            other => panic!("expected the new list's workspace, got {other:?}"),
        }
    }

    #[test]
    fn start_from_logs_leaves_a_note_for_the_new_list() {
        let dest = TempDestGuard::new("fromlogs");
        let mut app = app_ready_to_start(CreateMode::FromLogs, Game::BGEE, &dest);

        start_scratch(&mut app);

        let id = new_list_id(&app);
        assert_eq!(
            app.create_log_import,
            Some(CreateLogImport { modlist_id: id })
        );
        let state = &app.create_screen_state;
        assert_eq!(state.mode, CreateMode::FromLogs);
        assert_eq!(state.modlist_name.len(), 0);
    }

    #[test]
    fn start_in_logs_mode_needs_no_log() {
        let dest = TempDestGuard::new("nolog");
        let mut app = orch_for_create_test();
        app.create_screen_state.modlist_name = "No Logs Yet".to_string();
        app.create_screen_state.destination = dest.as_string();
        assert_eq!(app.create_screen_state.mode, CreateMode::FromLogs);

        start_scratch(&mut app);

        let id = new_list_id(&app);
        assert!(app.registry.find(&id).is_some());
        assert_eq!(
            app.create_log_import,
            Some(CreateLogImport { modlist_id: id })
        );
    }

    #[test]
    fn start_from_scratch_leaves_no_note() {
        let dest = TempDestGuard::new("fromscratch");
        let mut app = app_ready_to_start(CreateMode::FromScratch, Game::EET, &dest);

        start_scratch(&mut app);

        let _ = new_list_id(&app);
        assert_eq!(app.create_log_import, None);
        assert_eq!(app.create_screen_state.mode, CreateMode::FromScratch);
    }

    #[test]
    fn start_scratch_without_a_name_creates_the_list() {
        let dest = TempDestGuard::new("noname");
        let mut app = orch_for_create_test();
        app.create_screen_state.modlist_name = "No Author Build".to_string();
        app.create_screen_state.destination = dest.as_string();
        app.redesign_settings.user_name.clear();

        start_scratch(&mut app);

        assert_eq!(app.registry.entries.len(), 1);
        let entry = &app.registry.entries[0];
        assert_eq!(entry.name, "No Author Build");
        assert_eq!(entry.author, None);
        let workspace = app
            .workspace_state
            .get(&entry.id)
            .expect("the workspace state is seeded");
        assert!(
            workspace
                .scratch_mods_folder
                .as_deref()
                .is_some_and(|folder| !folder.trim().is_empty())
        );
        assert!(
            app.notification_manager
                .history()
                .iter()
                .all(|record| record.kind != ToastKind::Error)
        );
    }
}
