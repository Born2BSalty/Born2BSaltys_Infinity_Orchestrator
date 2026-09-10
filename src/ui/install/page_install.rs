// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::install_runtime::fork_pipeline_arm::{self, ForkArmRequest};
use crate::install_runtime::{fork_route, start_hooks};
use crate::registry::share_export;
use crate::ui::install::drawers;
use crate::ui::install::gallery::catalog::{self, GalleryEntry};
use crate::ui::install::stage_details::{self, DetailsHeader, DetailsOutcome};
use crate::ui::install::stage_downloading::{self, DownloadScreenCopy, DownloadingOutcome};
use crate::ui::install::stage_fork_download::{self, ForkDownloadOutcome};
use crate::ui::install::stage_gallery::{self, GalleryOutcome};
use crate::ui::install::stage_installing::{self, StageInstallingOutcome};
use crate::ui::install::stage_paste::{self, PasteOutcome};
use crate::ui::install::stage_review;
use crate::ui::install::state_install::{
    DrawerKind, DrawerState, InstallScreenState, InstallStage, PipelineKind, ReviewOrigin,
};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::NotificationManager;
use crate::ui::shared::redesign_tokens::ThemePalette;

#[derive(Debug, PartialEq, Eq)]
enum InstallRequest {
    Stage(InstallStage),
    Nav(NavDestination),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    let request = match orchestrator.install_screen_state.stage {
        InstallStage::Gallery => gallery_stage(
            ui,
            palette,
            &mut orchestrator.install_screen_state,
            &mut orchestrator.notification_manager,
        ),
        InstallStage::Details => details_stage(ui, palette, orchestrator, ctx),
        InstallStage::Paste => paste_stage(ui, palette, &mut orchestrator.install_screen_state),
        InstallStage::Downloading => downloading_stage(ui, orchestrator),
        InstallStage::InstallingStub => installing_stage(ui, orchestrator),
    };

    if let Some(req) = request {
        match req {
            InstallRequest::Stage(stage) => {
                orchestrator.install_screen_state.stage = stage;
            }
            InstallRequest::Nav(dest) => {
                orchestrator.nav = dest;
            }
        }
        ctx.request_repaint();
    }
}

fn gallery_stage(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    match stage_gallery::render(ui, palette, state) {
        GalleryOutcome::OpenPaste => Some(open_paste_from_gallery(state)),
        GalleryOutcome::OpenDetails(index) => {
            state.gallery.selected = Some(index);
            let entry = selected_entry(state)?;
            open_details_from_gallery_entry(entry, state, notification_manager)
        }
        GalleryOutcome::Stay => None,
    }
}

fn details_stage(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
) -> Option<InstallRequest> {
    let origin = orchestrator.install_screen_state.review.origin;
    let Some(counts) = orchestrator.install_screen_state.inside_counts().cloned() else {
        orchestrator.install_screen_state.gallery.selected = None;
        orchestrator.install_screen_state.clear_preview();
        return Some(InstallRequest::Stage(InstallStage::Gallery));
    };
    let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
        orchestrator.install_screen_state.gallery.selected = None;
        orchestrator.install_screen_state.clear_preview();
        return Some(InstallRequest::Stage(InstallStage::Gallery));
    };
    let header = if origin == ReviewOrigin::Details {
        let Some(entry) = selected_entry(&orchestrator.install_screen_state) else {
            orchestrator.install_screen_state.gallery.selected = None;
            orchestrator.install_screen_state.clear_preview();
            return Some(InstallRequest::Stage(InstallStage::Gallery));
        };
        DetailsHeader::from_gallery_entry(entry, &preview)
    } else {
        DetailsHeader::from_preview(
            &preview,
            &orchestrator.install_screen_state.review.name,
            origin,
        )
    };
    let availability = stage_review::modify_choice_available(origin, preview.allow_auto_install);
    let mut fork_info_open = orchestrator.install_screen_state.fork_info_open;

    let outcome = stage_details::render(
        ui,
        palette,
        ctx,
        &header,
        &counts,
        availability,
        &mut fork_info_open,
    );
    orchestrator.install_screen_state.fork_info_open = fork_info_open;

    let stage_request = match outcome {
        DetailsOutcome::Back => Some(details_back(
            &mut orchestrator.install_screen_state,
            &mut orchestrator.pending_reinstall_id,
        )),
        DetailsOutcome::OpenDrawer(kind) => {
            orchestrator.install_screen_state.drawer.open = Some(kind);
            None
        }
        DetailsOutcome::Stay => None,
    };

    stage_request.or_else(|| drawer_request(orchestrator, ctx, palette))
}

