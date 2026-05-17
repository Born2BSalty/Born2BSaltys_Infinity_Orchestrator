// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace` — Phase 6 workspace shell (Steps 2-5 hosted inside the
// orchestrator). Net-new module tree; no BIO source modifications. The
// workspace calls BIO's existing public per-step renderers
// (`bio::ui::step{2,3}::page_step{N}::render`) directly with the
// orchestrator's owned `WizardState`, and dispatches the returned actions via
// the same `bio::app::*` public entry points BIO's `WizardApp` ultimately
// uses (see `step_action_dispatch`).
//
// Phase 6 Run 1 (workspace spine) ships:
//   - `state_workspace`        → `WorkspaceViewState` (per-modlist view state).
//   - `workspace_state_loader` → populate / extract / sync-paths.
//   - `step2`                  → the Step-2 C4 chrome wrapper (P6.T2c):
//                                net-new redesign chrome (title, full-width
//                                flex search, no Restart-Diagnostics,
//                                Details hidden-by-default) reusing **only**
//                                BIO's tree / details / popup sub-renderers.
//   - `workspace_step_router`  → per-step dispatch (Step 2 → the C4 wrapper,
//                                Step 3 real, Step 4 placeholder until
//                                Run 2, Step 5 stub).
//   - `step_action_dispatch`   → `dispatch_step2` / `dispatch_step4`.
//   - `step2_log_glue`         → the `SelectVia*Log` sibling (rfd picker +
//                                settings-persistence trigger).
//   - `workspace_progress_bar` → the 4-step progress bar (SPEC §2.2).
//   - `workspace_nav_bar`      → the back/next nav bar (SPEC §2.2).
//   - `workspace_hint_line`    → the per-step hint line.
//   - `workspace_view`         → the top-level workspace renderer.
//   - `workspace_step5_stub`   → the Step 5 placeholder (Phase 7 replaces it).
//
// Phase 6 Run 2 (workspace header + Step-4 C4) adds:
//   - `step4`                  → the C4 orchestrator-side Step-4 renderer
//                                (Save row + game-tab strip + line-numbered
//                                three-colour review list / exact-log
//                                viewer). BIO's `page_step4::render` is NOT
//                                called (per C4).
//   - `widgets`                → workspace-local widgets (`weidu_line` —
//                                the §6.7 three-colour line renderer).
//   - `workspace_header`       → `Editing <name>` + ✎ inline rename + fork
//                                badge + `⑂ view fork details` (reused
//                                Phase-5 `ForkInfoPopup`) + `save draft`.
//
// Phase 6 Step-3 C4 (P6.T2d) adds:
//   - `step3`                  → the C4 orchestrator-side Step-3 chrome
//                                (action-row count + shared redesign
//                                GameTabs + aggregate conflict/prompt
//                                pills + redesign Undo/Redo/Collapse/Expand
//                                — **no** Export-diagnostics, **no** BIO
//                                heading) wrapping BIO's reused
//                                drag-reorder list (`list_step3::render`,
//                                read-only) in an orchestrator-owned
//                                hard-clipped rect. BIO's `page_step3` /
//                                `content_step3` / `render_toolbar` are NOT
//                                called (the 2026-05-17 SPEC-CONFLICT
//                                resolution — the wireframe's Step 3 is
//                                structurally different from BIO's and
//                                colour-only carve-out #6 cannot
//                                restructure `content_step3`).
//
// SPEC: §2.2, §6, §6.7, §7, §8, §10.9, §13.1, §13.14.

pub mod state_workspace;
pub mod step2;
pub mod step2_log_glue;
pub mod step3;
pub mod step4;
pub mod step_action_dispatch;
pub mod widgets;
pub mod workspace_header;
pub mod workspace_hint_line;
pub mod workspace_nav_bar;
pub mod workspace_progress_bar;
pub mod workspace_state_loader;
pub mod workspace_step5_stub;
pub mod workspace_step_router;
pub mod workspace_view;
