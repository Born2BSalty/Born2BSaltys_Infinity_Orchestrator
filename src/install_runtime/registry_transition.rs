// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::registry_transition` — the modlist registry
// state-transition helpers (SPEC §9.2 / §13.1 / §3.1).
//
// Net-new orchestrator code. The two transitions:
//   - `flip_to_installed(id, stats, registry, store, wizard_state)` — the
//     post-success transition (`InProgress → Installed`, set
//     `install_date`, refresh counts, **regenerate `latest_share_code`
//     with `allow_auto_install = true` via `registry::share_export
//     ::pack_meta`**, async size worker). Owned by **P7.T6 (Run 3)**.
//   - `flip_to_in_progress(id, registry, store)` — the Reinstall
//     install-start transition (`Installed → InProgress`). Owned by
//     **P7.T10 (Run 4b)** and called from `start_hooks::on_install_start`'s
//     reinstall branch.
//
// **Run 2 (this run) scope.** Per the dispatch brief, Run 2 is install-
// start + concurrency + C5 + flags + statusbar. The registry *transitions*
// are explicitly **out of Run-2 scope** (P7.T6 = Run 3 success path;
// P7.T10 = Run 4b reinstall). The module is registered now (the plan's
// `install_runtime/mod.rs` Run-2 inventory lists `registry_transition`, and
// `start_hooks` references the Reinstall flip as a commented Run-4b
// placeholder), but — mirroring Run 1's "only declare/create what the run
// needs; later-run code is added in its run" discipline — the transition
// functions are deliberately deferred to their owning runs rather than
// stubbed half-implemented here.
//
// SPEC: §9.2, §13.1, §3.1.

// Intentionally no transition functions in Run 2 — see the module note
// above. `flip_to_installed` lands in Run 3 (P7.T6); `flip_to_in_progress`
// in Run 4b (P7.T10). Registering the module now keeps the
// `install_runtime` inventory stable across runs (the same pattern Run 1
// used for `install_runtime/mod.rs` itself).
