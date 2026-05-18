// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_step_router` — dispatch the active `WorkspaceStep` to its
// renderer. **All dispatch happens at the router layer for consistency**:
// step renderers return their action; the router dispatches via
// `step_action_dispatch::dispatch_stepN`.
//
//   - Step 2: the **C4 chrome wrapper** `bio::ui::workspace::step2::
//     workspace_step2::render(ui, orchestrator) -> Option<Step2Action>`
//     (P6.T2c). Net-new redesign chrome (title, full-width `flex` search,
//     the SPEC §6 toolbar set, **no** "Restart App With Diagnostics",
//     Details pane hidden-by-default per SPEC §6) that reuses **only**
//     BIO's tree / details / popup sub-renderers. BIO's `page_step2` /
//     `frame_step2` are **not** called. Any returned action →
//     `step_action_dispatch::dispatch_step2`.
//   - Step 3: the **C4 chrome wrapper** `bio::ui::workspace::step3::
//     workspace_step3::render(ui, orchestrator)` (P6.T2d). Net-new redesign
//     chrome (action-row count + shared redesign GameTabs + aggregate
//     conflict/prompt pills + redesign Undo/Redo/Collapse/Expand, **no**
//     Export-diagnostics, **no** BIO heading/hint) that reuses **only**
//     BIO's drag-reorder list body (`list_step3::render`, read-only). BIO's
//     `page_step3` / `content_step3` / `render_toolbar` are **not** called.
//     Returns `()` per H2 (no `Step3Action` enum — the list + chrome
//     mutate `WizardState` directly: drag-reorder, undo/redo, collapse,
//     block-select). The router calls it and ignores the return; no
//     dispatch arm. The dirty-bit fingerprint over `wizard_state.step3.
//     <tab>_items` (in the persistence cycle) detects Step-3 mutations.
//   - Step 4: the **C4 orchestrator-side renderer** `bio::ui::workspace::
//     step4::workspace_step4::render(ui, orchestrator) -> Option<Step4Action>`
//     (P6.T2b). Net-new redesign chrome (Save row + EET game-tab strip +
//     line-numbered three-colour review list / exact-log viewer). BIO's
//     `page_step4::render` is **never** called by the workspace router (per
//     C4 — it would double the Save button). Any returned action →
//     `step_action_dispatch::dispatch_step4`.
//   - Step 5: `workspace_step5_stub::render` (Phase 7 replaces the stub).
//
// To satisfy the borrow checker, Step 2's returned action is captured first
// (the `&mut orchestrator.wizard_state` + `&orchestrator.exe_fingerprint`
// borrows must end before `dispatch_step2(&mut orchestrator)` runs). The
// `exe_fingerprint` is cloned for the same reason.
//
// **P6.T11 — Step-3 dirty-bit fingerprint (this run).** Step 3 has no action
// enum (H2) — drag-reorder / collapse-expand / undo-redo mutate
// `wizard_state.step3` directly through BIO's internal handlers, so they
// never flow through `step_action_dispatch` (which is where Step 2/4
// mutations set `workspace_state_dirty`). To persist Step-3 edits this run
// wraps the Step-3 render in a **cheap state fingerprint**: a `u64` over the
// active tab's `step3.<tab>_items` (order-vec length + first/last element
// ids + their `selected_order`) **and** its `<tab>_collapsed_blocks` (the
// group-collapse state — persisted to `step3_group_collapse`). The
// fingerprint is captured before the render and compared after; any change
// (a reorder shifts the end ids; undo/redo / collapse change the vecs) sets
// the dirty bit via `orchestrator.mark_workspace_dirty()`. This is the H1
// "much cheaper than a full `ModlistWorkspaceState` extract+compare every
// frame" detector the plan prescribes — O(1)-ish (length + the two end
// items + the collapsed-block list), not a deep clone.
//
// SPEC: §2.2, §6, §7, §8, §13.14 (Step-3 persistence dirty-marking).

use std::hash::{Hash, Hasher};

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::workspace_step5_stub;

