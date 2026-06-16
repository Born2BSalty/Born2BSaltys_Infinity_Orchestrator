// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::registry::model::ModlistEntry;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::state_create::CreateStage;
use crate::ui::home::page_home;
use crate::ui::install::page_install;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::{
    DestinationPrepFlow, OrchestratorApp, PendingInstallDestinationPrep,
    PendingWorkspaceDestinationPrep,
};
use crate::ui::orchestrator::registry_error_panel;
use crate::ui::orchestrator::stubs;
use crate::ui::settings::page_settings;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::state_workspace::{ForkMeta, WorkspaceStep, WorkspaceStep2State};
use crate::ui::workspace::step2::step2_resume_scan;
use crate::ui::workspace::{workspace_state_loader, workspace_view};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let previous_nav = orchestrator.last_rendered_nav.clone();
    let palette = orchestrator.theme_palette;

    if let Some(err) = orchestrator.registry_error.as_ref() {
        registry_error_panel::render_registry_error(
            ui,
            palette,
            err,
            orchestrator.registry_backup_path.as_ref(),
        );
        orchestrator.last_rendered_nav = orchestrator.nav.clone();
        return;
    }

    flush_workspace_on_nav_away(orchestrator);
    invalidate_destination_prep_on_route_change(orchestrator);

    reset_completed_install_route_on_nav_away(orchestrator);
    reset_completed_install_route_on_enter_install(orchestrator, &previous_nav);

    clear_pending_reinstall_on_nav_away_from_install(orchestrator);

    let rendered_nav = orchestrator.nav.clone();
    sync_ambient_to_nav(orchestrator);
    match rendered_nav.clone() {
        NavDestination::Home => page_home::render(ui, orchestrator, ctx),
        NavDestination::Install => page_install::render(ui, orchestrator, ctx),
        NavDestination::Create => crate::ui::create::page_create::render(ui, orchestrator, ctx),
        NavDestination::Settings => page_settings::render(ui, orchestrator, ctx),
        NavDestination::Workspace { modlist_id } => match modlist_id {
            Some(id) => render_workspace(ui, orchestrator, &id, ctx),
            None => stubs::render_workspace_stub(ui, palette, None),
        },
    }

    orchestrator.last_rendered_nav = rendered_nav;
}

fn sync_ambient_to_nav(orchestrator: &OrchestratorApp) {
    use crate::install_runtime::active_modlist_source_path;
    match &orchestrator.nav {
        NavDestination::Workspace {
            modlist_id: Some(id),
        } => active_modlist_source_path::set_ambient_for_modlist(id),
        _ => {
            if let Some(pipeline_id) = orchestrator.active_install_modlist_id.as_deref() {
                active_modlist_source_path::set_ambient_for_modlist(pipeline_id);
            } else {
                active_modlist_source_path::clear_ambient();
            }
        }
    }
}

