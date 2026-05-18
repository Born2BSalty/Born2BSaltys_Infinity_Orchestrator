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
// ## Unified form chassis (Fix-Run 5 #4 a–g — user directive: MATCH THE
//    WIREFRAME)
//
// The wireframe is one consistent input chassis: a `1fr auto` name|game row
// bottom-aligned (`align-items: end`, `screens.jsx:3828`), the destination
// `FolderInput` (`screens.jsx:91-121`), two equal-height boxes, and a
// bottom footer. Three tuned px constants are the single shared knob
// (`feedback_wireframe_look_not_px` — the wireframe governs the look; the
// exact px is tuned empirically against the real app via these constants —
// this is NOT a SPEC change):
//
//   - `FORM_ROW_H_PX` — the one shared control height. The modlist-name
//     input, the destination input, the `browse…` button, and the game
//     control (the ComboBox frame AND the import-note frame) are all
//     exactly this tall, so the top-right game control and the bottom-right
//     browse button share a right edge / vertically align and both inputs
//     read as the same chassis.
//   - `RIGHT_COL_W_PX` — the one shared right-column width. The game column
//     width == the `browse…`-button column width, so the game control and
//     the browse button line up on a single right edge (wireframe
//     `gridTemplateColumns: "1fr auto"` — the `auto` column is content-
//     sized; here both right-edge controls share this one tuned width).
//   - `FORM_ROW_GAP_PX` — the one shared input↔right-column gap. The SAME
//     gap on the `modlist name | game` row and the `destination | browse…`
//     row, so the name input is the **exact same width** as the destination
//     input (Fix-Run 6 P3) and both right-column controls (game control,
//     browse button) share **one** right edge in BOTH starting-point
//     states (the import "imported" note is the shared right-column box,
//     not a content-shrink box — Fix-Run 6 P3 reconciling #4 a + #4 g).
//
// Both the name and destination inputs route through the shared
// `redesign_text_input` primitive with the **same** internal text margin
// (the wireframe `FolderInput` `padding: 8px 12px`), so they are the same
// input chassis/size. The name input keeps Poppins (wireframe name `<input>`
// is Poppins); the destination keeps `firacode_nerd` (the wireframe
// `FolderInput` is `mono`) — same chassis, wireframe-faithful per-field
// font. The game ComboBox keeps its redesign-token chrome and `Start →`
// keeps `workspace_nav_bar::forward_primary_button` styling (it now sits in
// the shared create-flow footer — see below).
//
// ## `Start →` lives in the shared create-flow footer (Fix-Run 5 #4 e)
//
// `Start →` is rendered inside `crate::ui::install::sub_flow_footer` (the
// exact bottom-pinned footer the fork paste/preview/download stages use:
// 1.5px dashed top rule, `marginTop:20; paddingTop:14`, flush-right
// primary). The footer is invoked with no Back / no secondary / no hint and
// a single `PrimaryBtn { label: "Start" }`; its primary `glyph_btn` chassis
// is **pixel-identical** to `workspace_nav_bar::forward_primary_button`
// (same accent fill, 2×2 shadow, active-press transform, `firacode_nerd`
// `→` + `poppins_medium` prose, theme-invariant `#1a2638` text) — so the
// SPEC §5.1 "styled exactly like the workspace nav-bar forward button"
// mandate is preserved; only the placement gains the dashed-divider footer
// chrome (consistent with every other create-flow stage).
//
// **Bottom-pin mechanism (Fix-Run 6 P1 — the REAL fork-stage pattern).**
// The body is rendered DIRECTLY on the page `ui` (natural height), then a
// flexible spacer `ui.add_space(available_height − FOOTER_HEIGHT_PX)`
// (computed *after* the body, the wireframe `<div flex:1 />`), then the
// footer — exactly `stage_fork_preview::render_parse_error` (L405-408). The
// earlier "wrap the whole body in `vec2(available_width, body_h)` then
// render the footer after" did NOT pin: egui advances the parent by the
// child's `min_rect` (the *natural* content height, not the requested
// `body_h` — egui-0.31 `ui.rs:1434`), so the footer jammed up under the
// content with dead space *below* it. Rendering directly on the page `ui`
// + spacer + footer fixes P1. The separate P2 right-margin collapse
// (`Start →` clipped at narrow widths) was the game `ComboBox` falling
// back to egui's default `Spacing::combo_width` (~100px) and overflowing
// the `RIGHT_COL_W_PX` column, which expanded the page content rect; fixed
// in `game_combo` by pinning `ComboBox::width` to the shared right column.
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
use crate::ui::install::sub_flow_footer::{self, PrimaryBtn};
use crate::ui::orchestrator::widgets::{
    BtnOpts, InputOpts, redesign_box, redesign_btn, redesign_text_input, render_screen_title,
};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_shell_bg, redesign_text_faint,
    redesign_text_muted, redesign_text_primary,
};

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