/// Render the workspace's current step into `ui`.
pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    match orchestrator.workspace_view.current_step {
        WorkspaceStep::Step2 => {
            // C4 chrome wrapper (P6.T2c): net-new redesign chrome around
            // BIO's reused tree / details / popup sub-renderers. BIO's
            // `page_step2` / `frame_step2` are NOT called.
            //
            // **Fix-Run 1 (Bug A) — Step-2 selection dirty fingerprint.**
            // BIO's reused component-tree checkbox
            // (`tree_component_row_step2`) mutates
            // `wizard_state.step2.<tab>_mods[].components[].checked` /
            // `.selected_order` **directly** and emits **no
            // `Step2Action`** — so a checkbox toggle never flows through
            // `dispatch_step2` (where the other Step-2 mutations call
            // `mark_workspace_dirty()`) and the debounced workspace write
            // (P6.T11 / SPEC §13.14 #1) would never fire for the most
            // common Step-2 edit. The direct analogue of the shipped
            // P6.T11 Step-3 fingerprint (H2 — Step 3 also has no action
            // enum): capture a cheap selection fingerprint before the
            // render and compare after; any checkbox toggle changes it ⇒
            // mark dirty. (The returned-action path below still handles
            // scan / log / update / compat variants; they additionally
            // mark dirty in `dispatch_step2`. A doubly-marked frame is
            // harmless — the extract+compare in the persistence cycle is
            // the second guard.) Zero BIO edits — BIO's tree is reused
            // read-only; only the orchestrator-owned router observes it.
            let before = step2_selection_fingerprint(&orchestrator.wizard_state);
            let action = crate::ui::workspace::step2::workspace_step2::render(ui, orchestrator);
            if step2_selection_fingerprint(&orchestrator.wizard_state) != before {
                orchestrator.mark_workspace_dirty();
            }
            if let Some(a) = action {
                step_action_dispatch::dispatch_step2(a, orchestrator);
            }
        }
        WorkspaceStep::Step3 => {
            // C4 orchestrator-side renderer (P6.T2d): net-new redesign
            // chrome (action-row count + shared GameTabs + aggregate
            // conflict/prompt pills + redesign Undo/Redo/Collapse/Expand)
            // around BIO's reused drag-reorder list (`list_step3::render`,
            // read-only). BIO's `page_step3::render` / `content_step3::
            // render` / `render_toolbar` are intentionally NOT called (per
            // C4 — they would reintroduce the old-BIO Step-3 top bar +
            // heading + Export-diagnostics the wireframe replaced). Per H2:
            // Step 3 has no action enum — `workspace_step3::render` returns
            // `()`; no dispatch arm. The list + chrome mutate `WizardState`
            // directly and the dirty-bit fingerprint over
            // `wizard_state.step3.<tab>_items` picks up reorder/collapse/
            // undo for persistence (unchanged by the C4 wrapper). P6.T11:
            // capture the cheap fingerprint before the render, compare
            // after — any drag/collapse/undo changes it ⇒ mark dirty.
            let before = step3_fingerprint(&orchestrator.wizard_state);
            crate::ui::workspace::step3::workspace_step3::render(ui, orchestrator);
            if step3_fingerprint(&orchestrator.wizard_state) != before {
                orchestrator.mark_workspace_dirty();
            }
        }
        WorkspaceStep::Step4 => {
            // C4 orchestrator-side renderer (P6.T2b): net-new redesign
            // chrome (Save row + EET game-tab strip + line-numbered
            // three-colour review list / exact-log viewer). BIO's
            // `page_step4::render` is intentionally NOT called (per C4 — it
            // would render a second Save button). Any returned action →
            // `dispatch_step4` (M11 — all dispatch at the router layer).
            let action = crate::ui::workspace::step4::workspace_step4::render(ui, orchestrator);
            if let Some(a) = action {
                step_action_dispatch::dispatch_step4(a, orchestrator);
            }
        }
        WorkspaceStep::Step5 => workspace_step5_stub::render(ui, orchestrator),
    }
}

/// A cheap `u64` fingerprint of the active Step-3 tab's persistable state
/// (P6.T11 / H1). Hashes only:
///   - the active tab's order-vec **length**,
///   - the **first + last** items' `tp_file` / `component_id` /
///     `selected_order` (a reorder shifts which item is at each end and/or
///     changes `selected_order`; an undo/redo replaces the vec; add/remove
///     changes the length),
///   - the active tab's **collapsed-blocks** list (group-collapse is
///     persisted to `ModlistWorkspaceState.step3_group_collapse`).
///
/// This is intentionally **not** a deep hash of every item — H1 requires the
/// detector be far cheaper than a full `extract_workspace_state_from_wizard`
/// + compare every frame. A pathological reorder that keeps the exact same
/// first+last items AND identical end `selected_order`s AND the same length
/// would not flip this; in practice BIO's drag-reorder renumbers
/// `selected_order` and moves block boundaries, so a real reorder always
/// changes the end items' `selected_order` or the block list. The worst case
/// (a missed dirty) is bounded by the on-exit `flush_all` (H4) which always
/// re-extracts and compares on shutdown, and by any *other* edit (a Step-2
/// toggle, a different drag) re-dirtying. The active tab is
/// `step3.active_game_tab`; BIO buckets non-BG2EE (incl. IWDEE) into the
/// BGEE items, so anything not `"BG2EE"` reads `bgee_items`.
fn step3_fingerprint(state: &WizardState) -> u64 {
    let is_bg2ee = state.step3.active_game_tab == "BG2EE";
    let (items, collapsed) = if is_bg2ee {
        (
            &state.step3.bg2ee_items,
            &state.step3.bg2ee_collapsed_blocks,
        )
    } else {
        (&state.step3.bgee_items, &state.step3.bgee_collapsed_blocks)
    };

    let mut h = std::collections::hash_map::DefaultHasher::new();
    is_bg2ee.hash(&mut h);
    items.len().hash(&mut h);
    if let Some(first) = items.first() {
        first.tp_file.hash(&mut h);
        first.component_id.hash(&mut h);
        first.selected_order.hash(&mut h);
    }
    if let Some(last) = items.last() {
        last.tp_file.hash(&mut h);
        last.component_id.hash(&mut h);
        last.selected_order.hash(&mut h);
    }
    // Group-collapse state is persisted (SPEC §13.14 / the loader's
    // `step3_group_collapse`), so a collapse/expand must dirty too.
    collapsed.len().hash(&mut h);
    for block in collapsed {
        block.hash(&mut h);
    }
    h.finish()
}

