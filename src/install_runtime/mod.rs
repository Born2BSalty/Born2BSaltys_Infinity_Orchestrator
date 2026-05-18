// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::install_runtime` — Phase 7 net-new orchestrator install-runtime
// module. Composes **around** BIO's existing install pipeline
// (`bio::app::app_step5_flow`, the embedded terminal, the WeiDU child
// process, the auto-answer engine, the diagnostics writer) — it never
// modifies BIO's Step-5 source. Per SPEC §1 CRITICAL DIRECTIVE this whole
// tree is net-new orchestrator code.
//
// **Per-run module registration.** Per the Phase-7 plan + the Run-1
// dispatch brief, submodules are declared in the run that implements them
// (the brief: "Only declare/create the submodules Run 1 needs; later-run
// modules are added in their runs — do not stub the whole inventory").
// Run 1 (Step-5 runtime spine + workspace chrome) needs **no**
// `install_runtime` submodule (its only `install_runtime` requirement is
// the module's existence + registration per P7.T1), so this file declares
// none yet. The eventual inventory (added in their runs) per the plan's
// File inventory:
//   - Run 2 (P7.T3 / T9 / T9b): `import_code_writer`, `start_hooks`,
//     `registry_transition`, `install_concurrency`, `rail_lock_reason`,
//     `flag_policies`.
//   - Run 4 (P7.T17 / T10): `per_install_dirs`, `archive_store`,
//     `auto_build_driver`, `reinstall_route`.
//
// SPEC: §9, §13.12a, §13.13, §13.15, §1 CRITICAL DIRECTIVE.

// ── Run 2 (P7.T3 / T9 / T9b / T16) submodules. Per the per-run module-
//    registration discipline, only the submodules a run implements are
//    declared in that run. Run 2 = install-start hooks + concurrency gate
//    + C5 rail-lock reason + flag policies. ──
//
//   - `import_code_writer`   → P7.T3: `modlist-import-code.txt` write
//                              (SPEC §13.13). std::fs only.
//   - `start_hooks`          → P7.T3: `on_install_start` (flag policies →
//                              `pack_meta` share code → import-code write
//                              → `install_started_at`; SPEC §9.4/§13.13/
//                              §13.3). Composes BIO read-only.
//   - `registry_transition`  → declared now (the plan's Run-2 inventory);
//                              transition fns land in their owning runs
//                              (`flip_to_installed` = Run 3 P7.T6,
//                              `flip_to_in_progress` = Run 4b P7.T10).
//   - `install_concurrency`  → P7.T9: `install_in_progress` (SPEC §13.15
//                              single-install gate — powers the per-button
//                              gate AND the C5 rail lock).
//   - `rail_lock_reason`     → P7.T9b: the C5 `RailLockReason` + the
//                              verbatim SPEC §13.15 rail tooltip.
//   - `flag_policies`        → P7.T16: SPEC §13.12 #1 (`-s`/`-c`) + #5
//                              (`--download`) automatic flag policy.
//
// Run 4 (P7.T17 / T10) adds `per_install_dirs` / `archive_store` /
// `auto_build_driver` / `reinstall_route` in its run.
//
// ── Run 4a (P7.T17) submodules — the live download/extract/import
//    pipeline (SPEC §13.12a). Per the per-run module-registration
//    discipline, only the submodules this run implements are declared
//    here:
//      - `per_install_dirs`   → P7.T17 piece 1: derive the per-install
//                               Mods + #2/#3/#4 clone dirs INSIDE the
//                               destination + force the clone flags
//                               (SPEC §13.12a / §13.12 #2/#3/#4).
//      - `archive_store`      → P7.T17 piece 2: the net-new
//                               content-addressed staging layer that
//                               WRAPS BIO's reused-unchanged
//                               `app_step2_update_download` / `_extract`
//                               (SPEC §13.12a — dedupe / coexist /
//                               extract-by-hash). Zero BIO edit.
//      - `auto_build_driver`  → P7.T17 piece 3: drive BIO's import →
//                               auto-build pipeline read-only
//                               (`import_modlist_share_code` + the
//                               saved-log/auto-build flow via the
//                               orchestrator-owned receivers — SPEC
//                               §13.12a pipeline-reuse contract). Zero
//                               BIO edit.
// ── Run 4b (P7.T10) submodule — the Reinstall route (SPEC §3.1). Per the
//    per-run module-registration discipline, declared in the run that
//    implements it:
//      - `reinstall_route`    → P7.T10: `start_reinstall` — populate the
//                               Install-Modlist preview from the stored
//                               code + force overwrite-install +
//                               `pending_reinstall_id` + navigate to the
//                               Preview stage. **No registry flip** (that
//                               is the Install-click site's job via
//                               `registry_transition::flip_to_in_progress`).
//                               Composes BIO read-only. ──
pub mod archive_store;
pub mod auto_build_driver;
pub mod flag_policies;
pub mod import_code_writer;
pub mod install_concurrency;
pub mod per_install_dirs;
pub mod rail_lock_reason;
pub mod registry_transition;
pub mod reinstall_route;
pub mod start_hooks;
