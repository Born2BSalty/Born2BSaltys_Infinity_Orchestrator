// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::registry_transition` — the modlist registry
// state-transition helpers (SPEC §9.2 / §13.1 / §3.1).
//
// The two transitions:
//   - `flip_to_installed(id, registry, store, wizard_state) ->
//     Option<SizeWorkerHandle>` — the post-success transition
//     (`InProgress → Installed`, set `install_date`, refresh
//     `mod_count`/`component_count`, **regenerate `latest_share_code` with
//     `allow_auto_install = true` via `registry::share_export::pack_meta`**,
//     atomic write with `total_size_bytes = None`, then kick an async size
//     worker). Owned by **P7.T6 (Run 3 — this run)**.
//   - `flip_to_in_progress(id, registry, store)` — the Reinstall
//     install-start transition (`Installed → InProgress`). Owned by
//     **P7.T10 (Run 4b — this run)**: the symmetric inverse of
//     `flip_to_installed`'s state side (one field mutation + one atomic
//     write, same logged-no-op edges + return/style). Called at the
//     Reinstall route's **Install-click** (SPEC §3.1 — only when the
//     install starts, NOT at Reinstall-Kebab-click); the caller clears
//     `pending_reinstall_id`.
//
// **C3 fire-once contract.** `flip_to_installed` is called by the
// orchestrator's `update` loop **exactly once** on the C3 clean-exit edge —
// the frame `wizard_state.step5.install_running` transitions `true → false`
// while the C3 triple holds (`!install_running && last_exit_code == Some(0)
// && !last_install_failed` — `success_banner::clean_exit`). The
// orchestrator owns the edge (it already edge-detects `install_was_running`
// for `install_running_since`); this function is a pure side-effecting unit
// (registry mutate + atomic write + spawn worker), it does **not** itself
// detect the edge. The caller guarantees once-per-edge by guarding on the
// same `install_was_running && !install_running` transition the
// `install_running_since` reset uses.
//
// **`flip_to_installed` is the ONLY Infinity-Orchestrator code path that
// produces an `allow_auto_install = true` code** (SPEC §13.3). It NEVER
// patches `bio::app::modlist_share` (carve-out #5 "generation is not a BIO
// modification"). The true-bit code comes from one of **two sources by
// entry point** (the user's resolution, 2026-05-18 — see `flip_to_installed`
// step 3):
//   • **Workspace / build-from-scanned-mods** (`share_code_override ==
//     None`) — `share_export::pack_meta` regeneration (UNCHANGED;
//     `state.step3` is populated there, so regenerating is correct).
//   • **Install-Modlist paste / Reinstall** (`share_code_override ==
//     Some`) — the orchestrator's already-held code (the install-start
//     `latest_share_code`), bit-flipped to `true` via
//     `share_export::set_allow_auto_install` (NOT `pack_meta` —
//     `state.step3` is empty on that path; the pasted code's baked-in
//     provenance rides through verbatim). Both composers are
//     `share_export`'s own envelope; neither patches BIO.
// On a clean exit
// the on-disk `<destination>/modlist-import-code.txt` **is** rewritten with
// that `allow_auto_install = true` code (SPEC §13.13 — the file
// on disk next to the install IS the verified, auto-install-eligible code
// once the install succeeds, so forwarding the file recreates a directly-
// installable modlist). Only the **import-code file** is rewritten — H8
// still holds (no registry snapshot is ever written to disk; the live
// `orchestrator.registry` stays the only registry view). It is the SAME
// destination + filename `start_hooks::write_install_start_artifacts` wrote
// at install start (reuses `import_code_writer::write_modlist_import_code_txt`
// — the one canonical path/filename derivation, no divergent name). The
// disk write is **non-fatal** (logged, never panics) on a missing/empty
// destination or an I/O error — the registry's regenerated
// `latest_share_code` is the canonical "verified" code the Share dialog
// reads; the on-disk file is the recovery convenience.
//
// **Async size computation (plan P7.T6).** Recursive `du` on a large EET
// install can take minutes (worse on Windows with anti-virus scanning), so
// `flip_to_installed` writes the registry immediately with
// `total_size_bytes = None` and spawns a `std::thread` that walks the
// destination and reports the byte total back over an `mpsc` channel. The
// orchestrator drains that channel each frame and does a SECOND atomic
// write filling `total_size_bytes`. The success banner / post-install
// action row do NOT wait for the size. Failure modes (all per plan P7.T6):
//   - worker thread panic ⇒ the JoinHandle is dropped; nothing is sent;
//     `total_size_bytes` stays `None`; the Home card keeps rendering `—`.
//   - modlist deleted between worker start and result ⇒ the orchestrator's
//     drain looks the id up; absent ⇒ discard silently.
//   - registry write failure when storing the size ⇒ log + retry next
//     debounce (the orchestrator's drain re-attempts; size is meta, not
//     install-lifecycle-critical).
//   - worker > 5 min ⇒ keep waiting (no abort) — the channel just delivers
//     late; the banner/row never depended on it.
//
// SPEC: §9.2, §13.1, §13.3, §13.13, §3.1.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use chrono::Utc;
use tracing::warn;

use crate::app::state::WizardState;
use crate::install_runtime::import_code_writer;
use crate::registry::model::{ModlistRegistry, ModlistState};
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store::RegistryStore;
use crate::ui::workspace::step4::workspace_step4;

/// A size-worker result: `(modlist_id, total_size_bytes)`. The orchestrator
/// holds the [`Receiver`] (a net-new `OrchestratorApp` field) and drains it
/// each frame; on a value it looks the id up in the live registry and (if
/// still present) does the second atomic write filling `total_size_bytes`.
pub type SizeWorkerResult = (String, u64);

/// The receiver end the orchestrator owns to learn an async size result.
pub type SizeWorkerReceiver = Receiver<SizeWorkerResult>;

