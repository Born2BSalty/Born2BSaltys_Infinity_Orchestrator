// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::gallery_feed::index::FeedEntry;
use crate::install_runtime::fork_pipeline_arm::{self, ForkArmRequest};
use crate::install_runtime::{fork_route, start_hooks};
use crate::registry::share_export;
use crate::ui::install::drawers;
use crate::ui::install::gallery::catalog;
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
pub(crate) enum InstallRequest {
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
            &orchestrator.wizard_state.step1,
            &mut orchestrator.notification_manager,
        ),
        InstallStage::Details => details_stage(ui, palette, orchestrator, ctx),
        InstallStage::Paste => paste_stage(
            ui,
            palette,
            &mut orchestrator.install_screen_state,
            &orchestrator.wizard_state.step1,
        ),
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
    step1: &crate::app::state::Step1State,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    match stage_gallery::render(ui, palette, state, step1) {
        GalleryOutcome::OpenPaste => Some(open_paste_from_gallery(state)),
        GalleryOutcome::OpenFile => open_file_from_gallery(state, step1, notification_manager),
        GalleryOutcome::OpenDetails(id) => {
            state.gallery.selected = Some(id);
            let entry = selected_entry(state)?.clone();
            open_details_from_gallery_entry(&entry, state, step1, notification_manager)
        }
        GalleryOutcome::Stay => None,
    }
}

fn open_file_from_gallery(
    state: &mut InstallScreenState,
    step1: &crate::app::state::Step1State,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    let path = rfd::FileDialog::new()
        .add_filter(
            crate::app::modlist_biolist::BIOLIST_FILTER_LABEL,
            &[crate::app::modlist_biolist::BIOLIST_EXTENSION],
        )
        .pick_file()?;
    match crate::app::modlist_biolist::read_share_code(&path) {
        Ok(code) => open_details_from_code(state, step1, code, notification_manager),
        Err(msg) => {
            notification_manager.error(msg);
            None
        }
    }
}

