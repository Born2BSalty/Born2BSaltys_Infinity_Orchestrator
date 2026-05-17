// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step3_tab_row` — the **net-new redesign Step-3 tab row** (P6.T2d). The
// direct analogue of `step2_tab_row` (P6.T2c): net-new redesign layout,
// reusing **only** BIO data/state reads + the `pub(crate)` action helpers
// BIO's own `content_step3::render_toolbar` calls (directive
// decision-order step 1 — read-only reuse, no logic reimplementation).
//
// Mirrors the wireframe `ComponentsPanel` tab row exactly
// (`wireframe-preview/screens.jsx:3026-3056`):
//
//   <div flex alignItems:flex-start gap:4 position:relative zIndex:1>
//     {tabs.map(t => <GameTab active onClick=setGameTab>{t.label}</GameTab>)}
//     <div flex:1 flex gap:8 paddingLeft:12 paddingRight:4 height:30 wrap>
//       {conflictCount > 0 && <Pill tone=danger>{n} conflict[s]</Pill>}
//       {promptCount   > 0 && <Pill tone=warn>{n} prompt[s]</Pill>}
//       <span marginLeft:auto flex gap:6>
//         <TopButton>Undo</TopButton>     <TopButton>Redo</TopButton>
//         <TopButton>Collapse All</TopButton> <TopButton>Expand All</TopButton>
//       </span>
//     </div>
//   </div>
//
// **The wireframe has NO BIO toolbar** (no `content_step3::render_toolbar`).
// This row reproduces the wireframe shape with redesign primitives; it
// never calls BIO's `content_step3::render_*`. Curated exactly as Step-2's
// C4 dropped its dev button: **NO** "Export diagnostics" / "Restart App
// With Diagnostics" (BIO paints those in `render_toolbar`, never called —
// structurally absent by construction), **NO** BIO heading / hint (the
// workspace shell renders the per-step hint via `workspace_hint_line`).
//
// ## Action / state mapping (read off `content_step3::render_toolbar` —
//    the authoritative source, reproduced read-only):
//
//   - GameTabs      → write `state.step3.active_game_tab` via the SHARED
//                      `crate::ui::workspace::widgets::game_tab::game_tab`
//                      (reused verbatim — Step 2 / 3 / 4 render the one
//                      widget identically; the 2026-05-17 unification).
//                      Single-game modlists skip the strip exactly like
//                      Step-4 (`workspace_step4::is_dual_game`).
//   - aggregate conflict `Pill` → count = `summary.<tab>_summary.0`
//                      (`build_toolbar_summary`, `step3_toolbar.rs:76-77`);
//                      click → `toolbar_support_step3::
//                      open_toolbar_issue_popup(state, target)` with the
//                      tab's `<tab>_target` (the exact helper + target
//                      `render_toolbar:109-122` uses).
//   - aggregate prompt `Pill`   → count = `summary.<tab>_prompt_count`
//                      (`step3_toolbar.rs:57-72`); click →
//                      `prompt_popup_step2::open_toolbar_prompt_popup(
//                      state, "Prompt Components (<tab>)")` (the exact
//                      helper + title `render_toolbar:128-133/146-163`
//                      uses).
//   - `Undo` / `Redo`           → `toolbar_support_step3::{undo_active,
//                      redo_active}` (the exact `pub(crate)` fns
//                      `render_toolbar:206-219` calls); enable/disable via
//                      `state.step3.{bgee,bg2ee}_{undo,redo}_stack.
//                      is_empty()` (the exact predicates
//                      `render_toolbar:167-176` uses).
//   - `Collapse All` / `Expand All` → `toolbar_support_step3::
//                      {collapse_all_active, expand_all_active}` (the exact
//                      `pub(crate)` fns `render_toolbar:192-205` calls).
//
// None of these reimplement BIO logic — every action delegates to the same
// `pub(crate)` helper / public-field write `content_step3::render_toolbar`
// performs (directive decision-order step 1: reuse BIO's public-at-crate
// API; never call BIO's `render_*`). Step 3 has no action enum (H2) — the
// row mutates `WizardState` directly (drag-reorder / undo / collapse stay
// BIO's, reused) and returns `()`.
//
// SPEC: §7.1 (Step-3 chrome elements), §7.6 (Undo/Redo/Collapse/Expand),
//       §1 (decision order), §2.2; wireframe `screens.jsx:3026-3056`.

// rationale: f32→u8 corner-radius / pixel roundings of small positive
// layout constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step3_toolbar::Step3ToolbarSummary;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_pill_danger, redesign_pill_text, redesign_pill_warn,
};
use crate::ui::step2::prompt_popup_step2::open_toolbar_prompt_popup;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::step4::workspace_step4;
use crate::ui::workspace::widgets::game_tab::game_tab;

