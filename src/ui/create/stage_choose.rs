// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create — the initial `choose` mode (SPEC §5.1, P6.T7).
//
// Mirrors `wireframe-preview/screens.jsx::CreateScreen` (the `choose` branch,
// line 3819-3909):
//   <div flex justify-between>
//     <ScreenTitle title="Create / edit modlist"
//       sub="name your modlist, set destination + mods paths, then pick a
//            starting point" />
//     <Btn small onClick={() => setLoadDraftOpen(true)}>load draft</Btn>
//   </div>
//   <Box padding:"16px 20px" marginBottom:18>
//     grid 1fr:
//       grid "1fr auto" align:end:                       // split row
//         { Label hand "modlist name"  + <input> }
//         { Label hand "game"          + <select> }      // EET default
//       <FolderInput label="destination folder" .../>
//       {dest && <DestinationNotEmptyWarning allowPartial={false} />}
//   </Box>
//   <div grid "1fr 1fr" gap:14>
//     <Box onClick=scratch>     "New modlist from downloaded mods"   <Btn primary>start →</Btn>
//     <Box onClick=fork-paste>  "Import and modify another modlist"  <Btn primary>paste share code →</Btn>
//
// Behavior pinned by SPEC §5.1 / the dispatch brief:
//   - Game ComboBox options `EET, BGEE, BG2EE, IWDEE`; **EET is the
//     default** (the state is built via `CreateScreenState::new`, which
//     forces `Game::EET` — `Game::default()` is `BGEE`). Styled to match the
//     Settings → General Language dropdown (the `egui::ComboBox` +
//     `poppins_medium` 12px selected-text pattern from `tab_general.rs`).
//   - The destination `FolderInput` is the wireframe `FolderInput` component
//     (the canonical UI reference) — `stage_paste.rs` renders the same
//     component (its `folder_input` is module-private; there is no shared
//     FolderInput *widget* to reuse, so this is the same net-new
//     wireframe-faithful renderer, NOT a duplicated reusable widget). The
//     `DestinationNotEmptyWarning` **is** a reusable Phase-5 widget
//     (`crate::ui::install::destination_not_empty` — `pub mod`); it is
//     reused **verbatim** with `allow_partial = false` (SPEC §5.1:
//     "Continue-partial is disallowed in Create").
//   - The partial-install option is therefore structurally absent (the
//     reused widget's `allow_partial = false` drops it; `screens.jsx:3874`
//     `<DestinationNotEmptyWarning allowPartial={false} />`).
//   - Changing the destination resets `destination_choice` to `None`
//     (`screens.jsx:3776-3779` `handleBrowse` clears `destChoice`).
//   - The two starting-point cards are whole-Box click targets (wireframe
//     `<Box onClick=…>`) AND carry an explicit primary CTA button — clicking
//     anywhere on the card OR its button triggers the action (the wireframe
//     puts `onClick` on the Box; the inner `Btn` is the affordance).
//   - `start →`  → `ChooseOutcome::StartScratch` (the dispatcher calls
//     `operations_create::create_modlist` + sets nav).
//   - `paste share code →` → `ChooseOutcome::GoForkPaste`.
//   - `load draft` → `ChooseOutcome::OpenLoadDraft`.
//
// This renderer is **state-only** (takes `&mut CreateScreenState`, no
// `OrchestratorApp`) and returns the intent; `page_create` applies the
// app-level effects (create + registry persist + nav) after the borrow ends
// — the exact `page_install` / `page_home` deferred-intent pattern.
//
// SPEC: §5.1. Wireframe: screens.jsx:3819-3909.

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the `const` is scoped next to
// its sole use site and the doc-paragraph-length lint is subjective — Cat 3.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::too_long_first_doc_paragraph
)]

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::destination_not_empty;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_box, redesign_btn, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