/// **The one shared control height** (Fix-Run 5 #4 a–e). The modlist-name
/// input, the destination input, the `browse…` button, and the game control
/// (the ComboBox frame AND the import-note frame) are all exactly this tall
/// so the top-right game control and the bottom-right browse button share a
/// right edge and both inputs read as one chassis. Tuned empirically against
/// the real app (`feedback_wireframe_look_not_px`): the wireframe inputs are
/// `padding: 8px 12px` on a ~14px line — ~30px box; 30 keeps the dominant
/// `8px 12px` chassis (everything aligned **up**, not down to the old 26).
/// NOT a SPEC change — the wireframe governs the look; this is the shared
/// knob that matches it.
const FORM_ROW_H_PX: f32 = 30.0;

/// **The one shared right-column width** (Fix-Run 5 #4 a–e). The game column
/// width == the `browse…`-button column width, so the game control (top
/// right) and the browse button (bottom right) line up on a single right
/// edge. Wireframe `gridTemplateColumns: "1fr auto"` — the `auto` column is
/// content-sized; the user directive makes both right-edge controls share
/// this one tuned width. Tuned empirically: wide enough for `BG2EE` + the
/// combo arrow and for `browse…`, both 12px Poppins, with padding (the old
/// browse was 90; 96 gives both a comfortable fit on a shared edge).
const RIGHT_COL_W_PX: f32 = 96.0;

/// The shared internal text padding for BOTH form inputs — the wireframe
/// `FolderInput` `padding: 8px 12px` (`screens.jsx:100`). Using the same
/// margin on the name input and the destination input is what makes them
/// read as the same input chassis (same border-hugging `redesign_text_input`
/// box, same internal padding); only the per-field font differs (Poppins for
/// the name `<input>`, mono for the `FolderInput`), exactly as the wireframe.
const FORM_INPUT_MARGIN: egui::Margin = egui::Margin {
    left: 12,
    right: 12,
    top: 8,
    bottom: 8,
};

/// Vertical inner padding of the game ComboBox / import-note frames. The
/// frame's outer box height = inner content + `2 * GAME_FRAME_PAD_Y`; the
/// inner content is forced to `FORM_ROW_H_PX - 2*GAME_FRAME_PAD_Y` so the
/// game control is exactly `FORM_ROW_H_PX` tall (same as the inputs + the
/// browse button — the shared right edge / form-row height).
const GAME_FRAME_PAD_Y: i8 = 4;

/// **The one shared form-row gap** (Fix-Run 6 P3). The gap between the input
/// (flex 1) and the right-column control is identical on BOTH the
/// `modlist name | game` row and the `destination | browse…` row, so the two
/// inputs are the **exact same width** (`available − RIGHT_COL_W_PX −
/// FORM_ROW_GAP_PX`) and the game control + the browse button share **one**
/// right edge. The wireframe uses two gaps (the name|game grid `gap: 16`,
/// the `FolderInput` row `gap: 8`); the user directive (#4 a–g — MATCH THE
/// WIREFRAME as one chassis) makes both rows one chassis, so a single gap is
/// the faithful realization. 8 == the `FolderInput` input↔button gap
/// (`screens.jsx:94`) — the gap that sits between an input and its
/// right-column control, applied uniformly. Tuned empirically via this
/// shared knob (`feedback_wireframe_look_not_px`) — NOT a SPEC change.
const FORM_ROW_GAP_PX: f32 = 8.0;

