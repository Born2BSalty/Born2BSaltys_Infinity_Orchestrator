// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::install_concurrency` — the single-install-at-a-time
// gate (SPEC §13.15 "Install concurrency policy": "Only one install can run
// at a time across the entire app").
//
// Net-new orchestrator code. `install_in_progress(orchestrator)` is the one
// source of truth for "is an install running, and on which modlist". It
// powers BOTH:
//   - the per-button gate (P7.T9 — every Install entry point checks it
//     before starting; the Step-5 Install click site applies it just
//     before the start hooks), and
//   - the C5 rail-nav lock (P7.T9b — `OrchestratorApp::update` derives the
//     `RailLockReason` from it once per frame).
//
// **The running modlist id** is `orchestrator.workspace_view
// .loaded_workspace_id` (the modlist whose state was loaded into the shared
// `WizardState` when the install started — the C5 lock guarantees this
// cannot change mid-install), falling back to `workspace_view.modlist_id`
// if the loaded-id has not been recorded yet (an early-frame edge before
// the loader runs; the routed id is the next-best identity). The running
// flag is BIO's live `wizard_state.step5.install_running`.
//
// SPEC: §13.15.

use std::time::Instant;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// A running install: the modlist it is running on + a process-local
/// `Instant` marking when this gate first observed it running (used by the
/// statusbar's `<elapsed>` segment and the rail-lock reason).
#[derive(Debug, Clone)]
pub struct RunningInstall {
    /// Registry id of the modlist whose install is in flight.
    pub modlist_id: String,
    /// When the install was first observed running **this process run**.
    /// Not persisted — purely for the live `<elapsed>` readout. (The
    /// persisted authoritative start time is `ModlistEntry
    /// .install_started_at`, written by `start_hooks::on_install_start`;
    /// this `Instant` is the monotonic clock the UI ticks against, which a
    /// `DateTime<Utc>` cannot be subtracted-from monotonically.)
    pub started_at: Instant,
}

/// Returns `Some(RunningInstall { … })` while an install is in flight
/// anywhere in the app, else `None`.
///
/// "In flight" == `orchestrator.wizard_state.step5.install_running` (BIO's
/// live install flag, toggled true the frame the install starts and false
/// on every exit by `step5_runtime_status::process_exit_event`). Because
/// only one install can run at a time (SPEC §13.15) and the C5 rail lock
/// pins the user inside the running install's workspace, the single
/// `install_running` bool + the loaded workspace id fully identify the
/// running install.
#[must_use]
pub fn install_in_progress(orchestrator: &OrchestratorApp) -> Option<RunningInstall> {
    if !orchestrator.wizard_state.step5.install_running {
        return None;
    }
    let modlist_id = orchestrator
        .workspace_view
        .loaded_workspace_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| orchestrator.workspace_view.modlist_id.clone());
    Some(RunningInstall {
        modlist_id,
        started_at: orchestrator
            .install_running_since
            .unwrap_or_else(Instant::now),
    })
}

/// The verbatim SPEC §13.15 per-button tooltip for an Install button that
/// is gated because a *different* modlist's install is running. The
/// `<modlist A>` placeholder is filled with the running modlist's display
/// name.
///
/// SPEC §13.15: *"An install is already running for `<modlist A>`. Wait for
/// it to finish before starting another."*
#[must_use]
pub fn per_button_gate_tooltip(running_modlist_name: &str) -> String {
    format!(
        "An install is already running for {running_modlist_name}. \
         Wait for it to finish before starting another."
    )
}