/// What the choose stage wants the dispatcher (`page_create`) to do this
/// frame. Exactly one intent per click (mutually exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChooseOutcome {
    /// Nothing actioned this frame.
    #[default]
    Stay,
    /// `start →` clicked (or its card) — create a from-scratch modlist and
    /// route into its workspace. The dispatcher reads name/game/destination
    /// off `CreateScreenState`.
    StartScratch,
    /// `paste share code →` clicked (or its card) — enter the fork sub-flow
    /// (`CreateStage::ForkPaste` — Run 4).
    GoForkPaste,
    /// `load draft` clicked — open the non-blocking Load Draft dialog.
    OpenLoadDraft,
}

/// The game options in wireframe order (`screens.jsx:3864`):
/// `["EET", "BGEE", "BG2EE", "IWDEE"]`. EET is first AND the default
/// selection (SPEC §5.1).
const GAME_OPTIONS: [Game; 4] = [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE];

/// Render the choose stage. Mutates `state` (name / game / destination /
/// destination_choice) in place; returns the user's intent.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> ChooseOutcome {
    let mut outcome = ChooseOutcome::Stay;

    // ── Title row: ScreenTitle (left, grows) + `load draft` (right). ──
    // The wireframe wraps both in `flex justify-between align:flex-start`.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        // ScreenTitle in a left-growing cell so the `load draft` button sits
        // flush-right (the ScreenTitle widget owns its own 20px bottom gap).
        let title_w = (ui.available_width() - 120.0).max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, 0.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_screen_title(
                    ui,
                    palette,
                    "Create / edit modlist",
                    Some(
                        "name your modlist, set destination + mods paths, then pick a starting point",
                    ),
                );
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if redesign_btn(
                ui,
                palette,
                "load draft",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                outcome = ChooseOutcome::OpenLoadDraft;
            }
        });
    });

    // ── Setup Box (name+game split row, destination, optional warning). ──
    redesign_box(ui, palette, None, |ui| {
        ui.spacing_mut().item_spacing.y = 14.0; // wireframe grid `gap: 14`

        // Split row: name (flex 1) | game (auto). Wireframe
        // `gridTemplateColumns: "1fr auto"; align-items: end`.
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0; // wireframe grid `gap: 16`

            // Reserve the game column (label + a comfortably-wide ComboBox)
            // so the name input gets the remaining width (the `1fr`).
            const GAME_COL_W_PX: f32 = 150.0;
            let name_w = (ui.available_width() - GAME_COL_W_PX - 16.0).max(160.0);

            // ── modlist name (flex 1). ──
            ui.allocate_ui_with_layout(
                egui::vec2(name_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    field_label(ui, palette, "modlist name");
                    ui.add_space(4.0);
                    let resp = ui.add_sized(
                        egui::vec2(ui.available_width(), 30.0),
                        egui::TextEdit::singleline(&mut state.modlist_name)
                            .font(egui::FontId::new(
                                14.0,
                                egui::FontFamily::Name("poppins_light".into()),
                            ))
                            .hint_text(
                                egui::RichText::new("e.g. Tactical EET 2026")
                                    .family(egui::FontFamily::Name("poppins_light".into()))
                                    .color(redesign_text_faint(palette)),
                            )
                            .text_color(redesign_text_primary(palette))
                            .background_color(redesign_input_bg(palette))
                            .margin(egui::Margin::symmetric(12, 8)),
                    );
                    ui.painter().rect_stroke(
                        resp.rect,
                        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
                        egui::Stroke::new(
                            REDESIGN_BORDER_WIDTH_PX,
                            redesign_border_strong(palette),
                        ),
                        egui::StrokeKind::Outside,
                    );
                },
            );

            // ── game (auto width). ──
            ui.allocate_ui_with_layout(
                egui::vec2(GAME_COL_W_PX, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    field_label(ui, palette, "game");
                    ui.add_space(4.0);
                    game_combo(ui, palette, &mut state.game);
                },
            );
        });

        // ── destination folder (FolderInput — wireframe component). ──
        let dest_changed = folder_input(
            ui,
            palette,
            "destination folder",
            "D:\\BG2EE_install_test",
            &mut state.destination,
        );
        if dest_changed {
            // Wireframe `handleBrowse` resets `destChoice` to null on a
            // destination change (`screens.jsx:3776-3779`) — a new folder
            // means the previous not-empty answer no longer applies.
            state.destination_choice = None;
        }

        // ── DestinationNotEmptyWarning (conditional, REUSED Phase-5 widget).
        // SPEC §5.1: shown only when the destination is set AND non-empty on
        // disk (the wireframe `{dest && …}` is a mock stand-in for "the
        // picked folder has content"; we check the real filesystem, exactly
        // as `stage_paste.rs` does). `allow_partial = false` — Continue
        // partial is disallowed in Create (SPEC §5.1), so the reused widget
        // structurally omits that option (no re-implementation).
        if destination_is_non_empty(&state.destination)
            && let Some(picked) = destination_not_empty::render(
                ui,
                palette,
                state.destination_choice,
                false, // SPEC §5.1 — Create never offers Continue-partial
            )
        {
            state.destination_choice = Some(picked);
        }
    });

    ui.add_space(18.0); // wireframe Box `marginBottom: 18`

    // ── Two starting-point cards (grid "1fr 1fr", gap 14). ──
    let avail_w = ui.available_width();
    let gap = 14.0;
    let card_w = ((avail_w - gap) / 2.0).max(160.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        if starting_point_card(
            ui,
            palette,
            card_w,
            "New modlist from downloaded mods",
            "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.",
            "start \u{2192}",
            "create_card_scratch",
        ) {
            outcome = ChooseOutcome::StartScratch;
        }

        if starting_point_card(
            ui,
            palette,
            card_w,
            "Import and modify another modlist",
            "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.",
            "paste share code \u{2192}",
            "create_card_fork",
        ) {
            outcome = ChooseOutcome::GoForkPaste;
        }
    });

    outcome
}

