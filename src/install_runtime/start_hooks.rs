// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::start_hooks` — the **install-start hook** (plan P7.T3,
// SPEC §9.4 / §13.13 / §13.3 / §3.1).
//
// Called from the orchestrator's `Step5Action::StartInstall` dispatcher
// (`page_workspace_step5.rs`) **after** the install-concurrency gate
// (P7.T9) passes and **before** `state.step5.start_install_requested =
// true` is flipped. Per the Phase-7 plan it, in order:
//
//   1. Apply the automatic flag policies #1/#5 (P7.T16,
//      `flag_policies::apply_flags`) into the orchestrator-owned
//      `WizardState.step1` BIO's runner reads.
//   2. Compute the share code via the net-new `registry::share_export
//      ::pack_meta` (SPEC §13.3 Generation) with `allow_auto_install =
//      false` (install-start codes are unverified — SPEC §13.3 / §13.13)
//      and the provenance trio read off the registry `ModlistEntry`.
//   3. Update `entry.latest_share_code` to that value (the registry's
//      snapshot of the in-progress code, `allow_auto_install = false`).
//   4. Write `modlist-import-code.txt` to the destination — **variant-
//      gated per SPEC §13.13**: `Install` / `Restart` / `Reinstall`
//      write/overwrite; `Resume` skips (the prior attempt's file stays
//      canonical).
//   5. Record `install_started_at` in the entry (always, every variant —
//      every attempt is timestamped).
//   6. Atomic registry write (`RegistryStore::save`).
//
// **No `registry_snapshot`** (H8 — dropped: the C5 rail lock prevents the
// user navigating away mid-install, so there is no swap-mid-install path a
// snapshot would defend against). **`pack_meta` composes BIO read-only —
// it NEVER patches `bio::app::modlist_share`** (carve-out #5 "generation is
// not a BIO modification"). The Reinstall registry-flip (P7.T10) and the
// share-code-consuming download pipeline (P7.T17) are **out of this run's
// scope** — left as the exact commented placeholders below.
//
// Flipping `state.step5.start_install_requested = true` is the **caller's**
// step (plan P7.T3 step 3) so the start hook stays a pure side-effecting
// unit (registry + disk), independent of BIO's pipeline kick-off.
//
// SPEC: §9.4, §13.13, §13.3, §3.1.

use std::path::Path;

use chrono::Utc;
use tracing::warn;

use crate::app::state::WizardState;
use crate::install_runtime::flag_policies::{self, InstallWorkflow};
use crate::install_runtime::import_code_writer;
use crate::registry::model::ModlistRegistry;
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store::RegistryStore;
use crate::settings::model::Step1Settings;

/// Which install-button variant initiated this install — derived from
/// BIO's `state.step5` (the same logic BIO's
/// `content_install_row_step5.rs` uses for the button *label*) plus the
/// orchestrator's reinstall flag. Drives the SPEC §13.13
/// `modlist-import-code.txt` write/overwrite/skip matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallButtonVariant {
    /// "Install" — fresh first attempt (`!resume_available &&
    /// !has_run_once`). SPEC §13.13: write the file.
    Install,
    /// "Restart Install" — after a force-cancel (`has_run_once &&
    /// !resume_available`). SPEC §13.13: **overwrite** (the cancelled
    /// attempt's file is discarded).
    Restart,
    /// "Resume Install" — after a graceful cancel (`resume_available`).
    /// SPEC §13.13: **skip** the write (the prior attempt's file is the
    /// source of truth for the mid-install state).
    Resume,
    /// Reinstall (Home Kebab → Reinstall on an installed card). SPEC
    /// §13.13: **overwrite** (a fresh install with a potentially-updated
    /// share code). The registry `Installed → InProgress` flip happens
    /// **here at Install-click** (P7.T10) — left as a commented placeholder
    /// this run.
    Reinstall,
}

