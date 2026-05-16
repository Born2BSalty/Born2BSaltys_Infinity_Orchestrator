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
//   - `workspace_step_router`  → per-step dispatch (Step 2/3 real, Step 4
//                                placeholder until Run 2, Step 5 stub).
//   - `step_action_dispatch`   → `dispatch_step2` / `dispatch_step4`.
//   - `step2_log_glue`         → the `SelectVia*Log` sibling (rfd picker +
//                                settings-persistence trigger).
//   - `workspace_progress_bar` → the 4-step progress bar (SPEC §2.2).
//   - `workspace_nav_bar`      → the back/next nav bar (SPEC §2.2).
//   - `workspace_hint_line`    → the per-step hint line.
//   - `workspace_view`         → the top-level workspace renderer.
//   - `workspace_step5_stub`   → the Step 5 placeholder (Phase 7 replaces it).
//
// Later Phase-6 runs add `workspace_header` (rename + fork badge + save
// draft), `step4/` (the C4 orchestrator-side Step 4 renderer), and `widgets/`.
//
// SPEC: §2.2, §6, §7, §8, §13.1, §13.14.

pub mod state_workspace;
pub mod step2_log_glue;
pub mod step_action_dispatch;
pub mod workspace_hint_line;
pub mod workspace_nav_bar;
pub mod workspace_progress_bar;
pub mod workspace_state_loader;
pub mod workspace_step5_stub;
pub mod workspace_step_router;
pub mod workspace_view;