/// A `u64` fingerprint of the **persistable Step-2 selection** across both
/// game tabs (Fix-Run 1, Bug A). Hashes every *checked* component's
/// `tp_file` + `component_id` + `selected_order` on `step2.bgee_mods` and
/// `step2.bg2ee_mods`.
///
/// Unlike `step3_fingerprint` (a cheap end-only hash — Step 3's persisted
/// data is an *order* whose end items shift on any real reorder), the
/// Step-2 persisted data is the **whole selection set**: a toggle anywhere
/// in the tree must dirty, including one that doesn't change the first/last
/// checked component. So this hashes the full checked set. It is still
/// cheap — it iterates only the user's *selection* (the checked
/// components), not the full scanned mod set, and runs once per frame while
/// Step 2 is active (the same cadence as the shipped Step-3 fingerprint).
/// It deliberately ignores `step2.selected` (row-highlight, not persisted)
/// and `step2.search_query` (filter text, not persisted) so a row click or
/// a search keystroke does not spuriously dirty (matching the
/// `dispatch_step2` "`OpenSelected*` don't mark dirty" discipline; a
/// spurious dirty would still be safe — the persistence cycle's
/// extract+compare is the second guard — but staying tight avoids
/// needless debounced writes).
fn step2_selection_fingerprint(state: &WizardState) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (tag, mods) in [
        (0u8, &state.step2.bgee_mods),
        (1u8, &state.step2.bg2ee_mods),
    ] {
        tag.hash(&mut h);
        for m in mods {
            for c in &m.components {
                if c.checked {
                    m.tp_file.hash(&mut h);
                    c.component_id.hash(&mut h);
                    c.selected_order.hash(&mut h);
                }
            }
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step3ItemState;

    fn item(tp: &str, id: &str, order: usize) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: order,
            block_id: tp.to_string(),
            is_parent: false,
            parent_placeholder: false,
        }
    }

    #[test]
    fn fingerprint_is_stable_when_nothing_changes() {
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1), item("B/B.TP2", "2", 2)];
        let a = step3_fingerprint(&s);
        let b = step3_fingerprint(&s);
        assert_eq!(a, b, "identical state ⇒ identical fingerprint");
    }

    #[test]
    fn fingerprint_changes_on_reorder() {
        // A drag-reorder swaps the end items + renumbers selected_order —
        // the fingerprint must catch it (the P6.T11 Step-3 dirty path).
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1), item("B/B.TP2", "2", 2)];
        let before = step3_fingerprint(&s);
        s.step3.bgee_items = vec![item("B/B.TP2", "2", 1), item("A/A.TP2", "0", 2)];
        assert_ne!(before, step3_fingerprint(&s), "reorder must change it");
    }

    #[test]
    fn fingerprint_changes_on_collapse_and_on_length() {
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1)];
        let base = step3_fingerprint(&s);
        // Collapse a block (persisted via step3_group_collapse).
        s.step3.bgee_collapsed_blocks = vec!["A/A.TP2".to_string()];
        let after_collapse = step3_fingerprint(&s);
        assert_ne!(base, after_collapse, "collapse must change it");
        // Add an item (length changes).
        s.step3.bgee_items.push(item("B/B.TP2", "2", 2));
        assert_ne!(
            after_collapse,
            step3_fingerprint(&s),
            "length change must change it"
        );
    }

    #[test]
    fn fingerprint_is_per_active_tab() {
        // The same items under a different active tab hash differently
        // (BG2EE reads bg2ee_items, everything else reads bgee_items).
        let mut s = WizardState::default();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1)];
        s.step3.active_game_tab = "BGEE".to_string();
        let bgee = step3_fingerprint(&s);
        s.step3.active_game_tab = "BG2EE".to_string();
        let bg2ee = step3_fingerprint(&s);
        assert_ne!(bgee, bg2ee, "active tab is part of the fingerprint");
    }
}
