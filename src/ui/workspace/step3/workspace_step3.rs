// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step3` — the **Step-3 C4 chrome** (P6.T2d). The direct
// analogue of `step2::workspace_step2` (P6.T2c) and `step4::
// workspace_step4` (P6.T2b): a net-new orchestrator-side Step-3 renderer
// that **does not** call BIO's `page_step3::render` / `frame_step3::render`
// / `content_step3::render` / `render_toolbar`, and — like the Step-2/4 C4
// — **rebuilds the chrome net-new**, reusing only BIO's data/state + the
// heavy drag-reorder list body (`list_step3::render`) read-only.
//
// ## Why net-new chrome (premise-checked against the canonical wireframe)
//
// The canonical wireframe `ComponentsPanel` (`screens.jsx:3023-3056`) is
// structurally different from BIO's `content_step3` top area. Its Step-3
// chrome is exactly:
//   - (the workspace shell renders the per-step hint above this — NOT here)
//   - Tab row: shared `GameTab`s + a flex cluster: aggregate `conflict`
//     `Pill` + aggregate `prompt` `Pill` + a right-aligned `span` of
//     `Undo` / `Redo` / `Collapse All` / `Expand All` `TopButton`s.
//   - `Box`: the drag-reorder list (BIO's own `list_step3::render` paints
//     its `ui.group` border + scroll — this wrapper does NOT add a box).
// There is **no** BIO heading, no `content_step3` hint line, no raw-egui
// tab/badge row, and **no** `Export diagnostics` / `Restart App With
// Diagnostics` button — exactly as Step-2's C4 dropped its dev button.
// Reusing BIO's `content_step3::render` / `render_toolbar` would
// reintroduce the old-BIO Step-3 top bar the wireframe replaced — so the
// chrome is net-new redesign code (the C4 precedent), reusing only the
// data/state reads + the `pub(crate)` action helpers BIO's own toolbar
// calls (directive decision-order step 1 — reuse, not reimplement; never
// call BIO's `render_*`).
//
// SPEC §7.1: an action-row count line ("_N_ components ready to install on
// _<tab>_ · across _M_ mods") sits **above** the tab row. The wireframe
// `ComponentsPanel` does not draw it (only Step-4's `OrderPanel` does), but
// SPEC §7.1 explicitly specifies it for Step 3 too, and SPEC prose wins
// where the wireframe is silent (spec-authority order). It is the same
// string Step-4 renders (`step3_action_row` reuses the shared
// `workspace_step4::active_tab_items` resolver so the two never drift).
//
// **Net-new chrome (this file + `step3_action_row` + `step3_tab_row`,
//   redesign tokens, orchestrator-owned rects):**
//   - The action-row count (SPEC §7.1) — `step3_action_row`.
//   - The redesign tab row (`step3_tab_row`): the SHARED redesign GameTabs
//     (reused verbatim — `crate::ui::workspace::widgets::game_tab`; single
//     game skips the strip exactly like Step-4), the aggregate
//     conflict/prompt clickable `Pill`s, and redesign `Undo`/`Redo`/
//     `Collapse All`/`Expand All`.
//   - **No** `Export diagnostics` / `Restart App With Diagnostics`. BIO
//     paints those in `render_toolbar`, which is never called — so they
//     are structurally absent (guaranteed by construction).
//   - **No** BIO heading / hint. The workspace shell already renders the
//     per-step hint under the progress bar (`workspace_hint_line`); the
//     wireframe `ComponentsPanel` (`screens.jsx:3024-3025`) has only the
//     shell hint — a second one here was the Step-2 #6 duplicate.
//
// **Reused BIO surface (read-only — directive decision-order step 1; the
//   heavy interaction surface, NOT reimplemented), matching
//   `content_step3::render` (`content_step3.rs:224-261`) exactly:**
//   - `state_step3::normalize_active_tab(state)` — keep the active tab
//     valid before any read (BIO does this first; without it a fresh
//     workspace whose `active_game_tab` doesn't match the game renders the
//     wrong bucket / empty list).
//   - `toolbar_support_step3::build_toolbar_summary(state)` — the pill
//     counts + the `active_markers` the list needs. `pub(crate)`,
//     same-crate reachable via carve-out #3.
//   - `state.step3.{bgee,bg2ee}_has_conflict` — set EXACTLY as
//     `content_step3::render:232-235` does (the list body reads these to
//     tone its rows; omitting it would mis-tone the reused list).
//   - `list_step3::render(ui, state, &mut jump_to_selected, markers)` —
//     the BIO drag-reorder list (drag, undo/redo, collapse/expand,
//     shift-click range, conflict/prompt pills) inside an orchestrator-
//     owned HARD-CLIPPED fixed-size rect (the Step-2 `clipped_pane`
//     precedent) so it can never bleed into the workspace nav bar. The
//     jump-flag round-trip mirrors `content_step3:254-260`.
//   - `content_step2::render_compat_popup` + `prompt_popup_step2::
//     render_prompt_popup` — the compat / prompt popups, driven exactly as
//     `content_step3:257-258` (the conflict/prompt `Pill`s open them via
//     the same `pub(crate)` `open_toolbar_*` helpers).
//
// ## Layout-rect ownership + hard list containment (the Step-2 precedent)
//
// BIO's `frame_step3::render` is a trivial pass-through to
// `content_step3::render` — no hidden scroll/clip wrapper. BIO's
// `list_step3::render` DOES paint its own `ui.group` border + an inner
// `ScrollArea` with a hard-coded `nav_clearance = 26.0`, but that clearance
// is relative to whatever `available_height` it is handed — if this wrapper
// let it consume the raw `ui.available_*`, its scroll area could still grow
// the parent UI and push the workspace nav bar (and its `Next →`)
// off-screen on a short window. So this wrapper lays every rect out from
// the **bounded** `ui.available_rect_before_wrap()` the workspace handed it
// (`workspace_view` already reserved the nav-bar footprint) and runs the
// reused `list_step3::render` inside `clipped_pane` (the verbatim Step-2
// helper): a child UI with its clip rect hard-set to the list rect, after
// which the parent placer is advanced by **exactly** the bounded rect
// (never by BIO's overgrown internal min-rect). The BIO list keeps its full
// behavior; it simply can never break its box or push the nav bar.
//
// ## No action enum (H2) — `render` returns `()`
//
// Step 3 has no `Step3Action`: BIO's list + this row mutate `WizardState`
// directly (drag-reorder via `state_drag_step3`, undo/redo via
// `step3_history`, collapse via `step3_history`, block-select). The
// orchestrator detects Step-3 mutations for persistence via the
// dirty-bit fingerprint over `wizard_state.step3.<tab>_items` in
// `step_action_dispatch` / the persistence cycle — that path is unchanged
// (this wrapper neither adds nor needs an action return).
//
// SPEC: §7 (§7.1 chrome elements; §7.2 reused BIO drag list; §7.6
//       Undo/Redo/Collapse/Expand), §1 (decision order; carve-out
//       boundary), §2.2, §6 (the Step-2 C4 precedent); wireframe
//       `screens.jsx:3023-3056`.

