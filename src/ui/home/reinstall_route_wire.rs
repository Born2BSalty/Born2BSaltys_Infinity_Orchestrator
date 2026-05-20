// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use tracing::warn;

use crate::install_runtime::reinstall_route;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub fn confirm_reinstall(orchestrator: &mut OrchestratorApp, id: &str) {
    let Some(entry) = orchestrator.registry.find(id).cloned() else {
        warn!(
            target = "orchestrator",
            "Reinstall confirmed for {id} but the entry is no longer in the \
             registry (deleted between confirm-arm and confirm) — no-op"
        );
        return;
    };
    reinstall_route::start_reinstall(&entry, orchestrator);
}
