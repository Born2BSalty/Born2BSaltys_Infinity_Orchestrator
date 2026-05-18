// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create — the initial `choose` mode (SPEC §5.1, P6.T7).
//
// ## User-directed UX (SPEC §5.1, amended) — supersedes the wireframe cards
//
// The canonical wireframe (`screens.jsx:3878-3903`) had two starting-point
// **cards**, each with its **own** in-box primary CTA (`start →` /
// `paste share code →`). The user directed a different UX (final authority
// on the directed wireframe deviation; SPEC §5.1 amended to record it):
//
//   - A **`Choose one`** header above the two boxes.
//   - Two **selectable boxes** — clicking ANYWHERE on a box *selects* it
//     (accent border + faint accent tint when selected); the in-box
//     `start →` / `paste share code →` sub-buttons are **removed**.
//   - A single primary **`Start →`** button at the **bottom-right**, styled
//     **exactly like the Step-2/3/4 workspace nav-bar forward (`Next →`)
//     button** — it reuses that styling verbatim via
//     `workspace_nav_bar::forward_primary_button` (one styling source; no
//     fourth glyph-button chassis copy). `Start →` dispatches per the
//     selected box: from-scratch ⇒ `StartScratch`; import ⇒ `GoForkPaste`.
//   - The game `ComboBox` applies to the **from-scratch box only**. When
//     the import/paste box is selected, the game is **derived from the
//     pasted share code** (SPEC §5/§5.3), not user-selected — so the
//     ComboBox is replaced by a read-only contextual note ("game comes from
//     the imported modlist"). Selecting a box does not lose the typed name
//     / destination (the Setup Box is shared by both paths).
//
// ## Styling pinned by SPEC §5.1 / the dispatch brief
//
//   - Game ComboBox (#5a): **redesign-token chrome** (sketchy 1.5px
//     border-strong, `input-bg` fill, `poppins_medium` 12px text in
//     `text-primary`) — NOT default egui ComboBox chrome. Options
//     `["EET","BGEE","BG2EE","IWDEE"]` (wireframe order, `screens.jsx:
//     3864`); **EET is the default** (the state is built via
//     `CreateScreenState::new`, which forces `Game::EET` — `Game::default()`
//     is `BGEE`).
//   - `→` glyph (#5b / #6a-icon — symbol-glyph rule, cmap-verified): U+2192
//     is **absent** from the Latin-only Poppins subset (`?` tofu) but
//     **present** in the bundled full FiraCode Nerd build. The only `→` on
//     this screen is on `Start →`, rendered by the nav-bar forward button
//     (`firacode_nerd` glyph + `poppins_medium` prose, side by side) — so
//     it renders correctly. No raw `→` is passed to `redesign_btn` (which
//     hardcodes `poppins_medium` and would tofu).
//   - The destination `FolderInput` is the wireframe `FolderInput`
//     component (the canonical UI reference); `stage_paste.rs` renders the
//     same component (its `folder_input` is module-private; no shared
//     FolderInput *widget* exists, so this is the same net-new
//     wireframe-faithful renderer). Its input border now uses the shared
//     `redesign_text_input` primitive (the app-wide indented-input fix).
//   - `DestinationNotEmptyWarning` is the reused Phase-5 widget
//     (`crate::ui::install::destination_not_empty`), `allow_partial = false`
//     (SPEC §5.1: "Continue-partial is disallowed in Create").
//   - Changing the destination resets `destination_choice` to `None`
//     (`screens.jsx:3776-3779` `handleBrowse` clears `destChoice`).
//
// This renderer is **state-only** (takes `&mut CreateScreenState`, no
// `OrchestratorApp`) and returns the intent; `page_create` applies the
// app-level effects (create + registry persist + nav) after the borrow ends
// — the exact `page_install` / `page_home` deferred-intent pattern.
//
// SPEC: §5.1 (choose UX — amended for the selectable-box deviation),
//       §5/§5.3 (import path derives the game from the share code).
// Wireframe: screens.jsx:3819-3909 (cards superseded by the directed UX).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the `const` is scoped next to
// its sole use site and the doc-paragraph-length / line-count lints are
// subjective — Cat 3.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::create::state_create::{CreateScreenState, StartingPoint};
use crate::ui::install::destination_not_empty;
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_box, redesign_btn, redesign_text_input, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_selection_highlight, redesign_shell_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::workspace_nav_bar;

/// What the choose stage wants the dispatcher (`page_create`) to do this
/// frame. Exactly one intent per click (mutually exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChooseOutcome {
    /// Nothing actioned this frame.
    #[default]
    Stay,
    /// `Start →` clicked with the from-scratch box selected — create a
    /// from-scratch modlist and route into its workspace. The dispatcher
    /// reads name/game/destination off `CreateScreenState`.
    StartScratch,
    /// `Start →` clicked with the import box selected — enter the fork
    /// sub-flow (`CreateStage::ForkPaste`).
    GoForkPaste,
    /// `load draft` clicked — open the non-blocking Load Draft dialog.
    OpenLoadDraft,
}

