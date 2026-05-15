// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_install` — the Install Modlist destination's top-level renderer
// (SPEC §4). Dispatches on `InstallScreenState::stage`.
//
// **Run 4 scope.** `Paste`, `Preview`, and `InstallingStub` are fully
// implemented. `Downloading` (Run 5) still renders a minimal placeholder
// (the same chassis as the stage-4 stub) so the four-stage machine is whole
// and the flow is navigable end to end. The download/extract engines are
// NOT in Run 4. The only BIO-source touch in all of Phase 5 is the
// carve-out #5 schema-additive edit on `src/core/app/modlist_share.rs`
// (read here via `preview_modlist_share_code`).
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
use crate::ui::install::stage_installing_stub::{self, InstallingStubOutcome};
use crate::ui::install::stage_paste::{self, PasteOutcome};
use crate::ui::install::stage_preview::{self, PreviewOutcome};
use crate::ui::install::state_install::InstallStage;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

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
            // Run 5 — per-mod download/extract grid wired to BIO's existing
            // download/extract engines. Placeholder this run.
            if run_later_placeholder(
                ui,
                palette,
                "Downloading",
                "Downloading \u{2014} arrives in Run 5",
                "The per-mod download/extract progress grid (wired to BIO's existing fetch + archive engines) lands in Run 5.",
            ) {
                request = Some(InstallRequest::Stage(InstallStage::Preview));
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

/// The minimal "this stage arrives in a later run" placeholder — same chassis
/// as the §4.4 stage-4 stub (`ScreenTitle` + a faint context line + the
/// bottom-pinned `sub_flow_footer`). Routing the Back control through
/// `sub_flow_footer` (exactly like `stage_installing_stub`) keeps it
/// pixel-identical to the rest of the Install sub-flow. Returns `true` when
/// Back was clicked; the caller transitions to `Preview`.
fn run_later_placeholder(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    title: &str,
    sub: &str,
    detail: &str,
) -> bool {
    render_screen_title(ui, palette, title, Some(sub));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(detail)
            .size(13.0)
            .family(egui::FontFamily::Proportional)
            .color(redesign_text_faint(palette)),
    );

    // Bottom-pin the footer, reserving its footprint, exactly as the §4.4
    // stub does — so the Back button is visually consistent across stages.
    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    // No forward action on a placeholder → disabled primary, same as the
    // §4.4 stub's disabled `Install`. The footer always paints a primary.
    sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn {
            label: "Back to preview",
        }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        PrimaryBtn {
            label: "Install",
            disabled: true,
        },
    )
    .back_clicked
}
