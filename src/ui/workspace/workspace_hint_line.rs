// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_hint_line` — the one-line per-step hint shown directly under the
// workspace progress bar.
//
// Mirrors `wireframe-preview/screens.jsx::WorkspaceView` (line 3557-3559):
//   <div marginBottom:10 marginLeft:4>
//     <Label hand color:var(--text-faint) fontSize:14>{current.hint}</Label>
//   </div>
//
// The hint text comes from `WorkspaceStep::hint()` (wireframe
// `WORKSPACE_STEPS[*].hint`, verbatim).
//
// SPEC: §2.2 ("Below the progress bar is a one-line hint describing the
//       current step").

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, WORKSPACE_CONTENT_TEXT_INSET, redesign_text_faint,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;

/// Render the current step's hint line.
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, current: WorkspaceStep) {
    // Flush on the structural content edge (`WORKSPACE_CONTENT_TEXT_INSET`,
    // 0 = `X_shell`) — the same left edge as the progress bar, the
    // component pane, the Step-2 title and the search box, so the whole
    // workspace content shares one vertical line. Wireframe `marginLeft:4`
    // is the look-intent; the exact px is the single empirically-tuned knob
    // (2026-05-17 user directive: align to the progress-bar / pane edge).
    ui.horizontal(|ui| {
        ui.add_space(WORKSPACE_CONTENT_TEXT_INSET);
        ui.label(
            // `Label hand` is Poppins 14 in the wireframe; the explicit
            // `color: var(--text-faint)` override on this call site wins
            // over `redesign_label_hand`'s default accent-deep tint, so we
            // build the RichText directly with the faint color.
            egui::RichText::new(current.hint())
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
    });
    // Wireframe `marginBottom: 10`.
    ui.add_space(10.0);
}