/// A hand-style, muted field label (wireframe `<Label hand color:text-muted
/// marginBottom:4>`).
fn field_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
}

/// The game `ComboBox`. Styling mirrors the Settings → General Language
/// dropdown (`tab_general.rs:109-120`): an `egui::ComboBox` with a 12px
/// `poppins_medium` selected-text in `text_primary`. Options are the
/// wireframe set in wireframe order (`screens.jsx:3864`), EET first/default.
fn game_combo(ui: &mut egui::Ui, palette: ThemePalette, game: &mut Game) {
    let mut selected = *game;
    egui::ComboBox::from_id_salt("create_game_combo")
        .selected_text(
            egui::RichText::new(game_label(selected))
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        )
        .show_ui(ui, |ui| {
            for option in GAME_OPTIONS {
                ui.selectable_value(&mut selected, option, game_label(option));
            }
        });
    if selected != *game {
        *game = selected;
    }
}

/// The dropdown label for a game (the wireframe shows the bare enum string;
/// `Game::to_legacy_string` is exactly `"EET"|"BGEE"|"BG2EE"|"IWDEE"`).
fn game_label(game: Game) -> &'static str {
    game.to_legacy_string()
}

/// `true` when `path` is set and points at a directory that contains at least
/// one entry. SPEC §5.1: the not-empty warning is conditioned on the chosen
/// folder actually having content (an empty / non-existent / unset path shows
/// nothing). Identical predicate to `stage_paste::destination_is_non_empty`.
fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

