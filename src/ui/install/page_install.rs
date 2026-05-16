// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_install` — the Install Modlist destination's top-level renderer
// (SPEC §4). Dispatches on `InstallScreenState::stage`.
//
// **Run 5 scope.** All four stages render: `Paste`, `Preview`,
// `Downloading` (the net-new SPEC §4.3 / wireframe `ImportDownloadScreen`
// surface — `stage_downloading`), and `InstallingStub`. The only BIO-source
// touch in all of Phase 5 was the Run-4 carve-out #5 schema-additive edit
// on `src/core/app/modlist_share.rs` (read here via
// `preview_modlist_share_code`); **Run 5 modifies ZERO BIO source.**
//
// **SPEC CONFLICT / PLAN GAP (Run 5) — the Downloading stage's live
// download/extract pipeline is intentionally UNWIRED pending the user's
// decision.** `stage_downloading::render` ships the full §4.3 chassis +
// state, but `InstallScreenState::download_progress` is populated by nothing
// this run because BIO exposes no "share code → download list" surface and
// the only path that produces one is BIO's complex `modlist_auto_build`
// pipeline (scan → update-preview → update-check → download → extract →
// rescan), which the directive's decision order classifies as a
// complex-pipeline workflow — not a `WizardApp`-style channel poll, and not
// something to reimplement or fork. The Run-5 report escalates this; until
// it is resolved the screen renders an empty grid (navigable: Cancel →
// Preview) and the production auto-advance never fires. See
// `stage_downloading.rs`'s module header for the full analysis.
//
// The deferred-intent pattern mirrors `home/page_home.rs`: each stage
// renderer returns an outcome enum; the dispatcher applies the resulting
// `InstallStage` transition *and any `NavDestination` change* after the
// render borrow ends. The `Open in Create →` secondary CTA on the
// draft-code gate (SPEC §4.2) routes via that deferred `NavRequest`
// (Phase 6 wires the code pre-load handoff into Create — Run 4 only
// switches the destination).
//
// **Parse-on-transition.** The share-code parse
// (`preview_modlist_share_code`) runs once on the `Paste → Preview`
// transition (not per-frame): cheap to keep, expensive to redo every frame,
// and the pasted code can't change while on Preview. The result (or the
// parse error) is cached on `InstallScreenState`.
//
// SPEC: §4 (Install Modlist), §4.1, §4.2, §4.4, §1 (carve-out #5).

use eframe::egui;

use crate::app::modlist_share::preview_modlist_share_code;
use crate::ui::install::stage_downloading::{self, DownloadScreenCopy, DownloadingOutcome};
use crate::ui::install::stage_installing_stub::{self, InstallingStubOutcome};
use crate::ui::install::stage_paste::{self, PasteOutcome};
use crate::ui::install::stage_preview::{self, PreviewOutcome};
use crate::ui::install::state_install::InstallStage;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// A deferred app-level transition bubbled up from a stage renderer, applied
/// after the render borrow of `orchestrator` ends (same pattern as
/// `page_home`'s `NavRequest`). A stage transition and a nav change are
/// mutually exclusive per click — the enum encodes exactly one intent.
enum InstallRequest {
    /// Move to another Install stage.
    Stage(InstallStage),
    /// Leave Install for another top-level destination (the draft-code
    /// gate's `Open in Create →` — SPEC §4.2; Phase 6 wires the pre-load).
    Nav(NavDestination),
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // Deferred app-level transition (applied after the render borrow ends,
    // same pattern as `page_home`'s `NavRequest`).
    let mut request: Option<InstallRequest> = None;

    match orchestrator.install_screen_state.stage {
        InstallStage::Paste => {
            match stage_paste::render(ui, palette, &mut orchestrator.install_screen_state) {
                PasteOutcome::Advance(InstallStage::Preview) => {
                    // Parse-on-transition: run the share-code parse once now,
                    // cache the result (or the error), then enter Preview.
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
                    // Going back to Paste invalidates the cached preview (the
                    // pasted code may change before the next Preview).
                    orchestrator.install_screen_state.clear_preview();
                    request = Some(InstallRequest::Stage(InstallStage::Paste));
                }
                PreviewOutcome::OpenInCreate => {
                    request = Some(InstallRequest::Nav(NavDestination::Create));
                }
                PreviewOutcome::Advance => {
                    request = Some(InstallRequest::Stage(InstallStage::Downloading));
                }
                PreviewOutcome::Stay => {}
            }
        }
        InstallStage::Downloading => {
            // SPEC §4.3 — the net-new `ImportDownloadScreen` surface
            // (overall-progress Box + 4-col per-mod grid + footer). The
            // per-mod model (`download_progress`) is empty this run: the
            // live download/extract orchestration is escalated as a
            // SPEC CONFLICT / PLAN GAP (see this file's + `stage_downloading`'s
            // module headers) and is NOT wired pending the user's decision.
            // The screen is fully navigable (Cancel → Preview) and the
            // production auto-advance lights up automatically once the
            // resolved orchestration feeds `download_progress`.
            match stage_downloading::render(
                ui,
                palette,
                DownloadScreenCopy::INSTALL,
                &orchestrator.install_screen_state.download_progress,
            ) {
                DownloadingOutcome::Cancel => {
                    // SPEC §4.3: `Cancel` (← back) returns to Preview. Drop
                    // any accumulated grid so a re-parse can't inherit a
                    // stale list.
                    orchestrator.install_screen_state.download_progress =
                        crate::ui::install::stage_downloading::DownloadProgress::default();
                    request = Some(InstallRequest::Stage(InstallStage::Preview));
                }
                DownloadingOutcome::Advance => {
                    // Production auto-advance on download+extract completion
                    // (SPEC §4.3 → §4.4). Never fires this run (empty model).
                    request = Some(InstallRequest::Stage(InstallStage::InstallingStub));
                }
                DownloadingOutcome::Stay => {}
            }
        }
        InstallStage::InstallingStub => {
            match stage_installing_stub::render(ui, palette, &orchestrator.install_screen_state) {
                InstallingStubOutcome::Back(stage) => {
                    request = Some(InstallRequest::Stage(stage));
                }
                InstallingStubOutcome::Stay => {}
            }
        }
    }

    if let Some(req) = request {
        match req {
            InstallRequest::Stage(stage) => {
                orchestrator.install_screen_state.stage = stage;
            }
            InstallRequest::Nav(dest) => {
                // The pasted code stays on `install_screen_state` so Phase 6
                // can pick it up for the Create pre-load; Run 4 only switches
                // the destination (SPEC §4.2 — "Phase 6 wires the code
                // pre-load handoff").
                orchestrator.nav = dest;
            }
        }
    }
}

/// Run the share-code parse for the just-pasted code and cache the result on
/// `InstallScreenState`. On success: `parsed_preview = Some`, `preview_cached
/// = true`, error cleared. On failure: `preview_parse_error = Some(msg)`,
/// preview cleared (the Preview stage renders the error instead of a blank
/// box). Called exactly once per `Paste → Preview` transition.
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
