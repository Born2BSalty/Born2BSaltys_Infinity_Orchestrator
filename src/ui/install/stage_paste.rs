// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install Modlist — Stage 1 (paste). SPEC §4.1, P5.T9.
//
// Mirrors `wireframe-preview/screens.jsx` Install paste branch
// (line 624-670):
//   <ScreenTitle title="Install shared modlist"
//     sub={isPartial ? "destination has existing modlist — share code skipped"
//                     : "set destination + mods paths, paste a BIO share code,
//                        then preview before importing"} />
//   <Box padding:"16px 20px" marginBottom:14>
//     <FolderInput label="destination folder" placeholder value onBrowse />
//     {dest && <DestinationNotEmptyWarning choice setChoice />}
//   </Box>
//   {isPartial
//     ? <Box> "Continue partial installation" + hand sub … </Box>
//     : <Box label="import code"> "BIO-MODLIST-V1 share code" + textarea </Box>}
//   <div flex:1 />                         // spacer pushes the footer down
//   <SubFlowFooter
//     onPrimary={() => setStage(isPartial ? "installing" : "preview")}
//     primaryLabel={isPartial ? "Continue Install →" : "Preview →"}
//     hint={isPartial ? "no share code needed"
//                      : "no install starts until preview is accepted"} />
//
// Behavior pinned by the dispatch brief / SPEC §4.1:
//   - The `DestinationNotEmptyWarning` renders only when the destination is
//     **set AND non-empty on disk** (the wireframe's `{dest && …}` is a
//     mock-state stand-in for "the picked folder has content"; SPEC §4.1
//     "If the destination is non-empty"). An empty / unset / clean
//     destination shows no warning.
//   - Picking the `continue` option = partial mode: the import-code Box is
//     replaced by the wireframe's "Continue partial installation" info Box;
//     the footer primary becomes `Continue Install →` and the share-code
//     requirement is dropped.
//   - Footer primary is **disabled when the import-code textarea is empty**
//     and not in partial mode (SPEC §4.1 acceptance).
//   - Primary click: non-partial → `InstallStage::Preview` (Run 4
//     placeholder this run); partial → `InstallStage::InstallingStub`.
//
// **DestChoice → flag mapping (SPEC §13.12 #1/#6)** lives on
// `DestChoice::to_flags` in `state_install.rs` (pure + unit-tested). Run 3
// only records the choice; the mapping is applied to the orchestrator-owned
// `WizardState.step1` at install start (Phase 7) — no BIO state mutated here.
//
// FolderInput styling mirrors `src/ui/settings/widgets/path_row.rs` (mono
// FiraCode input, tinted sketchy border, transparent `browse…` button with
// `rfd::FileDialog::pick_folder`).
//
// SPEC: §4.1, §13.12 #1/#6. Wireframe: screens.jsx:624-670 (verbatim copy).

// rationale: `f32 as u8` casts are pixel roundings of small positive
// constants — correct by construction (Cat 2); the `const` is intentionally
// scoped next to its sole use site rather than hoisted (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements
)]

use eframe::egui;

use crate::ui::install::destination_not_empty;
use crate::ui::install::state_install::{InstallScreenState, InstallStage};
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent_deep,
    redesign_border_strong, redesign_input_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

/// What stage 1 wants the dispatcher to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasteOutcome {
    /// Stay on the paste stage.
    #[default]
    Stay,
    /// Advance to the next stage (`Preview` for a normal import,
    /// `InstallingStub` for a partial-continue).
    Advance(InstallStage),
}

/// Wireframe textarea placeholder (`screens.jsx:658`):
/// `BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.`
const CODE_PLACEHOLDER: &str =
    "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...\n\nPaste the full code here.";

