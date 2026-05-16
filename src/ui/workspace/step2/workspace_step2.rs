// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step2` — the **Step-2 C4 chrome wrapper** (P6.T2c). The direct
// analogue of `step4::workspace_step4` (P6.T2b): a net-new orchestrator-side
// Step-2 renderer that **does not** call BIO's `page_step2` / `frame_step2`,
// and — like Step-4's C4 — **rebuilds the chrome net-new**, reusing only
// BIO's data/state + heavy interaction sub-renderers (tree / details /
// popups), never BIO's `content_step2::render_header` / `render_controls` /
// `render_tabs`.
//
// ## Why net-new chrome (premise-checked against the canonical wireframe)
//
// The canonical wireframe `SourcesPanel` (`wireframe-preview/screens.jsx:
// 2786-2880`) has **no BIO controls row at all**. Its Step-2 chrome is
// exactly:
//   - Title `Mods / Components` (15px / 500).
//   - Search row: `<Input flex:1>` + `<TopButton>Rescan Mods Folder</TopButton>`.
//   - Tab row: `GameTab`s + (`!fork`) `Select <Tab> via WeiDU Log` +
//     `Updates...` + a compat `Pill` (live count) + a `PROMPT` `Pill`
//     (live count) + a `{sel}/{total} on <Tab>` count Label + a `Kebab`
//     (`Show Details panel` ✓-state, `Clear All`, `Select Visible`,
//     `Collapse All`, `Expand All`, `Jump to Selected`).
//   - Grid: `ComponentTree` Box + `DetailsPanel` (only when `detailsOpen`).
// There is **no** Scan / Cancel-Scan / Select-Visible / Collapse-All /
// Expand-All / Jump button *row*; those controls live in the Kebab + the
// Rescan button. Reusing BIO's `content_step2::render_controls` /
// `render_tabs` would reintroduce the old-BIO toolbar the user explicitly
// rejected — so the chrome is net-new redesign code (the C4 precedent),
// reusing only the data/state reads + the `pub(crate)` action helpers BIO's
// own toolbar calls (directive decision-order step 1 — reuse, not
// reimplement; never call BIO's `render_*`).
//
// **Net-new chrome (this file + `step2_search` + `step2_tab_row`,
//   redesign tokens, orchestrator-owned rects):**
//   - Title `Mods / Components` ONLY (no sub-hint — the workspace shell
//     already renders the per-step hint under the progress bar; the #6
//     fix removed the duplicate).
//   - A **full-width `flex` search** (`step2_search`) writing the *same*
//     `state.step2.search_query` field BIO's tree filters on, plus the
//     wireframe `Rescan Mods Folder` button — **disabled pre-Phase-7**
//     (the #2 fix: no per-install extracted-mods target yet, SPEC
//     §13.12a), replaced in place by **`Cancel Scan`** while a scan runs
//     (the #7 addition, emits `Step2Action::CancelScan`). The functional
//     scan path pre-Phase-7 is the dev-only scan affordance.
//   - The net-new redesign tab row (`step2_tab_row`): redesign GameTabs
//     writing `state.step2.active_game_tab`, the per-tab log/updates
//     buttons, the live compat / prompt `Pill`s, the count Label, and the
//     redesign Kebab.
//   - **No "Restart App With Diagnostics".** BIO paints that in
//     `render_header`, which is never called — so it is structurally absent
//     (guaranteed by construction).
//   - The Details pane defaults **hidden** (SPEC §6); the Kebab's "Show
//     Details panel" item toggles it, and a tree-row / `[?]` click
//     auto-opens it (the wireframe `ComponentTree`
//     `onSelect → setDetailsOpen(true)`).
//
// **Reused BIO sub-renderers (read-only public API — directive
//   decision-order step 1; the heavy interaction surfaces, NOT
//   reimplemented):**
//   - `list_pane_step2::render_list_pane` — the component tree.
//   - `details_pane_step2::render_pane` — the Details panel (only when
//     `details_open`).
//   - `compat_window_step2::render` / `prompt_popup_step2::
//     render_prompt_popup` / `update_check_popup_step2::render` — the
//     compat / prompt / updates popups (driven exactly as
//     `bio::ui::app::update_loop::render_shared_popups` does for
//     `WizardApp`).
//
// ## The #4 + #1 fixes — layout-rect ownership + hard pane containment
//
// (#4) BIO's `frame_step2::render` does `ui.allocate_exact_size(vec2(
// x.max(900), y.max(620)), …)` — it force-claims a 620px-min height
// regardless of the space the host gave it, which made the embedded panel
// bleed past the workspace nav bar. This wrapper instead lays every rect
// out from the **bounded** `ui.available_rect_before_wrap()` the workspace
// handed it (`workspace_view` already reserved the nav-bar footprint).
//
// (#1) Even so, BIO's `details_pane_step2::render_pane` does NOT clip and
// its inner `ScrollArea` could still grow the *parent* UI, pushing the nav
// bar (and its `Next →`) off-screen on a narrow window. So each reused BIO
// sub-renderer (tree + details) runs inside `clipped_pane`: a child UI with
// its clip rect hard-set to the pane rect, after which the parent placer is
// advanced by **exactly** the bounded rect (never by BIO's overgrown
// internal min-rect). The BIO pane may be visually clipped/cut (its
// internal restyle is Phase 8) but it can never break its box or push the
// nav bar.
//
// ## Background-channel poll
//
// `OrchestratorApp::update` drains the 6 Step-2 receivers every frame (see
// `orchestrator_app.rs::poll_step2_channels`); without it the scan worker
// starts but never reports and Cancel never completes. That poll is **not**
// in this file (it's a per-frame `update()` concern) — it's wired in
// `orchestrator_app.rs` mirroring `bio::ui::app::update_loop::run`.
//
// SPEC: §6, §1 (decision order; carve-out boundary), §13.12a, §2.2;
//       wireframe `screens.jsx:2786-2880`.

