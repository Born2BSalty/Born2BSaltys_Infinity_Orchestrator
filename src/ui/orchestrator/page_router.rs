// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::registry::model::ModlistRegistry;
use crate::ui::create::page_create;
use crate::ui::create::state_create::{CreateAction, CreateScreenState};
use crate::ui::home::page_home;
use crate::ui::home::state_home::HomeScreenState;
use crate::ui::install::page_install;
use crate::ui::install::stage_installing::InstallStep5Runtime;
use crate::ui::install::state_install::{InstallAction, InstallScreenState};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::stubs;
use crate::ui::settings::page_settings;
use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::tab_accounts::AccountAction;
use crate::ui::settings::tab_paths::PathsAction;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::state_step5::Step5ConsoleViewState;
use crate::ui::workspace::state_workspace::WorkspaceViewState;
use crate::ui::workspace::workspace_view;

pub enum PageAction {
    Navigate(NavDestination),
    SeedTestModlist,
    ConnectGitHub,
    DisconnectGitHub,
    ValidatePathsNow,
    Home(page_home::HomeAction),
    Create(CreateAction),
    Install(InstallAction),
}

pub struct PageRouterContext<'a> {
    pub nav: &'a NavDestination,
    pub dev_mode: bool,
    pub dev_seed_message: Option<&'a str>,
    pub home_state: &'a mut HomeScreenState,
    pub create_state: &'a mut CreateScreenState,
    pub install_state: &'a mut InstallScreenState,
    pub registry: Option<&'a ModlistRegistry>,
    pub wizard_state: &'a mut WizardState,
    pub settings_state: &'a mut SettingsScreenState,
    pub workspace_state: &'a mut WorkspaceViewState,
    pub step5_console_view: &'a mut Step5ConsoleViewState,
    pub step5_terminal: Option<&'a mut EmbeddedTerminal>,
    pub step5_terminal_error: Option<&'a str>,
    pub exe_fingerprint: &'a str,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    context: PageRouterContext<'_>,
) -> Option<PageAction> {
    match context.nav {
        NavDestination::Home => {
            let _ = (context.dev_mode, context.dev_seed_message);
            page_home::render(
                ui,
                palette,
                context.home_state,
                context.registry,
                context.wizard_state,
            )
            .map(PageAction::Home)
        }
        NavDestination::Install => page_install::render(
            ui,
            palette,
            context.install_state,
            context.wizard_state,
            InstallStep5Runtime {
                console_view: context.step5_console_view,
                terminal: context.step5_terminal,
                terminal_error: context.step5_terminal_error,
            },
            context.dev_mode,
            context.exe_fingerprint,
        )
        .map(PageAction::Install),
        NavDestination::Create => {
            page_create::render(ui, palette, context.create_state).map(PageAction::Create)
        }
        NavDestination::Settings => page_settings::render(
            ui,
            palette,
            context.settings_state,
            context.wizard_state.github_auth_login.as_str(),
        )
        .map(|action| match action {
            page_settings::SettingsAction::Account(AccountAction::ConnectGitHub) => {
                PageAction::ConnectGitHub
            }
            page_settings::SettingsAction::Account(AccountAction::DisconnectGitHub) => {
                PageAction::DisconnectGitHub
            }
            page_settings::SettingsAction::Paths(PathsAction::ValidatePathsNow) => {
                PageAction::ValidatePathsNow
            }
        }),
        NavDestination::Workspace { modlist_id } => {
            if modlist_id.is_some() {
                workspace_view::render(
                    ui,
                    palette,
                    context.workspace_state,
                    context.wizard_state,
                    context.dev_mode,
                    context.exe_fingerprint,
                );
                None
            } else {
                stubs::render_workspace_stub(ui, palette);
                None
            }
        }
    }
}
