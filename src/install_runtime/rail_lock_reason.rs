// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::rail_lock_reason` — **the C5 rail-navigation lock
// reason** (SPEC §13.15; plan P7.T9b).
//
// When an install is running, every left-rail item (Home / Install /
// Create / Settings) renders disabled with the verbatim SPEC §13.15
// tooltip. The user stays in the running install's workspace until cancel
// or completion — so the workspace state loader's
// `populate_wizard_state_from_workspace` is never invoked mid-install and
// `WizardState` is stable for the install's duration (the C5 risk
// closure). Only **navigation** is locked; the orchestrator's `update`
// poll loop keeps running so the install completes + post-install
// transitions fire.
//
// **PLAN GAP (premise-checked, resolved here — see the run report).** The
// plan (P7.T9b / P2.T2) names the rail renderer `src/ui/orchestrator
// /nav_rail.rs::render`, but no `nav_rail.rs` exists: the orchestrator's
// real rail renderer is `src/ui/orchestrator/left_rail.rs::render`
// (imported + called at `orchestrator_app.rs` as `left_rail::render(ui,
// palette, &mut self.nav, self.dev_mode, &self.path_validation, None)` —
// that trailing `None` is the Phase-2-provisioned `rail_locked` slot).
// Additionally, `left_rail.rs` already carried a **Phase-2 placeholder**
// `RailLockReason { InstallRunning { modlist_label: String } }` (different
// shape than the plan's `{ modlist_id, started_at }`). Per the plan's
// directive ("implement the lock in the REAL renderer; do NOT create a
// spurious `nav_rail.rs`"), this module is the **single canonical**
// `RailLockReason`; `left_rail.rs`'s signature is re-pointed to it and its
// Phase-2 placeholder enum is removed (the Phase-2 file-comment explicitly
// said that slot was "reserved for Phase 7 C5" — Phase 7 now wires it).
//
// SPEC: §13.15 (rail tooltip); §1 CRITICAL DIRECTIVE (net-new orchestrator
//        code — no BIO mutation).

use std::time::Instant;

/// Why the left-rail navigation is locked. Extensible (future versions may
/// add e.g. a critical-registry-corruption lock); for v1 alpha the only
/// variant is the install-running lock.
///
/// **PLAN GAP (resolved here — see the run report).** The plan's
/// `RailLockReason { InstallRunning { modlist_id, started_at } }` shape
/// omits the modlist *display name*, but SPEC §13.15's verbatim tooltip is
/// name-based (*"An install is already running for `<modlist A>`…"*) — the
/// rail cannot fill `<modlist A>` from an opaque id alone (it has no
/// registry handle). The Phase-2 `left_rail.rs` placeholder already carried
/// `modlist_label`; this canonical type keeps the plan's `modlist_id` +
/// `started_at` **and** adds `modlist_label` (the registry-resolved display
/// name the orchestrator looks up when it builds the reason) so the SPEC
/// §13.15 tooltip is exact. Not a behavior change vs the plan's intent —
/// the extra field only carries the name the spec's tooltip requires.
#[derive(Debug, Clone)]
pub enum RailLockReason {
    /// An install is in flight on `modlist_id` (started — process-local
    /// monotonic clock — at `started_at`). Every rail item is disabled with
    /// the SPEC §13.15 tooltip until the install cancels or completes.
    InstallRunning {
        /// Registry id of the modlist whose install is running.
        modlist_id: String,
        /// The modlist's display name (registry `ModlistEntry.name`),
        /// resolved by the orchestrator when it builds the reason — fills
        /// the SPEC §13.15 `<modlist A>` tooltip placeholder. Falls back to
        /// the id if the entry is somehow unresolvable.
        modlist_label: String,
        /// When the install was first observed running this process run
        /// (drives nothing in the rail itself — the tooltip is fixed copy —
        /// but kept on the reason so a single value flows from
        /// `install_concurrency::install_in_progress` through the rail and
        /// the statusbar without re-deriving it).
        started_at: Instant,
    },
}

/// The **verbatim** SPEC §13.15 rail-lock tooltip shown on every disabled
/// nav item while an install runs.
///
/// SPEC §13.15 (the rail-lock tooltip text, cited verbatim from the current
/// SPEC — see [`crate::install_runtime::rail_lock_reason`] tests for the
/// pinned string): *"An install is already running for `<modlist A>`. Wait
/// for it to finish before starting another."*
///
/// SPEC §13.15 specifies one tooltip string for every gated install entry
/// point (the Step-5 button, the Install-Modlist CTA, **and** the rail
/// items): the running modlist's display name fills the `<modlist A>`
/// placeholder.
#[must_use]
pub fn rail_lock_tooltip(running_modlist_name: &str) -> String {
    format!(
        "An install is already running for {running_modlist_name}. \
         Wait for it to finish before starting another."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_fills_the_modlist_placeholder_verbatim() {
        // Pin the exact SPEC §13.15 wording (a regression here = a spec
        // drift; the string is read verbatim from SPEC §13.15).
        assert_eq!(
            rail_lock_tooltip("Polished BG2EE"),
            "An install is already running for Polished BG2EE. \
             Wait for it to finish before starting another."
        );
    }

    #[test]
    fn reason_carries_modlist_label_and_start_instant() {
        let now = Instant::now();
        let r = RailLockReason::InstallRunning {
            modlist_id: "ABC0123".to_string(),
            modlist_label: "Polished BG2EE".to_string(),
            started_at: now,
        };
        match r {
            RailLockReason::InstallRunning {
                modlist_id,
                modlist_label,
                started_at,
            } => {
                assert_eq!(modlist_id, "ABC0123");
                assert_eq!(modlist_label, "Polished BG2EE");
                assert_eq!(started_at, now);
            }
        }
    }
}
