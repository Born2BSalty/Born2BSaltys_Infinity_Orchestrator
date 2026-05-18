// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::step3` — the orchestrator-side **Step-3 C4 chrome**
// (P6.T2d). The direct analogue of `step2/` (P6.T2c) and `step4/` (P6.T2b):
// net-new redesign chrome that **does not** call BIO's `page_step3::render`
// / `content_step3::render` / `render_toolbar` / `frame_step3::render`,
// reusing **only** BIO's drag-reorder list body (`list_step3::render`) +
// the `pub(crate)` toolbar-support helpers BIO's own toolbar calls
// (directive decision-order step 1 — read-only reuse, no logic
// reimplementation).
//
// Why net-new chrome (per the 2026-05-17 SPEC-CONFLICT resolution in
// `plan/overview.md`, mirroring the Step-2 precedent): the wireframe's
// Step-3 (`screens.jsx::ComponentsPanel`, ~L3023-3056) is structurally
// different from BIO's `content_step3` top area — a BIO heading + hint, a
// raw `egui::Button` tab/badge row, plus a dev/diagnostics button. The
// wireframe instead shows the shared redesign GameTabs + aggregate
// conflict/prompt pills + redesign `Undo`/`Redo`/`Collapse All`/`Expand
// All`, NO heading/hint (the shell renders the per-step hint), NO
// Export-diagnostics. Carve-out #6 is colour-only and cannot restructure
// `content_step3`'s toolbar; the CRITICAL DIRECTIVE forbids editing it. So
// the chrome is net-new; only the heavy interaction surface (the
// drag-reorder list) is reused read-only via BIO's `pub(crate)`
// rect-agnostic `list_step3::render` (directive decision-order step 1).
//
//   - `workspace_step3` → the top-level Step-3 renderer + layout-rect
//                          owner (the orchestrator owns the body-hint /
//                          tab-row / list rects so the list never bleeds
//                          into the workspace nav bar — the Step-2
//                          `clipped_pane` precedent). It renders BOTH the
//                          §7.1 body hint line (in addition to the shell
//                          per-step hint — SPEC §7.1 amended) and the tab
//                          row. **The Step-3 "_N_ components ready to
//                          install …" count line is REMOVED (Step-4-only —
//                          SPEC §7.1 amended; the wireframe
//                          `ComponentsPanel` never drew it).** The former
//                          `step3_action_row` module is deleted accordingly.
//   - `step3_tab_row`    → the net-new redesign tab row (shared GameTabs +
//                          aggregate conflict/prompt clickable Pills +
//                          redesign Undo/Redo/Collapse All/Expand All).
//                          Replaces BIO's `content_step3::render_toolbar`
//                          (the wireframe has no BIO toolbar —
//                          `screens.jsx:3026-3056`).
//
// SPEC: §7 (Step 3 — §7.1 chrome elements, §7.2 reused BIO drag list), §1
//       (decision order; carve-out boundary), §2.2, §6 (the Step-2 C4
//       precedent); wireframe `screens.jsx:3023-3056`.

pub mod step3_tab_row;
pub mod workspace_step3;