/// Gap between GameTabs (wireframe outer row `gap: 4`). The tab geometry
/// itself (height, padding) lives in the shared `widgets::game_tab` widget.
const TAB_GAP: f32 = 4.0;
/// Gap between action-row items (wireframe inner row `gap: 8`).
const ITEM_GAP: f32 = 8.0;
/// Left pad before the action sub-row (wireframe inner `paddingLeft: 12`).
const ACTION_LEFT_PAD: f32 = 12.0;
/// Gap between the right-cluster buttons (wireframe `span` `gap: 6`).
const BTN_GAP: f32 = 6.0;

/// Render the net-new redesign Step-3 tab row into `rect`. Returns `()` —
/// Step 3 has no action enum (H2); every control mutates `WizardState`
/// directly via BIO's reused `pub(crate)` helpers.
///
/// `pub(crate)` (not `pub` like the sibling Step-2/4 split helpers):
/// `summary` is BIO's `pub(crate)` `Step3ToolbarSummary` (`step3_toolbar.
/// rs:19`), so a `pub` signature would expose a more-private type
/// (`private_interfaces`). It is only ever called by the same-crate
/// `workspace_step3` — `pub(crate)` is the idiomatic visibility and is
/// behaviour-identical.
pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    palette: ThemePalette,
    summary: &Step3ToolbarSummary,
    rect: egui::Rect,
) {
    // Active-tab predicates (public-state reads only — the exact conditions
    // `content_step3::render_toolbar` uses, `content_step3.rs:97-176`).
    let active_is_bgee = state.step3.active_game_tab == "BGEE";

    // Aggregate conflict count + first-issue target for the active tab —
    // the exact `Step3ToolbarSummary` fields `render_toolbar:97-122` reads.
    let (conflict_count, conflict_target) = if active_is_bgee {
        (summary.bgee_summary.0, summary.bgee_target.clone())
    } else {
        (summary.bg2ee_summary.0, summary.bg2ee_target.clone())
    };
    // Aggregate prompt count for the active tab — the exact field
    // `render_toolbar:123-133` reads.
    let prompt_count = if active_is_bgee {
        summary.bgee_prompt_count
    } else {
        summary.bg2ee_prompt_count
    };

    // Undo/Redo enable predicates — the exact `render_toolbar:167-176`
    // conditions (per the active tab's history stacks).
    let can_undo = if active_is_bgee {
        !state.step3.bgee_undo_stack.is_empty()
    } else {
        !state.step3.bg2ee_undo_stack.is_empty()
    };
    let can_redo = if active_is_bgee {
        !state.step3.bgee_redo_stack.is_empty()
    } else {
        !state.step3.bg2ee_redo_stack.is_empty()
    };

    // Single-game modlists skip the GameTab strip exactly like Step-4
    // (SPEC §7.1 implies the same single-vs-dual tab model as §6/§8;
    // `workspace_step4::is_dual_game` is the shared EET-only predicate).
    let dual_game = workspace_step4::is_dual_game(state);

    // Deferred actions — collected during the `ui` closure (which holds
    // `&mut state` only for the GameTab writes), applied after, to keep the
    // BIO `pub(crate)` helper calls outside the tab-write borrow.
    let mut pending: Option<Step3RowIntent> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TAB_GAP;

            // ── GameTabs (the SHARED `game_tab` widget, reused verbatim —
            //    `screens.jsx:1609-1637`; single-game skips the strip). ──
            if dual_game {
                game_tab(ui, palette, "BGEE", &mut state.step3.active_game_tab);
                game_tab(ui, palette, "BG2EE", &mut state.step3.active_game_tab);
            }

            // ── Action sub-row (`flex:1`, the wireframe inner div). ──
            if dual_game {
                ui.add_space(ACTION_LEFT_PAD - TAB_GAP);
            }
            ui.spacing_mut().item_spacing.x = ITEM_GAP;

            // Aggregate conflict `Pill` (tone=danger) — shown only when the
            // count > 0 (wireframe `conflictCount > 0 &&`).
            if conflict_count > 0 {
                let word = if conflict_count == 1 {
                    "conflict"
                } else {
                    "conflicts"
                };
                let tip = format!(
                    "{conflict_count} compatibility {} in the {} Step 3 tab.",
                    if conflict_count == 1 {
                        "issue"
                    } else {
                        "issues"
                    },
                    state.step3.active_game_tab,
                );
                if clickable_pill(
                    ui,
                    palette,
                    &format!("{conflict_count} {word}"),
                    redesign_pill_danger(palette),
                    &tip,
                )
                .clicked()
                {
                    pending = Some(Step3RowIntent::OpenConflict);
                }
            }

            // Aggregate prompt `Pill` (tone=warn) — shown only when the
            // count > 0 (wireframe `promptCount > 0 &&`).
            if prompt_count > 0 {
                let word = if prompt_count == 1 {
                    "prompt"
                } else {
                    "prompts"
                };
                if clickable_pill(
                    ui,
                    palette,
                    &format!("{prompt_count} {word}"),
                    redesign_pill_warn(palette),
                    crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS,
                )
                .clicked()
                {
                    pending = Some(Step3RowIntent::OpenPrompt);
                }
            }

            // ── Right cluster (`marginLeft: auto`): Undo / Redo /
            //    Collapse All / Expand All (wireframe `span` order). With a
            //    right-to-left layout the items are laid out reversed, so
            //    push them in reverse to land Undo→Redo→Collapse→Expand
            //    left-to-right. ──
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = BTN_GAP;

                if redesign_btn(
                    ui,
                    palette,
                    "Expand All",
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPAND_ALL)
                .clicked()
                {
                    pending = Some(Step3RowIntent::ExpandAll);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Collapse All",
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_COLLAPSE_ALL)
                .clicked()
                {
                    pending = Some(Step3RowIntent::CollapseAll);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Redo",
                    BtnOpts {
                        small: true,
                        disabled: !can_redo,
                        ..Default::default()
                    },
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_REDO)
                .clicked()
                    && can_redo
                {
                    pending = Some(Step3RowIntent::Redo);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Undo",
                    BtnOpts {
                        small: true,
                        disabled: !can_undo,
                        ..Default::default()
                    },
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP3_UNDO)
                .clicked()
                    && can_undo
                {
                    pending = Some(Step3RowIntent::Undo);
                }
            });
        });
    });

    // Apply the deferred intent by delegating to the **exact** `pub(crate)`
    // helper `content_step3::render_toolbar` calls for the same control
    // (directive decision-order step 1 — reuse, not reimplement).
    if let Some(intent) = pending {
        match intent {
            Step3RowIntent::OpenConflict => {
                if let Some(target) = conflict_target.as_ref() {
                    toolbar_support_step3::open_toolbar_issue_popup(state, target);
                }
            }
            Step3RowIntent::OpenPrompt => {
                open_toolbar_prompt_popup(
                    state,
                    &format!("Prompt Components ({})", state.step3.active_game_tab),
                );
            }
            Step3RowIntent::Undo => toolbar_support_step3::undo_active(state),
            Step3RowIntent::Redo => toolbar_support_step3::redo_active(state),
            Step3RowIntent::CollapseAll => toolbar_support_step3::collapse_all_active(state),
            Step3RowIntent::ExpandAll => toolbar_support_step3::expand_all_active(state),
        }
    }
}