fn drawer_request(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
    palette: ThemePalette,
) -> Option<InstallRequest> {
    match drawers::render(
        ctx,
        palette,
        &mut orchestrator.install_screen_state,
        &orchestrator.registry,
        orchestrator.pending_reinstall_id.as_deref(),
    ) {
        drawers::DrawerOutcome::BeginInstall => {
            Some(InstallRequest::Stage(begin_install(orchestrator)))
        }
        drawers::DrawerOutcome::BeginImport => {
            begin_import(orchestrator).map(InstallRequest::Stage)
        }
        drawers::DrawerOutcome::Stay => None,
    }
}

fn details_back(
    state: &mut InstallScreenState,
    pending_reinstall_id: &mut Option<String>,
) -> InstallRequest {
    match state.review.origin {
        ReviewOrigin::Details => {
            state.clear_preview();
            state.import_code.clear();
            state.gallery.selected = None;
            InstallRequest::Stage(InstallStage::Gallery)
        }
        ReviewOrigin::Paste => {
            state.drawer = DrawerState::default();
            state.fork_info_open = false;
            InstallRequest::Stage(InstallStage::Paste)
        }
        ReviewOrigin::Reinstall => {
            state.clear_preview();
            *pending_reinstall_id = None;
            InstallRequest::Stage(InstallStage::Gallery)
        }
    }
}

fn open_paste_from_gallery(state: &mut InstallScreenState) -> InstallRequest {
    state.clear_preview();
    state.import_code.clear();
    InstallRequest::Stage(InstallStage::Paste)
}

fn paste_stage(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> Option<InstallRequest> {
    match stage_paste::render(ui, palette, state) {
        PasteOutcome::Advance(InstallStage::Details) => {
            run_preview_parse(state);
            if state.preview_parse_error.is_some() {
                return None;
            }
            state.review.origin = ReviewOrigin::Paste;
            state.review.name = state
                .parsed_preview
                .as_ref()
                .map_or_else(String::new, |preview| {
                    stage_review::display_name("", preview)
                });
            state.review.modify = false;
            state.gallery.selected = None;
            Some(InstallRequest::Stage(InstallStage::Details))
        }
        PasteOutcome::Advance(next) => Some(InstallRequest::Stage(next)),
        PasteOutcome::Stay => None,
    }
}

fn begin_install(orchestrator: &mut OrchestratorApp) -> InstallStage {
    {
        let state = &mut orchestrator.install_screen_state;
        state.destination = state.destination.trim().to_string();
        state.import_code = state.import_code.trim().to_string();
        state.pipeline_kind = PipelineKind::Install;
        state.drawer.open = None;

        let typed = state.review.name.trim().to_string();
        let current = state
            .parsed_preview
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("")
            .trim()
            .to_string();
        if !typed.is_empty() && typed != current {
            if let Some(preview) = state.parsed_preview.as_mut() {
                preview.name = Some(typed.clone());
            }
            match share_export::set_packed_name(&state.import_code, &typed) {
                Ok(renamed) => state.import_code = renamed,
                Err(err) => {
                    warn!(
                        target = "orchestrator",
                        "Begin Install: could not write the typed name into the share code: {err}"
                    );
                }
            }
        }
    }

    if let Some(reinstall_id) = orchestrator.pending_reinstall_id.clone() {
        let OrchestratorApp {
            wizard_state,
            registry,
            registry_store,
            pending_reinstall_id,
            ..
        } = &mut *orchestrator;
        start_hooks::reinstall_flip_at_install_click(
            &reinstall_id,
            wizard_state,
            registry,
            registry_store,
            pending_reinstall_id,
        );
    }

    InstallStage::Downloading
}

fn begin_import(orchestrator: &mut OrchestratorApp) -> Option<InstallStage> {
    if !orchestrator.ensure_creator_name() {
        return None;
    }
    let preview = orchestrator.install_screen_state.parsed_preview.clone()?;
    let name = orchestrator.install_screen_state.review.name.clone();
    let destination = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();
    let code = orchestrator
        .install_screen_state
        .import_code
        .trim()
        .to_string();
    let choice = orchestrator.install_screen_state.destination_choice;

    match fork_pipeline_arm::mint_and_arm(
        orchestrator,
        &ForkArmRequest {
            preview: &preview,
            name: &name,
            destination: &destination,
            code: &code,
            choice,
        },
    ) {
        Ok(_) => Some(InstallStage::Downloading),
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Details: fork mint_and_arm failed: {err}"
            );
            orchestrator
                .notification_manager
                .error(format!("Could not start the import: {err}"));
            None
        }
    }
}