/// `(mod_count, component_count)` derived from the orchestrator-owned
/// `WizardState` Step-3 order — the **same source** the Step-4 count line
/// uses (`step4_save_row::active_tab_counts`: installable leaves =
/// non-parent `Step3ItemState`s; mods = distinct `tp_file`s), summed across
/// **all** game tabs the modlist installs (both BGEE + BG2EE for EET, just
/// BGEE otherwise — `workspace_step4::is_dual_game`). Mirroring BIO/the
/// redesign's own resolver (NOT an invented count) keeps the registry
/// `mod_count`/`component_count` from ever drifting from what Step 4
/// displayed for the same modlist. Pure + testable in isolation.
#[must_use]
pub fn count_mods_and_components(state: &WizardState) -> (u32, u32) {
    // The game tabs this modlist installs. EET installs into BOTH BGEE and
    // BG2EE phases; every other game is single-tab (BGEE bucket — BIO
    // routes IWDEE through the BGEE bucket, same as `active_tab_items`).
    let tabs: &[&[crate::app::state::Step3ItemState]] = if workspace_step4::is_dual_game(state) {
        &[&state.step3.bgee_items, &state.step3.bg2ee_items]
    } else {
        &[&state.step3.bgee_items]
    };

    let mut component_count: usize = 0;
    let mut seen_tp: Vec<String> = Vec::new();
    for items in tabs {
        for it in items.iter() {
            // Synthetic parent group rows are not installable leaves — the
            // exact `!i.is_parent` filter `step4_save_row::active_tab_counts`
            // applies (the wireframe's `selected`).
            if it.is_parent {
                continue;
            }
            component_count += 1;
            // Distinct `tp_file` (case-insensitive — the same comparison
            // `active_tab_counts` uses for the unique-mod set, the
            // wireframe's `new Set(selected.map(c => c.tp2)).size`).
            if !seen_tp.iter().any(|t| t.eq_ignore_ascii_case(&it.tp_file)) {
                seen_tp.push(it.tp_file.clone());
            }
        }
    }

    // `u32` matches the registry field type; an install with > 4 billion
    // components is not physically possible, so the saturating cast is
    // exact in practice.
    (
        u32::try_from(seen_tp.len()).unwrap_or(u32::MAX),
        u32::try_from(component_count).unwrap_or(u32::MAX),
    )
}