/// Tab-row control intents — collected inside the (borrow-restricted) `ui`
/// closure, then applied after it returns (the BIO `pub(crate)` helper
/// calls need `&mut state` after the GameTab writes release it).
#[derive(Clone, Copy)]
enum Step3RowIntent {
    OpenConflict,
    OpenPrompt,
    Undo,
    Redo,
    CollapseAll,
    ExpandAll,
}

// The Step-3 GameTab is the ONE shared
// `crate::ui::workspace::widgets::game_tab::game_tab` widget (imported
// above; called at the GameTabs row). No per-step duplicate painter, and it
// has **no bottom bar in any state** — Step 2 / 3 / 4 render this one
// widget identically (the 2026-05-17 unification — the whole point).

/// A clickable wireframe `Pill` (`screens.jsx:759-788` with `onClick`):
/// rounded chip, tinted fill, fixed dark `pill_text`, `cursor: pointer`.
///
/// The shared `pill::render` widget is hover-only (`Sense::hover()`); the
/// wireframe's Step-3 conflict / prompt pills are clickable. Rather than
/// edit the out-of-scope shared widget, this paints the identical pill
/// chassis (same fill / radius / `pill_text` tokens) with `Sense::click()`
/// — a net-new redesign-token-styled control, the C4 "rebuild chrome,
/// reuse data" approach (verbatim from the established `step2_tab_row`
/// `clickable_pill`).
fn clickable_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    fill: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let pad_x = 6.0;
    let pad_y = 1.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let size = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Wireframe `Pill` `borderRadius: 7`.
        painter.rect_filled(rect, egui::CornerRadius::same(7), fill);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }
    response.on_hover_text(tooltip.to_owned())
}
