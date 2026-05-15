// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `page_install` — the Install Modlist destination's top-level renderer
// (SPEC §4). Dispatches on `InstallScreenState::stage`.
//
// **Run 3 scope.** `Paste` and `InstallingStub` are fully implemented.
// `Preview` (Run 4) and `Downloading` (Run 5) render a minimal placeholder —
// the same chassis as the stage-4 stub — so the four-stage machine is whole
// and the flow is navigable end to end (`Preview →` on a valid non-empty
// code advances `Paste → Preview`, which shows the placeholder until Run 4).
// The share-code parse, the 6 preview tabs, the `allow_auto_install` gate,
// and the download/extract engines are NOT in Run 3, and
// `src/core/app/modlist_share.rs` is untouched (Run 4 carve-out).
//
// The deferred-intent pattern mirrors `home/page_home.rs`: each stage
// renderer returns an outcome enum; the dispatcher applies the resulting
// `InstallStage` transition *after* the render borrow ends.
//
// SPEC: §4 (Install Modlist), §4.1, §4.4.

use eframe::egui;

use crate::ui::install::stage_installing_stub::{self, InstallingStubOutcome};
use crate::ui::install::stage_paste::{self, PasteOutcome};
use crate::ui::install::state_install::InstallStage;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, render_screen_title};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, _ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;

    // Deferred stage transition (applied after the render borrow ends, same
    // pattern as `page_home`'s `NavRequest`).
    let mut next_stage: Option<InstallStage> = None;

    match orchestrator.install_screen_state.stage {
        InstallStage::Paste => {
            match stage_paste::render(
                ui,
                palette,
                &mut orchestrator.install_screen_state,
            ) {
                PasteOutcome::Advance(stage) => next_stage = Some(stage),
                PasteOutcome::Stay => {}
            }
        }
        InstallStage::Preview => {
            // Run 4 — parsed share-code preview (Overview Box + 6 tabs +
            // `allow_auto_install` gate). Placeholder this run.
            if run_later_placeholder(
                ui,
                palette,
                "Preview",
                "Preview \u{2014} arrives in Run 4",
                "The parsed share-code preview (Overview + Summary / BGEE WeiDU / BG2EE WeiDU / User Downloads / Installed Refs / Mod Configs tabs) lands in Run 4.",
            ) {
                next_stage = Some(InstallStage::Paste);
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
                next_stage = Some(InstallStage::Paste);
            }
        }
        InstallStage::InstallingStub => {
            match stage_installing_stub::render(
                ui,
                palette,
                &orchestrator.install_screen_state,
            ) {
                InstallingStubOutcome::Back(stage) => next_stage = Some(stage),
                InstallingStubOutcome::Stay => {}
            }
        }
    }

    if let Some(stage) = next_stage {
        orchestrator.install_screen_state.stage = stage;
    }
}

/// The minimal "this stage arrives in a later run" placeholder — same chassis
/// as the §4.4 stage-4 stub (`ScreenTitle` + a faint context line). Includes a
/// `Back to paste` button so the placeholder is **not a dead-end**: without it
/// the user is trapped here (stage persists across rail navigation) until the
/// app restarts. Returns `true` when Back was clicked this frame; the caller
/// transitions back to `Paste`. Plain-ASCII label (no `←` glyph) — keeps it
/// off the Latin-subset Poppins symbol-glyph pitfall and is plenty for
/// throwaway Run-3 scaffolding (Run 4/5 replace these stages outright).
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
    ui.add_space(16.0);
    redesign_btn(ui, palette, "Back to paste", BtnOpts::default()).clicked()
}