fn render_workspace(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    id: &str,
    ctx: &egui::Context,
) {
    let palette = orchestrator.theme_palette;

    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        render_missing_modlist(ui, palette, id);
        return;
    };
    sync_share_provenance_from_entry(orchestrator, &entry);

    let install_in_progress: Option<String> = None;
    if let Some(running_id) = install_in_progress.as_ref()
        && running_id != id
    {
        orchestrator.nav = NavDestination::Workspace {
            modlist_id: Some(running_id.clone()),
        };
        return;
    }

    if orchestrator.workspace_view.loaded_workspace_id.as_deref() != Some(id) {
        if !orchestrator.workspace_state.contains_key(id) {
            let store = WorkspaceStore::new_for_id(id);
            let loaded = match store.load() {
                Ok(ws) => ws,
                Err(err) => {
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

        orchestrator.workspace_view.modlist_id = id.to_string();
        orchestrator
            .workspace_view
            .modlist_name
            .clone_from(&entry.name);
        orchestrator.workspace_view.game = entry.game;
        orchestrator.workspace_view.current_step = WorkspaceStep::Step2;
        orchestrator.workspace_view.completed_steps.clear();
        orchestrator.workspace_view.step2 =
            crate::ui::workspace::state_workspace::WorkspaceStep2State::default();
        orchestrator.workspace_view.loaded_workspace_id = Some(id.to_string());
        orchestrator.workspace_view.fork_meta = fork_meta_from_entry(&entry);

        step2_resume_scan::maybe_trigger_resume_scan(orchestrator, &workspace);
    }

    workspace_view::render(ui, orchestrator, id, ctx);
}

pub(crate) const fn restore_pending(step2: &WorkspaceStep2State) -> bool {
    step2.rescan_snapshot.is_some() || step2.resume_pending
}

fn flush_workspace_on_nav_away(orchestrator: &mut OrchestratorApp) {
    let Some(id) = orchestrator.workspace_view.loaded_workspace_id.clone() else {
        return;
    };
    if let NavDestination::Workspace {
        modlist_id: Some(cur),
    } = &orchestrator.nav
        && cur == &id
    {
        return;
    }

    if restore_pending(&orchestrator.workspace_view.step2) {
        orchestrator.workspace_view.loaded_workspace_id = None;
        return;
    }

    workspace_state_loader::sync_step3_from_step2_if_changed(&mut orchestrator.wizard_state);

    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );
    orchestrator
        .workspace_state
        .insert(id.clone(), extracted.clone());

    if let Some(store) = orchestrator.workspace_stores.get(&id) {
        match store.save(&extracted) {
            Ok(()) => {
                orchestrator
                    .persistence_cycle
                    .last_saved_workspaces
                    .insert(id.clone(), extracted);
            }
            Err(err) => warn!(
                target = "orchestrator",
                "nav-away workspace flush for {id} failed: {err} \
                 (in-memory state retained; on-exit flush_all is the backstop)"
            ),
        }
    } else {
        warn!(
            target = "orchestrator",
            "nav-away flush: no WorkspaceStore registered for {id} \
             (state kept in memory; on-exit flush_all is the backstop)"
        );
    }

    orchestrator.workspace_view.loaded_workspace_id = None;
}

fn clear_pending_reinstall_on_nav_away_from_install(orchestrator: &mut OrchestratorApp) {
    if orchestrator.pending_reinstall_id.is_none()
        && orchestrator.active_install_modlist_id.is_none()
    {
        return;
    }
    if matches!(orchestrator.nav, NavDestination::Install) {
        return;
    }
    if matches!(orchestrator.nav, NavDestination::Create)
        && orchestrator.create_screen_state.stage == CreateStage::ForkDownload
    {
        return;
    }
    if orchestrator.wizard_state.step5.install_running
        || orchestrator.wizard_state.step5.start_install_requested
        || orchestrator.wizard_state.step5.prep_running
    {
        return;
    }
    if orchestrator.pending_reinstall_id.take().is_some() {
        tracing::debug!(
            target = "orchestrator",
            "Reinstall cancelled (nav-away from Install before Install-click); \
             pending_reinstall_id cleared — modlist stays Installed (SPEC §3.1)"
        );
    }
    if orchestrator.active_install_modlist_id.take().is_some() {
        tracing::debug!(
            target = "orchestrator",
            "Install-Modlist install did not reach a clean exit \
             (nav-away from Install); active_install_modlist_id cleared — a \
             registered fresh-paste entry stays InProgress on Home (only the \
             clean-exit anchor is dropped, not the entry; SPEC §13.1)"
        );
    }
}

fn invalidate_destination_prep_on_route_change(orchestrator: &mut OrchestratorApp) {
    if orchestrator.create_destination_prep_rx.is_some()
        && (!matches!(orchestrator.nav, NavDestination::Create)
            || orchestrator.create_screen_state.stage != CreateStage::Choose)
    {
        orchestrator.abandon_create_destination_prep();
    }

    if orchestrator
        .install_destination_prep_rx
        .as_ref()
        .is_some_and(|pending| !install_destination_prep_route_matches(orchestrator, pending))
    {
        orchestrator.abandon_install_destination_prep();
    }

    if orchestrator
        .workspace_destination_prep_rx
        .as_ref()
        .is_some_and(|pending| !workspace_destination_prep_route_matches(orchestrator, pending))
    {
        orchestrator.abandon_workspace_destination_prep();
        orchestrator.wizard_state.step5.prep_running = false;
    }
}

fn install_destination_prep_route_matches(
    orchestrator: &OrchestratorApp,
    pending: &PendingInstallDestinationPrep,
) -> bool {
    match pending.token.flow {
        DestinationPrepFlow::InstallPipeline => matches!(orchestrator.nav, NavDestination::Install),
        DestinationPrepFlow::CreateForkDownload => {
            matches!(orchestrator.nav, NavDestination::Create)
                && orchestrator.create_screen_state.stage == CreateStage::ForkDownload
        }
        DestinationPrepFlow::CreateScratch | DestinationPrepFlow::WorkspaceStep5 => false,
    }
}

fn workspace_destination_prep_route_matches(
    orchestrator: &OrchestratorApp,
    pending: &PendingWorkspaceDestinationPrep,
) -> bool {
    matches!(
        &orchestrator.nav,
        NavDestination::Workspace {
            modlist_id: Some(id)
        } if id == &pending.modlist_id
    )
}

fn reset_completed_install_route_on_nav_away(orchestrator: &mut OrchestratorApp) {
    if !should_reset_completed_install_route_on_nav_away(
        &orchestrator.nav,
        orchestrator.post_install_reset_gate.is_pending(),
        &orchestrator.wizard_state,
        orchestrator.step5_prep_rx.is_some() || orchestrator.step5_pending_start.is_some(),
    ) {
        return;
    }

    reset_completed_install_runtime(orchestrator);
}

fn reset_completed_install_route_on_enter_install(
    orchestrator: &mut OrchestratorApp,
    previous_nav: &NavDestination,
) {
    if !should_reset_completed_install_route_on_enter_install(
        previous_nav,
        &orchestrator.nav,
        orchestrator.post_install_reset_gate.is_pending(),
        &orchestrator.wizard_state,
        orchestrator.step5_prep_rx.is_some() || orchestrator.step5_pending_start.is_some(),
    ) {
        return;
    }

    reset_completed_install_runtime(orchestrator);
}

fn reset_completed_install_runtime(orchestrator: &mut OrchestratorApp) {
    orchestrator.post_install_reset_gate =
        crate::ui::orchestrator::orchestrator_app::PostInstallResetGate::Idle;
    if let Some(term) = orchestrator.step5_terminal.as_mut() {
        term.clear_console();
    }
    orchestrator.step5_terminal = None;
    orchestrator.step5_terminal_error = None;
    orchestrator.step5_console_view =
        crate::ui::step5::state_step5::Step5ConsoleViewState::default();
    orchestrator.step5_prep_rx = None;
    orchestrator.step5_pending_start = None;
    orchestrator.abandon_install_destination_prep();
    orchestrator.abandon_workspace_destination_prep();
    orchestrator.install_running_since = None;
    orchestrator.pending_reinstall_id = None;
    orchestrator.active_install_modlist_id = None;
    orchestrator.install_screen_state.reset_to_paste();
    orchestrator.wizard_state.reset_workflow_keep_step1();
    crate::install_runtime::settings_sanitizer::sanitize_step1_for_settings_persistence(
        &mut orchestrator.wizard_state.step1,
        &orchestrator.bio_settings_last_saved.step1,
    );
}

const fn should_reset_completed_install_route_on_nav_away(
    nav: &NavDestination,
    pending_post_install_reset: bool,
    wizard_state: &crate::app::state::WizardState,
    has_pending_step5_start: bool,
) -> bool {
    !matches!(nav, NavDestination::Install)
        && should_reset_completed_install_route(
            pending_post_install_reset,
            wizard_state,
            has_pending_step5_start,
        )
}

const fn should_reset_completed_install_route_on_enter_install(
    previous_nav: &NavDestination,
    current_nav: &NavDestination,
    pending_post_install_reset: bool,
    wizard_state: &crate::app::state::WizardState,
    has_pending_step5_start: bool,
) -> bool {
    matches!(current_nav, NavDestination::Install)
        && !matches!(previous_nav, NavDestination::Install)
        && should_reset_completed_install_route(
            pending_post_install_reset,
            wizard_state,
            has_pending_step5_start,
        )
}

const fn should_reset_completed_install_route(
    pending_post_install_reset: bool,
    wizard_state: &crate::app::state::WizardState,
    has_pending_step5_start: bool,
) -> bool {
    pending_post_install_reset && !step5_attempt_in_progress(wizard_state, has_pending_step5_start)
}

const fn step5_attempt_in_progress(
    wizard_state: &crate::app::state::WizardState,
    has_pending_step5_start: bool,
) -> bool {
    wizard_state.step5.start_install_requested
        || wizard_state.step5.prep_running
        || wizard_state.step5.install_running
        || wizard_state.step5.cancel_pending
        || wizard_state.step5.cancel_requested
        || has_pending_step5_start
}

fn fork_meta_from_entry(entry: &ModlistEntry) -> Option<ForkMeta> {
    if entry.forked_from.is_empty() {
        return None;
    }
    let parent = entry.forked_from.last();
    Some(ForkMeta {
        parent_name: parent.map(|p| p.name.clone()).unwrap_or_default(),
        parent_author: parent.map(|p| p.author.clone()).unwrap_or_default(),
        mods: entry.mod_count,
        components: entry.component_count,
        forked_from: entry.forked_from.clone(),
    })
}

fn sync_share_provenance_from_entry(orchestrator: &mut OrchestratorApp, entry: &ModlistEntry) {
    orchestrator.wizard_state.set_modlist_share_provenance(
        Some(entry.name.clone()),
        entry.author.clone(),
        entry.forked_from.clone(),
    );
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};

    fn snap() -> RescanSnapshot {
        RescanSnapshot {
            bgee: vec![RescanSelection {
                tp2_upper: "BG1UB/BG1UB.TP2".to_string(),
                component_id: "0".to_string(),
                selected_order: Some(1),
                wlb_inputs: None,
            }],
            bg2ee: Vec::new(),
        }
    }

    #[test]
    fn fixrun4_resume_pending_blocks_the_save() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: Some(snap()),
            resume_pending: true,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "resume in flight (snapshot + resume_pending) must block extract/save \
             so the empty post-populate order is NOT written over workspace.json"
        );
    }

    #[test]
    fn fixrun4_rescan_snapshot_alone_blocks_the_save() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: Some(snap()),
            resume_pending: false,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "a snapshot in flight (rescan, no resume) must also block the save"
        );
    }

    #[test]
    fn fixrun4_resume_pending_without_snapshot_still_blocks() {
        let step2 = WorkspaceStep2State {
            rescan_snapshot: None,
            resume_pending: true,
            ..Default::default()
        };
        assert!(
            restore_pending(&step2),
            "resume_pending set (snapshot taken, reconcile mid-flight) must still block"
        );
    }

    #[test]
    fn fixrun4_no_restore_pending_lets_the_save_proceed() {
        let step2 = WorkspaceStep2State::default();
        assert!(
            step2.rescan_snapshot.is_none() && !step2.resume_pending,
            "precondition: nothing pending"
        );
        assert!(
            !restore_pending(&step2),
            "no restore pending ⇒ guard must NOT fire (a genuine deselect-all \
             edit still persists; the guard must not over-block)"
        );
    }

    #[test]
    fn completed_install_route_resets_only_after_nav_away() {
        let wizard = crate::app::state::WizardState::default();

        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Install,
            true,
            &wizard,
            false
        ));
        assert!(should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            true,
            &wizard,
            false
        ));

        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            false,
            &wizard,
            false
        ));
    }

    #[test]
    fn failed_install_route_does_not_reset_on_nav_away() {
        let wizard = crate::app::state::WizardState::default();
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            false,
            &wizard,
            false
        ));
    }

    #[test]
    fn unfinished_or_running_install_route_does_not_reset_on_nav_away() {
        let mut wizard = crate::app::state::WizardState::default();
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            false,
            &wizard,
            false
        ));

        wizard.step5.install_running = true;
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            true,
            &wizard,
            false
        ));
    }

    #[test]
    fn completed_install_route_resets_on_enter_install_from_other_page() {
        let wizard = crate::app::state::WizardState::default();

        assert!(should_reset_completed_install_route_on_enter_install(
            &NavDestination::Settings,
            &NavDestination::Install,
            true,
            &wizard,
            false
        ));
        assert!(should_reset_completed_install_route_on_enter_install(
            &NavDestination::Home,
            &NavDestination::Install,
            true,
            &wizard,
            false
        ));
        assert!(!should_reset_completed_install_route_on_enter_install(
            &NavDestination::Install,
            &NavDestination::Install,
            true,
            &wizard,
            false
        ));
        assert!(!should_reset_completed_install_route_on_enter_install(
            &NavDestination::Settings,
            &NavDestination::Create,
            true,
            &wizard,
            false
        ));
    }

    #[test]
    fn stale_success_does_not_reset_when_new_attempt_is_pending() {
        let mut wizard = crate::app::state::WizardState::default();

        wizard.step5.start_install_requested = true;
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            true,
            &wizard,
            false
        ));
        assert!(!should_reset_completed_install_route_on_enter_install(
            &NavDestination::Settings,
            &NavDestination::Install,
            true,
            &wizard,
            false
        ));

        wizard.step5.start_install_requested = false;
        wizard.step5.prep_running = true;
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            true,
            &wizard,
            false
        ));

        wizard.step5.prep_running = false;
        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Home,
            true,
            &wizard,
            true
        ));
        assert!(!should_reset_completed_install_route_on_enter_install(
            &NavDestination::Settings,
            &NavDestination::Install,
            true,
            &wizard,
            true
        ));
    }

    #[test]
    fn fork_then_modify_does_not_falsely_reset() {
        let mut wizard = crate::app::state::WizardState::default();
        wizard.step5.last_exit_code = Some(0);
        wizard.step5.last_install_failed = false;
        wizard.step5.install_running = false;

        assert!(!should_reset_completed_install_route_on_nav_away(
            &NavDestination::Workspace {
                modlist_id: Some("forked-id".to_string())
            },
            false,
            &wizard,
            false
        ));
    }

    use crate::app::mod_downloads::AMBIENT_TEST_LOCK;

    struct AmbientGuard(Option<std::path::PathBuf>);
    impl AmbientGuard {
        fn acquire() -> Self {
            Self(crate::app::mod_downloads::active_modlist_dir())
        }
    }
    impl Drop for AmbientGuard {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    #[test]
    fn sync_ambient_workspace_nav_sets_ambient_to_workspace_id() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let mut app = OrchestratorApp::new(false);
        app.nav = NavDestination::Workspace {
            modlist_id: Some("WS-AMBIENT-A".to_string()),
        };

        sync_ambient_to_nav(&app);

        let dir = crate::app::mod_downloads::active_modlist_dir();
        assert!(dir.is_some(), "ambient must be set for workspace nav");
        assert!(
            dir.unwrap().to_string_lossy().contains("WS-AMBIENT-A"),
            "ambient must point at the workspace modlist"
        );
    }

    #[test]
    fn sync_ambient_non_workspace_no_pipeline_clears_ambient() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let mut app = OrchestratorApp::new(false);
        app.nav = NavDestination::Create;
        app.active_install_modlist_id = None;

        sync_ambient_to_nav(&app);

        assert!(
            crate::app::mod_downloads::active_modlist_dir().is_none(),
            "ambient must be cleared when no workspace and no active pipeline"
        );
    }

    #[test]
    fn sync_ambient_non_workspace_with_pipeline_pins_to_pipeline_modlist() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let mut app = OrchestratorApp::new(false);
        app.nav = NavDestination::Create;
        app.active_install_modlist_id = Some("FORK-PIPELINE-ID".to_string());

        sync_ambient_to_nav(&app);

        let dir = crate::app::mod_downloads::active_modlist_dir();
        assert!(
            dir.is_some(),
            "ambient must remain set while a pipeline is active on non-workspace nav"
        );
        assert!(
            dir.unwrap().to_string_lossy().contains("FORK-PIPELINE-ID"),
            "ambient must point at the active pipeline modlist, not global"
        );
    }

    #[test]
    fn sync_ambient_workspace_wins_over_pipeline() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let mut app = OrchestratorApp::new(false);
        app.nav = NavDestination::Workspace {
            modlist_id: Some("OPEN-WS".to_string()),
        };
        app.active_install_modlist_id = Some("OTHER-PIPELINE".to_string());

        sync_ambient_to_nav(&app);

        let dir = crate::app::mod_downloads::active_modlist_dir().expect("ambient must be set");
        assert!(
            dir.to_string_lossy().contains("OPEN-WS"),
            "workspace nav takes priority over the active pipeline id"
        );
    }
}
