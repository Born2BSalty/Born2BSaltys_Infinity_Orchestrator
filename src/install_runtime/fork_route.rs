// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

pub(crate) fn extract_complete_route_to_workspace(orchestrator: &mut OrchestratorApp, id: String) {
    let name = orchestrator
        .registry
        .find(&id)
        .map_or_else(|| "modlist".to_string(), |e| e.name.clone());
    orchestrator
        .notification_manager
        .success(format!("Imported \"{name}\" \u{2014} ready to edit"));

    orchestrator.reset_install_screen_to_gallery();
    orchestrator.create_screen_state.modlist_name.clear();
    orchestrator.create_screen_state.destination.clear();
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.create_screen_state.resumed_build_id = Some(id.clone());
    orchestrator.nav = NavDestination::Workspace {
        modlist_id: Some(id),
    };
}
