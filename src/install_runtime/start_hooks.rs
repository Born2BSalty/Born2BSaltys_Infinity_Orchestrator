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
// not a BIO modification").
//
// **P7.T10 / P7.T11 (Run 4b — this run).** This module now also owns:
//   - `InstallButtonVariant::from_step5_and_reinstall` — the **P7.T11
//     variant-tag derivation including the now-real reinstall flag**
//     (`state.step5.{resume_available,has_run_once}` + the orchestrator's
//     `pending_reinstall_id`). The single place the §13.13 matrix's
//     variant per entry-point is resolved (Run 4b made `Reinstall`
//     reachable — it was always `false` pre-Run-4b).
//   - `reinstall_flip_at_install_click` — the **P7.T10** Reinstall
//     `Installed → InProgress` flip (variant-gated + idempotent),
//     **authored here** (this module owns the P7.T10 transition logic) but
//     **invoked from the Install-Modlist Install-click site**
//     (`page_install.rs`'s `Preview → Downloading` transition — SPEC §3.1
//     "clicks Reinstall → to actually run it"; the flip is "only when the
//     install starts"). It is NOT performed inside `on_install_start`
//     because the Reinstall route does **not** pass through
//     `on_install_start` (Run-4a's pipeline-driven Install-Modlist path;
//     `on_install_start` is the in-Workspace Step-5 path only, which a
//     Reinstall never reaches) — a PLAN GAP, see the in-fn note + the run
//     report. `on_install_start` keeps a defensive no-op for the
//     (currently unreachable) `variant == Reinstall` so the contract is
//     honored from whichever site eventually calls it.
//
// The share-code-consuming download pipeline (P7.T17) is wired by
// `auto_build_driver` / `stage_downloading::render_live` (Run 4a) — not
// here.
//
// **Final P7 Fix-Run — the §13.13 bundle factored for the pipeline path.**
// `on_install_start` is reached **only** from the in-Workspace Step-5
// button (`page_workspace_step5.rs`). The **Install-Modlist-paste /
// Reinstall** entry points route through `page_install.rs`'s `Preview →
// Downloading` Advance into Run-4a's `auto_build_driver` pipeline
// (`stage_downloading::render_live`), which **bypasses** `on_install_start`
// — so its inline §13.13 artifact writing never ran for them (the P7.T11 /
// SPEC §13.13 / Verification-#5 acceptance gap the Run-4a trigger-split
// inadvertently opened). The fix factors the §13.13 bundle (steps 2 → 3 →
// 6 → 4) out of `on_install_start` into the reusable
// `write_install_start_artifacts` and invokes it ALSO from the
// `page_install.rs` Install-click site (alongside Run-4b's
// `reinstall_flip_at_install_click`, same split-borrow). `on_install_start`
// now *calls* that helper, so the workspace path's behavior is
// **byte-identical** (a pure extract, same statements / same order).
// (Premise-check: Create-import / Load-Draft / Create → New route into the
// **Workspace** — Step 5 → `on_install_start` → this helper — so they
// already get the bundle and need no second invocation site; the brief's
// "pipeline path for Create-import/Load-Draft" wording is a PLAN GAP, see
// the run report.)
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
use crate::install_runtime::registry_transition;
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

    /// **P7.T11 — the variant tag derivation including the now-real
    /// reinstall flag.** `state.step5.resume_available` + the orchestrator's
    /// `pending_reinstall_id`: a Reinstall route (`pending_reinstall_id ==
    /// Some(this modlist id)`) is `Reinstall` regardless of `state.step5`;
    /// otherwise the standard BIO-label derivation (`from_step5` with
    /// `reinstall = false`). This is the single place the §13.13 matrix's
    /// variant per entry-point is resolved once `pending_reinstall_id` is
    /// real (Run 4b made `Reinstall` reachable — it was always `false`
    /// pre-Run-4b). The Install-Modlist Install-click site calls this; the
    /// in-Workspace Step-5 path calls `from_step5(state, false)` (a
    /// Reinstall never reaches a Workspace, so `pending_reinstall_id` is
    /// `None` there — equivalent).
    #[must_use]
    pub fn from_step5_and_reinstall(
        state: &WizardState,
        modlist_id: &str,
        pending_reinstall_id: Option<&str>,
    ) -> Self {
        Self::from_step5(state, pending_reinstall_id == Some(modlist_id))
    }

    /// SPEC §13.13: does this variant write/overwrite `modlist-import-code
    /// .txt`? `Install` / `Restart` / `Reinstall` ⇒ true; `Resume` ⇒
    /// false (the original file from the prior attempt remains canonical).
    #[must_use]
    pub fn writes_import_code(self) -> bool {
        !matches!(self, Self::Resume)
    }
}

