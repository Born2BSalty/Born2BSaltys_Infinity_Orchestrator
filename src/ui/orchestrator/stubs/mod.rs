// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Stub destination pages for Phase 2. Each stub renders a `ScreenTitle` + one
// faint sub-line that names the phase that builds it out.
//
// Per Phase 2 P2.T5:
//   - Home     → title "Welcome back, adventurer",  sub "Coming in Phase 5 — SPEC §3"
//   - Install  → title "Install shared modlist",    sub "Coming in Phase 5 — SPEC §4"
//   - Create   → title "Create / edit modlist",     sub "Coming in Phase 6 — SPEC §5"
//   - Settings → title "Settings",                  sub "Coming in Phase 4 — SPEC §11"
//   - Workspace → title "Workspace",                sub "Coming in Phase 6 — SPEC §2.2"
//
// The Home stub also renders a dev-mode-only `Open workspace stub (dev)`
// button that flips `orchestrator.nav` to `NavDestination::Workspace`. The
// button is a Phase 2 scaffolding affordance only; Phase 5 removes it.
//
// SPEC: §2.1 (each phase number above).

pub mod create_stub;
pub mod home_stub;
pub mod install_stub;
pub mod settings_stub;
pub mod workspace_stub;

pub use create_stub::render_create_stub;
pub use home_stub::render_home_stub;
pub use install_stub::render_install_stub;
pub use settings_stub::render_settings_stub;
pub use workspace_stub::render_workspace_stub;