/// Render Stage 1. Mutates `state` in place (destination text, chosen option,
/// pasted code) and returns whether to advance.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> PasteOutcome {
    let is_partial = state.is_partial();

    // ── ScreenTitle ──
    render_screen_title(
        ui,
        palette,
        "Install shared modlist",
        Some(if is_partial {
            "destination has existing modlist \u{2014} share code skipped"
        } else {
            "set destination + mods paths, paste a BIO share code, then preview before importing"
        }),
    );

    // ── Destination Box (FolderInput + optional warning). ──
    redesign_box(ui, palette, None, |ui| {
        let dest_changed = folder_input(
            ui,
            palette,
            "destination folder",
            "D:\\BG2EE_install_test",
            &mut state.destination,
        );
        if dest_changed {
            // Wireframe `handleBrowse` resets `destChoice` to null when the
            // destination changes (`screens.jsx:602-605`) — a new folder
            // means the previous not-empty answer no longer applies.
            state.destination_choice = None;
        }

        // SPEC §4.1: the warning shows only when the destination is set AND
        // non-empty on disk. The wireframe's `{dest && …}` is a mock stand-in
        // for "the chosen folder has content"; we check the real filesystem.
        if destination_is_non_empty(&state.destination)
            && let Some(picked) = destination_not_empty::render(
                ui,
                palette,
                state.destination_choice,
                true, // allow_partial — Install Modlist always offers continue
            )
        {
            state.destination_choice = Some(picked);
        }
    });

    ui.add_space(14.0);

    // ── Import-code Box (or the partial info Box). ──
    // The import-code box fills the space down to the footer and the textarea
    // scrolls INSIDE it: a very large pasted code must never grow the page or
    // push the footer off-screen (the panel has no outer scrollbar). This
    // mirrors the cap-to-footer region pattern `stage_preview.rs` uses for its
    // content box.
    if is_partial {
        partial_info_box(ui, palette, &state.destination);
        // Spacer pushes the footer to the bottom (wireframe `flex:1`),
        // reserving the footer's own footprint.
        let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
        if spacer > 0.0 {
            ui.add_space(spacer);
        }
    } else {
        let box_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), box_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| import_code_box(ui, palette, &mut state.import_code),
        );
    }

    // ── SubFlowFooter ──
    // Footer primary is disabled when the import-code textarea is empty and
    // we're not in partial mode (SPEC §4.1 acceptance).
    let code_empty = state.import_code.trim().is_empty();
    let primary_disabled = !is_partial && code_empty;
    let outcome = sub_flow_footer::render(
        ui,
        palette,
        // Stage 1 is the first stage of the sub-flow — no Back button (the
        // wireframe's paste branch renders no `onBack`).
        None::<BackBtn<'_>>,
        // No secondary CTA on the paste stage (the `Open in Create →` slot
        // is preview-only — SPEC §4.2).
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some(if is_partial {
            "no share code needed"
        } else {
            "no install starts until preview is accepted"
        }),
        PrimaryBtn {
            label: if is_partial {
                "Continue Install"
            } else {
                "Preview"
            },
            disabled: primary_disabled,
        },
    );

    if outcome.primary_clicked {
        return PasteOutcome::Advance(if is_partial {
            InstallStage::InstallingStub
        } else {
            InstallStage::Preview
        });
    }

    PasteOutcome::Stay
}

/// `true` when `path` is set and points at a directory that contains at least
/// one entry. SPEC §4.1: the not-empty warning is conditioned on the chosen
/// folder actually having content (an empty / non-existent / unset path shows
/// nothing).
fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    // A non-directory / missing / unreadable path → treat as "no warning"
    // (there is nothing to clear/back-up/continue).
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

/// `FolderInput` (wireframe `screens.jsx::FolderInput`, line 91-121). Mirrors
/// `path_row.rs`'s mono input + sketchy tinted border + transparent
/// `browse…` button. Returns `true` when the value changed this frame
/// (typed edit or a folder picked via `rfd`).
fn folder_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    placeholder: &str,
    value: &mut String,
) -> bool {
    let mut changed = false;

    // Label — hand-style, muted (wireframe `<Label hand color:text-muted>`).
    ui.label(
        egui::RichText::new(label)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        const BROWSE_W_PX: f32 = 90.0;
        let reserved = BROWSE_W_PX + 8.0;
        let edit_width = (ui.available_width() - reserved).max(120.0);

        let pre = value.clone();
        let response = ui.add_sized(
            egui::vec2(edit_width, 26.0),
            egui::TextEdit::singleline(value)
                .font(egui::FontId::new(
                    12.0,
                    egui::FontFamily::Name("firacode_nerd".into()),
                ))
                .hint_text(
                    egui::RichText::new(placeholder)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(redesign_text_faint(palette)),
                )
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(egui::Margin::symmetric(8, 5)),
        );
        ui.painter().rect_stroke(
            response.rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Outside,
        );
        if response.changed() || *value != pre {
            changed = true;
        }

        // `browse…` button — transparent fill + sketchy stroke (path_row
        // pattern). Opens an `rfd` folder picker.
        if ui
            .add_sized(
                egui::vec2(BROWSE_W_PX, 26.0),
                egui::Button::new(
                    egui::RichText::new("browse\u{2026}")
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_accent_deep(palette)),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                )),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            let s = path.to_string_lossy().to_string();
            if s != *value {
                *value = s;
                changed = true;
            }
        }
    });

    changed
}

/// The import-code Box (wireframe `Box label="import code"`, line 646-660):
/// a "BIO-MODLIST-V1 share code" label + a tall mono textarea.
fn import_code_box(ui: &mut egui::Ui, palette: ThemePalette, code: &mut String) {
    redesign_box(ui, palette, Some("import code"), |ui| {
        ui.label(
            egui::RichText::new("BIO-MODLIST-V1 share code")
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        // Wireframe textarea: `minHeight:200; FiraCode mono; fontSize:12;
        // input-bg; whiteSpace:pre-wrap`.
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
            // Scroll INSIDE the box: `auto_shrink([false, false])` makes the
            // scroll area fill the bounded frame instead of shrinking to
            // content, so a huge code scrolls here rather than growing the
            // page. `desired_width(INFINITY)` wraps to the box width (no
            // horizontal overflow); `desired_rows` is the empty-state height.
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

/// The partial-mode info Box (wireframe `screens.jsx:638-644`):
/// "Continue partial installation" + a hand-style explanation referencing the
/// destination path.
fn partial_info_box(ui: &mut egui::Ui, palette: ThemePalette, dest: &str) {
    redesign_box(ui, palette, None, |ui| {
        ui.label(
            egui::RichText::new("Continue partial installation")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "Existing mod files detected at {dest}. Share-code entry is skipped \u{2014} BIO will pick up where the previous install left off."
            ))
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
        );
    });
}