/// **P7.T6 — the post-success registry transition (the C3 clean-exit
/// edge).** Called by the orchestrator's `update` loop EXACTLY ONCE on the
/// frame the C3 triple first holds (see the module header's fire-once
/// contract). Performs, in order:
///
///   1. `state = ModlistState::Installed`, `install_date = now`.
///   2. Refresh `mod_count` / `component_count` from the live `WizardState`
///      via [`count_mods_and_components`] (the same source Step 4 shows —
///      never an invented count).
///   3. Produce the `allow_auto_install = true` `latest_share_code` (SPEC
///      §13.3 — the only auto-install-eligible code path), from one of two
///      sources by entry point (`share_code_override`): `None` ⇒ Workspace
///      path — regenerate via `registry::share_export::pack_meta`,
///      provenance off the entry (UNCHANGED); `Some(src)` ⇒ Install-Modlist
///      paste / Reinstall — bit-flip the orchestrator's already-held code
///      `src` via `share_export::set_allow_auto_install` (NOT `pack_meta` —
///      `state.step3` is empty there; provenance rides through verbatim;
///      verbatim fallback if undecodable). This OVERWRITES the install-start
///      `allow_auto_install = false` snapshot.
///   4. `total_size_bytes = None` (the async worker fills it later).
///   5. Atomic registry write (`RegistryStore::save`).
///   5b. Rewrite `<destination>/modlist-import-code.txt` with the same
///      regenerated `allow_auto_install = true` code (SPEC §13.13 — on a
///      clean exit the on-disk artifact becomes the verified, directly-
///      installable code). Reuses
///      `import_code_writer::write_modlist_import_code_txt` (the same path
///      / filename `write_install_start_artifacts` used at install start —
///      no divergent name). Non-fatal: a missing/empty destination or an
///      I/O failure is logged and skipped (the registry's
///      `latest_share_code` is canonical; mirrors
///      `write_install_start_artifacts`'s error handling). **H8: only the
///      import-code file is written — never a registry snapshot.**
///   6. Spawn the async size worker; return its [`SizeWorkerReceiver`] for
///      the orchestrator to drain.
///
/// Returns `Some(receiver)` on success (the orchestrator stores it and
/// drains it each frame for the deferred size fill). Returns `None` if the
/// modlist is not in the registry, the share-code regeneration failed, or
/// the atomic write failed — in every `None` case the failure is logged and
/// **no** size worker is spawned (there is nothing to fill; the install
/// itself already completed — SPEC §13.14 frames the registry flip failure
/// as non-fatal to the completed install). A `Some` is always returned with
/// a live worker even if `destination_folder` is empty/missing (the worker
/// then reports `0` and the second write records `Some(0)` — an honest
/// "nothing measurable on disk" rather than a perpetual `—`).
pub fn flip_to_installed(
    id: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
    wizard_state: &WizardState,
    share_code_override: Option<&str>,
) -> Option<SizeWorkerReceiver> {
    // ── 2. Counts from the live WizardState (same source as Step 4 —
    //    computed BEFORE the &mut entry borrow so the immutable
    //    `wizard_state` read does not overlap). ──
    let (mod_count, component_count) = count_mods_and_components(wizard_state);

    // ── 3. Produce the post-success `allow_auto_install = TRUE` code
    //    (SPEC §13.3 — `flip_to_installed` is the ONLY true-bit path).
    //    TWO sources, by entry point (the user's resolution, 2026-05-18):
    //
    //      • **Workspace / build-from-scanned-mods** (`share_code_override
    //        == None`) — UNCHANGED: regenerate via `share_export::pack_meta`
    //        (`export_modlist_share_code` read-only + provenance off the
    //        entry). `state.step3` IS populated on that path (built in the
    //        workspace), so regeneration is correct — it was never the
    //        broken case.
    //      • **Install-Modlist paste / Reinstall** (`share_code_override ==
    //        Some(src)`) — the orchestrator PERSISTS the code it already has
    //        (the install-start `latest_share_code` / the pasted code; `src`
    //        is passed by `maybe_flip_to_installed_on_clean_exit`), only
    //        flipping its decoded payload's bit to `true` via
    //        `share_export::set_allow_auto_install`. `pack_meta` is
    //        impossible here — `state.step3` is empty on the Install-Modlist
    //        path (the import's `reset_workflow_keep_step1` cleared it; even
    //        post-scan the resolution is "persist the held code, do NOT
    //        regenerate from internal state"). The pasted code's baked-in
    //        `name`/`author`/`forked_from` ride through verbatim (SPEC
    //        §13.3). A non-decodable `src` ⇒ persist it VERBATIM (the user's
    //        priority: the real code over the false→true draft nicety).
    //
    //    Done while only an immutable `registry` borrow is live (the &mut
    //    entry borrow starts after). ──
    let Some(entry_ref) = registry.find(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} not in registry on clean exit \
             (install completed; registry flip skipped — SPEC §13.14)"
        );
        return None;
    };
    let destination = entry_ref.destination_folder.trim().to_string();

    // ── Export-time hashing (the Wabbajack-compile step — the PRIMARY bake
    //    point per the run brief). This is a clean exit: the just-
    //    downloaded+verified archives are on disk and `archive_store`'s
    //    per-install lock recorded `name → hash` for THIS modlist. Bake
    //    those `{name, size, hash}` triples into the verified,
    //    re-shareable code so a recipient's install-time checksum-then-skip
    //    can avoid re-downloading archives they already have (SPEC §13.3 /
    //    §13.12a). Empty (no lock / not on disk) ⇒ the key is omitted (a
    //    code with no archive meta — today's-behavior fallback; never an
    //    error). Read-only on the lock + the archive dir; zero BIO edit. ──
    let archive_dir =
        std::path::PathBuf::from(wizard_state.step1.mods_archive_folder.trim().to_string());
    let archive_meta =
        share_export::build_archive_meta_from_install_lock(&destination, &archive_dir);

    let new_code = if let Some(src) = share_code_override {
        // Install-Modlist paste / Reinstall — flip the held code's bit to
        // true (NOT `pack_meta`). Verbatim fallback if it does not decode
        // (the real code is the priority — SPEC §13.13 / the user's
        // resolution); this never `None`s the flip (a degenerate code still
        // becomes the verified on-disk artifact + registry snapshot).
        let bit_flipped = match share_export::set_allow_auto_install(src.trim(), true) {
            Ok(code) => code,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: could not decode the held code for \
                     {id} to set allow_auto_install=true ({err}) — persisting \
                     it VERBATIM (the real code is the priority; SPEC §13.13)"
                );
                src.trim().to_string()
            }
        };
        // Bake the per-archive `{size,hash}` into the verified code the
        // same opaque way provenance rides (`bake_archive_meta_into_code` =
        // the `set_allow_auto_install` envelope with `archive_meta` instead
        // of the bit — every other key, incl. provenance, preserved
        // verbatim). Empty ⇒ a lossless re-encode (key omitted). A
        // non-decodable code ⇒ keep the bit-flipped/verbatim form (the
        // real code is the priority; the skip just won't help recipients).
        match share_export::bake_archive_meta_into_code(&bit_flipped, &archive_meta) {
            Ok(code) => code,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: could not bake archive_meta into the \
                     held code for {id} ({err}) — keeping it without the \
                     per-archive {{size,hash}} (recipients fall back to \
                     always-download; the real code is preserved)"
                );
                bit_flipped
            }
        }
    } else {
        // Workspace / build-from-scanned-mods — UNCHANGED: regenerate via
        // `pack_meta` (provenance off the entry; BIO base read-only) — now
        // ALSO carrying the per-archive `{size,hash}` baked the SAME opaque
        // way provenance is (the `ShareMeta::with_archive_meta` builder
        // feeds `pack_meta`'s shared `insert_archive_meta`).
        let meta = ShareMeta::from_entry(entry_ref, /* allow_auto_install */ true)
            .with_archive_meta(archive_meta.clone());
        match share_export::pack_meta(wizard_state, &meta) {
            Ok(code) => code,
            Err(err) => {
                // SPEC §13.14: the install itself completed; only the
                // post-success code regeneration failed. Log + bail (do NOT
                // half-flip the state). The install-start `latest_share_code`
                // (allow_auto_install = false) stays as-is.
                warn!(
                    target = "orchestrator",
                    "flip_to_installed: share-code regeneration for {id} \
                     failed: {err} (registry NOT flipped — install already \
                     completed)"
                );
                return None;
            }
        }
    };

    // ── 1 + 2 + 3 + 4. Mutate the entry: state, install_date, counts, the
    //    regenerated true-bit code, size cleared (worker fills it). ──
    let Some(entry) = registry.find_mut(id) else {
        // Vanished between the immutable read above and here (a delete
        // racing the flip). Nothing to write.
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} vanished mid-flip; skipping"
        );
        return None;
    };
    entry.state = ModlistState::Installed;
    entry.install_date = Some(Utc::now());
    entry.mod_count = mod_count;
    entry.component_count = component_count;
    // Keep a copy of the regenerated true-bit code for the on-disk
    // `modlist-import-code.txt` rewrite (step 5b) — the entry takes
    // ownership of the original below.
    let verified_code = new_code.clone();
    entry.latest_share_code = Some(new_code);
    // The async worker computes the real footprint; until it reports the
    // Home card renders `—` (per SPEC §13.1 / plan P7.T6 — `total_size_bytes
    // == None` ⇒ `—`). Always reset so a stale prior value can't show a
    // wrong size for the freshly-(re)installed modlist.
    entry.total_size_bytes = None;

    // ── 5. Atomic registry write. SPEC §13.14: if `modlists.json` is
    //    unwritable the install still completed — log it; the state flip is
    //    lost but the install is done. (No size worker is spawned in that
    //    case — there's no durable entry for it to update.) ──
    if let Err(err) = store.save(registry) {
        warn!(
            target = "orchestrator",
            "flip_to_installed: atomic registry write for {id} failed: {err} \
             (install completed; in-progress→installed flip not persisted — \
             SPEC §13.14)"
        );
        return None;
    }

    // ── 5b. Rewrite the on-disk `<destination>/modlist-import-code.txt`
    //    with the regenerated `allow_auto_install = true` code (SPEC
    //    §13.13). At install start `write_install_start_artifacts` wrote
    //    this file with the draft (`allow_auto_install = false`) code; on a
    //    clean exit the on-disk artifact becomes the verified, directly-
    //    installable code so forwarding the file recreates an
    //    auto-install-eligible modlist (matching the registry's
    //    `latest_share_code`). Reuses the SAME path/filename derivation
    //    `write_install_start_artifacts` used (`import_code_writer
    //    ::write_modlist_import_code_txt` → `<dest>/IMPORT_CODE_FILENAME`)
    //    — no divergent name. **H8: ONLY the import-code file is written —
    //    never a registry snapshot to disk; the live `orchestrator.registry`
    //    stays the only registry view.** Non-fatal + defensive, mirroring
    //    `write_install_start_artifacts`'s handling: an empty/missing
    //    destination or an I/O failure is logged and skipped (never panics)
    //    — the registry's `latest_share_code` (already persisted by step 5)
    //    is the canonical verified code the Share dialog reads; the on-disk
    //    file is the recovery convenience. Done AFTER the atomic registry
    //    write so the registry's record is durable first (the exact
    //    registry-then-disk order `write_install_start_artifacts` uses). ──
    if destination.is_empty() {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} has no destination_folder on \
             clean exit — skipping the modlist-import-code.txt rewrite \
             (nothing to write it next to; registry latest_share_code is \
             canonical — SPEC §13.13)"
        );
    } else if let Err(err) =
        import_code_writer::write_modlist_import_code_txt(Path::new(&destination), &verified_code)
    {
        warn!(
            target = "orchestrator",
            "flip_to_installed: rewriting modlist-import-code.txt to \
             {destination} on clean exit failed: {err} (non-fatal — the \
             registry holds the verified allow_auto_install=true code; the \
             on-disk file stays the install-start draft — SPEC §13.13/§13.14)"
        );
    }

    // ── 6. Spawn the async size worker. It walks `destination` recursively
    //    and reports `(id, total_bytes)` once. The orchestrator drains the
    //    receiver each frame + does the second atomic write. A panic in the
    //    worker drops the Sender silently (the orchestrator's `try_recv`
    //    just never yields a value ⇒ `total_size_bytes` stays `None` ⇒ the
    //    Home card keeps rendering `—` — exactly the plan's panic mode). ──
    let (tx, rx): (Sender<SizeWorkerResult>, Receiver<SizeWorkerResult>) = mpsc::channel();
    let id_for_worker = id.to_string();
    // Name the thread so a panic is attributable in logs (debugging aid;
    // does not change behavior).
    let spawn = std::thread::Builder::new()
        .name(format!("io-size-{id}"))
        .spawn(move || {
            let bytes = directory_size_bytes(Path::new(&destination));
            // If the receiver was already dropped (orchestrator shut down /
            // entry deleted and the field cleared), `send` errors — ignore;
            // there is nothing to update.
            let _ = tx.send((id_for_worker, bytes));
        });
    if let Err(err) = spawn {
        // OS refused the thread (extremely rare). The registry is already
        // flipped + written (steps 1-5 succeeded); only the deferred size
        // is unavailable — the Home card renders `—`, consistent with the
        // worker-panic mode. Not fatal.
        warn!(
            target = "orchestrator",
            "flip_to_installed: failed to spawn size worker for {id}: {err} \
             (size will render as — ; install + state flip already persisted)"
        );
        return None;
    }

    Some(rx)
}