fn downloading_stage(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
) -> Option<InstallRequest> {
    match orchestrator.install_screen_state.pipeline_kind {
        PipelineKind::Install => {
            match stage_downloading::render_live(ui, orchestrator, DownloadScreenCopy::INSTALL) {
                DownloadingOutcome::Cancel => {
                    orchestrator.reset_install_screen_to_gallery();
                    Some(InstallRequest::Stage(InstallStage::Gallery))
                }
                DownloadingOutcome::Advance => {
                    Some(InstallRequest::Stage(InstallStage::InstallingStub))
                }
                DownloadingOutcome::Stay => None,
            }
        }
        PipelineKind::Fork => match stage_fork_download::render_live(ui, orchestrator) {
            ForkDownloadOutcome::Cancel => {
                orchestrator.reset_install_screen_to_gallery();
                Some(InstallRequest::Stage(InstallStage::Gallery))
            }
            ForkDownloadOutcome::Import => {
                let id = orchestrator.active_install_modlist_id.clone()?;
                fork_route::extract_complete_route_to_workspace(orchestrator, id);
                None
            }
            ForkDownloadOutcome::Stay => None,
        },
    }
}

fn installing_stage(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
) -> Option<InstallRequest> {
    let outcome = stage_installing::render(ui, orchestrator);
    console_back_request(orchestrator, outcome)
}

fn console_back_request(
    orchestrator: &mut OrchestratorApp,
    outcome: StageInstallingOutcome,
) -> Option<InstallRequest> {
    match outcome {
        StageInstallingOutcome::Back(stage) => {
            if stage == InstallStage::Details {
                orchestrator.install_screen_state.drawer.open = Some(DrawerKind::Install);
            }
            Some(InstallRequest::Stage(stage))
        }
        StageInstallingOutcome::BackAfterCompletedInstall => {
            crate::ui::orchestrator::page_router::reset_completed_install_runtime(orchestrator);
            Some(InstallRequest::Stage(InstallStage::Gallery))
        }
        StageInstallingOutcome::Nav(dest) => Some(InstallRequest::Nav(dest)),
        StageInstallingOutcome::Stay => None,
    }
}

fn selected_entry(state: &InstallScreenState) -> Option<&'static GalleryEntry> {
    state
        .gallery
        .selected
        .and_then(|index| catalog::entries().get(index))
}

fn open_details_from_gallery_entry(
    entry: &GalleryEntry,
    state: &mut InstallScreenState,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    match catalog::share_code(entry) {
        Ok(code) => {
            state.import_code = code;
            run_preview_parse(state);
            if let Some(err) = state.preview_parse_error.clone() {
                notification_manager.error(format!("Could not open \"{}\": {err}", entry.name));
                state.gallery.selected = None;
                return None;
            }
            state.review.origin = ReviewOrigin::Details;
            state.review.name = entry.name.to_string();
            state.review.modify = false;
            Some(InstallRequest::Stage(InstallStage::Details))
        }
        Err(err) => {
            warn!(
                target = "orchestrator",
                "Gallery: share code for {} could not be generated: {err}", entry.id
            );
            state.clear_preview();
            state.preview_parse_error = Some(err.clone());
            state.gallery.selected = None;
            notification_manager.error(format!("Could not open \"{}\": {err}", entry.name));
            None
        }
    }
}