impl InstallButtonVariant {
    /// Derive the variant from BIO's live `state.step5` (mirrors the exact
    /// `content_install_row_step5.rs:62-67` label decision: `resume_available`
    /// ⇒ Resume; else `has_run_once` ⇒ Restart; else Install). `reinstall`
    /// is the orchestrator's `pending_reinstall_id == Some(this modlist)`
    /// flag (P7.T10 — Run 4b; always `false` this run, so Reinstall is
    /// never selected here).
    #[must_use]
    pub fn from_step5(state: &WizardState, reinstall: bool) -> Self {
        if reinstall {
            Self::Reinstall
        } else if state.step5.resume_available {
            Self::Resume
        } else if state.step5.has_run_once {
            Self::Restart
        } else {
            Self::Install
        }
    }

    /// SPEC §13.13: does this variant write/overwrite `modlist-import-code
    /// .txt`? `Install` / `Restart` / `Reinstall` ⇒ true; `Resume` ⇒
    /// false (the original file from the prior attempt remains canonical).
    #[must_use]
    pub fn writes_import_code(self) -> bool {
        !matches!(self, Self::Resume)
    }
}

/// Run the install-start hook. See the module header for the ordered
/// contract. Returns `Ok(())` on success; an `Err(String)` if the share
/// code could not be generated or the registry write failed (the caller
/// surfaces it per SPEC §13.14 and must **not** flip
/// `start_install_requested` on `Err`). A failed `modlist-import-code.txt`
/// write is logged but **not** fatal to the install (the artifact is a
/// recovery convenience, not a precondition for WeiDU — SPEC §13.13
/// frames it as the "I crashed, here's what I was trying" file).
///
/// Borrow-order note: `flag_policies::apply_flags` mutates
/// `wizard_state.step1`; `share_export::pack_meta` then reads
/// `&*wizard_state` immutably. Sequenced so the immutable `pack_meta`
/// borrow starts only after the mutable flag write ends (no overlap).
#[allow(clippy::too_many_arguments)]
pub fn on_install_start(
    modlist_id: &str,
    variant: InstallButtonVariant,
    workflow: InstallWorkflow,
    wizard_state: &mut WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    settings: &Step1Settings,
) -> Result<(), String> {
    // ── 1. Automatic flag policies #1/#5 (P7.T16) — write into the
    //    orchestrator-owned WizardState.step1 BIO's runner reads. (mut
    //    borrow of wizard_state.step1; ends before pack_meta's immut
    //    borrow below.) ──
    flag_policies::apply_flags(&mut wizard_state.step1, workflow, settings);

    // ── 2. Compute the share code via the net-new pack_meta envelope
    //    (SPEC §13.3 Generation). `allow_auto_install = false` — at
    //    install start the install has NOT succeeded (SPEC §13.3 / §13.13).
    //    Provenance is read off the registry entry (NOT WizardState). ──
    let entry = registry
        .find(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} not in registry at install start"))?;
    let destination = entry.destination_folder.trim().to_string();
    let meta = ShareMeta::from_entry(entry, /* allow_auto_install */ false);
    // `pack_meta` composes `bio::app::modlist_share::export_modlist_share_code`
    // read-only and round-trips the four keys — it NEVER patches BIO
    // (carve-out #5). Immutable `wizard_state` borrow (the mut flag-policy
    // borrow above has ended).
    let share_code = share_export::pack_meta(wizard_state, &meta)?;

    // ── 3. Update entry.latest_share_code to the in-progress code
    //    (allow_auto_install = false). Also stamp install_started_at
    //    (step 5 — every variant, every attempt). ──
    let entry_mut = registry
        .find_mut(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry mid-hook"))?;
    entry_mut.latest_share_code = Some(share_code.clone());
    // SPEC §13.13 / plan P7.T3 acceptance: `install_started_at` is recorded
    // ALWAYS, regardless of variant — every attempt (incl. Resume) is
    // timestamped. Distinct from `install_date` (clean-completion only,
    // P7.T6).
    entry_mut.install_started_at = Some(Utc::now());

    // ── 6. Atomic registry write (the only registry mutation during the
    //    install lifetime until P7.T6's flip_to_installed — H8: no
    //    snapshot, the live registry stays the only view). Done before the
    //    file write so the registry's record of the code is durable even
    //    if the disk write below fails. ──
    store
        .save(registry)
        .map_err(|err| format!("registry write at install start failed: {err}"))?;

    // ── 4. Write `modlist-import-code.txt` to the destination — variant-
    //    gated per SPEC §13.13 (Install/Restart/Reinstall write/overwrite;
    //    Resume skips). Written upfront (before WeiDU runs) so the artifact
    //    survives a crash/cancel/error. A write failure is logged, not
    //    fatal (the file is a recovery convenience — SPEC §13.13). ──
    if variant.writes_import_code() {
        if destination.is_empty() {
            warn!(
                target = "orchestrator",
                "modlist {modlist_id} has no destination_folder at install start — \
                 skipping modlist-import-code.txt (nothing to write it next to)"
            );
        } else if let Err(err) =
            import_code_writer::write_modlist_import_code_txt(Path::new(&destination), &share_code)
        {
            warn!(
                target = "orchestrator",
                "writing modlist-import-code.txt to {destination} failed: {err} \
                 (non-fatal — the install proceeds; the registry holds the code)"
            );
        }
    }
    // Resume Install: the file is intentionally NOT overwritten — the
    // original from the prior (gracefully-cancelled) attempt remains the
    // canonical mid-install record (SPEC §13.13). No-op by design.

    // ── P7.T10 (Run 4b): reinstall registry-flip — when `variant ==
    //    Reinstall` and `pending_reinstall_id == Some(modlist_id)`, flip
    //    the registry state `Installed → InProgress` HERE (at Install-click,
    //    not at Reinstall-click) via `registry_transition
    //    ::flip_to_in_progress` and clear `pending_reinstall_id`. Out of
    //    this run's scope — `reinstall_route` / `pending_reinstall_id`
    //    land in Run 4b. Deliberately not implemented here. ──

    // ── P7.T17 (Run 4a): share-code-consuming pipeline — for
    //    ShareCodeConsuming workflows, derive the per-install dirs +
    //    force the clone flags + kick off the import → auto-build →
    //    download/extract pipeline (content-addressed staging) HERE.
    //    `on_install_start` is the trigger point; P7.T17 owns the
    //    mechanism. Out of this run's scope — `per_install_dirs` /
    //    `archive_store` / `auto_build_driver` land in Run 4a.
    //    Deliberately not implemented here. ──

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::WizardState;

    #[test]
    fn variant_from_step5_mirrors_bio_label_logic() {
        // Fresh: !resume_available && !has_run_once ⇒ Install.
        let mut s = WizardState::default();
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Install
        );
        // has_run_once (force-cancel happened) ⇒ Restart.
        s.step5.has_run_once = true;
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Restart
        );
        // resume_available (graceful cancel) takes precedence ⇒ Resume.
        s.step5.resume_available = true;
        assert_eq!(
            InstallButtonVariant::from_step5(&s, false),
            InstallButtonVariant::Resume
        );
        // reinstall flag overrides everything ⇒ Reinstall.
        assert_eq!(
            InstallButtonVariant::from_step5(&s, true),
            InstallButtonVariant::Reinstall
        );
    }

    #[test]
    fn import_code_write_matrix_matches_spec_13_13() {
        // SPEC §13.13: Install / Restart / Reinstall write/overwrite;
        // Resume does NOT (the prior attempt's file stays canonical).
        assert!(InstallButtonVariant::Install.writes_import_code());
        assert!(InstallButtonVariant::Restart.writes_import_code());
        assert!(InstallButtonVariant::Reinstall.writes_import_code());
        assert!(
            !InstallButtonVariant::Resume.writes_import_code(),
            "Resume Install must NOT overwrite modlist-import-code.txt \
             (SPEC §13.13)"
        );
    }
}
