// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Infinity Orchestrator — module tree for the standalone `eframe::App`
// powering the new binary. Lives inside the library crate at
// `bio::ui::orchestrator::*` per the Phase 1 carve-out #3 split, so
// `pub(crate)` BIO items (e.g. `bio::app::app_bootstrap_init::initialize`)
// remain reachable from orchestrator code.
//
// Phase 2 ships:
//   - `OrchestratorApp` (the `eframe::App` impl)
//   - `NavDestination` (Home / Install / Create / Settings / Workspace)
//   - `LeftRail` widget (brand mark + 4 nav items + status indicator)
//   - `page_router` (per-destination dispatch)
//   - 5 stub destination pages
//   - 4 primitive redesign widgets: `ScreenTitle`, `Btn`, `Box`, `Label`
//   - `nav_status` (read-only path-validation summary helper)
//
// Subsequent phases extend `OrchestratorApp`'s field set + the widget surface;
// no edits to existing BIO source beyond the Phase 1 carve-out #3 split.

pub mod left_rail;
pub mod nav_destination;
pub mod nav_status;
pub mod orchestrator_app;
pub mod page_router;
pub mod registry_error_panel;
pub mod stubs;
pub mod widgets;

pub use nav_destination::NavDestination;
pub use nav_status::{PathValidationKind, PathValidationSummary};
pub use orchestrator_app::OrchestratorApp;