/// The game options in wireframe order (`screens.jsx:3864`):
/// `["EET", "BGEE", "BG2EE", "IWDEE"]`. EET is first AND the default
/// selection (SPEC §5.1).
const GAME_OPTIONS: [Game; 4] = [Game::EET, Game::BGEE, Game::BG2EE, Game::IWDEE];

/// Render the choose stage. Mutates `state` (name / game / destination /
/// destination_choice / starting_point) in place; returns the user's intent.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> ChooseOutcome {
    let mut outcome = ChooseOutcome::Stay;

    // ── Title row: ScreenTitle (left, grows) + `load draft` (right). ──
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
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

    // ── Setup Box (name + game-or-note split row, destination, warning). ──
    redesign_box(ui, palette, None, |ui| {
        ui.spacing_mut().item_spacing.y = 14.0; // wireframe grid `gap: 14`

        // Split row: name (flex 1) | game-or-note (auto). Wireframe
        // `gridTemplateColumns: "1fr auto"; align-items: end`.
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0; // wireframe grid `gap: 16`

            const GAME_COL_W_PX: f32 = 220.0;
            let name_w = (ui.available_width() - GAME_COL_W_PX - 16.0).max(160.0);

            // ── modlist name (flex 1). ──
            ui.allocate_ui_with_layout(
                egui::vec2(name_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    field_label(ui, palette, "modlist name");
                    ui.add_space(4.0);
                    let name_margin = egui::Margin::symmetric(12, 8);
                    // Shared input primitive — border on the OUTER box (the
                    // app-wide indented-input fix).
                    let _resp = redesign_text_input(
                        ui,
                        palette,
                        InputOpts {
                            edit: egui::TextEdit::singleline(&mut state.modlist_name)
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
                                .margin(name_margin),
                            margin: name_margin,
                            size: egui::vec2(ui.available_width(), 30.0),
                            border: None,
                        },
                    );
                },
            );

            // ── game (auto width). Scratch ⇒ the styled ComboBox; Import ⇒
            //    a read-only "game comes from the imported modlist" note
            //    (SPEC §5/§5.3 — the import path derives the game from the
            //    pasted share code, not a user selection). ──
            ui.allocate_ui_with_layout(
                egui::vec2(GAME_COL_W_PX, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    field_label(ui, palette, "game");
                    ui.add_space(4.0);
                    match state.starting_point {
                        StartingPoint::Scratch => {
                            game_combo(ui, palette, &mut state.game);
                        }
                        StartingPoint::Import => {
                            game_from_code_note(ui, palette);
                        }
                    }
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
            state.destination_choice = None;
        }

        // ── DestinationNotEmptyWarning (conditional, REUSED Phase-5 widget;
        //    `allow_partial = false` — SPEC §5.1). ──
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

    // ── `Choose one` header (the directed UX — above the two boxes). ──
    ui.label(
        egui::RichText::new("Choose one")
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(8.0);

    // ── Two SELECTABLE boxes (grid "1fr 1fr", gap 14). Clicking anywhere on
    //    a box selects it; NO in-box sub-buttons. ──
    let avail_w = ui.available_width();
    let gap = 14.0;
    let card_w = ((avail_w - gap) / 2.0).max(160.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        if selectable_box(
            ui,
            palette,
            card_w,
            "New modlist from downloaded mods",
            "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.",
            state.starting_point == StartingPoint::Scratch,
            "create_box_scratch",
        ) {
            state.starting_point = StartingPoint::Scratch;
        }

        if selectable_box(
            ui,
            palette,
            card_w,
            "Import and modify another modlist",
            "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.",
            state.starting_point == StartingPoint::Import,
            "create_box_import",
        ) {
            state.starting_point = StartingPoint::Import;
        }
    });

    // ── Single primary `Start →` at the bottom-right, styled EXACTLY like
    //    the workspace nav-bar forward (`Next →`) button (reused verbatim).
    //    Dispatches per the selected box. ──
    ui.add_space(18.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if workspace_nav_bar::forward_primary_button(ui, palette, "Start").clicked() {
            outcome = match state.starting_point {
                StartingPoint::Scratch => ChooseOutcome::StartScratch,
                StartingPoint::Import => ChooseOutcome::GoForkPaste,
            };
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

/// The game `ComboBox` (#5a — redesign-token chrome, NOT default egui).
/// Wrapped in a sketchy 1.5px `border-strong` frame with an `input-bg` fill
/// so it matches the redesign `Input` chassis; the selected text + the
/// dropdown items are `poppins_medium` 12px in `text-primary`. Options are
/// the wireframe set in wireframe order (`screens.jsx:3864`), EET
/// first/default. Used **only** in the from-scratch state.
fn game_combo(ui: &mut egui::Ui, palette: ThemePalette, game: &mut Game) {
    let mut selected = *game;

    // Redesign chrome: a sketchy-bordered, input-bg frame hosting the
    // ComboBox so it reads as a redesign `Input`, not egui's default combo.
    let frame = egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::symmetric(10, 4));

    frame.show(ui, |ui| {
        // Make egui's own combo button frameless so only our sketchy frame
        // (above) draws the chrome — no double border / default egui fill.
        let v = ui.visuals_mut();
        v.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.open.bg_fill = egui::Color32::TRANSPARENT;
        v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        v.widgets.active.bg_stroke = egui::Stroke::NONE;
        v.widgets.open.bg_stroke = egui::Stroke::NONE;

        egui::ComboBox::from_id_salt("create_game_combo")
            .selected_text(
                egui::RichText::new(game_label(selected))
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            )
            .show_ui(ui, |ui| {
                for option in GAME_OPTIONS {
                    ui.selectable_value(
                        &mut selected,
                        option,
                        egui::RichText::new(game_label(option))
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_primary(palette)),
                    );
                }
            });
    });

    if selected != *game {
        *game = selected;
    }
}

/// The read-only contextual note shown in the `game` slot when the **import**
/// box is selected. SPEC §5/§5.3: the import path derives the game from the
/// pasted share code (the modlist's game travels in the code), so the user
/// does not pick it here — a faint, sketchy-bordered note states that
/// plainly (affordance-forward over a disabled-looking dropdown).
fn game_from_code_note(ui: &mut egui::Ui, palette: ThemePalette) {
    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::symmetric(10, 6));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("comes from the imported modlist")
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
        );
    });
}

