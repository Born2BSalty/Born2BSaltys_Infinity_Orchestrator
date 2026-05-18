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
// **Downloading live data is RESOLVED-DEFERRED to Phase 7 P7.T17 (SPEC
// §13.12a) — not an open escalation.** `stage_downloading::render` ships the
// full §4.3 chassis + state; `InstallScreenState::download_progress` has no
// feed in Phase 5 by design, because the per-mod list is a byproduct of
// BIO's `modlist_auto_build` pipeline whose live wiring + the net-new
// content-addressed staging layer (zero BIO edit) the user assigned to
// Phase 7 P7.T17 on 2026-05-16 (SPEC §13.12a + overview revision log). The
// grid renders empty (navigable: Cancel → Preview) and auto-advance never
// fires until P7.T17 feeds it. See `stage_downloading.rs`'s module header.
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
            // SPEC §4.3 — the net-new `ImportDownloadScreen` surface.
            // **P7.T17 live (SPEC §13.12a):** `render_live` arms BIO's
            // import → auto-build pipeline once (per-install dirs +
            // `import_modlist_share_code` + `arm_auto_build`, NEVER
            // pre-flipping `start_install_requested`), interposes the
            // content-addressed staging layer at the download/extract
            // boundary AROUND BIO's reused-unchanged
            // `app_step2_update_download`/`_extract`, feeds the §4.3 grid
            // from the live BIO auto-build state every frame, and advances
            // to the stage-4 seam when the pipeline reaches the install
            // hand-off. The pipeline itself is driven by the
            // orchestrator's existing per-frame `poll_step2_channels` +
            // Step-5 poll (P6.T2c / P7.T1) — this screen only arms +
            // interposes + renders. (`render_live` takes `&mut
            // OrchestratorApp`; the parameterless `render` cannot drive
            // the pipeline — the minimal redesign-owned caller change for
            // P7.T17's live wiring.)
            match stage_downloading::render_live(ui, orchestrator, DownloadScreenCopy::INSTALL) {
                DownloadingOutcome::Cancel => {
                    // SPEC §4.3: `Cancel` (← back) returns to Preview. Drop
                    // the live grid + reset the pipeline-armed latch so a
                    // re-entry (possibly with a changed code/destination)
                    // re-arms cleanly and cannot inherit a stale grid.
                    orchestrator.install_screen_state.download_progress =
                        crate::ui::install::stage_downloading::DownloadProgress::default();
                    orchestrator.install_screen_state.pipeline_armed = false;
                    request = Some(InstallRequest::Stage(InstallStage::Preview));
                }
                DownloadingOutcome::Advance => {
                    // Production auto-advance: BIO's auto-build pipeline
                    // reached the install hand-off (SPEC §4.3 → §4.4 — the
                    // stage-4 seam). The real stage 4 (`stage_installing`)
                    // is Run 4b; for Run 4a this enters the documented
                    // §4.4 stub, which `start_step5_after_render` (already
                    // wired, P7.T1) backs with BIO's live install runtime
                    // (the pipeline already flipped `start_install_
                    // requested`). The seam is intentional + documented.
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
