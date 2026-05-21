// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::install_runtime::start_hooks;
use crate::ui::install::stage_downloading::{self, DownloadScreenCopy, DownloadingOutcome};
use crate::ui::install::stage_installing::{self, StageInstallingOutcome};
use crate::ui::install::stage_paste::{self, PasteOutcome};
use crate::ui::install::stage_preview::{self, PreviewOutcome};
use crate::ui::install::state_install::InstallStage;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

enum InstallRequest {
    Stage(InstallStage),
    Nav(NavDestination),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    let mut request: Option<InstallRequest> = None;

    match orchestrator.install_screen_state.stage {
        InstallStage::Paste => {
            match stage_paste::render(ui, palette, &mut orchestrator.install_screen_state) {
                PasteOutcome::Advance(InstallStage::Preview) => {
                    run_preview_parse(&mut orchestrator.install_screen_state);
                    request = Some(InstallRequest::Stage(InstallStage::Preview));
                }
                PasteOutcome::Advance(stage) => {
                    request = Some(InstallRequest::Stage(stage));
                }
                PasteOutcome::Stay => {}
            }
        }
        InstallStage::Preview => {
            match stage_preview::render(ui, palette, ctx, &mut orchestrator.install_screen_state) {
                PreviewOutcome::Back => {
                    orchestrator.install_screen_state.clear_preview();
                    request = Some(InstallRequest::Stage(InstallStage::Paste));
                }
                PreviewOutcome::OpenInCreate => {
                    request = Some(InstallRequest::Nav(NavDestination::Create));
                }
                PreviewOutcome::Advance => {
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
                    request = Some(InstallRequest::Stage(InstallStage::Downloading));
                }
                PreviewOutcome::Stay => {}
            }
        }
        InstallStage::Downloading => {
            match stage_downloading::render_live(ui, orchestrator, DownloadScreenCopy::INSTALL) {
                DownloadingOutcome::Cancel => {
                    // Cancel must tear the whole pipeline (channels +
                    // BIO auto-build latches + saved-log latches +
                    // shared hash/extract snapshots) back to the paste
                    // stage — without dropping the receivers the worker
                    // chain keeps producing events into a dead grid and
                    // eventually fires Step-5's auto-start, locking the
                    // rail behind an install the user already cancelled.
                    orchestrator.reset_install_screen_to_paste();
                    request = Some(InstallRequest::Stage(InstallStage::Paste));
                }
                DownloadingOutcome::Advance => {
                    request = Some(InstallRequest::Stage(InstallStage::InstallingStub));
                }
                DownloadingOutcome::Stay => {}
            }
        }
        InstallStage::InstallingStub => match stage_installing::render(ui, orchestrator) {
            StageInstallingOutcome::Back(stage) => {
                request = Some(InstallRequest::Stage(stage));
            }
            StageInstallingOutcome::Nav(dest) => {
                request = Some(InstallRequest::Nav(dest));
            }
            StageInstallingOutcome::Stay => {}
        },
    }

    if let Some(req) = request {
        match req {
            InstallRequest::Stage(stage) => {
                orchestrator.install_screen_state.stage = stage;
            }
            InstallRequest::Nav(dest) => {
                orchestrator.nav = dest;
            }
        }
    }
}

fn run_preview_parse(state: &mut crate::ui::install::state_install::InstallScreenState) {
    state.clear_preview();
    match preview_modlist_share_code(state.import_code.trim()) {
        Ok(preview) => {
            state.parsed_preview = Some(preview);
            state.preview_cached = true;
            state.active_preview_tab = crate::ui::install::state_install::PreviewTab::default();
        }
        Err(msg) => {
            state.preview_parse_error = Some(msg);
        }
    }
}