/// Recursively sum the byte size of every regular file under `root`
/// (symlinks are NOT followed — a recursive `du`-style walk; broken /
/// unreadable entries are skipped, not fatal — a partial figure is better
/// than none, and the plan treats size as a non-critical meta detail). An
/// empty / non-existent / non-directory `root` yields `0` (an honest
/// "nothing measurable"). Iterative (explicit stack) so a deeply-nested EET
/// install tree cannot blow the thread stack.
fn directory_size_bytes(root: &Path) -> u64 {
    if !root.is_dir() {
        return 0;
    }
    let mut total: u64 = 0;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            // Unreadable dir (permissions / vanished mid-walk) — skip it.
            continue;
        };
        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                // Do not follow symlinks (avoid double-counting / cycles).
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            }
        }
    }
    total
}

/// **P7.T10 — the Reinstall install-start registry transition
/// (`Installed → InProgress`).** The exact symmetric inverse of
/// [`flip_to_installed`]'s state side: one field mutation
/// (`state = ModlistState::InProgress`) + one atomic
/// [`RegistryStore::save`], same logged-no-op edges, same return/style.
///
/// Per SPEC §3.1 ("the modlist state flips to `in-progress` **only when the
/// install starts**, not when the preview opens") this is called at the
/// **Install-click** of the Reinstall route — NOT when the user clicks
/// Reinstall in the Home Kebab (that only opens the confirm + populates the
/// Install-Modlist preview via `reinstall_route::start_reinstall`). The
/// caller (the Install-Modlist Install-click site) is responsible for
/// clearing `pending_reinstall_id`; this fn touches **only** the registry
/// (mirroring `flip_to_installed`'s pure-side-effecting-unit contract — it
/// does not own the in-memory flag).
///
/// Returns `true` iff the entry existed AND the atomic write succeeded
/// (i.e. the transition is durable). Returns `false` — logged, non-fatal,
/// state left untouched — when:
///   - the modlist is not in the registry (a delete raced the Reinstall);
///   - it vanished between the read and the `&mut` borrow;
///   - it was **not** in `Installed` (defensive: only `Installed →
///     InProgress` is valid — Reinstall acts only on installed modlists per
///     SPEC §3.1; a non-`Installed` entry is left as-is and logged);
///   - the atomic `modlists.json` write failed (SPEC §13.14: surface +
///     do not half-flip — the install simply does not begin, the entry
///     stays `Installed`).
///
/// Asymmetry with `flip_to_installed` (intentional, not drift): there is no
/// counts refresh / share-code regenerate / size worker — a Reinstall does
/// **not** change the component set or produce a verified
/// `allow_auto_install = true` code (that is exclusively
/// `flip_to_installed`'s post-success job, SPEC §13.3); the install-start
/// `latest_share_code` / `modlist-import-code.txt` are written by the
/// install-start path (SPEC §13.13), not here. So this is a pure
/// state-only flip + atomic write.
pub fn flip_to_in_progress(
    id: &str,
    registry: &mut ModlistRegistry,
    store: &RegistryStore,
) -> bool {
    let Some(entry) = registry.find_mut(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: modlist {id} not in registry at Reinstall \
             install-start (Reinstall aborted — nothing to flip)"
        );
        return false;
    };

    // Defensive: Reinstall only acts on `Installed` modlists (SPEC §3.1 —
    // the Home Kebab `Reinstall` item only exists on an installed card).
    // A non-`Installed` entry here is an inconsistent state; do NOT flip it
    // (an `InProgress` entry is already where Reinstall would put it; a
    // would-be flip to `InProgress` of something else is meaningless) —
    // log + bail, mirroring `flip_to_installed`'s "vanished/raced" no-op
    // discipline rather than silently mutating an unexpected state.
    if entry.state != ModlistState::Installed {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: modlist {id} is not Installed \
             (state = {:?}); Reinstall flip skipped (only Installed → \
             InProgress is valid — SPEC §3.1)",
            entry.state
        );
        return false;
    }

    entry.state = ModlistState::InProgress;

    // Atomic write. SPEC §13.14: if `modlists.json` is unwritable, do NOT
    // proceed as if flipped — revert the in-memory mutation so the live
    // registry view stays consistent with disk (the entry stays
    // `Installed`; the caller will not have started the install on an
    // `Err`). Symmetric to `flip_to_installed`'s write-failure bail, with
    // the extra in-memory revert because this is a single-field flip with
    // no other durable change to anchor it.
    if let Err(err) = store.save(registry) {
        warn!(
            target = "orchestrator",
            "flip_to_in_progress: atomic registry write for {id} failed: \
             {err} (Reinstall NOT started; entry reverted to Installed — \
             SPEC §13.14)"
        );
        if let Some(entry) = registry.find_mut(id) {
            entry.state = ModlistState::Installed;
        }
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step3ItemState;
    use crate::registry::model::{Game, ModlistEntry};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A unique temp-dir `RegistryStore` path — the DATA-LOSS-safe
    /// precedent (`store.rs::temp_path` / the orchestrator-handoff
    /// invariant). A `flip_to_installed` test MUST NEVER touch the real
    /// `%APPDATA%\bio\modlists.json` (it calls `RegistryStore::save`).
    fn temp_registry_store(label: &str) -> (RegistryStore, std::path::PathBuf) {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!(
                "bio_flip_to_installed_test_{}_{}_{}",
                std::process::id(),
                n,
                label
            ))
            .with_extension("json");
        (RegistryStore::new_with_path(&path), path)
    }

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("comp {id}"),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }
    fn parent(tp: &str) -> Step3ItemState {
        let mut p = leaf(tp, "__PARENT__");
        p.is_parent = true;
        p
    }

    #[test]
    fn counts_skip_parents_and_dedupe_tp_single_game() {
        // Single-game (BGEE) — only `bgee_items` counts (BIO routes IWDEE
        // through the BGEE bucket too; this is the non-EET path).
        let mut s = WizardState::default();
        s.step1.game_install = "BGEE".to_string();
        s.step3.bgee_items = vec![
            parent("EEFIXPACK.TP2"),
            leaf("EEFIXPACK.TP2", "0"),
            leaf("eefixpack.tp2", "2"), // case-insensitive ⇒ same mod
            parent("BG1UB.TP2"),
            leaf("BG1UB.TP2", "0"),
        ];
        // bg2ee_items must be IGNORED for a single-game modlist.
        s.step3.bg2ee_items = vec![leaf("SHOULD_NOT_COUNT.TP2", "0")];
        let (mods, comps) = count_mods_and_components(&s);
        assert_eq!(comps, 3, "3 installable leaves, parents excluded");
        assert_eq!(mods, 2, "2 distinct tp_files (case-insensitive)");
    }

    #[test]
    fn counts_sum_both_tabs_for_eet() {
        // EET installs into BOTH phases — count BGEE + BG2EE leaves.
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![parent("A.TP2"), leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0"), leaf("C.TP2", "0")];
        let (mods, comps) = count_mods_and_components(&s);
        assert_eq!(comps, 4, "2 BGEE leaves + 2 BG2EE leaves");
        assert_eq!(mods, 3, "A + B + C distinct across both tabs");
    }

    #[test]
    fn flip_sets_installed_state_date_counts_and_true_bit_code() {
        let (store, path) = temp_registry_store("happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "FLIPME000001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            // No destination on disk ⇒ the async worker reports 0; the
            // flip itself (state/date/counts/code) must still succeed and
            // a worker receiver is still returned.
            destination_folder: String::new(),
            state: ModlistState::InProgress,
            // install-start snapshot — overwritten by the regeneration.
            latest_share_code: Some("BIO-MODLIST-V1:STALE".to_string()),
            ..Default::default()
        });

        let mut s = WizardState::default();
        // EET (dual-game) + Step-3 leaves on both tabs. BIO's
        // `export_modlist_share_code` (composed read-only by `pack_meta`)
        // builds the share payload's weidu-logs straight from
        // `step3.{bgee,bg2ee}_items` (non-exact-log mode —
        // `build_weidu_export_lines` filters non-parent items), so these
        // leaves are sufficient for a non-empty export (no `state.step2`
        // log fields are read on this path).
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0")];

        // Workspace path: no override ⇒ `pack_meta` regeneration (the
        // existing, UNCHANGED behavior — `state.step3` is populated here).
        let rx = flip_to_installed("FLIPME000001", &mut registry, &store, &s, None);

        let entry = registry.find("FLIPME000001").expect("entry present");
        assert_eq!(entry.state, ModlistState::Installed, "state flipped");
        assert!(entry.install_date.is_some(), "install_date stamped");
        assert_eq!(entry.mod_count, 2, "A + B distinct across both EET tabs");
        assert_eq!(entry.component_count, 3, "2 BGEE leaves + 1 BG2EE leaf");
        assert_eq!(
            entry.total_size_bytes, None,
            "size is None until the async worker reports"
        );
        let code = entry.latest_share_code.as_deref().expect("code");
        assert!(
            code.starts_with("BIO-MODLIST-V1:"),
            "regenerated a BIO-MODLIST-V1 code, not the stale snapshot"
        );
        assert_ne!(
            code, "BIO-MODLIST-V1:STALE",
            "the install-start snapshot was overwritten"
        );
        // Decode + assert allow_auto_install == true (SPEC §13.3 — this is
        // the ONLY true-bit path). `pack_meta`'s own tests prove the
        // envelope; here we just confirm the bit flipped to true through
        // the real `flip_to_installed` path.
        assert!(
            decoded_allow_auto_install(code),
            "flip_to_installed must regenerate with allow_auto_install = true"
        );

        // A worker receiver was returned even with an empty destination
        // (it reports 0 — an honest "nothing measurable", not a perpetual
        // —). Drain it (the worker is quick on an empty path).
        let rx = rx.expect("size worker spawned");
        let (got_id, bytes) = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("worker reports");
        assert_eq!(got_id, "FLIPME000001");
        assert_eq!(bytes, 0, "no destination on disk ⇒ 0 bytes");

        // The atomic write actually hit the temp path (NOT %APPDATA%).
        assert!(path.exists(), "registry written to the temp path");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn flip_on_missing_entry_is_a_logged_noop() {
        let (store, path) = temp_registry_store("missing");
        let mut registry = ModlistRegistry::default();
        let s = WizardState::default();
        let rx = flip_to_installed("DOES_NOT_EXIST", &mut registry, &store, &s, None);
        assert!(
            rx.is_none(),
            "no entry ⇒ None (no worker, no write) — SPEC §13.14 non-fatal"
        );
        // Nothing was written (the early return precedes the save).
        assert!(!path.exists());
    }

    // ───────────────────── flip_to_in_progress (P7.T10) ─────────────────────
    //
    // The Reinstall install-start transition — the symmetric inverse of
    // `flip_to_installed`'s state side. Uses the SAME temp-path
    // `RegistryStore` precedent (DATA-LOSS-safe — `flip_to_in_progress`
    // calls the real `RegistryStore::save`; a test MUST NEVER bind
    // `%APPDATA%\bio\modlists.json`).

    #[test]
    fn flip_to_in_progress_installed_to_inprogress_and_persists() {
        let (store, path) = temp_registry_store("reinstall_happy");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "REINSTALL0001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: String::new(),
            state: ModlistState::Installed,
            // A verified post-success code: Reinstall must NOT touch it
            // here (the install-start path rewrites it per SPEC §13.13;
            // flip_to_in_progress is state-only).
            latest_share_code: Some("BIO-MODLIST-V1:VERIFIED".to_string()),
            mod_count: 9,
            component_count: 136,
            ..Default::default()
        });

        let ok = flip_to_in_progress("REINSTALL0001", &mut registry, &store);
        assert!(ok, "Installed → InProgress flip succeeds + persists");

        let entry = registry.find("REINSTALL0001").expect("entry present");
        assert_eq!(
            entry.state,
            ModlistState::InProgress,
            "state flipped Installed → InProgress (SPEC §3.1)"
        );
        // State-only: counts + the verified share code are untouched (the
        // intentional asymmetry vs flip_to_installed — Reinstall does not
        // change the component set or regenerate the code here).
        assert_eq!(entry.mod_count, 9, "counts untouched (state-only flip)");
        assert_eq!(entry.component_count, 136);
        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:VERIFIED"),
            "the verified code is NOT rewritten by flip_to_in_progress \
             (install-start path owns the code per SPEC §13.13)"
        );
        assert!(
            path.exists(),
            "registry written to the temp path (NOT %APPDATA%)"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn flip_to_in_progress_missing_entry_is_a_logged_noop() {
        let (store, path) = temp_registry_store("reinstall_missing");
        let mut registry = ModlistRegistry::default();
        let ok = flip_to_in_progress("DOES_NOT_EXIST", &mut registry, &store);
        assert!(
            !ok,
            "no entry ⇒ false (no write) — Reinstall aborts, nothing to flip"
        );
        assert!(!path.exists(), "the early return precedes the save");
    }

    #[test]
    fn flip_to_in_progress_non_installed_is_skipped() {
        // Defensive (SPEC §3.1 — Reinstall only acts on Installed cards).
        // An already-`InProgress` entry must NOT be re-flipped + must NOT
        // be persisted (the no-op leaves disk untouched).
        let (store, path) = temp_registry_store("reinstall_notinstalled");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "NOTINSTALLED1".to_string(),
            name: "Draft".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let ok = flip_to_in_progress("NOTINSTALLED1", &mut registry, &store);
        assert!(
            !ok,
            "non-Installed ⇒ false (skipped, only Installed→InProgress)"
        );
        assert_eq!(
            registry.find("NOTINSTALLED1").unwrap().state,
            ModlistState::InProgress,
            "state left untouched"
        );
        assert!(!path.exists(), "a skipped flip does not write the registry");
    }

    #[test]
    fn directory_size_sums_files_recursively() {
        let dir = std::env::temp_dir().join(format!(
            "bio_dirsize_test_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), [0u8; 100]).unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), [0u8; 50]).unwrap();
        assert_eq!(directory_size_bytes(&dir), 150, "100 + 50 recursively");
        assert_eq!(
            directory_size_bytes(&dir.join("does_not_exist")),
            0,
            "non-existent path ⇒ 0 (honest 'nothing measurable')"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ───────── FIX 2 — on-disk modlist-import-code.txt rewrite ─────────
    //
    // SPEC §13.13 (FIX 2 reversal): on a clean exit `flip_to_installed`
    // rewrites `<destination>/modlist-import-code.txt` with the regenerated
    // `allow_auto_install = true` code (matching the registry's
    // `latest_share_code`). Pre-fix the on-disk file stayed the install-
    // start draft forever. `flip_to_installed` is ONLY called on the C3
    // clean-exit edge (the caller's gate, preserved — these tests do not
    // change that), so:
    //   - calling it (clean exit) ⇒ the file IS rewritten with the true-bit
    //     code, byte-equal to the entry's regenerated `latest_share_code`;
    //   - NOT calling it (a non-clean exit — the C3 gate never fires) ⇒ the
    //     install-start draft on disk is UNCHANGED (the rewrite lives
    //     exclusively inside `flip_to_installed`); proven additionally by
    //     the early-`None` paths (regen failure / missing entry) NOT
    //     touching the file.
    // Temp-path `RegistryStore` + temp destination (DATA-LOSS-safe — never
    // `%APPDATA%\bio`).

    /// A unique temp destination dir (DATA-LOSS hygiene — never %APPDATA%).
    fn temp_destination(label: &str) -> std::path::PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bio_flip_dest_test_{}_{}_{}",
            std::process::id(),
            n,
            label
        ))
    }

    /// An EET `WizardState` with Step-3 leaves on both tabs — sufficient
    /// for a non-empty `export_modlist_share_code` (the same construction
    /// the existing happy test uses; no `state.step2` log fields read on
    /// this path).
    fn eet_state_with_leaves() -> WizardState {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.bgee_items = vec![leaf("A.TP2", "0"), leaf("A.TP2", "1")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0")];
        s
    }

    #[test]
    fn flip_to_installed_rewrites_ondisk_import_code_with_true_bit() {
        let (store, store_path) = temp_registry_store("ondisk_rewrite");
        let dest = temp_destination("rewrite");
        std::fs::create_dir_all(&dest).unwrap();
        // Simulate the install-start artifact: the draft
        // (allow_auto_install=false) code already written to disk by
        // `write_install_start_artifacts`.
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);
        std::fs::write(&draft_path, "BIO-MODLIST-V1:INSTALL-START-DRAFT").unwrap();

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "ONDISK000001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            latest_share_code: Some("BIO-MODLIST-V1:STALE".to_string()),
            ..Default::default()
        });
        let s = eet_state_with_leaves();

        let rx = flip_to_installed("ONDISK000001", &mut registry, &store, &s, None);
        assert!(rx.is_some(), "clean-exit flip succeeded");

        // The on-disk file was REWRITTEN (no longer the install-start
        // draft) — SPEC §13.13 FIX 2.
        let on_disk = std::fs::read_to_string(&draft_path).expect("file still present");
        assert_ne!(
            on_disk, "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "the install-start draft on disk must be overwritten on clean exit"
        );
        assert!(
            on_disk.starts_with("BIO-MODLIST-V1:"),
            "rewritten with a BIO-MODLIST-V1 code"
        );
        // It is byte-equal to the registry's regenerated latest_share_code
        // (the SAME verified code — they cannot disagree).
        let entry = registry.find("ONDISK000001").unwrap();
        assert_eq!(
            Some(on_disk.as_str()),
            entry.latest_share_code.as_deref(),
            "on-disk file == registry latest_share_code (the verified code)"
        );
        // And that code carries allow_auto_install = true (FIX 2's whole
        // point — the on-disk file becomes the directly-installable code).
        assert!(
            decoded_allow_auto_install(&on_disk),
            "on-disk modlist-import-code.txt carries allow_auto_install=true \
             on clean exit (SPEC §13.13 FIX 2)"
        );

        // Drain the size worker so the thread is joined before cleanup.
        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_install_modlist_override_uses_held_code_not_pack_meta() {
        // Run 2 (re-scoped) — the Install-Modlist paste / Reinstall path:
        // `share_code_override = Some(the install-start held code)`.
        // `flip_to_installed` must (a) NOT regenerate via `pack_meta` (the
        // WizardState here has NO step3 leaves — `pack_meta` would `Err`;
        // the OLD behavior would then `None` the flip and starve the
        // lifecycle, the exact pinned symptom), (b) bit-flip the held code
        // to `allow_auto_install = true`, (c) carry the held code's
        // baked-in provenance verbatim, (d) write that to the registry +
        // the on-disk file. (`share_export` / `ShareMeta` are in scope via
        // the module-level `use super::*`.)

        let (store, store_path) = temp_registry_store("im_override");
        let dest = temp_destination("im_override");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);

        // The install-start held code: a real BIO-MODLIST-V1 code with
        // `allow_auto_install=false` + baked-in provenance — the exact form
        // `write_install_start_artifacts_with_code` persists from the pasted
        // code. Built via the REAL generate path (`pack_meta` over a
        // populated WizardState + a `ShareMeta` carrying the provenance)
        // then `set_allow_auto_install(.., false)` — i.e. precisely what a
        // pasted code that had once been generated looks like at
        // install-start. (This is just *constructing the fixture*; the
        // production override path never calls `pack_meta`.)
        let provenance = ShareMeta {
            allow_auto_install: false,
            name: Some("Tactical EET 2026".to_string()),
            author: Some("@sharer".to_string()),
            forked_from: vec![crate::app::modlist_share::ForkAncestor {
                name: "Root".to_string(),
                author: "@root".to_string(),
            }],
            archive_meta: vec![],
        };
        let generated = share_export::pack_meta(&eet_state_with_leaves(), &provenance)
            .expect("fixture: pack_meta over a populated WizardState");
        // false-form (install-start) held code — the form persisted by
        // `write_install_start_artifacts_with_code`.
        let held_false = share_export::set_allow_auto_install(&generated, false).unwrap();
        std::fs::write(&draft_path, &held_false).unwrap();

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "IMOVR0000001".to_string(),
            name: "Tactical EET 2026".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            // the install-start snapshot the Install-Modlist path persisted
            latest_share_code: Some(held_false.clone()),
            ..Default::default()
        });
        // Deliberately EMPTY step3 — `pack_meta` would `Err` here. The
        // override path must NOT touch `pack_meta` at all.
        let empty_step3 = WizardState::default();

        let rx = flip_to_installed(
            "IMOVR0000001",
            &mut registry,
            &store,
            &empty_step3,
            Some(held_false.as_str()), // the Install-Modlist override
        );
        assert!(
            rx.is_some(),
            "the Install-Modlist override path flips even with EMPTY step3 \
             (it does NOT regenerate via pack_meta — the pinned symptom fix)"
        );

        let entry = registry.find("IMOVR0000001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed, "state flipped");
        let code = entry.latest_share_code.as_deref().expect("code");
        assert!(decoded_allow_auto_install(code), "true-bit on clean exit");

        // The on-disk file == registry latest_share_code (the verified
        // code) and carries allow_auto_install=true.
        let on_disk = std::fs::read_to_string(&draft_path).unwrap();
        assert_eq!(Some(on_disk.as_str()), entry.latest_share_code.as_deref());
        assert!(decoded_allow_auto_install(&on_disk));

        // Provenance from the PASTED code rode through verbatim (SPEC
        // §13.3 — the real code carries the name; the "Shared modlist"
        // fallback stops). Decode and assert.
        let encoded = on_disk.strip_prefix("BIO-MODLIST-V1:").unwrap();
        let mut vals: Vec<u8> = Vec::new();
        for ch in encoded.chars().filter(|c| !c.is_whitespace()) {
            vals.push(match ch {
                'A'..='Z' => ch as u8 - b'A',
                'a'..='z' => ch as u8 - b'a' + 26,
                '0'..='9' => ch as u8 - b'0' + 52,
                '-' => 62,
                '_' => 63,
                _ => panic!("bad b64url"),
            });
        }
        let mut bytes = Vec::new();
        for chunk in vals.chunks(4) {
            let c0 = chunk[0];
            let c1 = *chunk.get(1).unwrap_or(&0);
            let c2 = *chunk.get(2).unwrap_or(&0);
            let c3 = *chunk.get(3).unwrap_or(&0);
            bytes.push((c0 << 2) | (c1 >> 4));
            if chunk.len() > 2 {
                bytes.push(((c1 & 0x0F) << 4) | (c2 >> 2));
            }
            if chunk.len() > 3 {
                bytes.push(((c2 & 0x03) << 6) | c3);
            }
        }
        let mut json = String::new();
        {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            ZlibDecoder::new(&bytes[..])
                .read_to_string(&mut json)
                .unwrap();
        }
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            v["name"],
            serde_json::json!("Tactical EET 2026"),
            "the pasted code's packed name rode through verbatim"
        );
        assert_eq!(v["author"], serde_json::json!("@sharer"));
        assert_eq!(
            v["forked_from"],
            serde_json::json!([{ "name": "Root", "author": "@root" }]),
            "the pasted code's lineage rode through verbatim (every \
             ancestor stays credited — SPEC §13.3)"
        );

        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_override_verbatim_fallback_on_undecodable_held_code() {
        // The user's priority: persisting the real code beats the
        // false→true draft nicety. A non-decodable held code ⇒ persist it
        // VERBATIM (NOT a `None`/failed flip).
        let (store, store_path) = temp_registry_store("im_verbatim");
        let dest = temp_destination("im_verbatim");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);

        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "IMVERB000001".to_string(),
            name: "Shared modlist".to_string(),
            game: Game::BGEE,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            latest_share_code: Some("BIO-MODLIST-V1:undecodable!!!".to_string()),
            ..Default::default()
        });
        let empty_step3 = WizardState::default();

        let rx = flip_to_installed(
            "IMVERB000001",
            &mut registry,
            &store,
            &empty_step3,
            Some("BIO-MODLIST-V1:undecodable!!!"),
        );
        assert!(
            rx.is_some(),
            "an undecodable held code still flips (verbatim fallback — the \
             real code is the priority; never a None/failed flip)"
        );
        let entry = registry.find("IMVERB000001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed);
        assert_eq!(
            entry.latest_share_code.as_deref(),
            Some("BIO-MODLIST-V1:undecodable!!!"),
            "the held code persisted VERBATIM (the user's priority)"
        );
        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:undecodable!!!",
            "on-disk file == the verbatim held code"
        );
        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn ondisk_import_code_unchanged_on_non_clean_exit() {
        // A non-clean exit ⇒ the orchestrator's C3 gate never calls
        // `flip_to_installed` (the rewrite lives EXCLUSIVELY inside it). So
        // the install-start draft on disk must remain byte-identical. We
        // assert the contract two ways without changing the C3 gate:
        //
        //   (a) NOT calling flip_to_installed leaves the file unchanged
        //       (trivially — nothing else writes it); and
        //   (b) the internal early-`None` paths that a degenerate clean
        //       call can still hit (share-code regeneration failure,
        //       missing entry) ALSO do not touch the on-disk file — so even
        //       if the edge fired on bad state, the draft is preserved.
        let dest = temp_destination("noclean");
        std::fs::create_dir_all(&dest).unwrap();
        let draft_path = dest.join(import_code_writer::IMPORT_CODE_FILENAME);
        std::fs::write(&draft_path, "BIO-MODLIST-V1:INSTALL-START-DRAFT").unwrap();

        // (a) No flip_to_installed call at all (the non-clean-exit reality —
        // the C3 triple did not hold so the orchestrator never invoked it).
        // The file is, of course, still the draft.
        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "no flip call ⇒ on-disk draft untouched (rewrite is only inside \
             flip_to_installed, gated by the caller's C3 triple)"
        );

        // (b) Share-code regeneration failure: `WizardState::default()` has
        // no weidu entries ⇒ `export_modlist_share_code` (inside pack_meta)
        // Errs ⇒ flip_to_installed returns None at step 3 — BEFORE step 5b.
        // The on-disk draft must be untouched (and the registry not
        // flipped).
        let (store, store_path) = temp_registry_store("noclean_regenfail");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "NOCLEAN00001".to_string(),
            name: "Draft".to_string(),
            game: Game::EET,
            destination_folder: dest.to_string_lossy().into_owned(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let empty = WizardState::default(); // ⇒ pack_meta Err
        let rx = flip_to_installed("NOCLEAN00001", &mut registry, &store, &empty, None);
        assert!(
            rx.is_none(),
            "share-code regen failure ⇒ None (no flip, no step 5b)"
        );
        assert_eq!(
            std::fs::read_to_string(&draft_path).unwrap(),
            "BIO-MODLIST-V1:INSTALL-START-DRAFT",
            "an early-None flip_to_installed must NOT rewrite the on-disk \
             draft (step 5b is reached only after a successful regen + save)"
        );
        assert_eq!(
            registry.find("NOCLEAN00001").unwrap().state,
            ModlistState::InProgress,
            "entry not flipped on the regen-failure path"
        );

        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_file(&store_path);
    }

    #[test]
    fn flip_to_installed_empty_destination_skips_ondisk_write_no_panic() {
        // Defensive (mirrors write_install_start_artifacts): an empty
        // destination_folder ⇒ the step-5b rewrite is a logged no-op, never
        // a panic; the state/counts/true-bit-code flip still succeeds and a
        // size worker is still returned (the existing happy test already
        // covers the flip itself with an empty destination; this pins that
        // FIX 2's added on-disk write does not break or panic that path).
        let (store, store_path) = temp_registry_store("empty_dest");
        let mut registry = ModlistRegistry::default();
        registry.entries.push(ModlistEntry {
            id: "EMPTYDEST001".to_string(),
            name: "Polished EET".to_string(),
            game: Game::EET,
            destination_folder: String::new(), // empty ⇒ skip 5b, no panic
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let s = eet_state_with_leaves();

        let rx = flip_to_installed("EMPTYDEST001", &mut registry, &store, &s, None);
        assert!(
            rx.is_some(),
            "the flip still succeeds with an empty destination (5b skipped, \
             not fatal)"
        );
        let entry = registry.find("EMPTYDEST001").unwrap();
        assert_eq!(entry.state, ModlistState::Installed, "state still flipped");
        assert!(
            entry
                .latest_share_code
                .as_deref()
                .is_some_and(|c| c.starts_with("BIO-MODLIST-V1:")),
            "latest_share_code still regenerated (5b skip is independent)"
        );
        let _ = rx.unwrap().recv_timeout(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_file(&store_path);
    }

    /// Test-only inverse of `pack_meta`'s envelope: strip prefix,
    /// base64url-decode, zlib-inflate, read `allow_auto_install`. Uses
    /// `flate2` (already a dep) + the same URL-safe alphabet
    /// `share_export` emits. Confirms the bit `flip_to_installed` set
    /// without needing BIO's private decoder.
    fn decoded_allow_auto_install(code: &str) -> bool {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let encoded = code
            .strip_prefix("BIO-MODLIST-V1:")
            .expect("BIO-MODLIST-V1 prefix");
        // Inverse of `share_export::base64url_encode` (URL-safe, no `=`).
        let mut vals: Vec<u8> = Vec::new();
        for ch in encoded.chars().filter(|c| !c.is_whitespace()) {
            vals.push(match ch {
                'A'..='Z' => ch as u8 - b'A',
                'a'..='z' => ch as u8 - b'a' + 26,
                '0'..='9' => ch as u8 - b'0' + 52,
                '-' => 62,
                '_' => 63,
                _ => panic!("invalid base64url char"),
            });
        }
        let mut bytes = Vec::new();
        for chunk in vals.chunks(4) {
            let c0 = chunk[0];
            let c1 = *chunk.get(1).unwrap_or(&0);
            let c2 = *chunk.get(2).unwrap_or(&0);
            let c3 = *chunk.get(3).unwrap_or(&0);
            bytes.push((c0 << 2) | (c1 >> 4));
            if chunk.len() > 2 {
                bytes.push(((c1 & 0x0F) << 4) | (c2 >> 2));
            }
            if chunk.len() > 3 {
                bytes.push(((c2 & 0x03) << 6) | c3);
            }
        }
        let mut json = String::new();
        ZlibDecoder::new(&bytes[..])
            .read_to_string(&mut json)
            .expect("zlib inflate");
        let v: serde_json::Value = serde_json::from_str(&json).expect("json");
        v["allow_auto_install"].as_bool().expect("bit present")
    }
}
