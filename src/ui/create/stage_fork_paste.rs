// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create → Import-and-modify — Fork-paste stage (`ForkPasteScreen`).
// SPEC §5.3, P6.T8.
//
// Mirrors `wireframe-preview/screens.jsx` Create fork-paste branch:
//   <ScreenTitle title="Import and modify another modlist"
//                sub="paste a share code — preview, then BIO downloads +
//                     preselects" />
//   <Box label="import code">
//     "BIO-MODLIST-V1 share code" + the same tall mono textarea Install uses
//   </Box>
//   <div flex:1 />                                 // spacer pushes footer down
//   <SubFlowFooter onBack={→ choose} backLabel="Back"
//                  onPrimary={→ preview} primaryLabel="Preview →"
//                  hint="no download starts until preview is accepted" />
//
// **Reuse posture (the Run-3 precedent, recorded in overview 2026-05-17).**
// The truly-shared chassis is reused verbatim: `sub_flow_footer::render`
// (the exact footer Install's paste stage uses — `Back`/`Preview →`/hint),
// `render_screen_title`, `redesign_box`. The paste textarea itself
// (`stage_paste::import_code_box`) is **module-private** to `stage_paste.rs`,
// exactly as `stage_paste`'s `FolderInput` was — so per the Run-3 PLAN-GAP
// resolution (overview 2026-05-17: "`FolderInput` rendered net-new
// wireframe-faithful — no shared FolderInput widget exists … same precedent
// `stage_paste` set"), the import-code box here is a net-new
// wireframe-faithful clone of the *visual*, not a copy of `stage_paste`'s
// internals (grep-prove: no `stage_paste` import). It is pixel-identical to
// Install's textarea (same FiraCode mono, input-bg, scroll-inside-the-box,
// same placeholder), differing **only** in the surrounding stage labels +
// the footer CTA — exactly the SPEC §5.3 contract ("uses the same chassis as
// the Install Modlist flow … with a different `continueLabel`").
//
// SPEC: §5.3 (fork-paste — `ForkPasteScreen`). Wireframe: the Create
//       fork-paste branch (the Install paste-stage textarea reused).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length lint
// is subjective style on a faithfully-mirrored screen (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph
)]

use eframe::egui;

use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

/// Wireframe textarea placeholder — identical to Install's paste stage
/// (`stage_paste::CODE_PLACEHOLDER`; the SPEC §5.3 "same chassis" contract).
const CODE_PLACEHOLDER: &str =
    "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.";

/// What the fork-paste stage wants the dispatcher to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkPasteOutcome {
    /// Stay on the fork-paste stage.
    #[default]
    Stay,
    /// `← Back` clicked — return to the `choose` stage.
    Back,
    /// `Preview →` clicked (enabled only when the textarea is non-empty) —
    /// run the share-code parse and advance to `ForkPreview`.
    Preview,
}

/// Render the fork-paste stage. Mutates `state.fork_code` in place; returns
/// what the dispatcher should do next.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> ForkPasteOutcome {
    render_screen_title(
        ui,
        palette,
        "Import and modify another modlist",
        Some("paste a share code \u{2014} preview, then BIO downloads + preselects"),
    );
    ui.add_space(8.0);

    // ── Import-code Box (SPEC §5.3 `Box label="import code"`). ──
    // The box fills the space down to the footer and the textarea scrolls
    // INSIDE it (a very large pasted code must never grow the page or push
    // the footer off-screen) — the exact cap-to-footer pattern Install's
    // paste/preview stages use.
    let box_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(160.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), box_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| import_code_box(ui, palette, &mut state.fork_code),
    );

    // ── SubFlowFooter (SPEC §5.3: `Back` → choose + `Preview →`). ──
    // The primary is disabled until a code has been pasted (same gate as
    // Install's paste stage). `← Back` returns to the choose stage always.
    let code_empty = state.fork_code.trim().is_empty();
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(if code_empty {
            "paste a BIO share code to preview"
        } else {
            "no download starts until preview is accepted"
        }),
        PrimaryBtn {
            label: "Preview",
            disabled: code_empty,
        },
    );

    if footer.back_clicked {
        ForkPasteOutcome::Back
    } else if footer.primary_clicked {
        // `primary_clicked` is suppressed while disabled, so this is only
        // reachable with a non-empty code — the gate holds.
        ForkPasteOutcome::Preview
    } else {
        ForkPasteOutcome::Stay
    }
}

/// The import-code Box — a net-new wireframe-faithful clone of Install's
/// paste textarea (`stage_paste::import_code_box`, which is module-private;
/// the Run-3 `FolderInput` precedent). Pixel-identical: "BIO-MODLIST-V1
/// share code" label + a tall mono textarea that scrolls inside the box.
fn import_code_box(ui: &mut egui::Ui, palette: ThemePalette, code: &mut String) {
    redesign_box(ui, palette, Some("import code"), |ui| {
        ui.label(
            egui::RichText::new("BIO-MODLIST-V1 share code")
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        let frame = egui::Frame::default()
            .fill(redesign_input_bg(palette))
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            ))
            .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
            .inner_margin(egui::Margin::same(12));
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Scroll INSIDE the box (`auto_shrink([false, false])`), wrap to
            // the box width (`desired_width(INFINITY)`) — identical to
            // Install's paste textarea so a huge code scrolls here rather
            // than growing the page.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(code)
                            .desired_width(f32::INFINITY)
                            .desired_rows(8)
                            .font(egui::FontId::new(
                                12.0,
                                egui::FontFamily::Name("firacode_nerd".into()),
                            ))
                            .frame(false)
                            .hint_text(
                                egui::RichText::new(CODE_PLACEHOLDER)
                                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                                    .color(redesign_text_faint(palette)),
                            )
                            .text_color(redesign_text_primary(palette))
                            .background_color(redesign_input_bg(palette)),
                    );
                });
        });
    });
}
