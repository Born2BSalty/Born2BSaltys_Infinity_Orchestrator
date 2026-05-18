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

// Intentionally empty in Run 1 — see the module-level note above. Later
// runs add their `pub mod <name>;` lines here.
