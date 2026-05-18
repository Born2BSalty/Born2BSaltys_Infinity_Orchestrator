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
//     **P7.T10 (Run 4b)** and called from `start_hooks::on_install_start`'s
//     reinstall branch — left as a deferred placeholder below (Run 3
//     implements `flip_to_installed` ONLY).
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
// **`pack_meta` composes BIO read-only** — `flip_to_installed` is the ONLY
// Infinity-Orchestrator code path that produces an `allow_auto_install =
// true` code (SPEC §13.3). It NEVER patches `bio::app::modlist_share`
// (carve-out #5 "generation is not a BIO modification"). The on-disk
// `modlist-import-code.txt` is **not** rewritten on success (SPEC §13.13
// closing paragraph) — it remains the install-start `allow_auto_install =
// false` artifact; only the registry's `latest_share_code` is regenerated.
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
///   3. Regenerate `latest_share_code` via `registry::share_export
///      ::pack_meta` with **`allow_auto_install = true`** (SPEC §13.3 — the
///      only auto-install-eligible code path), provenance read off the
///      entry. This OVERWRITES the install-start `allow_auto_install =
///      false` snapshot. The on-disk `modlist-import-code.txt` is **NOT**
///      rewritten (SPEC §13.13 closing para).
///   4. `total_size_bytes = None` (the async worker fills it later).
///   5. Atomic registry write (`RegistryStore::save`).
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
) -> Option<SizeWorkerReceiver> {
    // ── 2. Counts from the live WizardState (same source as Step 4 —
    //    computed BEFORE the &mut entry borrow so the immutable
    //    `wizard_state` read does not overlap). ──
    let (mod_count, component_count) = count_mods_and_components(wizard_state);

    // ── 3. Regenerate the share code with allow_auto_install = TRUE
    //    (SPEC §13.3 — flip_to_installed is the ONLY true-bit path).
    //    Provenance off the entry; the BIO base is composed read-only.
    //    Done while only an immutable `registry` borrow is live (the
    //    &mut entry borrow starts after). ──
    let Some(entry_ref) = registry.find(id) else {
        warn!(
            target = "orchestrator",
            "flip_to_installed: modlist {id} not in registry on clean exit \
             (install completed; registry flip skipped — SPEC §13.14)"
        );
        return None;
    };
    let destination = entry_ref.destination_folder.trim().to_string();
    let meta = ShareMeta::from_entry(entry_ref, /* allow_auto_install */ true);
    let new_code = match share_export::pack_meta(wizard_state, &meta) {
        Ok(code) => code,
        Err(err) => {
            // SPEC §13.14: the install itself completed; only the
            // post-success code regeneration failed. Log + bail (do NOT
            // half-flip the state). The install-start `latest_share_code`
            // (allow_auto_install = false) stays as-is.
            warn!(
                target = "orchestrator",
                "flip_to_installed: share-code regeneration for {id} failed: \
                 {err} (registry NOT flipped — install already completed)"
            );
            return None;
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

// ── P7.T10 (Run 4b): `flip_to_in_progress(id, registry, store)` — the
//    Reinstall install-start transition (`Installed → InProgress`), called
//    from `start_hooks::on_install_start`'s reinstall branch when
//    `pending_reinstall_id == Some(id)`. Out of THIS run's scope (Run 3
//    implements `flip_to_installed` ONLY); `reinstall_route` /
//    `pending_reinstall_id` land in Run 4b. Deliberately left as this
//    documented placeholder rather than a half-implemented stub — the same
//    per-run discipline the module used before this run. ──

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

        let rx = flip_to_installed("FLIPME000001", &mut registry, &store, &s);

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
        let rx = flip_to_installed("DOES_NOT_EXIST", &mut registry, &store, &s);
        assert!(
            rx.is_none(),
            "no entry ⇒ None (no worker, no write) — SPEC §13.14 non-fatal"
        );
        // Nothing was written (the early return precedes the save).
        assert!(!path.exists());
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