// rationale: f32→u8 corner-radius roundings of small positive layout
// constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, redesign_border_strong, redesign_text_faint, redesign_text_primary,
    redesign_warning_soft,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::{step2_log_confirm, step2_search, step2_tab_row};

/// Width of the Details pane when open (wireframe `gridTemplateColumns:
/// "1fr 420px"`).
const DETAILS_PANE_W: f32 = 420.0;
/// Title height (wireframe Label 15px line ≈ 22 + a little slack).
const TITLE_H: f32 = 24.0;
/// Gap between the title and the search row (wireframe title
/// `marginBottom: 8`).
const TITLE_GAP: f32 = 8.0;
/// Search-input height (wireframe `Input` row).
const SEARCH_H: f32 = 30.0;
/// Gap between the search row and the tab row (wireframe search row
/// `marginBottom: 10`).
const SEARCH_GAP: f32 = 10.0;
/// Tab-row height (wireframe action sub-row `height: 30`).
const TAB_ROW_H: f32 = 30.0;
/// The tab row overlaps the tree pane's top edge by 1.5px (the wireframe
/// `GameTab` / Settings-tab `marginBottom: -1.5px`, `screens.jsx:1630/499`):
/// the tree pane starts 1.5px **above** the tab row's bottom so the active
/// tab's shell-bg fill masks the pane's top border (the #3 "tab merges into
/// the pane" behavior). A negative seam.
const TAB_TO_GRID_OVERLAP: f32 = 1.5;
/// Gap between the tree Box and the Details pane (wireframe grid `gap: 12`).
const GRID_GAP: f32 = 12.0;
/// Scan-status footer height.
const FOOTER_H: f32 = 20.0;
/// Vertical gap above the footer.
const FOOTER_GAP: f32 = 8.0;
/// Minimum tree width when the Details pane is open.
const LEFT_MIN_W: f32 = 420.0;
/// Minimum Details-pane width.
const RIGHT_MIN_W: f32 = 240.0;
/// Minimum content height (never force-grown past the bounded rect).
const CONTENT_MIN_H: f32 = 160.0;

