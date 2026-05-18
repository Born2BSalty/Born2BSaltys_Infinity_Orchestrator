// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ui::home::reinstall_route_wire` — Phase 7 P7.T10 (Run 4b).
//
// Wires the Phase-5 Home Kebab → Reinstall confirm dialog's **Confirm**
// button to the real Reinstall route. Phase 5 (P5.T18) shipped the confirm
// modal with a placeholder: on Confirm it showed
// `operations::queue_reinstall_stub`'s "install runtime arrives in Phase 7"
// toast. Phase 7 replaces that single seam with the real route — per SPEC
// §3.1, on confirm the app **routes through the Install Modlist preview
// stage** using the modlist's stored share code (it does **not** flip the
// registry yet; the flip is at the preview's Install-click).
//
// This is the single, obvious replacement seam the Phase-5 stub left:
// `page_home::render_reinstall_confirm`'s `ConfirmOutcome::Confirmed` arm
// calls `confirm_reinstall(orchestrator, &id)` instead of the stub toast.
// Kept as a named wiring fn (not inlined into `page_home`) per the plan's
// File inventory (`src/ui/home/reinstall_route_wire.rs` — "wires the Home
// Reinstall confirm dialog's Confirm button to
// `install_runtime::reinstall_route::start_reinstall`").
//
// SPEC: §3.1 (Reinstall semantics — routes through the Install-Modlist
//        preview; no registry flip until Install-click).

use tracing::warn;

use crate::install_runtime::reinstall_route;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// Handle the Home Reinstall confirm's **Confirm** for modlist `id`.
///
/// Looks the entry up in the live registry and delegates to
/// `install_runtime::reinstall_route::start_reinstall` (populate the
/// Install-Modlist preview from the stored code + force overwrite-install +
/// arm `pending_reinstall_id` + navigate to the Preview stage — **no
/// registry flip**, SPEC §3.1). If the entry vanished between arming the
/// confirm and confirming (a delete raced the Reinstall), this is a logged
/// no-op — there is nothing to reinstall; the caller already cleared
/// `reinstall_target`.
pub fn confirm_reinstall(orchestrator: &mut OrchestratorApp, id: &str) {
    // Clone the entry out so the immutable `orchestrator.registry` borrow
    // ends before `start_reinstall` takes `&mut orchestrator` (the same
    // clone-the-entry borrow discipline `page_home` / `page_router` use).
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        warn!(
            target = "orchestrator",
            "Reinstall confirmed for {id} but the entry is no longer in the \
             registry (deleted between confirm-arm and confirm) — no-op"
        );
        return;
    };
    reinstall_route::start_reinstall(&entry, orchestrator);
}
