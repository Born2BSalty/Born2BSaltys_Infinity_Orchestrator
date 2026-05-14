// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign shell chrome modules — used by the `infinity_orchestrator` binary.
//
// Per Phase 1 P1.T4-T6: these modules exist and compile in Phase 1 but are
// not yet invoked anywhere. Phase 2 wires them into `OrchestratorApp::update`.

pub mod shell_chrome;
pub mod shell_statusbar;
pub mod shell_titlebar;