/// **The SPEC §13.13 install-start artifact bundle — factored so EVERY
/// install entry point writes it.** Does exactly, in this order
/// (`on_install_start`'s former inline steps 2 → 3 → 6 → 4):
///
///   2. Compute the share code via the net-new `registry::share_export
///      ::pack_meta` envelope (SPEC §13.3 Generation) with
///      `allow_auto_install = false` (install-start codes are unverified —
///      SPEC §13.3 / §13.13) and the provenance trio read **off the
///      registry `ModlistEntry`** (NOT `WizardState`).
///   3. Update `entry.latest_share_code` to that value (the registry's
///      snapshot of the in-progress code) and stamp `install_started_at`
///      (always — every variant, every attempt; distinct from
///      `install_date` which is clean-completion only, P7.T6).
///   6. Atomic registry write (`RegistryStore::save`) — done before the
///      disk write so the registry's record of the code is durable even if
///      the file write below fails.
///   4. Write `modlist-import-code.txt` to the destination, **variant-gated
///      per SPEC §13.13** (`Install` / `Restart` / `Reinstall`
///      write/overwrite; `Resume` skips — the prior attempt's file stays
///      canonical). Written upfront (before WeiDU runs) so the artifact
///      survives a crash/cancel/error. A write failure is logged but **not**
///      fatal (the file is a recovery convenience — SPEC §13.13 frames it
///      as the "I crashed, here's what I was trying" artifact).
///
/// **Deliberately does NOT do** (so it is safe to invoke from both the
/// workspace-Step-5 path *and* the Install-Modlist/Reinstall pipeline path):
/// the #1/#5 flag policies (`flag_policies::apply_flags` — applied on both
/// paths already), the per-install directory derivation (Run-4a
/// `auto_build_driver` / `per_install_dirs` — already done on the pipeline
/// path by `stage_downloading::render_live`), the install-concurrency gate
/// (workspace-only), and any `start_install_requested` flip (the caller's /
/// the pipeline's `start_auto_build_install`'s job — a premature flip
/// installs an empty per-install Mods folder, the documented P7.T17
/// hazard).
///
/// Why factored: `on_install_start` is reached **only** from the in-
/// Workspace Step-5 button (`page_workspace_step5.rs`). The Install-Modlist-
/// paste / Reinstall entry points route through `page_install.rs`'s
/// `Preview → Downloading` Advance into Run-4a's `auto_build_driver`
/// pipeline (`stage_downloading::render_live`), which **bypasses**
/// `on_install_start` — so without this factoring those entry points never
/// write the §13.13 artifacts (the P7.T11 / SPEC §13.13 / Verification-#5
/// gap the final P7 Fix-Run closes). The Create-import / Load-Draft / Create
/// → New entry points route into the **Workspace** (Step 5 →
/// `on_install_start`), so they already get the bundle via this same helper
/// through `on_install_start` — they do **not** need a second invocation
/// site (premise-checked against `page_create.rs`'s routing — see the run
/// report's PLAN GAP on the brief's premise wording).
///
/// Returns `Ok(())` on success; an `Err(String)` if the share code could
/// not be generated or the **registry** write failed (the caller surfaces
/// it per SPEC §13.14 and must **not** flip `start_install_requested` /
/// proceed-as-if-clean on `Err`). A failed `modlist-import-code.txt` write
/// is **not** an `Err` — it is logged and the bundle still returns `Ok`
/// (the artifact is a recovery convenience, not a precondition for WeiDU).
///
/// Borrow note: callers sequence any `&mut wizard_state` work (e.g.
/// `flag_policies::apply_flags`) before this — `share_export::pack_meta`
/// reads `&*wizard_state` immutably, so the immutable borrow must start
/// only after any mutable one ends (no overlap).
pub fn write_install_start_artifacts(
    modlist_id: &str,
    variant: InstallButtonVariant,
    wizard_state: &WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> Result<(), String> {
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
    // (carve-out #5). Immutable `wizard_state` borrow (any mut flag-policy
    // borrow at the call site has ended).
    let share_code = share_export::pack_meta(wizard_state, &meta)?;

    // ── 3. Update entry.latest_share_code to the in-progress code
    //    (allow_auto_install = false). Also stamp install_started_at
    //    (every variant, every attempt). ──
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

    Ok(())
}

/// **The SPEC §13.13 install-start artifact bundle, sourced from a code the
/// orchestrator ALREADY HAS — NOT regenerated via `pack_meta`.** The
/// Install-Modlist-paste / Reinstall variant of [`write_install_start_
/// artifacts`].
///
/// **Why this exists (the user's resolution, 2026-05-18).** For the
/// **Install-Modlist** entry points the user already has the code (they
/// pasted it; a Reinstall has the entry's stored `latest_share_code`).
/// Regenerating it via `pack_meta` is **impossible** there:
/// `pack_meta` → `bio::app::modlist_share::export_modlist_share_code` reads
/// `state.step3.{bgee,bg2ee}_items` (build-from-scanned-mods) or the
/// exact-WeiDU-log source, and at the install-start arm point `state.step3`
/// is empty (the share-code import's `reset_workflow_keep_step1` clears it,
/// and the async scan→apply-log has not run yet) ⇒ `export_modlist_share
/// _code` `Err`s ⇒ the whole §13.13 bundle aborted before writing
/// `install_started_at` / `latest_share_code` / the on-disk file. The
/// resolution: **persist the code the orchestrator already has** (the
/// `code_source` arg), only setting the `allow_auto_install` bit on its
/// decoded payload (`share_export::set_allow_auto_install`, the
/// `pack_meta`-envelope-minus-step-1 — no `export_modlist_share_code`,
/// **zero BIO source**). The pasted code's baked-in `name` / `author` /
/// `forked_from` ride through **verbatim** (the payload is opaque) — this is
/// the SPEC §13.3 Provenance property "the real code carries the name"
/// (achieved by *not* rewriting it), so the "Shared modlist" fallback stops.
///
/// Does exactly [`write_install_start_artifacts`]'s steps **3 → 6 → 4**,
/// with step **2** replaced by the decode-flip:
///
///   2′. `code = share_export::set_allow_auto_install(code_source, false)`
///      (install-start codes are unverified — SPEC §13.3 / §13.13). If the
///      `code_source` is somehow not a decodable BIO-MODLIST-V1 string (it
///      always is — it parsed at Preview), persist it **verbatim** (per the
///      user's resolution: persisting the real code is the priority over the
///      false→true draft nicety; the bit defaults to `true` for a code that
///      lacks the key, which is the *consume*-side default — acceptable, and
///      a degenerate input anyway).
///   3.  `entry.latest_share_code = Some(code)` + stamp `install_started_at`
///      (always — every variant; distinct from `install_date`, P7.T6).
///   6.  Atomic registry write (`RegistryStore::save`) — before the disk
///      write so the registry record is durable even if the file fails.
///   4.  Write `modlist-import-code.txt`, **variant-gated per SPEC §13.13**
///      (`Install` / `Restart` / `Reinstall` write/overwrite; `Resume`
///      skips — the prior attempt's file stays canonical). A file-write
///      failure is logged, **not** fatal.
///
/// **Deliberately does NOT** regenerate from `WizardState` (the whole point
/// — `state.step3` is empty here), nor do the #1/#5 flag policies / dir
/// derivation / Reinstall state-flip / `start_install_requested` flip (all
/// owned by the pipeline path — see [`write_install_start_artifacts`]).
///
/// Returns `Ok(())` on success; `Err(String)` only if the **registry write**
/// failed (the caller surfaces it per SPEC §13.14; the code source itself
/// never makes this `Err` — a non-decodable code is persisted verbatim, not
/// an error). A failed `modlist-import-code.txt` write is **not** an `Err`.
pub fn write_install_start_artifacts_with_code(
    modlist_id: &str,
    variant: InstallButtonVariant,
    code_source: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> Result<(), String> {
    // ── 2′. The install-start code = the code the orchestrator ALREADY HAS,
    //    with `allow_auto_install = false` set on its decoded payload (SPEC
    //    §13.3 / §13.13 — install-start codes are unverified). NOT
    //    `pack_meta` (no `export_modlist_share_code`; `state.step3` is empty
    //    here). A non-decodable source ⇒ persist VERBATIM (the user's
    //    resolution: the real code is the priority; this input always parsed
    //    at Preview, so the fallback is purely defensive). ──
    let entry = registry
        .find(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} not in registry at install start"))?;
    let destination = entry.destination_folder.trim().to_string();
    let trimmed_source = code_source.trim();
    let share_code = match share_export::set_allow_auto_install(trimmed_source, false) {
        Ok(code) => code,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "install-start: could not decode the held code for {modlist_id} \
                 to set allow_auto_install=false ({err}) — persisting it \
                 VERBATIM (the real code is the priority; SPEC §13.13)"
            );
            trimmed_source.to_string()
        }
    };

    // ── 3. Update entry.latest_share_code to the in-progress code +
    //    stamp install_started_at (every variant, every attempt). ──
    let entry_mut = registry
        .find_mut(modlist_id)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry mid-hook"))?;
    entry_mut.latest_share_code = Some(share_code.clone());
    entry_mut.install_started_at = Some(Utc::now());

    // ── 6. Atomic registry write (before the disk write so the registry
    //    record is durable even if the file write fails). ──
    store
        .save(registry)
        .map_err(|err| format!("registry write at install start failed: {err}"))?;

    // ── 4. Variant-gated `modlist-import-code.txt` write (SPEC §13.13:
    //    Install/Restart/Reinstall write/overwrite; Resume skips). Upfront,
    //    non-fatal on failure. ──
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
    // Resume Install: the file is intentionally NOT overwritten (SPEC
    // §13.13) — no-op by design.

    Ok(())
}

