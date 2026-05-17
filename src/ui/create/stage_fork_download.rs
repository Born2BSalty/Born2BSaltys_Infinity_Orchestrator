// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create → Import-and-modify — Fork-download stage. SPEC §5.3 + §13.12a,
// P6.T8.
//
// **SPEC §5.3: "uses `ImportDownloadScreen` with title 'Downloading fork',
// hint 'after download: components auto-selected · order applied · lands on
// Step 2', continueLabel 'continue to Step 2 →'."** This stage drives the
// **reused Phase-5 `stage_downloading::render` chassis VERBATIM** (the exact
// `ImportDownloadScreen` Install's §4.3 Downloading stage uses) — the same
// overall-progress Box + 4-column per-mod grid + footer — passing the
// fork-specific `DownloadScreenCopy` (title/sub/hint) constructed here at
// the call site (the `DownloadScreenCopy` fields are `pub` + `Copy`; the
// chassis is **not** edited — only the Install path's `INSTALL` const lives
// in `stage_downloading.rs`, the fork copy is built here so no Phase-5
// widget internal is touched).
//
// The wireframe's `continueLabel "continue to Step 2 →"` is the
// wireframe-only manual-advance affordance the chassis **intentionally does
// not ship** (production auto-advances on download completion — the footer
// paints a disabled `Waiting…` placeholder; documented in
// `stage_downloading.rs`'s module header / `DownloadingOutcome::Advance`).
// So this stage supplies only `title` / `sub` / `hint`; the
// post-completion forward transition is the `Advance` outcome.
//
// **Phasing (SPEC §13.12a — chassis only).** Phase 6 ships the fork-paste +
// fork-preview FULLY, the `create_forked_modlist` registry/lineage append,
// and the fork-download **chassis navigation** + the post-completion route
// into the forked Workspace. The **live** fork download/extract — driving
// BIO's import → auto-build pipeline + the per-install dirs +
// content-addressed staging — is **Phase 7 P7.T17** (the pipeline
// terminates in the install runtime), exactly as Install's §4.3 Downloading
// stage ships the chassis with no live data until P7.T17. Until then the
// grid renders empty (navigable: `Cancel` → fork-preview) and the live
// auto-advance never fires; it lights up additively the moment P7.T17 binds
// the pipeline (the same forward-compatible model the rest of Phase 5/6
// uses — forward-compatible, not a bug).
//
// **Where the registry/lineage append happens.** Per the SPEC §5.3
// "lineage committed at import, not display" rule, the
// `create_forked_modlist` call + the caller-anchored `workspace.json` write
// + the atomic `modlists.json` persist + the route into the forked
// Workspace all live in `page_create` (the `start_scratch` precedent — the
// only party with `OrchestratorApp` access + the post-mint id). This stage
// is a **pure renderer**: it returns `Cancel` / `Advance` and `page_create`
// applies the deferred effect (the established Run-3 `CreateRequest`
// pattern). Pre-P7.T17 there is no live download feed so the chassis never
// auto-`Advance`s on its own; the dispatcher exposes the import via the
// footer/route per the brief (the §13.12a chassis-nav contract).
//
// SPEC: §5.3 (fork-download), §13.12a (the live pipeline is Phase 7 —
//       chassis only here), §4.3 (the reused `ImportDownloadScreen`).

// rationale: the doc-paragraph-length lint is subjective style on a
// faithfully-mirrored screen carrying a load-bearing phasing note (Cat 3).
#![allow(clippy::too_long_first_doc_paragraph)]

use eframe::egui;

use crate::ui::install::stage_downloading::{self, DownloadProgress, DownloadScreenCopy};
use crate::ui::shared::redesign_tokens::ThemePalette;

/// The fork-download screen copy (SPEC §5.3). Constructed here — **not** as a
/// const in the Phase-5 `stage_downloading.rs` (that file is reused verbatim,
/// not edited; only its Install-path `INSTALL` const lives there). The
/// `title` / `sub` / `hint` are SPEC §5.3-verbatim; `continueLabel` is the
/// wireframe-only manual-advance the chassis intentionally doesn't ship.
fn fork_download_copy() -> DownloadScreenCopy {
    DownloadScreenCopy {
        title: "Downloading fork",
        sub: "fetching the parent's mods \u{2014} Step 2 opens automatically when ready",
        hint: Some(
            "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2",
        ),
    }
}

/// What the fork-download stage wants the dispatcher to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkDownloadOutcome {
    /// Stay on the fork-download stage (chassis still showing / nothing
    /// clicked).
    #[default]
    Stay,
    /// `← Cancel` clicked — back to the fork-preview stage (SPEC §5.3 /
    /// §4.3: `Cancel` (← back)).
    Cancel,
    /// Download + extract finished — create the forked registry entry +
    /// route into the forked Workspace (the dispatcher does the
    /// `create_forked_modlist` + caller-anchored IO + nav). In production
    /// (Phase 7 P7.T17) this fires automatically on completion; pre-P7.T17
    /// the chassis has no live feed, so it never auto-fires here (the
    /// §13.12a deferral — forward-compatible).
    Import,
}

/// Render the fork-download stage by driving the reused Phase-5
/// `stage_downloading` chassis. `progress` is the per-mod model (empty until
/// Phase 7 P7.T17 binds the live pipeline — SPEC §13.12a). Returns what the
/// dispatcher should do next.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    progress: &DownloadProgress,
) -> ForkDownloadOutcome {
    match stage_downloading::render(ui, palette, fork_download_copy(), progress) {
        stage_downloading::DownloadingOutcome::Cancel => ForkDownloadOutcome::Cancel,
        // Production auto-advance on download+extract completion (SPEC §5.3
        // → Step 2). Never fires pre-P7.T17 (the model is empty — §13.12a).
        stage_downloading::DownloadingOutcome::Advance => ForkDownloadOutcome::Import,
        stage_downloading::DownloadingOutcome::Stay => ForkDownloadOutcome::Stay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_copy_is_spec_5_3_verbatim() {
        // SPEC §5.3: title "Downloading fork" + the hint string.
        let c = fork_download_copy();
        assert_eq!(c.title, "Downloading fork");
        assert_eq!(
            c.hint,
            Some(
                "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2"
            )
        );
        assert!(c.sub.contains("Step 2 opens automatically"));
    }
}