/// The wireframe `FolderInput` component (`screens.jsx:91-121`): a hand-style
/// muted label + a row of [mono input (flex 1)] [browse… button]. Returns
/// `true` when the value changed this frame (typed edit or a folder picked
/// via `rfd`). Mirrors `stage_paste.rs::folder_input` (the same wireframe
/// component; that one is module-private — there is no shared FolderInput
/// widget, so this is net-new wireframe-faithful code, not widget
/// duplication).
fn folder_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    placeholder: &str,
    value: &mut String,
) -> bool {
    let mut changed = false;

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

        if ui
            .add_sized(
                egui::vec2(BROWSE_W_PX, 26.0),
                egui::Button::new(
                    egui::RichText::new("browse\u{2026}")
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                )
                .fill(redesign_shell_bg(palette))
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

/// One starting-point card (wireframe `<Box onClick=… padding:"20px 22px"
/// flex-col>`): a hand-style 18px title, a muted description that flexes to
/// fill, and a primary CTA at the bottom-left. The whole Box is a click
/// target (wireframe `onClick` on the Box) AND the inner button is clickable
/// — either fires. Returns `true` when activated this frame.
fn starting_point_card(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    width: f32,
    title: &str,
    desc: &str,
    cta: &str,
    id_salt: &str,
) -> bool {
    let mut activated = false;

    // Card chassis: sketchy Box, 20×22 padding, pointer cursor. Fixed width
    // (the grid column); height grows to content.
    let chassis = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin {
            left: 22,
            right: 22,
            top: 20,
            bottom: 20,
        });

    let inner = ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            chassis
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // Title — hand-style 18px (wireframe `<Label hand
                    // fontSize:18 marginBottom:8>`).
                    ui.label(
                        egui::RichText::new(title)
                            .size(18.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_primary(palette)),
                    );
                    ui.add_space(8.0);
                    // Description — muted (wireframe `<Label
                    // color:text-muted marginBottom:16 flex:1>`).
                    ui.label(
                        egui::RichText::new(desc)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_muted(palette)),
                    );
                    ui.add_space(16.0);
                    // Primary CTA, bottom-left (wireframe `<Btn primary
                    // alignSelf:flex-start>`).
                    redesign_btn(
                        ui,
                        palette,
                        cta,
                        BtnOpts {
                            primary: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                })
                .inner
        },
    );

    // The inner CTA button click.
    if inner.inner {
        activated = true;
    }

    // The whole-Box click target (wireframe puts `onClick` on the Box).
    // Sense a click over the card's allocated rect; ignore it if it landed on
    // the CTA (already handled) so a single press isn't double-counted.
    let card_resp = ui.interact(
        inner.response.rect,
        ui.make_persistent_id(("create_starting_card", id_salt)),
        egui::Sense::click(),
    );
    if card_resp.clicked() && !inner.inner {
        activated = true;
    }
    if card_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    activated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_options_are_wireframe_order_eet_first() {
        // screens.jsx:3864 — `["EET","BGEE","BG2EE","IWDEE"]`, EET first
        // (and the default per SPEC §5.1 / `CreateScreenState::new`).
        assert_eq!(
            GAME_OPTIONS,
            [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE]
        );
        assert_eq!(GAME_OPTIONS[0], Game::EET);
    }

    #[test]
    fn game_labels_are_bare_enum_strings() {
        assert_eq!(game_label(Game::EET), "EET");
        assert_eq!(game_label(Game::BGEE), "BGEE");
        assert_eq!(game_label(Game::BG2EE), "BG2EE");
        assert_eq!(game_label(Game::IWDEE), "IWDEE");
    }

    #[test]
    fn non_empty_predicate_matches_stage_paste_semantics() {
        // Unset / blank → false (no warning).
        assert!(!destination_is_non_empty(""));
        assert!(!destination_is_non_empty("   "));
        // A real, non-empty temp dir → true.
        let dir =
            std::env::temp_dir().join(format!("bio_create_choose_nonempty_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("f.txt"), b"x").unwrap();
        assert!(destination_is_non_empty(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn choose_outcome_default_is_stay() {
        assert_eq!(ChooseOutcome::default(), ChooseOutcome::Stay);
    }
}
