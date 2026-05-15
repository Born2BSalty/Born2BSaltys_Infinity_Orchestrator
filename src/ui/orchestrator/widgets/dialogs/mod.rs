// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/orchestrator/widgets/dialogs/` — shared non-blocking popup widgets
// (SPEC §10).
//
// Phase 5 ships:
//   - `confirm_dialog` (Run 2) — the generic `ConfirmDialog` (SPEC §10.7),
//     used by Home Delete (P5.T7) + Home Reinstall (P5.T18).
//   - `fork_info_popup` (Run 4) — the read-only `ForkInfoPopup`
//     (SPEC §10.9); opened from the Install/fork preview's `⑂ fork info`
//     affordance. Reused by Phase 6's workspace header + fork-preview.
//
// Later phases add the remaining §10 dialogs as net-new siblings here.

pub mod confirm_dialog;
pub mod fork_info_popup;

pub use confirm_dialog::{ConfirmDialog, ConfirmOutcome, render as render_confirm_dialog};
// `fork_info_popup` is consumed via the module path directly
// (`dialogs::fork_info_popup::{render, SelfNode, ForkInfoOutcome}`) — the
// established pattern at every call site. No convenience re-export: its API
// traffics in BIO's `pub(crate)` `ForkAncestor` (carve-out #5 visibility),
// so a `pub use` alias would be a crate-internal dead re-export.
