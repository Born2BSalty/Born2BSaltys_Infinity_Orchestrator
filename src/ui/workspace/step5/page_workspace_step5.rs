// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::install_runtime::install_concurrency;
use crate::install_runtime::start_hooks::{self, InstallButtonVariant};
use crate::registry::operations;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::workspace::step5::state_workspace_step5::PostInstallAction;
use crate::ui::workspace::step5::{post_install_actions, share_paste_code_dialog, success_banner};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, modlist_id: &str) {
    if orchestrator.workspace_step5.install_clicked
        && orchestrator.workspace_view.loaded_workspace_id.as_deref()
            != Some(orchestrator.workspace_view.modlist_id.as_str())
    {
        orchestrator.workspace_step5.reset_for_modlist();
    }

    let palette = orchestrator.theme_palette;

    let entry = orchestrator.registry.find(modlist_id).cloned();

    if let Some(e) = entry.as_ref() {
        success_banner::render(ui, palette, &orchestrator.wizard_state, e);
    }

    let post_install_action: Option<PostInstallAction> = entry
        .as_ref()
        .and_then(|e| post_install_actions::render(ui, palette, &orchestrator.wizard_state, e));

    let exe_fingerprint = orchestrator.exe_fingerprint.clone();
    let action: Option<Step5Action> = crate::ui::step5::page_step5::render(
        ui,
        &mut orchestrator.wizard_state,
        &mut orchestrator.step5_console_view,
        orchestrator.step5_terminal.as_mut(),
        orchestrator.step5_terminal_error.as_deref(),
        orchestrator.dev_mode,
        &exe_fingerprint,
    );

    if action == Some(Step5Action::StartInstall) && !handle_start_install(orchestrator, modlist_id)
    {
        return;
    }

    match post_install_action {
        Some(PostInstallAction::ReturnToHome) => {
            orchestrator.nav = NavDestination::Home;
        }
        Some(PostInstallAction::OpenInstallFolder) => {
            if let Some(e) = entry.as_ref()
                && let Err(msg) = operations::open_install_folder(e)
            {
                orchestrator.home_screen_state.toast =
                    Some(crate::ui::home::state_home::ToastMessage::error(msg));
            }
        }
        None => {}
    }

    if orchestrator.workspace_step5.share_dialog_open {
        let ctx = ui.ctx().clone();
        let entry_for_dialog = entry.unwrap_or_default();
        share_paste_code_dialog::render(
            &ctx,
            palette,
            &mut orchestrator.workspace_step5,
            &entry_for_dialog,
        );
    }
}

fn handle_start_install(orchestrator: &mut OrchestratorApp, modlist_id: &str) -> bool {
    orchestrator.workspace_step5.install_clicked = true;

    if let Some(running) = install_concurrency::install_in_progress(orchestrator)
        && running.modlist_id != modlist_id
    {
        let running_name = orchestrator
            .registry
            .find(&running.modlist_id)
            .map_or_else(|| running.modlist_id.clone(), |e| e.name.clone());
        warn!(
            target = "orchestrator",
            "Install refused for {modlist_id}: {}",
            install_concurrency::per_button_gate_tooltip(&running_name)
        );
        return false;
    }

    let variant = InstallButtonVariant::from_step5(&orchestrator.wizard_state, false);

    let workflow = orchestrator
        .registry
        .find(modlist_id)
        .filter(|e| !e.forked_from.is_empty())
        .map_or(InstallWorkflow::FreshCreate, |_| {
            InstallWorkflow::ShareCodeConsuming
        });

    let settings: crate::settings::model::Step1Settings =
        orchestrator.wizard_state.step1.clone().into();

    let OrchestratorApp {
        wizard_state,
        registry,
        registry_store,
        ..
    } = &mut *orchestrator;

    match start_hooks::on_install_start(
        modlist_id,
        variant,
        workflow,
        wizard_state,
        registry,
        registry_store,
        &settings,
    ) {
        Ok(()) => {
            orchestrator.wizard_state.step5.start_install_requested = true;
        }
        Err(err) => {
            warn!(
                target = "orchestrator",
                "install-start hook failed for {modlist_id}: {err} \
                 (install not started)"
            );
        }
    }

    true
}
