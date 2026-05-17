// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `load_draft_dialog` — the **Resume in-progress build** dialog (SPEC §5.2 /
// §10.2, P6.T9), opened from Create's top-right `load draft` button.
//
// Mirrors `wireframe-preview/screens.jsx::LoadDraftDialog` (line 2560-2648):
//   sketchyBorder; background var(--shell-bg); boxShadow 5px 5px 0
//   var(--shadow); padding 22; maxWidth 620; width 94%
//   header:  ▾chevron + <Label fontSize:18 fontWeight:500>Resume in-progress build</Label>
//   sub:     <Label color:text-muted fontSize:13>Pick a build to resume.
//            BIO restores its order, selection, and settings and drops you
//            back where you left off.</Label>
//   body:    in-progress builds → a vertical list of cards
//              { <Label>{name}</Label> <Label hand faint>{meta}</Label> }
//              { <Btn small primary>resume</Btn> <Kebab[Copy import code, Delete]/> }
//            empty → <Box><Label color:text-faint>No in-progress builds.
//                    Start a new modlist from Create.</Label></Box>
//   footer:  <Btn small>Cancel</Btn>   (Cancel ONLY — each card owns `resume`)
//   toast:   transient `✓ Copied import code for "<modlist>"` inside the dialog
//
// ## SPEC §5.2 vs §10.2 — the §5.2 / wireframe card-list form is canonical
//
// SPEC §10.2's prose ("Load draft" / "Pick a saved .txt draft" / a
// `FolderInput` "draft file" picker / `Load → Step 2`) is **stale**: it is
// contradicted by (a) the canonical wireframe `LoadDraftDialog` above (a
// card list with per-card `resume`, NO file picker), (b) SPEC §5.2's own
// detailed spec ("a vertical list of in-progress build cards … The dialog is
// intentionally **not a file picker**"), and (c) the plan task P6.T9
// (explicitly the §5.2 card-list form, reusing the Phase-5 `modlist_card`).
// Per spec-authority priority (wireframe > SPEC prose; the more-specific
// §5.2 > the stale §10.2) the §5.2/wireframe form is implemented. The §10.2
// staleness is reported for a doc-sync (orchestrator-owned), not silently
// resolved here.
//
// ## Reuse — the Phase-5 `modlist_card` (verbatim, no behavior change)
//
// The card chassis IS the Phase-5 Home `modlist_card::render` (the brief:
// "Load Draft reuses the Phase-5 shared `modlist_card` … same widget,
// filtered to in-progress registry entries"). It is reused **verbatim** —
// its in-progress Kebab is `Copy import code / Rename / Delete`; the wireframe
// LoadDraft Kebab is `Copy import code / Delete`. The extra `Rename` item in
// the reused widget is **inert by design** (`|| {}`) and is itself an
// already-recorded staged deviation (SPEC §3.2: "Phase 5 ships this item
// visible but inert … do not re-flag"). Reusing the widget verbatim (the
// brief's hard constraint: "reuse Phase-5 widgets verbatim … no behavior
// changes to reused widgets") is correct; forking it to drop one inert item
// would be the violation.
//
// ## Wired actions (Run 3 = the Create *entry path*)
//
//   - `resume`           → live: closes the dialog + opens the Workspace at
//     Step 2 for that build (P6.T14 — the dispatcher sets
//     `orchestrator.nav = NavDestination::Workspace { Some(id) }`; the
//     loader/route is the already-shipped P6.T12 `render_workspace`).
//   - `Copy import code` → live: copies the build's BIO-MODLIST-V1 code +
//     shows the in-dialog `✓ Copied import code for "<name>"` confirmation
//     (SPEC §5.2; the same `operations::share_code_for` + `ctx.copy_text`
//     path Home uses).
//   - `Delete`           → **not wired in Run 3.** This matches the canonical
//     wireframe, where the LoadDraft Kebab Delete is `onClick: () => {}`
//     (inert). In-progress build deletion is fully available from Home
//     (shipped Phase 5, with the guarded `operations::delete_modlist` +
//     danger confirm). Re-plumbing a delete-confirm flow *inside* this
//     dialog is neither in the canonical wireframe nor in the Run-3 scope
//     (the Create entry path); deferring it keeps the reuse verbatim and the
//     footprint minimal. Recorded as a wireframe-faithful judgment call.
//
// **Non-blocking** per SPEC §10 — a borderless centered `egui::Window`
// (`title_bar(false)`, no modal area / backdrop / focus trap), exactly the
// `confirm_dialog.rs` chassis. The wireframe's full-screen `rgba(0,0,0,0.55)`
// overlay is a wireframe-rendering convention only. The collapse chevron
// (SPEC §10) is a Phase-8 carve-out-#2 concern (consistent with every
// net-new redesign popup shipped so far — `confirm_dialog.rs`); this dialog
// ships non-collapsible by design.
//
// SPEC: §5.2 (Resume in-progress build — card list, not a file picker),
//       §10 (non-blocking popup chassis), §3.2 (the reused card shape),
//       §13.1 (registry is the in-progress source).

// rationale: `f32 as u8`/`i8` casts are pixel-radius / shadow-offset
// roundings of small positive constants — correct by construction (Cat 2);
// the doc-paragraph-length lint is subjective style (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph
)]

use eframe::egui;

use crate::registry::model::{ModlistRegistry, ModlistState};
use crate::ui::home::modlist_card::{self, ModlistCardActions};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_PX, ThemePalette,
    redesign_border_strong, redesign_shadow, redesign_shell_bg, redesign_success,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// What the user did in the Load Draft dialog this frame. Exactly one intent
