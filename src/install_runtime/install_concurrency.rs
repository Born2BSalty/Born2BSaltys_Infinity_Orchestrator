// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

#[derive(Debug, Clone)]
pub struct RunningInstall {
    pub modlist_id: String,

    pub started_at: Instant,
}

#[must_use]
pub fn install_in_progress(orchestrator: &OrchestratorApp) -> Option<RunningInstall> {
    if !orchestrator.wizard_state.step5.install_running {
        return None;
    }
    let modlist_id = orchestrator
        .workspace_view
        .loaded_workspace_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| orchestrator.workspace_view.modlist_id.clone());
    Some(RunningInstall {
        modlist_id,
        started_at: orchestrator
            .install_running_since
            .unwrap_or_else(Instant::now),
    })
}

#[must_use]
pub fn per_button_gate_tooltip(running_modlist_name: &str) -> String {
    format!(
        "An install is already running for {running_modlist_name}. \
         Wait for it to finish before starting another."
    )
}