/// Render the Step-2 C4 chrome. Returns any `Step2Action` produced by the
/// net-new chrome or the reused BIO tree / details sub-renderers; the router
/// dispatches it via `step_action_dispatch::dispatch_step2`.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step2Action> {
    let palette = orchestrator.theme_palette;
    let dev_mode = orchestrator.dev_mode;

    // Normalize the active game tab before any sub-renderer reads it —
    // exactly as BIO's `frame_step2::render` does (`frame_step2.rs:59`).
    // Without this a freshly-populated workspace whose `active_game_tab`
    // doesn't match the modlist's game would render the wrong bucket / an
    // empty tree.
    crate::ui::step2::state_step2::normalize_active_tab(&mut orchestrator.wizard_state);

    // ── Own the layout rects from the BOUNDED available space (the #4 fix —
    //    never `allocate_exact_size` a 620-min like BIO's frame_step2). ──
    let root = ui.available_rect_before_wrap();
    let x = root.left();
    let w = root.width();
    let mut y = root.top();

    let title_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TITLE_H));
    y += TITLE_H + TITLE_GAP;

    // #6 fix: NO per-step sub-hint here. The workspace shell already renders
    // the per-step hint under the progress bar (`workspace_hint_line`,
    // wireframe `WorkspaceView` L3557-3559); the wireframe `SourcesPanel`
    // (`screens.jsx:2794-2802`) has only the "Mods / Components" title, no
    // sub-hint. A second hint here was a duplicate. Its rect is reclaimed.

    let search_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, SEARCH_H));
    y += SEARCH_H + SEARCH_GAP;

    let tab_row_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
    // The tree pane starts 1.5px ABOVE the tab row's bottom (negative seam)
    // so the active GameTab's shell-bg fill overlaps & masks the pane's top
    // border — the wireframe "tab merges into the pane" behavior (#3).
    y += TAB_ROW_H - TAB_TO_GRID_OVERLAP;

    // The content region takes whatever vertical space remains down to the
    // bottom of the bounded rect minus the footer — NOT force-grown, so it
    // can never push past the nav bar `workspace_view` reserved.
    let content_h = (root.bottom() - y - FOOTER_H - FOOTER_GAP).max(CONTENT_MIN_H);
    let content_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, content_h));
    let footer_rect = egui::Rect::from_min_size(
        egui::pos2(x, content_rect.bottom() + FOOTER_GAP),
        egui::vec2(w, FOOTER_H),
    );

    let mut action: Option<Step2Action> = None;

    // ── 1. Net-new title (`Mods / Components`, wireframe Label 15px/500). ──
    //    NO sub-hint follows (the #6 fix — the shell already renders the
    //    per-step hint under the progress bar; the wireframe `SourcesPanel`
    //    has only this title).
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        ui.label(
            egui::RichText::new("Mods / Components")
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
    });

    // ── 3. Net-new full-width flex search + wireframe `Rescan Mods Folder`
    //    button (emits StartScan) + (dev-only) scan affordance. ──
    if let Some(a) = step2_search::render(ui, orchestrator, palette, search_rect, dev_mode) {
        action = Some(a);
    }

    // ── 4. Net-new redesign tab row (GameTabs + log/updates buttons +
    //    live compat / prompt Pills + count Label + Kebab). Replaces BIO's
    //    `render_controls` + `render_tabs` entirely (the wireframe has no
    //    BIO controls row). Returns only the two log-picker actions. ──
    if let Some(a) = step2_tab_row::render(ui, orchestrator, palette, tab_row_rect) {
        action = Some(a);
    }

    // Auto-open the Details pane on a NEW tree selection (wireframe
    // `ComponentTree` `onSelect → setDetailsOpen(true)`; BIO's tree has no
    // separate detail-open signal — a row / `[?]` click sets
    // `state.step2.selected`).
    let live_sel = orchestrator.wizard_state.step2.selected.clone();
    if live_sel.is_some() && live_sel != orchestrator.workspace_view.step2.last_selection {
        orchestrator.workspace_view.step2.details_open = true;
    }
    orchestrator.workspace_view.step2.last_selection = live_sel;

    // ── 5. Content split. The orchestrator owns these rects so the panel
    //    never bleeds into the nav bar (the #4 fix). Tree always; Details
    //    pane only when `details_open` (SPEC §6 — hidden by default). ──
    let details_open = orchestrator.workspace_view.step2.details_open;
    let (left_rect, right_rect) = if details_open {
        let right_w = DETAILS_PANE_W.min((content_rect.width() - LEFT_MIN_W).max(0.0));
        let right_w = right_w.max(RIGHT_MIN_W.min(content_rect.width()));
        let left_w = (content_rect.width() - right_w - GRID_GAP).max(LEFT_MIN_W);
        let left =
            egui::Rect::from_min_size(content_rect.min, egui::vec2(left_w, content_rect.height()));
        let right = egui::Rect::from_min_size(
            egui::pos2(left.right() + GRID_GAP, content_rect.top()),
            egui::vec2(right_w, content_rect.height()),
        );
        (left, Some(right))
    } else {
        (content_rect, None)
    };

    // #1 fix — render each reused BIO sub-renderer inside a HARD-CLIPPED,
    // FIXED-SIZE child UI. BIO's `details_pane_step2::render_pane`
    // (`scope_builder(max_rect)` + a `group` with
    // `set_min_size(rect - 12)`) does not clip and lets its inner
    // `ScrollArea` grow the *parent* UI — which pushed the workspace nav
    // bar (and its `Next →`) off-screen on a narrow window. `clipped_pane`
    // (a) sets the child's clip rect so BIO cannot paint outside its rect,
    // and (b) advances the parent placer by EXACTLY the bounded rect (never
    // by BIO's overgrown internal min-rect) so the BIO pane can never
    // expand the parent or shove the nav bar. Per the user directive it is
    // OK for the BIO pane to be visually clipped / cut off here — its
    // internal restyle is Phase 8; the hard rule is "never break its box or
    // push the nav bar".
    clipped_pane(ui, left_rect, |ui| {
        crate::ui::step2::list_pane_step2::render_list_pane(
            ui,
            &mut orchestrator.wizard_state,
            &mut action,
            left_rect,
        );
    });

    if let Some(right_rect) = right_rect {
        clipped_pane(ui, right_rect, |ui| {
            crate::ui::step2::details_pane_step2::render_pane(
                ui,
                &mut orchestrator.wizard_state,
                &mut action,
                right_rect,
            );
        });
        // Divider between tree + details (redesign-token styled).
        let dx = left_rect.right() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(dx, left_rect.top() + 1.0),
                egui::pos2(dx, left_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        );
    }

    // ── 6. Popups — driven exactly as `update_loop::render_shared_popups`
    //    does for `WizardApp` (same `bio::ui::step2::*` public entry
    //    points). `update_check_popup_step2::render` takes a `ctx`; it uses
    //    it only for `egui::Window::show`, so `ui.ctx()` is correct. ──
    crate::ui::step2::compat_window_step2::render(ui, &mut orchestrator.wizard_state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, &mut orchestrator.wizard_state);
    let ctx = ui.ctx().clone();
    crate::ui::step2::update_check_popup_step2::render(
        &ctx,
        &mut orchestrator.wizard_state,
        &mut action,
    );

    // ── 6b. Select-via-WeiDU-Log destructive confirm (SPEC §6.10 + wireframe
    //    `askWeiduImport`). The tab-row button arms
    //    `workspace_view.step2.pending_weidu_log_confirm`; the danger
    //    `ConfirmDialog` (the same shared widget Home Delete/Reinstall use)
    //    renders here, alongside the other Step-2 popups. On **Confirm** the
    //    matching `Step2Action::Select{Bgee,Bg2ee}ViaLog` is emitted — the
    //    router dispatches it through the *unchanged* `step2_log_glue`
    //    picker+apply path. On **Cancel/dismiss** the pending state is just
    //    cleared: nothing is applied, the selection is untouched. ──
    if let Some(a) = render_weidu_log_confirm(orchestrator, &ctx) {
        action = Some(a);
    }

    // ── 7. Scan-status footer (mirrors BIO `frame_step2.rs:163-166`):
    //    recompute selection counts, then show `scan_status` so the user
    //    sees scan progress ("0/0" → "..." → done) and completion. Painted
    //    in a redesign-token style instead of egui's default label colour.
    //    The recompute also feeds the tab row's `{sel}/{total}` count
    //    Label (same source BIO uses for its count text). This footer is
    //    also "where scan results report" per SPEC §6.3, so the
    //    rescan-reconcile drop warning (the #2 fix —
    //    `step2_rescan_reconcile`) is appended here, warn-toned. ──
    crate::ui::step2::service_list_ops_step2::recompute_selection_counts(
        &mut orchestrator.wizard_state,
    );
    let drop_warning = orchestrator
        .workspace_view
        .step2
        .rescan_drop_warning
        .clone();
    ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&orchestrator.wizard_state.step2.scan_status)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
            // #2 fix — the rescan-reconcile drop warning (SPEC §6.3:
            // _"N component(s) dropped — M mod(s) no longer present"_) is
            // surfaced in the scan-status footer, warn-toned so it reads as
            // a warning (no dialog — the reconcile is non-destructive by
            // construction). Set by `step2_rescan_reconcile` on
            // scan-completion; cleared on the next scan trigger.
            if let Some(warning) = drop_warning.as_deref() {
                ui.label(
                    egui::RichText::new(format!("\u{2014} {warning}"))
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_warning_soft(palette)),
                );
            }
        });
    });

    action
}