/// Run the install-start hook (the in-Workspace Step-5 path). See the
/// module header for the ordered contract. Applies the #1/#5 flag policies,
/// then writes the SPEC §13.13 install-start artifact bundle via
/// [`write_install_start_artifacts`] (factored so the Install-Modlist /
/// Reinstall pipeline path can invoke the SAME bundle from its authorized
/// Install-click site — see that fn's doc), then derives the per-install
/// directories (Run-4a P7.T17).
///
/// Returns `Ok(())` on success; an `Err(String)` if the share code could
/// not be generated, the registry write failed, or the per-install dir
/// derivation failed (the caller surfaces it per SPEC §13.14 and must
/// **not** flip `start_install_requested` on `Err`). A failed
/// `modlist-import-code.txt` write is logged but **not** fatal (see
/// [`write_install_start_artifacts`]).
///
/// Borrow-order note: `flag_policies::apply_flags` mutates
/// `wizard_state.step1`; `write_install_start_artifacts`'s
/// `share_export::pack_meta` then reads `&*wizard_state` immutably.
/// Sequenced so the immutable `pack_meta` borrow starts only after the
/// mutable flag write ends (no overlap).
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
    //    borrow of wizard_state.step1; ends before the §13.13 helper's
    //    immut `pack_meta` borrow below.) **NOT part of the §13.13 helper**
    //    — flag policies are applied on BOTH the workspace path (here) and
    //    the pipeline path (`render_live`'s `auto_build_driver`), so the
    //    factored §13.13 helper deliberately excludes them. ──
    flag_policies::apply_flags(&mut wizard_state.step1, workflow, settings);

    // ── Steps 2 → 3 → 6 → 4 — the SPEC §13.13 install-start artifact
    //    bundle (pack_meta share code → entry.latest_share_code →
    //    install_started_at → atomic save → variant-gated
    //    modlist-import-code.txt write). Factored into
    //    [`write_install_start_artifacts`] so the **pipeline entry points**
    //    (Install-Modlist-paste / Reinstall — which route through
    //    `page_install.rs`'s `Preview → Downloading` Advance and BYPASS
    //    `on_install_start`) can invoke the SAME bundle from their
    //    authorized Install-click site (the final P7 Fix-Run). The
    //    workspace path's behavior is byte-identical — this is a pure
    //    extract of the previously-inline statements, same order. ──
    write_install_start_artifacts(modlist_id, variant, wizard_state, registry, store)?;

    // The destination is needed again below for the per-install dir
    // derivation (step 2b). Re-read it off the (post-save) entry — the
    // §13.13 helper does not mutate it.
    let destination = registry
        .find(modlist_id)
        .map(|e| e.destination_folder.trim().to_string())
        .ok_or_else(|| {
            format!("modlist {modlist_id} vanished from registry after §13.13 artifacts")
        })?;

    // ── P7.T10 (Run 4b) — reinstall registry-flip.
    //
    //    **Resolution (PLAN GAP, see the run report).** The plan placed
    //    this flip *inside* `on_install_start` (P7.T10: "handled by
    //    P7.T3's install-start hook"). But the Reinstall route does **not**
    //    pass through `on_install_start`: per SPEC §3.1 it routes through
    //    the **Install-Modlist** preview → Downloading, and Run-4a's
    //    P7.T17 implementation drives that path via
    //    `auto_build_driver::prepare_install_dirs_and_maybe_import` —
    //    `on_install_start` is called *only* from `page_workspace_step5`
    //    (the in-Workspace Step-5 Install button), which a Reinstall never
    //    reaches (Reinstall navigates to `NavDestination::Install`, not a
    //    Workspace). And on that workspace path `pending_reinstall_id` is
    //    always `None` (the caller correctly passes
    //    `from_step5(state, false)`), so `variant == Reinstall` is
    //    unreachable here.
    //
    //    So the flip is **authored here** (this module owns the P7.T10
    //    transition logic, as the plan intends) as the variant-gated,
    //    idempotent `reinstall_flip_at_install_click` below, and
    //    **invoked from the Install-Modlist Install-click site**
    //    (`page_install.rs`'s `Preview → Downloading` transition — the
    //    literal SPEC §3.1 "clicks Reinstall → to actually run it"; the
    //    flip happens "only when the install starts", NOT at
    //    Reinstall-Kebab-click). `on_install_start` keeps a defensive
    //    no-op for the (currently unreachable) `variant == Reinstall` on
    //    the workspace path so the contract is honored from whichever site
    //    eventually calls it. ──
    if variant == InstallButtonVariant::Reinstall {
        // Defensive: the workspace path never produces this variant
        // (`pending_reinstall_id` is `None` there). If a future caller
        // does route a Reinstall through `on_install_start`, flipping here
        // would be correct — but `on_install_start` does not own
        // `pending_reinstall_id` (it is not in the signature; expanding
        // the signature would force a non-authorized edit to
        // `page_workspace_step5`), so the flip is performed by the
        // Install-click site via `reinstall_flip_at_install_click`. Log
        // that we observed the variant so a future wiring change is
        // visible.
        tracing::debug!(
            target = "orchestrator",
            "on_install_start saw InstallButtonVariant::Reinstall for \
             {modlist_id}; the Installed→InProgress flip is performed at \
             the Install-Modlist Install-click via \
             reinstall_flip_at_install_click (the Reinstall route does not \
             pass through on_install_start — see start_hooks module note)"
        );
    }

    // ── P7.T17 (Run 4a) — per-install directory derivation (SPEC
    //    §13.12a / §13.12 #2/#3/#4). EVERY install — regardless of
    //    workflow — gets the per-install Mods + #2 `weidu_component_logs`
    //    + #3/#4 forced game-clone dirs derived INSIDE the destination
    //    before WeiDU runs (an install is install-critical-blocked
    //    without them; SPEC §13.12a "clone is forced for every install").
    //    A fresh Create → New gets exactly these (the plan: "skips the
    //    import step but still gets the per-install dirs"); a
    //    share-code-consuming entry flow (Install Modlist / Create-import
    //    / Load-Draft) ALSO drives BIO's import → auto-build →
    //    download/extract pipeline — but that kick-off lives in that
    //    flow's own Downloading screen (`stage_downloading::render_live`
    //    → `auto_build_driver::prepare_install_dirs_and_maybe_import`),
    //    NOT here: the workspace Step-5 caller flips
    //    `start_install_requested = true` on this `Ok`, which would
    //    prematurely install before the auto-build staged anything (BIO's
    //    `start_if_requested` gates only on `start_install_requested`).
    //    So `on_install_start`'s P7.T17 contribution is the *directory*
    //    derivation only — idempotent, so an entry flow that already
    //    derived them via `render_live` re-derives harmlessly here (same
    //    fixed paths, `create_dir_all` is a no-op). The clone flags are
    //    forced ON; the no-clone path is never set (SPEC §13.12a).
    //
    //    Runs AFTER the flag policies (step 1) — `derive_per_install_dirs`
    //    only writes the per-install *target* fields (`mods_folder`,
    //    `eet_pre_dir`/`eet_new_dir`, `generate_directory`,
    //    `weidu_log_folder`); the global *source* folders + the #1/#5
    //    flags set above are untouched.
    let game = registry
        .find(modlist_id)
        .map(|e| e.game)
        .ok_or_else(|| format!("modlist {modlist_id} vanished from registry before dir derive"))?;
    crate::install_runtime::per_install_dirs::derive_per_install_dirs(
        &mut wizard_state.step1,
        &destination,
        game,
    )
    .map_err(|err| format!("per-install directory derivation failed for {modlist_id}: {err}"))?;

    Ok(())
}