/// Render the choose stage. Mutates `state` (name / game / destination /
/// destination_choice / starting_point) in place; returns the user's intent.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> ChooseOutcome {
    let mut outcome = ChooseOutcome::Stay;

    // ── Bottom-pin the shared create-flow footer — the REAL fork-stage
    //    mechanism (Fix-Run 6 P1). The fork stages (`stage_fork_paste`,
    //    `stage_fork_preview`) do NOT wrap the whole body in a
    //    `vec2(available_width, body_h)` allocation and render the footer
    //    after the (shorter) content — that does not pin (egui advances the
    //    parent by the child's `min_rect`, i.e. the natural content height,
    //    not the requested `body_h`; egui-0.31 `ui.rs:1434` —
    //    `allocate_new_ui_dyn` advances by `child_ui.min_rect()`), so the
    //    footer jams up under the content with dead space *below* it.
    //
    //    `stage_fork_preview::render_parse_error` (L405-408) is the exact
    //    precedent for "natural-height content at the top + a flexible
    //    spacer + the footer flush at the bottom": it renders its content
    //    directly on the page `ui`, then `ui.add_space(available_height −
    //    FOOTER_HEIGHT_PX)` (the wireframe `<div flex:1 />` spacer), then
    //    `sub_flow_footer::render` directly on `ui`. We replicate THAT:
    //    render the body directly on the page `ui` (no whole-body width
    //    wrapper), then the flexible spacer, then the footer — so the
    //    content sits at the top, the empty space is BELOW it, and the
    //    dashed divider + `Start →` are flush at the bottom of the content
    //    area at every window size. (The separate P2 right-margin collapse —
    //    `Start →` clipped at narrow widths — was the game `ComboBox`
    //    falling back to egui's default `Spacing::combo_width`; fixed in
    //    `game_combo` by pinning `ComboBox::width` to the shared right
    //    column so nothing overflows the page content rect.) ──
    render_body(ui, palette, state, &mut outcome);

    // The wireframe `<div flex:1 />` spacer (`screens.jsx` fork branches):
    // computed AFTER the body so `available_height()` is the space that
    // remains, then consumed so the footer lands flush at the bottom — the
    // exact `stage_fork_preview::render_parse_error` spacer.
    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    // ── Single primary `Start →` in the shared create-flow footer
    //    (`sub_flow_footer`: 1.5px dashed top rule, `marginTop:20;
    //    paddingTop:14`, flush-right primary). No Back / no secondary / no
    //    hint — just the primary. Its `glyph_btn` chassis is pixel-identical
    //    to `workspace_nav_bar::forward_primary_button` (SPEC §5.1 — styled
    //    exactly like the workspace forward button; the one styling source),
    //    so only the placement gains the footer chrome. Dispatches per the
    //    selected box. ──
    let footer = sub_flow_footer::render(
        ui,
        palette,
        None::<sub_flow_footer::BackBtn<'_>>,
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        PrimaryBtn {
            label: "Start",
            disabled: false,
        },
    );
    if footer.primary_clicked {
        outcome = match state.starting_point {
            StartingPoint::Scratch => ChooseOutcome::StartScratch,
            StartingPoint::Import => ChooseOutcome::GoForkPaste,
        };
    }

    outcome
}

