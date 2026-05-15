// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install Modlist — Stage 4 stub (SPEC §4.4, P5.T13). The real install
// runtime (the embedded `mod_installer` console, prompt routing, etc.) is
// Phase 7; this run renders only the placeholder so the four-stage machine is
// whole and navigable.
//
// Per the plan (P5.T13) + dispatch brief:
//   - `ScreenTitle title="Installing modlist"
//      sub="Install runtime arrives in Phase 7"`
//   - one faint sub-line giving more context
//   - a single `← Back to preview` button that returns to the `Preview`
//     stage, or to `Paste` if no preview has been cached
//     (`InstallScreenState::preview_cached`).
//
// SPEC §4.4 defers the full implementation to Phase 7 — there is no spec /
// wireframe conflict here (the wireframe's `InstallProgressScreen` is the
// Phase-7 surface; Phase 5 ships the placeholder per the plan).
//
// The `←` glyph is rendered in `firacode_nerd` (HANDOFF caveat — Poppins is a
// Latin-only subset and tofus arrows). We reuse `sub_flow_footer`'s
// glyph-aware Back button so the stub stays consistent with the rest of the
// Install sub-flow.
//
// SPEC: §4.4.

use eframe::egui;

use crate::ui::install::state_install::{InstallScreenState, InstallStage};
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

/// What the stub wants the dispatcher to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallingStubOutcome {
    /// Stay on the stub.
    #[default]
    Stay,
    /// `← Back to preview` clicked — go to the cached preview, or back to
    /// paste when none is cached (SPEC §4.4 acceptance).
    Back(InstallStage),
}

/// Render the Stage 4 stub. Returns whether the Back button was clicked and
/// where it should route.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &InstallScreenState,
) -> InstallingStubOutcome {
    render_screen_title(
        ui,
        palette,
        "Installing modlist",
        Some("Install runtime arrives in Phase 7"),
    );

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "The live install console (mod_installer output, auto-answered prompts, cancel controls) is wired in Phase 7. The paste \u{2192} preview \u{2192} download flow that leads here is functional now.",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );

    // Push the footer to the bottom (same chassis as the paste stage) while
    // reserving its own footprint so it stays in the visible area.
    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    // `← Back to preview` — Preview if a parse is cached, else Paste
    // (SPEC §4.4: "returns to stage 2, or paste if no preview is cached").
    // Rendered as the footer's Back button so the glyph + chassis match the
    // rest of the sub-flow. The footer always paints a right-aligned primary;
    // there is no forward action on the stub, so the primary is a disabled
    // placeholder (no click is ever emitted while disabled).
    let back_target = if state.preview_cached {
        InstallStage::Preview
    } else {
        InstallStage::Paste
    };

    let outcome = sub_flow_footer::render(
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
    );

    if outcome.back_clicked {
        InstallingStubOutcome::Back(back_target)
    } else {
        InstallingStubOutcome::Stay
    }
}