/// per frame; `page_create` applies it after the render borrow ends.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoadDraftOutcome {
    /// Still open, nothing actioned.
    #[default]
    Pending,
    /// `Cancel` clicked (or — see below — no live action). Close the dialog.
    Cancelled,
    /// A card's `resume` clicked — close the dialog and open that build's
    /// workspace at Step 2 (P6.T14). Carries the modlist id.
    Resume(String),
    /// A card's Kebab `Copy import code` clicked. Carries the modlist id; the
    /// dispatcher resolves + copies the code and feeds the in-dialog
    /// confirmation back via [`render`]'s `copied_name` arg next frame.
    CopyImportCode(String),
}

/// `maxWidth: 620` (wireframe). egui windows shrink-wrap; this caps width.
const MAX_WIDTH_PX: f32 = 620.0;

/// Render the dialog as a centered, non-blocking `egui::Window`.
///
/// - `registry` — the in-progress builds are `registry.entries` filtered to
///   `ModlistState::InProgress` (SPEC §5.2 / §13.1).
/// - `copied_name` — when `Some(name)`, the transient `✓ Copied import code
///   for "<name>"` confirmation is shown inside the dialog (SPEC §5.2). The
///   caller owns this string's lifetime (it sets it on a `CopyImportCode`
///   outcome and clears it after a short delay / on close — exactly the
///   `home` toast-lifetime pattern, kept caller-side so the dialog stays a
///   pure renderer).
///
/// Returns the outcome this frame; the caller owns the open/closed state and
/// clears it on `Cancelled` / `Resume`.
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    registry: &ModlistRegistry,
    copied_name: Option<&str>,
) -> LoadDraftOutcome {
    let mut outcome = LoadDraftOutcome::Pending;

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(22))
        .shadow(egui::epaint::Shadow {
            // Wireframe `boxShadow: 5px 5px 0 var(--shadow)`.
            offset: [
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
            ],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        });

    egui::Window::new("Resume in-progress build")
        .id(egui::Id::new("create_load_draft_dialog"))
        // Non-blocking per SPEC §10 — no modal area / backdrop / focus trap.
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);
            ui.set_width(MAX_WIDTH_PX.min(ui.available_width()));

            // ── Header (18px / weight 500). ──
            ui.label(
                egui::RichText::new("Resume in-progress build")
                    .size(18.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(6.0);

            // ── Sub (muted, 13px). ──
            ui.label(
                egui::RichText::new(
                    "Pick a build to resume. BIO restores its order, selection, and settings and drops you back where you left off.",
                )
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
            );
            ui.add_space(16.0);

            // ── Body: in-progress build cards, or the empty state. ──
            let in_progress: Vec<&crate::registry::model::ModlistEntry> = registry
                .entries
                .iter()
                .filter(|e| e.state == ModlistState::InProgress)
                .collect();

            if in_progress.is_empty() {
                // Empty-state Box (wireframe `screens.jsx:2607-2610`):
                // faint copy verbatim per SPEC §5.2.
                let empty_box = egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 20,
                        top: 16,
                        bottom: 16,
                    });
                empty_box.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new(
                            "No in-progress builds. Start a new modlist from Create.",
                        )
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_faint(palette)),
                    );
                });
            } else {
                // Scrollable list (a long in-progress set must not grow the
                // dialog past the screen). Each row is the REUSED Phase-5
                // `modlist_card::render` (verbatim).
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        for entry in in_progress {
                            match modlist_card::render(ui, palette, entry) {
                                ModlistCardActions::Resume => {
                                    outcome = LoadDraftOutcome::Resume(entry.id.clone());
                                }
                                ModlistCardActions::CopyImportCode => {
                                    outcome =
                                        LoadDraftOutcome::CopyImportCode(entry.id.clone());
                                }
                                // `Delete` from the reused card's danger Kebab
                                // is intentionally NOT wired here (matches the
                                // canonical wireframe's inert `() => {}`;
                                // in-progress deletion lives on Home — see the
                                // module header). `Open`/`OpenInstallFolder`/
                                // `Reinstall` cannot occur on an in-progress
                                // card. `None` = no action.
                                _ => {}
                            }
                        }
                    });
            }

            ui.add_space(14.0);

            // ── Footer: `Cancel` only (wireframe — each card owns `resume`).
            // Fixed-height band so the auto-sizing window shrink-wraps
            // (the `confirm_dialog.rs` footer precedent).
            let footer_h = 30.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), footer_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if crate::ui::orchestrator::widgets::redesign_btn(
                        ui,
                        palette,
                        "Cancel",
                        crate::ui::orchestrator::widgets::BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        outcome = LoadDraftOutcome::Cancelled;
                    }
                },
            );

            // ── Transient in-dialog copy confirmation (SPEC §5.2). ──
            if let Some(name) = copied_name {
                ui.add_space(10.0);
                let toast_frame = egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 6,
                        bottom: 6,
                    });
                toast_frame.show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "\u{2713} Copied import code for \"{name}\""
                        ))
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_success(palette)),
                    );
                });
            }
        });

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_default_is_pending() {
        assert_eq!(LoadDraftOutcome::default(), LoadDraftOutcome::Pending);
    }

    #[test]
    fn resume_and_copy_carry_the_id() {
        assert_eq!(
            LoadDraftOutcome::Resume("ABC".to_string()),
            LoadDraftOutcome::Resume("ABC".to_string())
        );
        assert_ne!(
            LoadDraftOutcome::Resume("ABC".to_string()),
            LoadDraftOutcome::CopyImportCode("ABC".to_string())
        );
    }
}