/// Run a reused BIO sub-renderer inside a **hard-clipped, fixed-size** child
/// UI bounded to `rect` — the #1 containment fix.
///
/// `ui.new_child` creates a child that does **not** advance the parent's
/// placer; we set the child's `clip_rect` to `rect` so the BIO renderer
/// physically cannot paint outside its box, then advance the parent placer
/// by **exactly `rect`** (`allocate_rect`) — never by the BIO pane's
/// (possibly overgrown) internal `min_rect`. Net effect: the BIO pane can
/// be visually clipped/cut off, but it can never expand `workspace_step2`'s
/// parent UI or push the workspace nav bar (and its `Next →`) off-screen.
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
    // overgrown min_rect so the parent (and the nav bar after it) is never
    // pushed. `Sense::hover()` keeps it inert (the BIO child already
    // handled its own interactions).
    ui.allocate_rect(rect, egui::Sense::hover());
}

/// Render the Select-via-WeiDU-Log destructive `ConfirmDialog` (SPEC §6.10 +
/// wireframe `askWeiduImport`) when `workspace_view.step2.
/// pending_weidu_log_confirm` is armed. Returns:
///   - `Some(Step2Action::Select{Bgee,Bg2ee}ViaLog)` on **Confirm** — the
///     pending state is cleared and the caller dispatches it through the
///     unchanged `step_action_dispatch::dispatch_step2` → `step2_log_glue`
///     picker+apply path (the flow is unchanged *after* confirm).
///   - `None` on **Cancel/dismiss** — the pending state is just cleared,
///     nothing is applied, the user's Step-2 selection is untouched.
///   - `None` while still pending (dialog open, nothing clicked).
///
/// Mirrors the Home `render_delete_confirm` / `render_reinstall_confirm`
/// arm-render-clear pattern (`home::page_home`): the orchestrator owns the
/// open/closed state and clears it on `Confirmed`/`Cancelled`.
fn render_weidu_log_confirm(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
) -> Option<Step2Action> {
    let bgee = orchestrator
        .workspace_view
        .step2
        .pending_weidu_log_confirm?;

    let (title, body) = step2_log_confirm::weidu_log_dialog_text(bgee);
    let dialog = step2_log_confirm::weidu_log_confirm(&title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.workspace_view.step2.pending_weidu_log_confirm = None;
            // The flow is UNCHANGED after confirm — emit the same action the
            // Run-1d button emitted directly; the router routes it to the
            // `step2_log_glue` sibling (picker + apply).
            Some(if bgee {
                Step2Action::SelectBgeeViaLog
            } else {
                Step2Action::SelectBg2eeViaLog
            })
        }
        ConfirmOutcome::Cancelled => {
            // Abort: clear the gate, change nothing.
            orchestrator.workspace_view.step2.pending_weidu_log_confirm = None;
            None
        }
        ConfirmOutcome::Pending => None,
    }
}