pub(crate) fn open_details_from_code(
    state: &mut InstallScreenState,
    step1: &crate::app::state::Step1State,
    code: String,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    state.clear_preview();
    state.import_code = code;
    run_preview_parse(state, step1);
    if let Some(msg) = state.preview_parse_error.clone() {
        notification_manager.error(format!("Couldn't read that modlist file: {msg}"));
        state.import_code.clear();
        return None;
    }
    state.review.origin = ReviewOrigin::File;
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
    let mut header = if origin == ReviewOrigin::Details {
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
    header
        .source_compat_issue
        .clone_from(&orchestrator.install_screen_state.source_compat_issue);
    header
        .source_residue_issue
        .clone_from(&orchestrator.install_screen_state.source_residue_issue);
    header
        .unresolved_sources_issue
        .clone_from(&orchestrator.install_screen_state.unresolved_sources_issue);
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
        drawers::install_drawer::DrawerIds {
            pending_reinstall_id: orchestrator.pending_reinstall_id.as_deref(),
            installing_id: orchestrator.active_install_modlist_id.as_deref(),
        },
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
        ReviewOrigin::Details | ReviewOrigin::File => {
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
    step1: &crate::app::state::Step1State,
) -> Option<InstallRequest> {
    match stage_paste::render(ui, palette, state) {
        PasteOutcome::Advance(InstallStage::Details) => {
            run_preview_parse(state, step1);
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
    refresh_source_compat_issue(
        &mut orchestrator.install_screen_state,
        &orchestrator.wizard_state.step1,
    );
    if stage_review::source_notice_blocks(&orchestrator.install_screen_state)
        || orchestrator
            .install_screen_state
            .game_version_issue
            .is_some()
    {
        orchestrator.install_screen_state.drawer.open = Some(DrawerKind::Install);
        return InstallStage::Details;
    }
    {
        let state = &mut orchestrator.install_screen_state;
        state.destination = state.destination.trim().to_string();
        state.import_code = state.import_code.trim().to_string();
        state.pipeline_kind = PipelineKind::Install;
        state.set_auto_start_fired(false);
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
    refresh_source_compat_issue(
        &mut orchestrator.install_screen_state,
        &orchestrator.wizard_state.step1,
    );
    if orchestrator
        .install_screen_state
        .game_version_issue
        .is_some()
    {
        orchestrator.install_screen_state.drawer.open = Some(DrawerKind::Install);
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
                DownloadingOutcome::OpenWorkspace => {
                    open_workspace_or_reset_to_gallery(orchestrator)
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
                let labels = orchestrator
                    .wizard_state
                    .step2
                    .skipped_manual_downloads
                    .clone();
                fork_route::extract_complete_route_to_workspace(orchestrator, id);
                if !labels.is_empty() {
                    warn_continued_without(orchestrator, &labels);
                }
                None
            }
            ForkDownloadOutcome::Stay => None,
        },
    }
}

fn open_workspace_or_reset_to_gallery(
    orchestrator: &mut OrchestratorApp,
) -> Option<InstallRequest> {
    let Some(id) = orchestrator.active_install_modlist_id.clone() else {
        orchestrator
            .notification_manager
            .error("The install lost track of its modlist; start it again from the gallery.");
        orchestrator.reset_install_screen_to_gallery();
        return Some(InstallRequest::Stage(InstallStage::Gallery));
    };
    let labels = orchestrator
        .wizard_state
        .step2
        .skipped_manual_downloads
        .clone();
    fork_route::route_to_workspace(orchestrator, id);
    warn_continued_without(orchestrator, &labels);
    None
}

fn warn_continued_without(orchestrator: &mut OrchestratorApp, labels: &[String]) {
    let n = labels.len();
    let word = if n == 1 { "mod" } else { "mods" };
    orchestrator.notification_manager.warn_persistent(format!(
        "Continued without {n} {word}: {}",
        labels.join(", ")
    ));
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

fn selected_entry(state: &InstallScreenState) -> Option<&FeedEntry> {
    let id = state.gallery.selected.as_deref()?;
    catalog::entries().iter().find(|entry| entry.id == id)
}

fn open_details_from_gallery_entry(
    entry: &FeedEntry,
    state: &mut InstallScreenState,
    step1: &crate::app::state::Step1State,
    notification_manager: &mut NotificationManager,
) -> Option<InstallRequest> {
    state.import_code.clone_from(&entry.code);
    run_preview_parse(state, step1);
    if let Some(err) = state.preview_parse_error.clone() {
        notification_manager.error(format!("Could not open \"{}\": {err}", entry.name));
        state.gallery.selected = None;
        return None;
    }
    state.review.origin = ReviewOrigin::Details;
    state.review.name.clone_from(&entry.name);
    state.review.modify = false;
    Some(InstallRequest::Stage(InstallStage::Details))
}

fn run_preview_parse(state: &mut InstallScreenState, step1: &crate::app::state::Step1State) {
    state.clear_preview();
    match preview_modlist_share_code(state.import_code.trim()) {
        Ok(preview) => {
            state.parsed_preview = Some(preview);
            state.preview_cached = true;
            refresh_source_compat_issue(state, step1);
        }
        Err(msg) => {
            state.preview_parse_error = Some(msg);
        }
    }
}

pub(crate) fn refresh_source_compat_issue(
    state: &mut InstallScreenState,
    step1: &crate::app::state::Step1State,
) {
    state.source_compat_issue = state
        .parsed_preview
        .as_ref()
        .and_then(|preview| crate::app::compat_dlc_source::preview_issue(step1, preview));
    state.source_residue_issue = state.parsed_preview.as_ref().and_then(|preview| {
        crate::app::compat_dlc_source::residue_issue(step1, &preview.game_install)
    });
    state.unresolved_sources_issue = state.parsed_preview.as_ref().and_then(|preview| {
        crate::app::compat_dlc_source::unresolved_sources_issue(&preview.unresolved_mods)
    });
    state.missing_source_issue = state.parsed_preview.as_ref().and_then(|preview| {
        crate::app::game_authority::missing_source_message(
            &crate::app::game_authority::missing_source_folders(step1, &preview.game_install),
        )
    });
    state.game_version_issue = state.parsed_preview.as_ref().and_then(|preview| {
        preview.game_version.as_deref().and_then(|tag| {
            crate::app::game_version::version_mismatch_message(step1, &preview.game_install, tag)
        })
    });
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
        let code = entry.code.clone();
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
    fn preview_issue_is_stored_when_the_preview_is_parsed() {
        use crate::app::compat_dlc_source::refresh_source_check;
        use crate::app::modlist_share::ModlistSharePreview;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let source = std::env::temp_dir().join(format!(
            "bio-page-install-source-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&source).expect("create source fixture");
        std::fs::write(source.join("sod-dlc.zip"), b"synthetic archive fixture")
            .expect("write synthetic archive");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bgee_game_folder = source.to_string_lossy().into_owned();
        app.wizard_state.step1.game_install = "BGEE".to_string();
        assert!(refresh_source_check(&mut app.wizard_state.step1));
        app.install_screen_state.parsed_preview = Some(ModlistSharePreview {
            bio_version: String::new(),
            game_install: "BGEE".to_string(),
            game_version: None,
            install_mode: "custom".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: "~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010 // x: v1".to_string(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        });

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        let notice = app
            .install_screen_state
            .source_compat_issue
            .as_ref()
            .expect("notice");
        assert_eq!(
            notice.severity,
            crate::app::compat_dlc_source::SourceNoticeSeverity::Warning
        );
        assert_eq!(
            notice.text,
            "Your BGEE source contains DLC that needs merging. This modlist includes CDTweaks, which requires DLC Merger for this source."
        );

        std::fs::remove_file(source.join("sod-dlc.zip")).expect("remove synthetic archive");
        app.wizard_state.step1.bgee_game_folder.clear();
        assert!(refresh_source_check(&mut app.wizard_state.step1));

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.source_compat_issue.is_none());

        std::fs::remove_dir_all(&source).expect("clean up source fixture");
    }

    #[test]
    fn refresh_source_compat_issue_fills_the_unresolved_sources_slot_from_the_preview() {
        use crate::app::modlist_share::ModlistSharePreview;

        let mut app = orch_for_install_test();
        app.install_screen_state.parsed_preview = Some(ModlistSharePreview {
            bio_version: String::new(),
            game_install: "BGEE".to_string(),
            game_version: None,
            install_mode: "custom".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: vec!["Alpha".to_string(), "Beta".to_string()],
        });

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        let notice = app
            .install_screen_state
            .unresolved_sources_issue
            .as_ref()
            .expect("unresolved sources notice");
        assert_eq!(
            notice.text,
            "2 mods have no download source: Alpha, Beta. BIO will not be able to download them."
        );

        if let Some(preview) = app.install_screen_state.parsed_preview.as_mut() {
            preview.unresolved_mods.clear();
        }
        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.unresolved_sources_issue.is_none());
    }

    #[test]
    fn refresh_fills_the_missing_source_slot_for_an_iwdee_preview() {
        use crate::app::modlist_share::ModlistSharePreview;

        fn preview(game_install: &str) -> ModlistSharePreview {
            ModlistSharePreview {
                bio_version: String::new(),
                game_install: game_install.to_string(),
                game_version: None,
                install_mode: "custom".to_string(),
                bgee_entries: 0,
                bg2ee_entries: 0,
                has_source_overrides: false,
                has_installed_refs: false,
                bgee_log_text: String::new(),
                bg2ee_log_text: String::new(),
                source_overrides_text: String::new(),
                installed_refs_text: String::new(),
                mod_config_count: 0,
                mod_configs_text: String::new(),
                allow_auto_install: true,
                name: None,
                author: None,
                description: None,
                forked_from: Vec::new(),
                unresolved_mods: Vec::new(),
            }
        }

        let mut app = orch_for_install_test();
        app.wizard_state.step1.iwdee_game_folder.clear();
        app.install_screen_state.parsed_preview = Some(preview("IWDEE"));

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert_eq!(
            app.install_screen_state.missing_source_issue,
            Some(
                "The IWDEE game folder is not set. Set it in Settings \u{2192} Paths.".to_string()
            )
        );

        app.wizard_state.step1.iwdee_game_folder = "D:\\games\\iwdee".to_string();
        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.missing_source_issue.is_none());

        app.wizard_state.step1.bgee_game_folder = "D:\\games\\bgee".to_string();
        app.install_screen_state.parsed_preview = Some(preview("BGEE"));
        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.missing_source_issue.is_none());
    }

    struct GameVersionFixture {
        path: std::path::PathBuf,
    }

    impl GameVersionFixture {
        fn new(name: &str) -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_page_install_gameversion_{name}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create game version fixture");
            Self { path }
        }

        fn write_key(&self, bytes: &[u8]) {
            std::fs::write(self.path.join("chitin.key"), bytes).expect("write chitin.key");
        }
    }

    impl Drop for GameVersionFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn versioned_preview(
        game_install: &str,
        game_version: Option<&str>,
    ) -> crate::app::modlist_share::ModlistSharePreview {
        crate::app::modlist_share::ModlistSharePreview {
            bio_version: String::new(),
            game_install: game_install.to_string(),
            game_version: game_version.map(str::to_string),
            install_mode: "custom".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        }
    }

    #[test]
    fn untagged_list_has_no_version_issue() {
        let fixture = GameVersionFixture::new("untagged");
        fixture.write_key(b"data/PATCH26.BIF\0data/PATCH27.bif");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", None));

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.game_version_issue.is_none());
    }

    #[test]
    fn tagged_list_on_the_wrong_version_gets_an_issue() {
        let fixture = GameVersionFixture::new("wrong_version");
        fixture.write_key(b"data/PATCH26.BIF\0data/PATCH27.bif");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", Some("2.6")));

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert_eq!(
            app.install_screen_state.game_version_issue,
            Some(
                "This modlist needs game version 2.6. The BG2EE game folder is version 2.7. Point Settings \u{2192} Paths at a game folder on version 2.6."
                    .to_string()
            )
        );
    }

    #[test]
    fn tagged_list_on_the_matching_folder_has_no_issue() {
        let fixture = GameVersionFixture::new("matching_version");
        fixture.write_key(b"data/PATCH26.BIF");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", Some("2.6")));

        refresh_source_compat_issue(&mut app.install_screen_state, &app.wizard_state.step1);
        assert!(app.install_screen_state.game_version_issue.is_none());
    }

    #[test]
    fn begin_install_rechecks_the_game_version() {
        let fixture = GameVersionFixture::new("begin_install_stale");
        fixture.write_key(b"data/PATCH27.bif");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", Some("2.6")));

        let stage = begin_install(&mut app);

        assert_eq!(stage, InstallStage::Details);
        assert!(app.install_screen_state.game_version_issue.is_some());
        assert_eq!(
            app.install_screen_state.drawer.open,
            Some(DrawerKind::Install)
        );
    }

    #[test]
    fn begin_install_proceeds_on_the_matching_version() {
        let fixture = GameVersionFixture::new("begin_install_matching");
        fixture.write_key(b"data/PATCH26.BIF");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.import_code = "BIO-MODLIST-V1:STUB".to_string();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", Some("2.6")));
        app.install_screen_state.review.name = "Tactical EET".to_string();
        app.install_screen_state.destination = "D:\\eet install".to_string();

        let stage = begin_install(&mut app);

        assert_eq!(stage, InstallStage::Downloading);
    }

    #[test]
    fn begin_import_rechecks_the_game_version() {
        let fixture = GameVersionFixture::new("begin_import_stale");
        fixture.write_key(b"data/PATCH27.bif");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bg2ee_game_folder = fixture.path.to_string_lossy().into_owned();
        app.install_screen_state.parsed_preview = Some(versioned_preview("BG2EE", Some("2.6")));

        let outcome = begin_import(&mut app);

        assert!(outcome.is_none());
        assert!(app.install_screen_state.game_version_issue.is_some());
        assert_eq!(
            app.install_screen_state.drawer.open,
            Some(DrawerKind::Install)
        );
    }

    #[test]
    fn begin_install_keeps_the_packed_name_when_the_typed_name_is_blank() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
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
    fn open_workspace_without_id_resets_to_gallery() {
        use egui_toast::ToastKind;

        let mut app = orch_for_install_test();
        app.install_screen_state.stage = InstallStage::Downloading;
        app.active_install_modlist_id = None;

        let request = open_workspace_or_reset_to_gallery(&mut app);

        assert_eq!(request, Some(InstallRequest::Stage(InstallStage::Gallery)));
        assert_eq!(app.install_screen_state.stage, InstallStage::Gallery);
        let history = app.notification_manager.history();
        assert_eq!(history.len(), 1);
        let record = history.back().unwrap();
        assert_eq!(record.kind, ToastKind::Error);
        assert_eq!(
            record.text,
            "The install lost track of its modlist; start it again from the gallery."
        );
    }

    #[test]
    fn begin_import_without_a_name_forks() {
        use egui_toast::ToastKind;

        struct AmbientRestore(Option<std::path::PathBuf>);

        impl Drop for AmbientRestore {
            fn drop(&mut self) {
                crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
            }
        }

        struct DestRoot(std::path::PathBuf);

        impl Drop for DestRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient = AmbientRestore(crate::app::mod_downloads::active_modlist_dir());
        let dest_root = DestRoot(std::env::temp_dir().join(format!(
            "bio_pageinstalltest_noname_dest_{}",
            std::process::id()
        )));

        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut app = orch_for_install_test();
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "Gate Test".to_string();
        app.install_screen_state.destination = dest_root.0.to_string_lossy().into_owned();
        app.install_screen_state.stage = InstallStage::Details;
        app.redesign_settings.user_name.clear();

        let stage = begin_import(&mut app);

        assert_eq!(stage, Some(InstallStage::Downloading));
        assert_eq!(app.registry.entries.len(), 1);
        let forked = &app.registry.entries[0];
        assert_eq!(forked.name, "Gate Test");
        assert_eq!(forked.author, None);
        assert_eq!(
            app.active_install_modlist_id.as_deref(),
            Some(forked.id.as_str())
        );
        assert!(
            app.notification_manager
                .history()
                .iter()
                .all(|record| record.kind != ToastKind::Error)
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
        assert_eq!(app.install_screen_state.review.name.len(), 0);
        assert_eq!(app.install_screen_state.destination.len(), 0);
        assert_eq!(app.install_screen_state.import_code.len(), 0);
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

        let step1 = crate::app::state::Step1State::default();
        let request =
            open_details_from_gallery_entry(entry, &mut state, &step1, &mut notification_manager);

        assert!(matches!(
            request,
            Some(InstallRequest::Stage(InstallStage::Details))
        ));
        assert_eq!(state.review.origin, ReviewOrigin::Details);
        assert_eq!(state.review.name, "EET Essentials");
        assert!(!state.review.modify);
        assert!(state.preview_cached);
        assert!(state.parsed_preview.is_some());
        assert_ne!(state.import_code.len(), 0);
    }

    #[test]
    fn a_gallery_entry_generates_its_code_once_per_transition() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let mut state = InstallScreenState::default();
        let step1 = crate::app::state::Step1State::default();
        let mut notification_manager = NotificationManager::new();
        open_details_from_gallery_entry(entry, &mut state, &step1, &mut notification_manager);
        let first = state.import_code.clone();
        open_details_from_gallery_entry(entry, &mut state, &step1, &mut notification_manager);
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
        let code = entry.code.clone();
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.gallery.selected = Some(entry.id.clone());
        let mut pending_reinstall_id = None;

        let request = details_back(&mut state, &mut pending_reinstall_id);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Gallery));
        assert!(state.parsed_preview.is_none());
        assert!(state.gallery.selected.is_none());
        assert_eq!(state.import_code.len(), 0);
    }

    #[test]
    fn details_back_from_a_pasted_code_keeps_the_code_and_returns_to_paste() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
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

    fn minted_file_code() -> String {
        use crate::app::modlist_share::encode_share_payload_text;

        let payload_text = r#"{
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" },
            "description": "A minted file for tests"
        }"#;
        encode_share_payload_text(payload_text).expect("encode a minimal payload")
    }

    #[test]
    fn a_file_code_opens_details_with_the_file_origin() {
        let code = minted_file_code();
        let mut state = InstallScreenState::default();
        let step1 = crate::app::state::Step1State::default();
        let mut notification_manager = NotificationManager::new();

        let request =
            open_details_from_code(&mut state, &step1, code.clone(), &mut notification_manager);

        assert_eq!(request, Some(InstallRequest::Stage(InstallStage::Details)));
        assert_eq!(state.review.origin, ReviewOrigin::File);
        assert!(!state.review.modify);
        assert!(state.gallery.selected.is_none());
        assert_eq!(state.import_code, code);
        assert!(state.parsed_preview.is_some());
    }

    #[test]
    fn an_unreadable_file_code_stays_on_the_gallery_with_an_error() {
        let mut state = InstallScreenState::default();
        let step1 = crate::app::state::Step1State::default();
        let mut notification_manager = NotificationManager::new();

        let request = open_details_from_code(
            &mut state,
            &step1,
            "not a valid share code".to_string(),
            &mut notification_manager,
        );

        assert_eq!(request, None);
        assert_eq!(state.import_code.len(), 0);
        assert!(state.parsed_preview.is_none());
        let history = notification_manager.history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn details_back_from_a_file_returns_to_the_gallery_and_clears_the_preview() {
        let code = minted_file_code();
        let preview = preview_modlist_share_code(&code).expect("minted code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.review.origin = ReviewOrigin::File;
        state.gallery.selected = Some("some-gallery-id".to_string());
        let mut pending_reinstall_id = None;

        let request = details_back(&mut state, &mut pending_reinstall_id);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Gallery));
        assert!(state.parsed_preview.is_none());
        assert_eq!(state.import_code.len(), 0);
        assert!(state.gallery.selected.is_none());
    }

    #[test]
    fn details_back_from_a_reinstall_clears_the_pending_id_and_returns_to_the_gallery() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
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
        let code = entry.code.clone();
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut state = InstallScreenState {
            import_code: code,
            parsed_preview: Some(preview),
            ..Default::default()
        };
        state.gallery.selected = Some(entry.id.clone());
        let mut pending_reinstall_id = None;

        details_back(&mut state, &mut pending_reinstall_id);
        let request = open_paste_from_gallery(&mut state);

        assert_eq!(request, InstallRequest::Stage(InstallStage::Paste));
        assert_eq!(state.import_code.len(), 0);
        assert!(state.parsed_preview.is_none());
    }

    #[test]
    fn begin_install_closes_the_drawer() {
        use crate::ui::install::state_install::DrawerKind;

        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
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
    fn begin_install_proceeds_past_an_info_notice() {
        use crate::app::compat_dlc_source::{SourceNoticeSeverity, refresh_source_check};
        use crate::app::modlist_share::ModlistSharePreview;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let source = std::env::temp_dir().join(format!(
            "bio-page-install-info-source-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&source).expect("create source fixture");
        std::fs::write(source.join("sod-dlc.zip"), b"synthetic archive fixture")
            .expect("write synthetic archive");
        let destination = std::env::temp_dir().join(format!(
            "bio-page-install-info-dest-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&destination).expect("create destination fixture");

        let mut app = orch_for_install_test();
        app.wizard_state.step1.bgee_game_folder = source.to_string_lossy().into_owned();
        app.wizard_state.step1.game_install = "BGEE".to_string();
        assert!(refresh_source_check(&mut app.wizard_state.step1));

        app.install_screen_state.import_code = "BIO-MODLIST-V1:STUB".to_string();
        app.install_screen_state.parsed_preview = Some(ModlistSharePreview {
            bio_version: String::new(),
            game_install: "BGEE".to_string(),
            game_version: None,
            install_mode: "custom".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: "~DLCMERGER/SETUP-DLCMERGER.TP2~ #0 #1 // DLC Merger: v1\n~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010 // x: v1".to_string(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: None,
            author: None,
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        });
        app.install_screen_state.review.name = "Tactical EET".to_string();
        app.install_screen_state.destination = destination.to_string_lossy().into_owned();
        app.install_screen_state.review.modify = false;
        app.install_screen_state.drawer.open = Some(DrawerKind::Install);

        let stage = begin_install(&mut app);

        assert_eq!(stage, InstallStage::Downloading);
        assert!(app.install_screen_state.drawer.open.is_none());
        assert_eq!(
            app.install_screen_state
                .source_compat_issue
                .as_ref()
                .map(|notice| notice.severity),
            Some(SourceNoticeSeverity::Info)
        );

        std::fs::remove_dir_all(&source).expect("clean up source fixture");
        std::fs::remove_dir_all(&destination).expect("clean up destination fixture");
    }

    #[test]
    fn begin_install_rearms_the_auto_start() {
        let entry = catalog::entries()
            .first()
            .expect("the catalog is not empty");
        let code = entry.code.clone();
        let preview = preview_modlist_share_code(&code).expect("stub code parses");

        let mut app = OrchestratorApp::new_isolated_for_test("auto_start_rearm");
        app.install_screen_state.import_code = code;
        app.install_screen_state.parsed_preview = Some(preview);
        app.install_screen_state.review.name = "Tactical EET".to_string();
        app.install_screen_state.destination = "D:\\eet install".to_string();
        app.install_screen_state.set_auto_start_fired(true);

        let stage = begin_install(&mut app);

        assert!(!app.install_screen_state.auto_start_fired());
        assert_eq!(stage, InstallStage::Downloading);
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
