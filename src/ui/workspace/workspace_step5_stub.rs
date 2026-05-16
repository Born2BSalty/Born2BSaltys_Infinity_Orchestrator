// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step5_stub` — Phase-6 placeholder for the workspace's Step 5.
//
// Per the Phase-6 file inventory: renders the "install runtime arrives in
// Phase 7" message + a WeiDU-command preview Box + a **disabled** `Install`
// button. Phase 7 P7.T* replaces this stub with BIO's real
// `page_step5::render` wrapped in the new success/post-install chrome.
//
// SPEC: §2.2 (Step 5 is the install step), Phase-7 scope (install runtime).

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_box, redesign_btn};
use crate::ui::shared::redesign_tokens::{redesign_text_faint, redesign_text_muted};

/// Render the Step 5 placeholder. Signature matches the other workspace step
/// renderers' call shape from `workspace_step_router::render`.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    ui.label(
        egui::RichText::new("Step 5 \u{2014} Install")
            .size(15.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "The live install runtime (console, prompts, diagnostics) arrives in Phase 7.",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );

    ui.add_space(12.0);

    // WeiDU-command preview Box — content placeholder until Phase 7 binds
    // the real command builder.
    redesign_box(ui, palette, Some("weidu command"), |ui| {
        ui.label(
            egui::RichText::new(
                "The resolved WeiDU command line will be previewed here in Phase 7.",
            )
            .size(13.0)
            .family(egui::FontFamily::Name("firacode_nerd".into()))
            .color(redesign_text_faint(palette)),
        );
    });

    ui.add_space(12.0);

    // Disabled Install button (Phase 7 wires the install-start hook +
    // concurrency gate).
    let _ = redesign_btn(
        ui,
        palette,
        "Install",
        BtnOpts {
            primary: true,
            disabled: true,
            ..Default::default()
        },
    );
}