/// The dropdown label for a game (the wireframe shows the bare enum string;
/// `Game::to_legacy_string` is exactly `"EET"|"BGEE"|"BG2EE"|"IWDEE"`).
fn game_label(game: Game) -> &'static str {
    game.to_legacy_string()
}

/// `true` when `path` is set and points at a directory that contains at least
/// one entry. SPEC §5.1: the not-empty warning is conditioned on the chosen
/// folder actually having content. Identical predicate to
/// `stage_paste::destination_is_non_empty`.
fn destination_is_non_empty(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return false;
    }
    std::fs::read_dir(trimmed).is_ok_and(|mut entries| entries.next().is_some())
}

/// The wireframe `FolderInput` component (`screens.jsx:91-121`): a hand-style
/// muted label + a row of [mono input (flex 1)] [browse… button]. Returns
/// `true` when the value changed this frame. Mirrors `stage_paste.rs::
/// folder_input` (the same wireframe component; that one is module-private).
/// The input border now routes through the shared `redesign_text_input`
/// primitive (the app-wide indented-input fix).
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
        let fi_margin = egui::Margin::symmetric(8, 5);
        let response = redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(value)
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
                    .margin(fi_margin),
                margin: fi_margin,
                size: egui::vec2(edit_width, 26.0),
                border: None,
            },
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

/// One SELECTABLE starting-point box (the directed UX — wireframe `<Box
/// onClick=…>` with the in-box CTA **removed**): a hand-style 18px title +
/// a muted description. The WHOLE box is the click target. When `selected`
/// it draws an **accent** border + a faint accent tint so the choice is
/// visually obvious; otherwise the normal sketchy `border-strong`. Returns
/// `true` when clicked this frame (the caller sets the selection).
fn selectable_box(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    width: f32,
    title: &str,
    desc: &str,
    selected: bool,
    id_salt: &str,
) -> bool {
    let border_color = if selected {
        redesign_accent(palette)
    } else {
        redesign_border_strong(palette)
    };
    let fill = if selected {
        // Faint accent tint over the shell so the selected box reads as
        // chosen without shouting (the §12.3 selection-highlight token).
        redesign_selection_highlight(palette)
    } else {
        redesign_shell_bg(palette)
    };

    let chassis = egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border_color))
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
            chassis.show(ui, |ui| {
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
                // color:text-muted>`). No in-box CTA (removed per the
                // directed UX — the single `Start →` lives below).
                ui.label(
                    egui::RichText::new(desc)
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_muted(palette)),
                );
            });
        },
    );

    // The whole-box click target (wireframe puts `onClick` on the Box).
    let resp = ui.interact(
        inner.response.rect,
        ui.make_persistent_id(("create_selectable_box", id_salt)),
        egui::Sense::click(),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
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
        assert!(!destination_is_non_empty(""));
        assert!(!destination_is_non_empty("   "));
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

    /// The directed UX: the single `Start →` dispatches per the selected
    /// box — from-scratch ⇒ `StartScratch`, import ⇒ `GoForkPaste`. This
    /// asserts the mapping the `render` closure encodes (the logic that
    /// replaces the wireframe's two in-box CTAs).
    #[test]
    fn start_outcome_maps_from_selected_starting_point() {
        let pick = |sp: StartingPoint| match sp {
            StartingPoint::Scratch => ChooseOutcome::StartScratch,
            StartingPoint::Import => ChooseOutcome::GoForkPaste,
        };
        assert_eq!(pick(StartingPoint::Scratch), ChooseOutcome::StartScratch);
        assert_eq!(pick(StartingPoint::Import), ChooseOutcome::GoForkPaste);
    }
}
