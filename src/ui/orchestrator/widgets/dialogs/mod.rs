// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/orchestrator/widgets/dialogs/` — shared non-blocking popup widgets
// (SPEC §10).
//
// Phase 5 / Run 2 ships:
//   - `confirm_dialog` — the generic `ConfirmDialog` (SPEC §10.7), used by
//     Home Delete (P5.T7) + Home Reinstall (P5.T18).
//
// Later phases add the remaining §10 dialogs as net-new siblings here.

pub mod confirm_dialog;

pub use confirm_dialog::{render as render_confirm_dialog, ConfirmDialog, ConfirmOutcome};