// rationale: f32→u8 corner-radius / pixel roundings of small positive
// layout constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step3::list_step3;
use crate::ui::step3::state_step3;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::step3::{step3_action_row, step3_tab_row};

/// Action-row height (the count Label line; wireframe `<Label hand>` 14px
/// ≈ 22 + a little slack).
const ACTION_ROW_H: f32 = 24.0;
/// Gap below the action row (wireframe `OrderPanel`/Step-4 action-row
/// `marginBottom: 10`; Step 3's mirrors it for a consistent action row).
const ACTION_ROW_GAP: f32 = 10.0;
/// Tab-row height (wireframe action sub-row `height: 30`, same as the
/// shared `game_tab::TAB_H`).
const TAB_ROW_H: f32 = 30.0;
/// The tab row overlaps the list box's top edge by 1.5px so the active
/// tab's shell-bg fill masks the box's top border (the wireframe `GameTab`
/// `marginBottom: -1.5px`, `screens.jsx:1630`). A negative seam — the same
/// `TAB_TO_GRID_OVERLAP` Step-2 uses.
const TAB_TO_LIST_OVERLAP: f32 = 1.5;
/// Minimum list height (never force-grown past the bounded rect).
const LIST_MIN_H: f32 = 160.0;

/// Render the Step-3 C4 chrome. Returns `()` per H2 — Step 3 has no action
/// enum; BIO's reused list + this chrome mutate `WizardState` directly and
/// the orchestrator's dirty-bit fingerprint over `step3.<tab>_items` picks
/// up reorder/collapse/undo for persistence (unchanged by this wrapper).
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;
    let state = &mut orchestrator.wizard_state;

    // ── Reused BIO seam — EXACTLY `content_step3::render:225-235`. ──
    // 1. Keep the active tab valid before any read.
    state_step3::normalize_active_tab(state);
    // 2. The pill counts + the `active_markers` the reused list needs.
    let toolbar_summary = toolbar_support_step3::build_toolbar_summary(state);
    // 3. Set `<tab>_has_conflict` EXACTLY as BIO does (the list body reads
    //    these to tone its rows — omitting it would mis-tone the list).
    state.step3.bgee_has_conflict = toolbar_summary.show_bgee
        && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bgee_markers);
    state.step3.bg2ee_has_conflict = toolbar_summary.show_bg2ee
        && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bg2ee_markers);

    // ── Own the layout rects from the BOUNDED available space (the Step-2
    //    precedent — never let BIO's list scroll grow the parent past the
    //    nav bar `workspace_view` reserved). ──
    let root = ui.available_rect_before_wrap();
    let x = root.left();
    let w = root.width();
    let mut y = root.top();

    // #6-style: NO per-step sub-hint here — the workspace shell already
    // renders the per-step hint under the progress bar (`workspace_hint_
    // line`); the wireframe `ComponentsPanel` (`screens.jsx:3024-3025`) has
    // only the shell hint. A second hint here would be the Step-2 #6
    // duplicate.

    let action_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, ACTION_ROW_H));
    y += ACTION_ROW_H + ACTION_ROW_GAP;

    let tab_row_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
    // The list box starts 1.5px ABOVE the tab row's bottom (negative seam)
    // so the active GameTab's shell-bg fill overlaps & masks the box's top
    // border — the wireframe "tab merges into the box" behavior.
    y += TAB_ROW_H - TAB_TO_LIST_OVERLAP;

    // The list region takes whatever vertical space remains down to the
    // bottom of the bounded rect — NOT force-grown, so BIO's list scroll
    // can never push past the nav bar.
    let list_h = (root.bottom() - y).max(LIST_MIN_H);
    let list_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, list_h));

    // ── 1. Net-new action-row count (SPEC §7.1). ──
    ui.scope_builder(egui::UiBuilder::new().max_rect(action_rect), |ui| {
        step3_action_row::render(ui, state, palette);
    });

    // ── 2. Net-new redesign tab row (shared GameTabs + aggregate
    //    conflict/prompt Pills + redesign Undo/Redo/Collapse/Expand).
    //    Replaces BIO's `render_toolbar` entirely (the wireframe has no BIO
    //    toolbar). Mutates `WizardState` via BIO's reused `pub(crate)`
    //    helpers; no action return (H2). ──
    step3_tab_row::render(ui, state, palette, &toolbar_summary, tab_row_rect);

    // ── 3. The reused BIO drag-reorder list inside a HARD-CLIPPED,
    //    FIXED-SIZE child UI (the verbatim Step-2 `clipped_pane`). BIO's
    //    `list_step3::render` paints its own `ui.group` border + scroll;
    //    `clipped_pane` (a) hard-sets the child's clip rect so the list
    //    cannot paint outside `list_rect`, and (b) advances the parent
    //    placer by EXACTLY the bounded rect so the list scroll can never
    //    expand the parent or shove the workspace nav bar off-screen. The
    //    BIO list keeps its full behavior (drag, undo/redo, collapse,
    //    shift-click range, pills) — it just can't break its box. The
    //    jump-flag round-trip mirrors `content_step3:254-260`. ──
    let active_markers = if state.step3.active_game_tab == "BGEE" {
        toolbar_summary.bgee_markers.clone()
    } else {
        toolbar_summary.bg2ee_markers.clone()
    };
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    state.step3.jump_to_selected_requested = false;
    clipped_pane(ui, list_rect, |ui| {
        list_step3::render(ui, state, &mut jump_to_selected_requested, &active_markers);
    });
    state.step3.jump_to_selected_requested =
        state.step3.jump_to_selected_requested || jump_to_selected_requested;

    // ── 4. Compat / prompt popups — driven EXACTLY as
    //    `content_step3:257-258` (the conflict/prompt `Pill`s opened them
    //    via the same `pub(crate)` `open_toolbar_*` helpers). ──
    crate::ui::step2::content_step2::render_compat_popup(ui, state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
}

/// Run the reused BIO list inside a **hard-clipped, fixed-size** child UI
/// bounded to `rect` — the verbatim Step-2 containment helper
/// (`workspace_step2::clipped_pane`).
///
/// `ui.new_child` creates a child that does **not** advance the parent's
/// placer; we set the child's `clip_rect` to `rect` so the BIO list
/// physically cannot paint outside its box, then advance the parent placer
/// by **exactly `rect`** (`allocate_rect`) — never by the list's internal
/// `min_rect`. Net effect: the BIO list keeps its full drag/scroll
/// behavior but can never expand `workspace_step3`'s parent UI or push the
/// workspace nav bar (and its `Next →`) off-screen.
fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    // Intersect with the inherited clip so we never paint OUTSIDE the rect
    // (nor outside any ancestor clip — e.g. the workspace scroll area).
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    // Advance the parent by EXACTLY the bounded rect — discard the child's
    // internal min_rect so the parent (and the nav bar after it) is never
    // pushed. `Sense::hover()` keeps it inert (the BIO child already
    // handled its own interactions).
    ui.allocate_rect(rect, egui::Sense::hover());
}