fn run_preview_parse(state: &mut InstallScreenState) {
    state.clear_preview();
    match preview_modlist_share_code(state.import_code.trim()) {
        Ok(preview) => {
            state.parsed_preview = Some(preview);
            state.preview_cached = true;
        }
        Err(msg) => {
            state.preview_parse_error = Some(msg);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn orch_for_install_test() -> OrchestratorApp {
        OrchestratorApp::new_isolated_for_test("pageinstalltest")
    }

    #[test]
    fn begin_install_writes_the_typed_name_into_the_preview_and_code() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut app = orch_for_install_test();
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "My Renamed List".to_string();
        app.install_screen_state.destination = "D:\\eet install".to_string();

        begin_install(&mut app);

        assert_eq!(
            app.install_screen_state
                .parsed_preview
                .as_ref()
                .and_then(|p| p.name.as_deref()),
            Some("My Renamed List")
        );
        let reparsed = preview_modlist_share_code(&app.install_screen_state.import_code)
            .expect("renamed code still parses");
        assert_eq!(reparsed.name.as_deref(), Some("My Renamed List"));
    }

    #[test]
    fn begin_install_keeps_the_packed_name_when_the_typed_name_is_blank() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");
        let original_name = preview.name.clone();

        let mut app = orch_for_install_test();
        app.install_screen_state.import_code = code.clone();
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "   ".to_string();
        app.install_screen_state.destination = "D:\\eet install".to_string();

        begin_install(&mut app);

        assert_eq!(app.install_screen_state.import_code, code.trim());
        assert_eq!(
            app.install_screen_state
                .parsed_preview
                .as_ref()
                .and_then(|p| p.name.clone()),
            original_name
        );
    }

    #[test]
    fn begin_import_requires_creator_name() {
        use egui_toast::ToastKind;

        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut app = orch_for_install_test();
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "Gate Test".to_string();
        let tmp_dest = std::env::temp_dir()
            .join("bio_pageinstalltest_gate_dest")
            .to_string_lossy()
            .to_string();
        app.install_screen_state.destination = tmp_dest;
        app.install_screen_state.stage = InstallStage::Details;
        app.redesign_settings.user_name.clear();

        app.install_screen_state.drawer.open =
            Some(crate::ui::install::state_install::DrawerKind::Install);
        let stage_before = app.install_screen_state.stage;
        let stage = begin_import(&mut app);

        assert_eq!(stage, None);
        assert_eq!(app.install_screen_state.stage, stage_before);
        assert_eq!(
            app.install_screen_state.drawer.open,
            Some(crate::ui::install::state_install::DrawerKind::Install),
            "a refused Begin Import must leave the Install drawer open"
        );
        assert!(app.registry.entries.is_empty());
        let history = app.notification_manager.history();
        assert_eq!(history.len(), 1);
        let record = history.back().unwrap();
        assert_eq!(record.kind, ToastKind::Error);
        assert_eq!(
            record.text,
            "Set your name in Settings > General before creating or sharing a modlist."
        );
    }

    #[test]
    fn console_back_after_a_finished_install_resets_the_screen_to_the_gallery() {
        use crate::ui::orchestrator::orchestrator_app::PostInstallResetGate;

        let mut app = orch_for_install_test();
        app.install_screen_state.review.name = "Old".to_string();
        app.install_screen_state.destination = "C:\\old".to_string();
        app.install_screen_state.import_code = "x".to_string();
        app.install_screen_state.preview_cached = true;
        app.install_screen_state.stage = InstallStage::InstallingStub;
        app.post_install_reset_gate = PostInstallResetGate::Pending;

        let request =
            console_back_request(&mut app, StageInstallingOutcome::BackAfterCompletedInstall);

        assert_eq!(request, Some(InstallRequest::Stage(InstallStage::Gallery)));
        assert!(app.install_screen_state.review.name.is_empty());
        assert!(app.install_screen_state.destination.is_empty());
        assert!(app.install_screen_state.import_code.is_empty());
        assert!(!app.install_screen_state.preview_cached);
        assert!(!app.post_install_reset_gate.is_pending());
    }

    #[test]
    fn console_back_during_a_running_install_returns_to_details_with_the_install_drawer_open() {
        use crate::ui::orchestrator::orchestrator_app::PostInstallResetGate;
        use crate::ui::orchestrator::page_router;

        let mut app = orch_for_install_test();
        app.install_screen_state.review.name = "Old".to_string();
        app.install_screen_state.destination = "C:\\old".to_string();
        app.install_screen_state.import_code = "x".to_string();
        app.install_screen_state.preview_cached = true;
        app.install_screen_state.stage = InstallStage::InstallingStub;
        app.post_install_reset_gate = PostInstallResetGate::Pending;
        app.wizard_state.step5.install_running = true;

        assert!(!page_router::completed_install_reset_due(&app));

        let request = console_back_request(
            &mut app,
            StageInstallingOutcome::Back(InstallStage::Details),
        );

        assert_eq!(request, Some(InstallRequest::Stage(InstallStage::Details)));
        assert_eq!(app.install_screen_state.review.name, "Old");
        assert_eq!(app.install_screen_state.destination, "C:\\old");
        assert_eq!(
            app.install_screen_state.drawer.open,
            Some(DrawerKind::Install)
        );
    }

    #[test]
    fn opening_details_generates_the_code_and_arms_the_review_state() {
        let entry = catalog::entries()
            .iter()
            .find(|e| e.id == "eet-essentials")
            .expect("EET Essentials is in the catalog");
        let mut state = InstallScreenState::default();
        let mut notification_manager = NotificationManager::new();

        let request = open_details_from_gallery_entry(entry, &mut state, &mut notification_manager);

        assert!(matches!(
            request,
            Some(InstallRequest::Stage(InstallStage::Details))
        ));
        assert_eq!(state.review.origin, ReviewOrigin::Details);
        assert_eq!(state.review.name, "EET Essentials");
        assert!(!state.review.modify);
        assert!(state.preview_cached);
        assert!(state.parsed_preview.is_some());
        assert!(!state.import_code.is_empty());
    }

    #[test]
    fn a_gallery_entry_generates_its_code_once_per_transition() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let mut state = InstallScreenState::default();
        let mut notification_manager = NotificationManager::new();
        open_details_from_gallery_entry(entry, &mut state, &mut notification_manager);
        let first = state.import_code.clone();
        open_details_from_gallery_entry(entry, &mut state, &mut notification_manager);
        assert_eq!(
            first, state.import_code,
            "regenerating the same entry must be deterministic"
        );
    }

    #[test]
    fn details_back_clears_the_preview_and_the_selection() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.gallery.selected = Some(0);
        let mut pending_reinstall_id = None;

        let request = details_back(&mut state, &mut pending_reinstall_id);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Gallery));
        assert!(state.parsed_preview.is_none());
        assert!(state.gallery.selected.is_none());
        assert!(state.import_code.is_empty());
    }

    #[test]
    fn details_back_from_a_pasted_code_keeps_the_code_and_returns_to_paste() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code.clone(),
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.review.origin = ReviewOrigin::Paste;
        state.fork_info_open = true;
        state.drawer.open = Some(DrawerKind::IncludedMods);
        let mut pending_reinstall_id = None;

        let request = details_back(&mut state, &mut pending_reinstall_id);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Paste));
        assert_eq!(state.import_code, code);
        assert!(state.parsed_preview.is_some());
        assert!(!state.fork_info_open);
        assert!(state.drawer.open.is_none());
    }

    #[test]
    fn details_back_from_a_reinstall_clears_the_pending_id_and_returns_to_the_gallery() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.review.origin = ReviewOrigin::Reinstall;
        let mut pending_reinstall_id = Some("REINSTALL0001".to_string());

        let request = details_back(&mut state, &mut pending_reinstall_id);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Gallery));
        assert!(state.parsed_preview.is_none());
        assert!(pending_reinstall_id.is_none());
    }

    #[test]
    fn paste_opens_with_an_empty_code_box_after_details() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.gallery.selected = Some(0);
        let mut pending_reinstall_id = None;

        details_back(&mut state, &mut pending_reinstall_id);
        let request = open_paste_from_gallery(&mut state);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Paste));
        assert!(state.import_code.is_empty());
        assert!(state.parsed_preview.is_none());
    }

    #[test]
    fn begin_install_closes_the_drawer() {
        use crate::ui::install::state_install::DrawerKind;

        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = catalog::share_code(entry).expect("stub code generates");
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut app = orch_for_install_test();
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "Tactical EET".to_string();
        app.install_screen_state.destination = "D:\\eet install".to_string();
        app.install_screen_state.drawer.open = Some(DrawerKind::Install);

        begin_install(&mut app);

        assert!(app.install_screen_state.drawer.open.is_none());
    }

    #[test]
    fn a_drawer_install_click_swaps_to_the_install_form() {
        use crate::ui::install::drawers::after_included_mods;
        use crate::ui::install::drawers::included_mods::IncludedModsOutcome;
        use crate::ui::install::state_install::{DrawerKind, DrawerState};

        let mut drawer = DrawerState {
            open: Some(DrawerKind::IncludedMods),
            ..Default::default()
        };

        after_included_mods(&IncludedModsOutcome::Install, &mut drawer);

        assert_eq!(drawer.open, Some(DrawerKind::Install));
    }
}