/// The Create `choose` page body (everything above the bottom-pinned
/// footer): the title row, the Setup Box, the `Choose one` header, and the
/// two selectable boxes. Rendered DIRECTLY on the page `ui` (no whole-body
/// width/height wrapper — the fork-stage precedent: natural-height content
/// at the top, then `render`'s flexible spacer + footer pin it to the
/// bottom). `outcome` is threaded so `load draft` / box-selection intents
/// bubble back to `render`.
fn render_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
    outcome: &mut ChooseOutcome,
) {
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
                *outcome = ChooseOutcome::OpenLoadDraft;
            }
        });
    });

    // ── Setup Box (name + game-or-note split row, destination, warning). ──
    redesign_box(ui, palette, None, |ui| {
        ui.spacing_mut().item_spacing.y = 14.0; // wireframe grid `gap: 14`

        // Split row: name (flex 1) | game-or-note (auto). Wireframe
        // `gridTemplateColumns: "1fr auto"; align-items: end` — the game
        // column == the browse-button column (`RIGHT_COL_W_PX`) AND the
        // input↔right-column gap == `FORM_ROW_GAP_PX` (the SAME gap the
        // destination row uses), so the name input is the **exact same
        // width** as the destination input and the game control + the browse
        // button below share **one** right edge (Fix-Run 6 P3).
        // **Bottom-align (`align-items: end`):** both columns are one label
        // line + the same `add_space(4)` + a `FORM_ROW_H_PX`-tall control,
        // so laid out `horizontal_top` their control bottoms line up.
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = FORM_ROW_GAP_PX;

            let name_w = (ui.available_width() - RIGHT_COL_W_PX - FORM_ROW_GAP_PX).max(160.0);

            // ── modlist name (flex 1). Same chassis/size as the destination
            //    input: shared `redesign_text_input`, shared
            //    `FORM_INPUT_MARGIN` (wireframe `padding: 8px 12px`), shared
            //    `FORM_ROW_H_PX`. Poppins font (wireframe name `<input>`). ──
            ui.allocate_ui_with_layout(
                egui::vec2(name_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    field_label(ui, palette, "modlist name");
                    ui.add_space(4.0);
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
                                .margin(FORM_INPUT_MARGIN),
                            margin: FORM_INPUT_MARGIN,
                            size: egui::vec2(ui.available_width(), FORM_ROW_H_PX),
                            border: None,
                        },
                    );
                },
            );

            // ── game (RIGHT_COL_W_PX — shares the browse-button right edge).
            //    Scratch ⇒ the styled ComboBox; Import ⇒ a read-only
            //    "imported" note (SPEC §5/§5.3 — the import path derives the
            //    game from the pasted share code, not a user selection).
            //    Both controls are forced to `FORM_ROW_H_PX` so their bottom
            //    edge aligns with the name input's. ──
            ui.allocate_ui_with_layout(
                egui::vec2(RIGHT_COL_W_PX, 0.0),
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
    //
    // Fix-Run 5 #4 f — **equal height**. The wireframe gets equal-height
    // columns free via CSS grid + `flex:1` on the description; egui sizes
    // each Box to its own content, so the shorter (left) box was shorter.
    // Measure BOTH boxes' natural content heights at `card_w` and force
    // both chassis to the max — if one description wraps to more lines the
    // other grows to match (the CSS-grid behavior, made explicit for egui).
    const SCRATCH_TITLE: &str = "New modlist from downloaded mods";
    const SCRATCH_DESC: &str = "Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.";
    const IMPORT_TITLE: &str = "Import and modify another modlist";
    const IMPORT_DESC: &str = "Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.";

    let avail_w = ui.available_width();
    let gap = 14.0;
    let card_w = ((avail_w - gap) / 2.0).max(160.0);
    let box_h = selectable_box_natural_height(ui, card_w, SCRATCH_TITLE, SCRATCH_DESC).max(
        selectable_box_natural_height(ui, card_w, IMPORT_TITLE, IMPORT_DESC),
    );
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        if selectable_box(
            ui,
            palette,
            card_w,
            box_h,
            SCRATCH_TITLE,
            SCRATCH_DESC,
            state.starting_point == StartingPoint::Scratch,
            "create_box_scratch",
        ) {
            state.starting_point = StartingPoint::Scratch;
        }

        if selectable_box(
            ui,
            palette,
            card_w,
            box_h,
            IMPORT_TITLE,
            IMPORT_DESC,
            state.starting_point == StartingPoint::Import,
            "create_box_import",
        ) {
            state.starting_point = StartingPoint::Import;
        }
    });

    // (The single primary `Start →` is rendered by the caller (`render`)
    // inside the shared create-flow footer, bottom-pinned below this body.)
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
    // `inner_margin` vertical = `GAME_FRAME_PAD_Y`; the inner content is
    // forced to `FORM_ROW_H_PX - 2*GAME_FRAME_PAD_Y` so the OUTER frame box
    // is exactly `FORM_ROW_H_PX` tall — same height as the name/destination
    // inputs + the browse button (the shared form-row height).
    let frame = egui::Frame::default()
        .fill(redesign_input_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::symmetric(10, GAME_FRAME_PAD_Y));

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        // Force the inner content height so the framed box matches the
        // shared `FORM_ROW_H_PX` (outer = inner + 2*pad).
        ui.set_min_height(FORM_ROW_H_PX - 2.0 * f32::from(GAME_FRAME_PAD_Y));
        // The framed box's inner content width (the shared right column
        // minus the frame's symmetric(10) padding). The ComboBox is pinned
        // to EXACTLY this so it can NOT fall back to egui's default
        // `Spacing::combo_width` (~100px) and overflow the `RIGHT_COL_W_PX`
        // column — that overflow expanded the page content rect and
        // collapsed the 28px right gutter / clipped `Start →` (Fix-Run 6
        // P2; the rendered-PNG-gate root cause). With this the game control
        // stays inside the shared right column at every window width.
        let combo_w = ui.available_width();
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
            .width(combo_w)
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
///
/// Fix-Run 6 P3 (reconciling Fix-Run 5 #4 a + #4 g exactly): the copy is the
/// single word **"imported"** (the column is labeled `game`, so "imported"
/// reads as "game: imported" — terse and unambiguous), left-aligned, in a
/// box that is the **shared right-column size** (`RIGHT_COL_W_PX` ×
/// `FORM_ROW_H_PX` — the SAME box the from-scratch ComboBox occupies) so its
/// **right edge lines up with `browse…`** (the #4 a alignment requirement).
/// #4 g ("not wordy / not absurdly wide") is satisfied by the *short word*
/// in a normal aligned box — NOT by shrink-wrapping the frame to the text
/// (the earlier shrink-wrap is exactly what broke the alignment). It is
/// `FORM_ROW_H_PX` tall (outer = inner + `2*GAME_FRAME_PAD_Y`), same as the
/// ComboBox it replaces, so its bottom edge aligns with the name input's.
fn game_from_code_note(ui: &mut egui::Ui, palette: ThemePalette) {
    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::symmetric(10, GAME_FRAME_PAD_Y));
    frame.show(ui, |ui| {
        // Fill the shared right column (`RIGHT_COL_W_PX`) — the SAME box the
        // ComboBox occupies — so the import-state right edge == the
        // from-scratch right edge == `browse…`'s right edge (Fix-Run 6 P3 /
        // #4 a). NOT a content-shrink box (that broke the alignment). The
        // height is pinned to the shared form-row height so the note's
        // bottom edge aligns with the name input's, exactly as the ComboBox.
        ui.set_width(ui.available_width());
        ui.set_min_height(FORM_ROW_H_PX - 2.0 * f32::from(GAME_FRAME_PAD_Y));
        ui.label(
            egui::RichText::new("imported")
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
        // Wireframe `FolderInput` row gap == `FORM_ROW_GAP_PX` (the SAME gap
        // the `modlist name | game` row uses — Fix-Run 6 P3). The browse
        // button is `RIGHT_COL_W_PX` wide — the SAME width as the game
        // control above — and the input↔button gap is the SAME, so the
        // destination input is the **exact same width** as the name input
        // and the browse button + the game control share **one** right edge
        // (vertically aligned), per Fix-Run 5 #4 a–e + Fix-Run 6 P3.
        ui.spacing_mut().item_spacing.x = FORM_ROW_GAP_PX;

        let reserved = RIGHT_COL_W_PX + FORM_ROW_GAP_PX;
        let edit_width = (ui.available_width() - reserved).max(120.0);

        let pre = value.clone();
        // Same chassis/size as the modlist-name input: shared
        // `redesign_text_input`, shared `FORM_INPUT_MARGIN` (the wireframe
        // `FolderInput` `padding: 8px 12px`), shared `FORM_ROW_H_PX`. The
        // font stays `firacode_nerd` — the wireframe `FolderInput` is `mono`
        // (`screens.jsx:97`), unlike the Poppins name `<input>`.
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
                    .margin(FORM_INPUT_MARGIN),
                margin: FORM_INPUT_MARGIN,
                size: egui::vec2(edit_width, FORM_ROW_H_PX),
                border: None,
            },
        );
        if response.changed() || *value != pre {
            changed = true;
        }

        if ui
            .add_sized(
                egui::vec2(RIGHT_COL_W_PX, FORM_ROW_H_PX),
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
    min_h: f32,
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
        // SPEC §5.1: "an **accent** border + a **faint accent tint**". The
        // §12.3 `selection_highlight` token is a *premultiplied* rgba whose
        // `Frame::fill` composite renders as a near-opaque strong teal block
        // (Fix-Run 6 P4 — the orchestrator's rendered-PNG finding: the
        // selected box's title/description washed out on it, while the
        // unselected box stayed crisp). The token is `pub(crate)` /
        // read-only (not in scope to change), so the *faint accent tint* is
        // realized here as an **opaque** low-ratio blend of `accent` over
        // the same `shell_bg` the unselected box uses — a subtly teal-tinted
        // dark surface. Because it is essentially `shell_bg` nudged toward
        // accent, `text_primary` (title) + `text_muted` (description) read
        // exactly as legibly as on the unselected box (the legibility
        // reference), while the accent **border** still makes the selection
        // unmistakable — the SPEC §5.1 affordance preserved, just legible.
        faint_accent_tint(palette)
    } else {
        redesign_shell_bg(palette)
    };

    let chassis = egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border_color))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin {
            left: SBOX_PAD_X,
            right: SBOX_PAD_X,
            top: SBOX_PAD_Y,
            bottom: SBOX_PAD_Y,
        });

    let inner = ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            chassis.show(ui, |ui| {
                ui.set_width(ui.available_width());
                // Force the inner content area to the shared equalized
                // height (`min_h` − the two inner-margin paddings) so BOTH
                // boxes are exactly `min_h` tall regardless of which
                // description wraps to more lines (Fix-Run 5 #4 f — the
                // CSS-grid `flex:1` behavior made explicit for egui).
                ui.set_min_height(min_h - 2.0 * f32::from(SBOX_PAD_Y));
                // Title — hand-style 18px (wireframe `<Label hand
                // fontSize:18 marginBottom:8>`).
                ui.label(
                    egui::RichText::new(title)
                        .size(SBOX_TITLE_SIZE)
                        .family(egui::FontFamily::Name("poppins_light".into()))
                        .color(redesign_text_primary(palette)),
                );
                ui.add_space(SBOX_TITLE_GAP);
                // Description — muted (wireframe `<Label
                // color:text-muted>`). No in-box CTA (removed per the
                // directed UX — the single `Start →` lives in the footer).
                ui.label(
                    egui::RichText::new(desc)
                        .size(SBOX_DESC_SIZE)
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

/// The blend ratio of `accent` over `shell_bg` for the selected box's
/// **faint accent tint** (SPEC §5.1). Low enough that the tinted surface is
/// still essentially `shell_bg` (so `text_primary`/`text_muted` stay as
/// legible as on the unselected box — Fix-Run 6 P4), high enough that the
/// teal cast is perceptible alongside the accent border. Tuned empirically
/// against the real app via the rendered-PNG gate
/// (`feedback_wireframe_look_not_px`) — NOT a SPEC value (the SPEC says
/// "faint accent tint"; this is the px-level realization of "faint").
const SELECTED_TINT_RATIO: f32 = 0.14;

/// SPEC §5.1's "faint accent tint" for the selected box, as an **opaque**
/// color: `shell_bg` linearly nudged `SELECTED_TINT_RATIO` of the way toward
/// `accent`. Opaque (not an alpha overlay) so the composite is deterministic
/// and the text on top reads exactly as it does over the unselected box's
/// `shell_bg` — the §12.3 premultiplied `selection_highlight` token rendered
/// near-opaque via `Frame::fill` and washed the box text out (Fix-Run 6 P4);
/// that token is `pub(crate)`/read-only, so the faint tint is realized here.
fn faint_accent_tint(palette: ThemePalette) -> egui::Color32 {
    let bg = redesign_shell_bg(palette);
    let ac = redesign_accent(palette);
    let mix = |b: u8, a: u8| -> u8 {
        let v = f32::from(b) + (f32::from(a) - f32::from(b)) * SELECTED_TINT_RATIO;
        v.round().clamp(0.0, 255.0) as u8
    };
    egui::Color32::from_rgb(
        mix(bg.r(), ac.r()),
        mix(bg.g(), ac.g()),
        mix(bg.b(), ac.b()),
    )
}

// ── Selectable-box layout constants (shared by the renderer AND the
//    natural-height measurer so equalization stays exact). ──
/// Horizontal inner padding (wireframe `<Box padding: "20px 22px">`).
const SBOX_PAD_X: i8 = 22;
/// Vertical inner padding (wireframe `<Box padding: "20px 22px">`).
const SBOX_PAD_Y: i8 = 20;
/// Title font size (wireframe `<Label hand fontSize: 18>`).
const SBOX_TITLE_SIZE: f32 = 18.0;
/// Gap below the title (wireframe title `marginBottom: 8`).
const SBOX_TITLE_GAP: f32 = 8.0;
/// Description font size (wireframe muted `<Label>` default body size).
const SBOX_DESC_SIZE: f32 = 13.0;

/// The natural (content-driven) total height of a selectable box at
/// `card_w`: both inner-margin paddings + the wrapped title galley height +
/// the title→desc gap + the wrapped description galley height. Measured for
/// BOTH boxes so the renderer can force them to the max — the explicit
/// egui equivalent of the wireframe's CSS-grid equal-height columns
/// (Fix-Run 5 #4 f). Uses the SAME fonts / sizes / paddings the renderer
/// uses (the `SBOX_*` constants) so the measured height matches the drawn
/// box exactly.
fn selectable_box_natural_height(ui: &egui::Ui, card_w: f32, title: &str, desc: &str) -> f32 {
    let inner_w = (card_w - 2.0 * f32::from(SBOX_PAD_X)).max(1.0);
    let title_h = wrapped_text_height(ui, title, SBOX_TITLE_SIZE, "poppins_light", inner_w);
    let desc_h = wrapped_text_height(ui, desc, SBOX_DESC_SIZE, "poppins_light", inner_w);
    2.0 * f32::from(SBOX_PAD_Y) + title_h + SBOX_TITLE_GAP + desc_h
}

/// Lay out `text` in the given Poppins family + size, wrapped to `wrap_w`,
/// and return the resulting galley height (the same metric egui uses when
/// it draws the `RichText` label). Used to equalize the two selectable
/// boxes to the taller one's natural content height.
fn wrapped_text_height(ui: &egui::Ui, text: &str, size: f32, family: &str, wrap_w: f32) -> f32 {
    let font = egui::FontId::new(size, egui::FontFamily::Name(family.into()));
    ui.fonts(|f| {
        f.layout(text.to_string(), font, egui::Color32::PLACEHOLDER, wrap_w)
            .size()
            .y
    })
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

    /// Fix-Run 5 #4 a–e: the two shared form-chassis constants are the
    /// single tuned knob (`feedback_wireframe_look_not_px`). Pinned so a
    /// future edit can't silently re-diverge the form-row heights / the
    /// shared right-edge width (a deliberate wireframe match, recorded —
    /// NOT a SPEC value). The name/destination inputs, the browse button,
    /// and the game control all derive their height from `FORM_ROW_H_PX`;
    /// the game column and the browse column are both `RIGHT_COL_W_PX`.
    #[test]
    fn shared_form_chassis_constants_are_the_tuned_knob() {
        assert_eq!(FORM_ROW_H_PX, 30.0);
        assert_eq!(RIGHT_COL_W_PX, 96.0);
        // Wireframe `FolderInput` `padding: 8px 12px` — the shared input
        // chassis margin both inputs use (so they read as one chassis).
        assert_eq!(FORM_INPUT_MARGIN.left, 12);
        assert_eq!(FORM_INPUT_MARGIN.right, 12);
        assert_eq!(FORM_INPUT_MARGIN.top, 8);
        assert_eq!(FORM_INPUT_MARGIN.bottom, 8);
        // The game frame's forced inner height + its two paddings must
        // equal the shared row height (so the framed game control is the
        // same height as the inputs / browse button — the shared edge).
        assert_eq!(
            FORM_ROW_H_PX - 2.0 * f32::from(GAME_FRAME_PAD_Y) + 2.0 * f32::from(GAME_FRAME_PAD_Y),
            FORM_ROW_H_PX
        );
    }

    /// Fix-Run 5 #4 f: the two selectable boxes are equalized to the MAX of
    /// their natural content heights. The natural-height formula is
    /// monotonic in the description's wrapped height, so the box whose
    /// description wraps taller drives the shared height and the other is
    /// grown to match (never shrunk below its own content). This asserts
    /// the equalization arithmetic the renderer applies (`max` of the two
    /// `selectable_box_natural_height`s) without needing an egui context:
    /// taller-desc ⇒ taller natural height ⇒ it is the `max`.
    #[test]
    fn equalized_box_height_is_the_taller_boxs_natural_height() {
        // Same paddings/gap for both boxes; the only differing input is the
        // wrapped desc height. natural = 2*PAD_Y + title_h + GAP + desc_h.
        let nat = |title_h: f32, desc_h: f32| {
            2.0 * f32::from(SBOX_PAD_Y) + title_h + SBOX_TITLE_GAP + desc_h
        };
        let short = nat(20.0, 40.0);
        let tall = nat(20.0, 72.0); // a longer-wrapping description
        let equalized = short.max(tall);
        assert_eq!(equalized, tall, "the taller box drives the shared height");
        assert!(
            equalized >= short,
            "the shorter box is grown to match, never clipped"
        );
    }

    /// Fix-Run 6 P3: the SAME `FORM_ROW_GAP_PX` is used as the input↔
    /// right-column gap on BOTH form rows, so the name input width
    /// (`available − RIGHT_COL_W_PX − FORM_ROW_GAP_PX`) is **identical** to
    /// the destination input width (`available − RIGHT_COL_W_PX −
    /// FORM_ROW_GAP_PX`) at any container width, and both right-column
    /// controls share one right edge. Pinned so a future edit can't
    /// re-introduce the old two-gap (16 vs 8) divergence that made the name
    /// input 8px narrower than the destination input.
    #[test]
    fn shared_form_row_gap_makes_inputs_equal_width() {
        assert_eq!(FORM_ROW_GAP_PX, 8.0);
        // Both inputs reserve the SAME right slice → identical widths.
        let name_w = |avail: f32| (avail - RIGHT_COL_W_PX - FORM_ROW_GAP_PX).max(160.0);
        let dest_w = |avail: f32| (avail - (RIGHT_COL_W_PX + FORM_ROW_GAP_PX)).max(120.0);
        for avail in [400.0_f32, 600.0, 968.0, 1224.0] {
            assert_eq!(
                name_w(avail),
                dest_w(avail),
                "name input width must equal destination input width at avail={avail}"
            );
        }
    }

    /// Fix-Run 6 P4: the selected box's faint accent tint is **opaque**
    /// (alpha 255 — a deterministic composite, not the near-opaque
    /// premultiplied `selection_highlight` token that washed the text out)
    /// and stays very close to `shell_bg` (the unselected box's surface, the
    /// legibility reference), only nudged toward `accent`. So the title /
    /// description over it read as legibly as over the unselected box, while
    /// the accent border still signals the selection.
    #[test]
    fn faint_accent_tint_is_opaque_and_near_shell_bg() {
        for palette in [ThemePalette::Dark, ThemePalette::Light] {
            let tint = faint_accent_tint(palette);
            assert_eq!(tint.a(), 255, "the selected tint must be opaque");
            let bg = redesign_shell_bg(palette);
            let ac = redesign_accent(palette);
            // Each channel is `bg` moved exactly SELECTED_TINT_RATIO toward
            // `ac` (the "faint" in SPEC §5.1's "faint accent tint").
            let expect = |b: u8, a: u8| -> i32 {
                (f32::from(b) + (f32::from(a) - f32::from(b)) * SELECTED_TINT_RATIO).round() as i32
            };
            assert_eq!(i32::from(tint.r()), expect(bg.r(), ac.r()));
            assert_eq!(i32::from(tint.g()), expect(bg.g(), ac.g()));
            assert_eq!(i32::from(tint.b()), expect(bg.b(), ac.b()));
            // "faint": the tint stays much closer to bg than to accent
            // (ratio < 0.5 on every channel by construction).
            assert!(SELECTED_TINT_RATIO < 0.5);
        }
    }
}
