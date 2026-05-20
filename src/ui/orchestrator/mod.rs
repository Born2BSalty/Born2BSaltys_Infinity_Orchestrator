// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