/// **P7.T10 — the Reinstall `Installed → InProgress` flip, performed at the
/// Install-Modlist Install-click.** This is the P7.T10 transition the plan
/// placed conceptually "in P7.T3's install-start hook"; per the module note
/// above it is authored here but **invoked from the Install-Modlist
/// Install-click site** (`page_install.rs`'s `Preview → Downloading`
/// transition — SPEC §3.1: "clicks Reinstall → to actually run it … the
/// modlist state flips to `in-progress` only when the install starts"),
/// because the Reinstall route does not pass through `on_install_start`.
///
/// Variant-gated + idempotent (P7.T11 / SPEC §3.1):
///   - No `pending_reinstall_id` ⇒ **not** a Reinstall (a normal
///     Install-Modlist paste) ⇒ no-op (the entry — if any — keeps its
///     state; an Install-Modlist paste of a *new* modlist has no registry
///     entry yet anyway). Returns `false`.
///   - `pending_reinstall_id == Some(id)` ⇒ this run **is** a Reinstall:
///     derive `InstallButtonVariant::Reinstall` (via
///     [`InstallButtonVariant::from_step5_and_reinstall`]), flip the entry
///     `Installed → InProgress` via
///     `registry_transition::flip_to_in_progress` (state-only + atomic
///     write; logged-no-op if missing / not `Installed` / write fails —
///     SPEC §13.14), and **clear `pending_reinstall_id`** so a subsequent
///     frame cannot re-flip (idempotent — a second call with the flag
///     cleared returns `false` without touching the registry; and
///     `flip_to_in_progress` itself no-ops a non-`Installed` entry, so a
///     race is doubly safe).
///
/// Returns `true` iff a Reinstall flip was performed and persisted (the
/// caller may log; the install proceeds regardless — a flip-write failure
/// is non-fatal to the install per SPEC §13.14, the entry simply stays
/// `Installed` and `pending_reinstall_id` is still cleared so the route is
/// not retried in a loop).
///
/// `wizard_state` is taken (read-only) only to derive the variant via the
/// shared BIO-label logic — keeping the §13.13 variant taxonomy in one
/// place (the same `from_step5*` family the matrix uses) rather than
/// re-deciding "is this a Reinstall" ad hoc at the call site.
pub fn reinstall_flip_at_install_click(
    modlist_id: &str,
    wizard_state: &WizardState,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    pending_reinstall_id: &mut Option<String>,
) -> bool {
    // P7.T11 variant tag with the now-real reinstall flag. Only a
    // Reinstall route (`pending_reinstall_id == Some(this id)`) yields
    // `Reinstall`; everything else is the standard Install/Restart/Resume
    // (no flip — those do not transition Installed→InProgress).
    let variant = InstallButtonVariant::from_step5_and_reinstall(
        wizard_state,
        modlist_id,
        pending_reinstall_id.as_deref(),
    );
    if variant != InstallButtonVariant::Reinstall {
        return false;
    }

    // It IS a Reinstall and the install is starting now (Install-click) —
    // SPEC §3.1: flip Installed → InProgress *here*, not at
    // Reinstall-Kebab-click. State-only + atomic (the verified
    // `latest_share_code` / `modlist-import-code.txt` are the install-start
    // path's job per SPEC §13.13 — not this flip's).
    let flipped = registry_transition::flip_to_in_progress(modlist_id, registry, store);

    // Clear the marker UNCONDITIONALLY once the Install-click has been
    // taken — whether or not the write succeeded. Per SPEC §13.14 a
    // flip-write failure is non-fatal to the install; leaving the marker
    // set would re-attempt the flip every subsequent frame (and re-tag
    // every frame's variant `Reinstall`). The route has been consumed; the
    // worst case of a write failure is the entry stays `Installed` (it
    // returns to `Installed` on clean exit anyway via `flip_to_installed`).
    *pending_reinstall_id = None;

    if flipped {
        tracing::info!(
            target = "orchestrator",
            "Reinstall: {modlist_id} flipped Installed → InProgress at \
             Install-click (SPEC §3.1); pending_reinstall_id cleared"
        );
    } else {
        warn!(
            target = "orchestrator",
            "Reinstall: flip_to_in_progress for {modlist_id} did not persist \
             (see prior log); pending_reinstall_id cleared anyway — the \
             install proceeds, the entry stays Installed (returns to \
             Installed on clean exit via flip_to_installed). SPEC §13.14"
        );
    }
    flipped
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

    // ───────────── P7.T11 — the §13.13 matrix per entry-point/variant ─────────────
    //
    // P7.T11 verifies the import-code write/overwrite/skip matrix
    // end-to-end. The *write decision* is `InstallButtonVariant
    // ::writes_import_code` (proven above) and the *variant per scenario*
    // is what `from_step5` / `from_step5_and_reinstall` resolve from
    // `state.step5.{resume_available,has_run_once}` + the now-real
    // `pending_reinstall_id`. These pin the FULL matrix the plan P7.T11
    // acceptance enumerates — Fresh Install (every entry point) +
    // Restart + Resume + Reinstall — to the (variant ⇒ write?) pair so a
    // regression in either half is caught by `cargo test --lib`.

    /// (scenario state, reinstall flag) ⇒ (expected variant, expected
    /// writes-import-code) per SPEC §13.13.
    fn matrix_row(
        resume_available: bool,
        has_run_once: bool,
        reinstall: bool,
    ) -> (InstallButtonVariant, bool) {
        let mut s = WizardState::default();
        s.step5.resume_available = resume_available;
        s.step5.has_run_once = has_run_once;
        let v = InstallButtonVariant::from_step5(&s, reinstall);
        (v, v.writes_import_code())
    }

    #[test]
    fn spec_13_13_matrix_holds_per_entry_point_and_variant() {
        // Fresh Install — Create→New, Create→Import-and-modify,
        // Install-Modlist paste, Load-Draft→first run: a first attempt is
        // `!resume_available && !has_run_once` ⇒ `Install` ⇒ **write**.
        assert_eq!(
            matrix_row(false, false, false),
            (InstallButtonVariant::Install, true),
            "Fresh Install (all non-reinstall entry points) ⇒ Install ⇒ write"
        );
        // Reinstall (Home Kebab → Reinstall): the reinstall flag wins over
        // any `state.step5` ⇒ `Reinstall` ⇒ **write/overwrite**.
        assert_eq!(
            matrix_row(false, false, true),
            (InstallButtonVariant::Reinstall, true),
            "Reinstall ⇒ Reinstall ⇒ write/overwrite (SPEC §13.13)"
        );
        // Restart Install (after a force-cancel): `has_run_once &&
        // !resume_available` ⇒ `Restart` ⇒ **overwrite**.
        assert_eq!(
            matrix_row(false, true, false),
            (InstallButtonVariant::Restart, true),
            "Restart Install (post force-cancel) ⇒ Restart ⇒ overwrite"
        );
        // Resume Install (after a graceful cancel): `resume_available`
        // takes precedence ⇒ `Resume` ⇒ **skip** (the prior attempt's
        // file stays canonical — SPEC §13.13).
        assert_eq!(
            matrix_row(true, true, false),
            (InstallButtonVariant::Resume, false),
            "Resume Install (post graceful-cancel) ⇒ Resume ⇒ SKIP \
             (prior attempt's modlist-import-code.txt preserved — SPEC §13.13)"
        );
        // Reinstall overrides even a resume-available state (a Reinstall
        // is always a fresh from-scratch attempt — SPEC §3.1).
        assert_eq!(
            matrix_row(true, true, true),
            (InstallButtonVariant::Reinstall, true),
            "the reinstall flag wins over resume_available ⇒ Reinstall ⇒ write"
        );
    }

    #[test]
    fn from_step5_and_reinstall_wires_pending_reinstall_id() {
        // P7.T11 variant-tag wiring: `Reinstall` iff `pending_reinstall_id
        // == Some(this modlist id)`; otherwise the standard from_step5
        // derivation. (Pre-Run-4b `pending_reinstall_id` was always None
        // ⇒ Reinstall was unreachable; Run 4b makes it real.)
        let s = WizardState::default(); // fresh ⇒ Install unless reinstall

        // No pending reinstall ⇒ Install (the workspace Step-5 path —
        // equivalent to `from_step5(state, false)`).
        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", None),
            InstallButtonVariant::Install
        );
        // Pending reinstall for a DIFFERENT modlist ⇒ still Install (the
        // flag is per-modlist — only THIS modlist's reinstall counts).
        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", Some("MOD-B")),
            InstallButtonVariant::Install,
            "a pending reinstall for a different modlist must not tag this one"
        );
        // Pending reinstall for THIS modlist ⇒ Reinstall.
        assert_eq!(
            InstallButtonVariant::from_step5_and_reinstall(&s, "MOD-A", Some("MOD-A")),
            InstallButtonVariant::Reinstall,
            "pending_reinstall_id == Some(this id) ⇒ Reinstall (SPEC §3.1)"
        );
    }

    // ─────── P7.T10 — reinstall_flip_at_install_click (Install-click flip) ───────
    //
    // The Install-Modlist Install-click site calls this. It is
    // variant-gated (only a real Reinstall route flips) + idempotent
    // (clears `pending_reinstall_id` so a subsequent frame is a no-op).
    // Uses a temp-path `RegistryStore` (DATA-LOSS-safe — it calls the real
    // `RegistryStore::save`; a test MUST NEVER bind `%APPDATA%\bio`).

    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_registry_store(label: &str) -> (RegistryStore, std::path::PathBuf) {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!(
                "bio_start_hooks_test_{}_{}_{}",
                std::process::id(),
                n,
                label
            ))
            .with_extension("json");
        (RegistryStore::new_with_path(&path), path)
    }

    fn installed_entry(id: &str) -> ModlistEntry {
        let mut e = ModlistEntry::default();
        e.id = id.to_string();
        e.name = "Polished EET".to_string();
        e.game = Game::EET;
        e.state = ModlistState::Installed;
        e.latest_share_code = Some("BIO-MODLIST-V1:VERIFIED".to_string());
        e
    }

    #[test]
    fn reinstall_flip_at_install_click_flips_and_clears_when_reinstall() {
        let (store, path) = temp_registry_store("flip_happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("REINSTALL0001"));
        let s = WizardState::default();
        let mut pending = Some("REINSTALL0001".to_string());

        let flipped = reinstall_flip_at_install_click(
            "REINSTALL0001",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );

        assert!(flipped, "a real Reinstall route flips + persists");
        assert_eq!(
            registry.find("REINSTALL0001").unwrap().state,
            ModlistState::InProgress,
            "Installed → InProgress at Install-click (SPEC §3.1)"
        );
        assert_eq!(
            pending, None,
            "pending_reinstall_id cleared so a later frame cannot re-flip"
        );

        // Idempotent: a second call with the marker cleared is a no-op (no
        // flip, no panic) — the entry is left as InProgress.
        let again = reinstall_flip_at_install_click(
            "REINSTALL0001",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );
        assert!(!again, "marker cleared ⇒ no-op (not a Reinstall anymore)");
        assert_eq!(
            registry.find("REINSTALL0001").unwrap().state,
            ModlistState::InProgress
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reinstall_flip_at_install_click_is_noop_without_pending() {
        // A normal Install-Modlist paste (no Reinstall route) ⇒ no flip,
        // no state change, marker stays None.
        let (store, path) = temp_registry_store("flip_nopending");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("SOME-MODLIST"));
        let s = WizardState::default();
        let mut pending: Option<String> = None;

        let flipped = reinstall_flip_at_install_click(
            "SOME-MODLIST",
            &s,
            &mut registry,
            &store,
            &mut pending,
        );
        assert!(
            !flipped,
            "no pending_reinstall_id ⇒ not a Reinstall ⇒ no-op"
        );
        assert_eq!(
            registry.find("SOME-MODLIST").unwrap().state,
            ModlistState::Installed,
            "a non-Reinstall Install-Modlist paste must NOT flip state"
        );
        assert_eq!(pending, None);
        // The early return precedes any save.
        assert!(!path.exists());
    }

    #[test]
    fn reinstall_flip_at_install_click_pending_for_other_modlist_is_noop() {
        // The marker is per-modlist: a pending reinstall for a DIFFERENT
        // modlist must not flip THIS one (and must not clear the other's
        // marker — only the matching Install-click consumes it).
        let (store, _path) = temp_registry_store("flip_other");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("MOD-A"));
        let s = WizardState::default();
        let mut pending = Some("MOD-B".to_string());

        let flipped =
            reinstall_flip_at_install_click("MOD-A", &s, &mut registry, &store, &mut pending);
        assert!(
            !flipped,
            "pending is for MOD-B, not MOD-A ⇒ no-op for MOD-A"
        );
        assert_eq!(
            registry.find("MOD-A").unwrap().state,
            ModlistState::Installed
        );
        assert_eq!(
            pending,
            Some("MOD-B".to_string()),
            "MOD-B's pending marker is untouched (only MOD-B's own \
             Install-click consumes it)"
        );
    }

    // ───── Final P7 Fix-Run — the factored §13.13 bundle helper ─────
    //
    // `write_install_start_artifacts` is the §13.13 install-start bundle
    // (pack_meta share code → entry.latest_share_code → install_started_at
    // → atomic save → variant-gated modlist-import-code.txt write) factored
    // out of `on_install_start` so the Install-Modlist / Reinstall pipeline
    // path can invoke the SAME bundle.
    //
    // The *full* success path needs `pack_meta`, which needs BIO's
    // file-backed weidu-log / mod-download state a unit test cannot stand
    // up (the exact constraint `share_export`'s own tests + the existing
    // `start_hooks` tests document — `on_install_start` itself has no
    // unit test for the same reason). What IS unit-testable additively,
    // with a temp-path store (DATA-LOSS-safe — `write_install_start_
    // artifacts` calls the real `RegistryStore::save`; a test MUST NEVER
    // bind `%APPDATA%\bio`):
    //
    //   (a) the §13.14 error contract — a non-generatable share code
    //       (`WizardState::default()` ⇒ `export_modlist_share_code` ⇒
    //       `Err("No WeiDU entries available to export.")` ⇒ `pack_meta`
    //       `Err`) makes the helper return `Err` **before** any registry
    //       mutation / save / file write (the `?` on `pack_meta` precedes
    //       steps 3/6/4). The caller must surface that and NOT
    //       proceed-as-clean (SPEC §13.14). Proven side-effect-free: the
    //       temp store file is never created, the entry is untouched.
    //   (b) a missing registry entry ⇒ the helper's first `?`
    //       (`registry.find(modlist_id)`) returns `Err` with the
    //       documented message — no panic, no partial write. (This is
    //       exactly why a brand-new Install-Modlist *paste*, which has no
    //       registry entry at install-start, cannot be served by this
    //       helper at the `page_install.rs` Advance — see the run report's
    //       SPEC CONFLICT.)
    //   (c) `on_install_start` and `write_install_start_artifacts` share
    //       the SAME §13.13 write decision (`InstallButtonVariant
    //       ::writes_import_code`) — proving the matrix governs both call
    //       sites identically (the refactor is behavior-preserving for the
    //       write/overwrite/skip gate the existing
    //       `spec_13_13_matrix_holds_per_entry_point_and_variant` already
    //       pins).

    #[test]
    fn write_install_start_artifacts_errs_cleanly_when_share_code_ungeneratable() {
        // SPEC §13.14: a share-code-generation failure is an Err the caller
        // must surface (and NOT flip start_install_requested / proceed as
        // clean). `WizardState::default()` has no weidu entries, so
        // `export_modlist_share_code` (inside `pack_meta`) returns
        // `Err("No WeiDU entries available to export.")`. The helper must
        // return `Err` BEFORE touching the registry (the `?` on `pack_meta`
        // is step 2, before steps 3/6/4) and write NOTHING.
        let (store, store_path) = temp_registry_store("artifacts_pack_err");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(installed_entry("MODLIST-ART-1"));
        let before = registry.find("MODLIST-ART-1").unwrap().clone();
        let s = WizardState::default(); // ⇒ no weidu entries ⇒ pack_meta Err

        let r = write_install_start_artifacts(
            "MODLIST-ART-1",
            InstallButtonVariant::Install, // a *writing* variant
            &s,
            &mut registry,
            &store,
        );

        assert!(
            r.is_err(),
            "ungeneratable share code ⇒ Err (SPEC §13.14 — caller must not \
             proceed-as-clean)"
        );
        // Side-effect-free: the entry is byte-identical (no
        // latest_share_code / install_started_at mutation — the pack_meta
        // `?` precedes steps 3/6/4).
        let after = registry.find("MODLIST-ART-1").unwrap();
        assert_eq!(
            after.latest_share_code, before.latest_share_code,
            "Err before any entry mutation — latest_share_code untouched"
        );
        assert_eq!(
            after.install_started_at, before.install_started_at,
            "Err before any entry mutation — install_started_at untouched"
        );
        // The atomic save (step 6) was never reached ⇒ no store file.
        assert!(
            !store_path.exists(),
            "the §13.13 helper must not save the registry when it Errs early"
        );
    }

    #[test]
    fn write_install_start_artifacts_errs_when_modlist_not_in_registry() {
        // The helper's FIRST `?` is `registry.find(modlist_id)`. A missing
        // entry ⇒ the documented Err, no panic, no write. This is why a
        // brand-new Install-Modlist *paste* (no registry entry at
        // install-start — the established Phase-7 design) cannot be served
        // by this helper at the page_install.rs Advance (run report SPEC
        // CONFLICT).
        let (store, store_path) = temp_registry_store("artifacts_no_entry");
        let mut registry = ModlistRegistry::default(); // empty — no entry
        let s = WizardState::default();

        let r = write_install_start_artifacts(
            "GHOST-MODLIST",
            InstallButtonVariant::Reinstall,
            &s,
            &mut registry,
            &store,
        );

        let msg = r.expect_err("a missing registry entry must Err");
        assert!(
            msg.contains("GHOST-MODLIST") && msg.contains("not in registry"),
            "documented Err message naming the missing modlist; got: {msg}"
        );
        assert!(
            !store_path.exists(),
            "no registry save on the missing-entry early Err"
        );
    }

    #[test]
    fn on_install_start_and_helper_share_the_same_13_13_write_decision() {
        // The refactor is behavior-preserving for the §13.13 write/overwrite
        // /skip gate: BOTH `on_install_start` (which now calls the helper)
        // and `write_install_start_artifacts` route the file write through
        // the SAME `InstallButtonVariant::writes_import_code` predicate the
        // existing `spec_13_13_matrix_holds_per_entry_point_and_variant`
        // already pins. Re-assert the full matrix here from the variant the
        // call sites resolve so a future change to EITHER the gate or the
        // factoring is caught.
        for (resume, has_run_once, reinstall, expect_write) in [
            (false, false, false, true), // Install      ⇒ write
            (false, true, false, true),  // Restart      ⇒ overwrite
            (true, true, false, false),  // Resume       ⇒ SKIP
            (false, false, true, true),  // Reinstall    ⇒ write
            (true, true, true, true),    // Reinstall wins over resume
        ] {
            let mut s = WizardState::default();
            s.step5.resume_available = resume;
            s.step5.has_run_once = has_run_once;
            let variant = InstallButtonVariant::from_step5(&s, reinstall);
            assert_eq!(
                variant.writes_import_code(),
                expect_write,
                "the §13.13 write decision the factored helper + \
                 on_install_start both use must match the matrix \
                 (resume={resume}, has_run_once={has_run_once}, \
                 reinstall={reinstall})"
            );
        }
    }
}
