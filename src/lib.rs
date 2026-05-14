// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio` library crate root.
//
// Per SPEC §1 CRITICAL DIRECTIVE carve-out #3, the project is restructured
// from its original single-binary layout (everything declared in `main.rs`)
// into a `lib + 2 bins` layout. The module tree below is identical to
// today's `src/main.rs` `mod` block, but with `pub` visibility — so both
// binaries (`BIO` and `infinity_orchestrator`) link against this library and
// reach BIO's public surface as `bio::*`.
//
// `pub(crate)` items inside the library stay reachable to other code that
// lives inside the same crate (notably the orchestrator's modules under
// `src/ui/orchestrator/` introduced in Phase 2). The orchestrator binary's
// `main.rs` is a thin shim into this library.
//
// **Behavior is preserved bit-for-bit** vs. today's BIO — no logic changes
// in any file the carve-out touches.

#[path = "core/app/mod.rs"]
pub mod app;
#[path = "core/cli/mod.rs"]
pub mod cli;
#[path = "core/config/mod.rs"]
pub mod config;
#[path = "core/install/mod.rs"]
pub mod install;
#[path = "core/logging/mod.rs"]
pub mod logging;
#[path = "core/mods/mod.rs"]
pub mod mods;
#[path = "core/parser/mod.rs"]
pub mod parser;
#[path = "core/platform_defaults.rs"]
pub mod platform_defaults;
pub mod settings;
pub mod ui;

// Phase 3 — Modlist registry. New top-level module under the orchestrator's
// data layer. Registered here under the CRITICAL DIRECTIVE carve-out #3's
// companion provision (additive `pub mod` lines for orchestrator-owned data).
pub mod registry;

// Redesign modules (Infinity Orchestrator) are registered in their natural
// places in the BIO module tree per CRITICAL DIRECTIVE carve-out #3's
// companion provision (additive `pub mod` lines in existing `mod.rs` files):
//   - `bio::ui::shared::redesign_fonts` (registered in `src/ui/shared/mod.rs`)
//   - `bio::ui::shared::redesign_tokens` (registered in `src/ui/shared/mod.rs`)
//   - `bio::ui::shell` (registered in `src/ui/mod.rs`)
// No lib-root re-exports are needed.
