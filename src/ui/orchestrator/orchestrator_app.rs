// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use chrono::Utc;
use eframe::egui;

use crate::app::app_step1_github_oauth::GitHubOAuthFlowResult;
use crate::app::controller::util::open_in_shell;
use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::registry::dev_seed;
use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistState;
use crate::registry::model::{ModlistEntry, ModlistRegistry};
use crate::registry::persistence_cycle::RegistryPersistenceCycle;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::settings::model::Step1Settings;
use crate::settings::redesign_fields::RedesignSettings;
use crate::settings::redesign_store::RedesignSettingsStore;
use crate::settings::store::SettingsStore;
use crate::ui::create::state_create::{CreateAction, CreateScreenState};
use crate::ui::home::modlist_card::ModlistCardAction;
use crate::ui::home::page_home;
use crate::ui::home::state_home::HomeScreenState;
use crate::ui::install::state_install::{
    InstallAction, InstallPreviewTab, InstallScreenState, InstallStage,
};
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_rail;
use crate::ui::orchestrator::nav_status;
use crate::ui::orchestrator::page_router;
use crate::ui::orchestrator::registry_error_panel;
use crate::ui::settings::oauth_glue;
use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::settings::validate_debounce;
use crate::ui::shared::redesign_tokens::{REDESIGN_NAV_WIDTH_PX, ThemePalette, redesign_shell_bg};
use crate::ui::shell::shell_chrome;
use crate::ui::step1::action_step1::Step1Action;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::state_step5::Step5ConsoleViewState;
use crate::ui::workspace::state_workspace::WorkspaceViewState;
use crate::ui::workspace::step5::page_workspace_step5::{
    WorkspaceStep5Action, WorkspaceStep5Runtime, WorkspaceStep5SuccessInfo,
};
use crate::ui::workspace::workspace_state_loader;
use crate::ui::workspace::workspace_view::{self, WorkspaceRuntimeOptions};

const INSTALL_NAV_LOCK_TOOLTIP: &str =
    "An install is in progress — cancel or wait for completion before navigating.";
const MODLIST_IMPORT_CODE_FILE_NAME: &str = "modlist-import-code.txt";

#[derive(Debug, Clone, Default)]
struct CurrentWorkspaceEntrySummary {
    mod_count: usize,
    component_count: usize,
    latest_share_code: Option<String>,
}

pub struct OrchestratorApp {
    nav: NavDestination,
    wizard_state: WizardState,
    settings_store: SettingsStore,
    original_cli_dev_mode: bool,
    dev_mode: bool,
    exe_fingerprint: String,
    theme_palette: ThemePalette,
    redesign_settings_store: RedesignSettingsStore,
    redesign_settings: RedesignSettings,
    registry_store: RegistryStore,
    registry: Option<ModlistRegistry>,
    registry_error: Option<RegistryError>,
    registry_backup_path: Option<PathBuf>,
    registry_persistence: Option<RegistryPersistenceCycle>,
    dev_seed_message: Option<String>,
    home_screen_state: HomeScreenState,
    pending_clipboard_text: Option<String>,
    create_screen_state: CreateScreenState,
    install_screen_state: InstallScreenState,
    settings_screen_state: SettingsScreenState,
    workspace_view_state: WorkspaceViewState,
    current_workspace: Option<ModlistWorkspaceState>,
    current_workspace_store: Option<WorkspaceStore>,
    github_auth_rx: Option<Receiver<GitHubOAuthFlowResult>>,
    step2_scan_rx: Option<Receiver<Step2ScanEvent>>,
    step2_cancel: Option<Arc<AtomicBool>>,
    step2_progress_queue: VecDeque<(usize, usize, String)>,
    step2_update_check_rx:
        Option<Receiver<crate::app::app_step2_update_check_worker::Step2UpdateCheckEvent>>,
    step2_update_download_rx:
        Option<Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent>>,
    step2_update_extract_rx:
        Option<Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>>,
    step5_terminal: Option<EmbeddedTerminal>,
    step5_terminal_error: Option<String>,
    step5_console_view: Step5ConsoleViewState,
    step5_prep_rx: Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_pending_start: Option<PendingInstallStart>,
}

struct ShellRenderState {
    modlist_count: usize,
    install_runtime_busy: bool,
    clean_install_success: bool,
    current_workspace_entry: CurrentWorkspaceEntrySummary,
}

impl OrchestratorApp {
    #[must_use]
    pub fn new(dev_mode: bool) -> Self {
        let bootstrap = crate::app::app_bootstrap_init::initialize(dev_mode);
        let mut wizard_state = WizardState {
            step1: bootstrap.step1,
            github_auth_login: bootstrap.github_auth_login,
            ..Default::default()
        };
        wizard_state.step1_path_check = Some(crate::app::state_validation::run_path_check(
            &wizard_state.step1,
        ));
        let redesign_settings_store = RedesignSettingsStore::new_default();
        let redesign_settings = redesign_settings_store.load();
        let runtime_dev_mode = dev_mode || redesign_settings.diagnostic_mode;
        let registry_store = RegistryStore::new_default();
        let (registry, registry_error, registry_backup_path, registry_persistence) =
            match registry_store.load() {
                Ok(registry) => {
                    let persistence = RegistryPersistenceCycle::new(registry.clone());
                    (Some(registry), None, None, Some(persistence))
                }
                Err(err @ RegistryError::Corrupt { .. }) => {
                    let backup_path = registry_store.backup_corrupt_file().ok();
                    (None, Some(err), backup_path, None)
                }
                Err(err) => (None, Some(err), None, None),
            };

        let mut settings_screen_state =
            SettingsScreenState::from_redesign_settings(&redesign_settings);
        initialize_settings_paths_from_step1(&mut settings_screen_state, &wizard_state.step1);

        Self {
            nav: NavDestination::Home,
            wizard_state,
            settings_store: bootstrap.settings_store,
            original_cli_dev_mode: dev_mode,
            dev_mode: runtime_dev_mode,
            exe_fingerprint: bootstrap.exe_fingerprint,
            theme_palette: redesign_settings.theme_palette,
            redesign_settings_store,
            redesign_settings,
            registry_store,
            registry,
            registry_error,
            registry_backup_path,
            registry_persistence,
            dev_seed_message: None,
            home_screen_state: HomeScreenState::default(),
            pending_clipboard_text: None,
            create_screen_state: CreateScreenState::default(),
            install_screen_state: InstallScreenState::default(),
            settings_screen_state,
            workspace_view_state: WorkspaceViewState::default(),
            current_workspace: None,
            current_workspace_store: None,
            github_auth_rx: None,
            step2_scan_rx: None,
            step2_cancel: None,
            step2_progress_queue: VecDeque::new(),
            step2_update_check_rx: None,
            step2_update_download_rx: None,
            step2_update_extract_rx: None,
            step5_terminal: None,
            step5_terminal_error: None,
            step5_console_view: Step5ConsoleViewState::default(),
            step5_prep_rx: None,
            step5_pending_start: None,
        }
    }

    fn flush_registry_state(&mut self) {
        let _ = self.flush_current_workspace_state();
        let (Some(registry), Some(persistence)) =
            (self.registry.as_ref(), self.registry_persistence.as_mut())
        else {
            return;
        };

        let workspaces: HashMap<String, ModlistWorkspaceState> =
            current_workspace_map(&self.workspace_view_state, self.current_workspace.as_ref());
        let workspace_stores: HashMap<String, WorkspaceStore> = current_workspace_store_map(
            &self.workspace_view_state,
            self.current_workspace_store.as_ref(),
        );
        if let Err(err) = persistence.flush_all(
            registry,
            &self.registry_store,
            &workspaces,
            &workspace_stores,
        ) {
            eprintln!("Failed to flush modlist registry on shutdown: {err}");
        }
    }

    fn poll_update_before_render(&mut self) -> bool {
        oauth_glue::poll_github_flow(&mut self.wizard_state, &mut self.github_auth_rx);
        self.sync_settings_paths_to_step1();
        if validate_debounce::tick(&mut self.settings_screen_state, std::time::Instant::now()) {
            self.wizard_state.step1_path_check = Some(
                crate::app::state_validation::run_path_check(&self.wizard_state.step1),
            );
        }
        let install_was_running = self.wizard_state.step5.install_running;
        let step5_requested_repaint = crate::app::app_update_cycle::poll_before_render(
            &mut self.wizard_state,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
            &mut self.step2_update_check_rx,
            &mut self.step2_update_download_rx,
            &mut self.step2_update_extract_rx,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
            &mut self.step5_prep_rx,
            &mut self.step5_pending_start,
        );
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
        }
        step5_requested_repaint
    }

    fn render_orchestrator_shell(
        &mut self,
        ctx: &egui::Context,
        nav_status: &nav_status::PathValidationSummary,
        shell_state: &ShellRenderState,
    ) {
        let rail_locked_tooltip = shell_state
            .install_runtime_busy
            .then_some(INSTALL_NAV_LOCK_TOOLTIP);
        shell_chrome::render_shell(
            ctx,
            self.theme_palette,
            shell_state.modlist_count,
            0,
            |ui| {
                self.render_nav_panel(ui, nav_status, rail_locked_tooltip);
                self.render_central_panel(ui, shell_state);
            },
        );
    }

    fn render_nav_panel(
        &mut self,
        ui: &mut egui::Ui,
        nav_status: &nav_status::PathValidationSummary,
        rail_locked_tooltip: Option<&str>,
    ) {
        egui::SidePanel::left("orchestrator_nav")
            .exact_width(REDESIGN_NAV_WIDTH_PX)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                nav_rail::render(
                    ui,
                    self.theme_palette,
                    &mut self.nav,
                    nav_status,
                    rail_locked_tooltip,
                );
            });
    }

    fn render_central_panel(&mut self, ui: &mut egui::Ui, shell_state: &ShellRenderState) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(redesign_shell_bg(self.theme_palette)))
            .show_inside(ui, |ui| {
                if let Some(err) = self.registry_error.as_ref() {
                    registry_error_panel::render_registry_error(
                        ui,
                        self.theme_palette,
                        err,
                        self.registry_backup_path.as_deref(),
                    );
                } else {
                    self.render_active_page(ui, shell_state);
                }
            });
    }

    fn render_active_page(&mut self, ui: &mut egui::Ui, shell_state: &ShellRenderState) {
        if let NavDestination::Workspace {
            modlist_id: Some(id),
        } = self.nav.clone()
        {
            self.ensure_workspace_loaded(&id);
        }
        let action = if matches!(
            self.nav,
            NavDestination::Workspace {
                modlist_id: Some(_)
            }
        ) {
            self.render_workspace_page(ui, shell_state);
            None
        } else {
            self.render_routed_page(ui)
        };
        if self.settings_screen_state.take_general_changed() {
            self.sync_redesign_settings_from_general();
        }
        if let Some(action) = action {
            self.handle_page_action(action);
        }
        if let Some(text) = self.pending_clipboard_text.take() {
            ui.ctx().copy_text(text);
        }
        self.flush_workspace_draft_if_requested();
    }

    fn render_workspace_page(&mut self, ui: &mut egui::Ui, shell_state: &ShellRenderState) {
        if let Some(step5_action) = workspace_view::render_with_step5_runtime(
            ui,
            self.theme_palette,
            &mut self.workspace_view_state,
            &mut self.wizard_state,
            WorkspaceRuntimeOptions {
                step5_runtime: Some(WorkspaceStep5Runtime {
                    console_view: &mut self.step5_console_view,
                    terminal: self.step5_terminal.as_mut(),
                    terminal_error: self.step5_terminal_error.as_deref(),
                }),
                disable_prev: shell_state.install_runtime_busy || shell_state.clean_install_success,
                latest_share_code: shell_state
                    .current_workspace_entry
                    .latest_share_code
                    .as_deref(),
                step5_success_info: WorkspaceStep5SuccessInfo {
                    mod_count: shell_state.current_workspace_entry.mod_count,
                    component_count: shell_state.current_workspace_entry.component_count,
                },
            },
            self.dev_mode,
            self.exe_fingerprint.as_str(),
        ) {
            self.handle_workspace_step5_action(step5_action);
        }
    }

    fn render_routed_page(&mut self, ui: &mut egui::Ui) -> Option<page_router::PageAction> {
        page_router::render(
            ui,
            self.theme_palette,
            page_router::PageRouterContext {
                nav: &self.nav,
                dev_mode: self.dev_mode,
                dev_seed_message: self.dev_seed_message.as_deref(),
                home_state: &mut self.home_screen_state,
                create_state: &mut self.create_screen_state,
                install_state: &mut self.install_screen_state,
                registry: self.registry.as_ref(),
                wizard_state: &mut self.wizard_state,
                settings_state: &mut self.settings_screen_state,
                workspace_state: &mut self.workspace_view_state,
                step5_console_view: &mut self.step5_console_view,
                step5_terminal: self.step5_terminal.as_mut(),
                step5_terminal_error: self.step5_terminal_error.as_deref(),
                exe_fingerprint: self.exe_fingerprint.as_str(),
            },
        )
    }

    fn flush_workspace_draft_if_requested(&mut self) {
        if self.workspace_view_state.save_draft_requested {
            self.workspace_view_state.save_draft_requested = false;
            if self.flush_current_workspace_state().is_ok() {
                self.workspace_view_state.save_draft_flash_until =
                    Some(Instant::now() + Duration::from_millis(1600));
            }
        }
    }

    fn finish_update_after_render(
        &mut self,
        ctx: &egui::Context,
        previous_nav: &NavDestination,
        mut step5_requested_repaint: bool,
    ) {
        let install_was_running = self.wizard_state.step5.install_running;
        step5_requested_repaint |= crate::app::app_update_cycle::start_after_render(
            &mut self.wizard_state,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
            &mut self.step5_prep_rx,
            &mut self.step5_pending_start,
        );
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
        }
        if step5_requested_repaint {
            ctx.request_repaint();
        }
        self.record_install_success_if_needed();

        if workspace_nav_changed_away(previous_nav, &self.nav) {
            let _ = self.flush_current_workspace_state();
        }

        if let Some(action) = oauth_glue::render_github_popup_if_open(ctx, &mut self.wizard_state) {
            self.handle_step1_action(action);
        }

        if self.github_auth_rx.is_some() || self.wizard_state.github_auth_running {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        if self.needs_runtime_repaint() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn needs_runtime_repaint(&self) -> bool {
        crate::app::app_update_cycle::needs_repaint(
            self.github_auth_rx.as_ref(),
            self.step2_scan_rx.as_ref(),
            &self.step2_progress_queue,
            self.step2_update_check_rx.as_ref(),
            self.step2_update_download_rx.as_ref(),
            self.step2_update_extract_rx.as_ref(),
            self.step5_terminal.as_ref(),
            self.step5_prep_rx.as_ref(),
            &self.wizard_state,
        )
    }
}

impl eframe::App for OrchestratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let previous_nav = self.nav.clone();
        let step5_requested_repaint = self.poll_update_before_render();
        let _ = (
            &self.wizard_state,
            &self.settings_store,
            self.dev_mode,
            &self.exe_fingerprint,
            &self.settings_screen_state,
            self.original_cli_dev_mode,
        );
        let nav_status = nav_status::compute_path_validation_summary(&self.wizard_state);
        let modlist_count = self
            .registry
            .as_ref()
            .map_or(0, |registry| registry.entries.len());
        let shell_state = ShellRenderState {
            modlist_count,
            install_runtime_busy: self.is_install_runtime_busy(),
            clean_install_success: self.is_clean_install_success(),
            current_workspace_entry: self.current_workspace_entry_summary(),
        };
        self.render_orchestrator_shell(ctx, &nav_status, &shell_state);
        self.finish_update_after_render(ctx, &previous_nav, step5_requested_repaint);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_registry_state();
    }
}

impl Drop for OrchestratorApp {
    fn drop(&mut self) {
        self.flush_registry_state();
    }
}

impl OrchestratorApp {
    fn handle_page_action(&mut self, action: page_router::PageAction) {
        match action {
            page_router::PageAction::Navigate(next_nav) => {
                if workspace_nav_changed_away(&self.nav, &next_nav) {
                    let _ = self.flush_current_workspace_state();
                }
                self.nav = next_nav;
            }
            page_router::PageAction::SeedTestModlist => {
                self.seed_test_modlist();
            }
            page_router::PageAction::ConnectGitHub => {
                self.handle_step1_action(Step1Action::ConnectGitHub);
            }
            page_router::PageAction::DisconnectGitHub => {
                self.handle_step1_action(Step1Action::DisconnectGitHub);
            }
            page_router::PageAction::ValidatePathsNow => {
                self.wizard_state.step1_path_check = Some(
                    crate::app::state_validation::run_path_check(&self.wizard_state.step1),
                );
            }
            page_router::PageAction::Home(action) => {
                self.handle_home_action(action);
            }
            page_router::PageAction::Create(action) => {
                Self::handle_create_action(action);
            }
            page_router::PageAction::Install(action) => {
                self.handle_install_action(action);
            }
        }
    }

    fn handle_install_action(&mut self, action: InstallAction) {
        match action {
            InstallAction::Step5(action) => self.handle_step5_action(&action),
            InstallAction::BeginInstallPreviewAccepted => self.begin_install_preview_accepted(),
        }
    }

    const fn handle_create_action(action: CreateAction) {
        match action {
            CreateAction::StartNewModlist
            | CreateAction::PasteShareCode
            | CreateAction::LoadDraftRequested => {}
        }
    }

    fn handle_home_action(&mut self, action: page_home::HomeAction) {
        match action {
            page_home::HomeAction::OpenInstall => {
                self.nav = NavDestination::Install;
            }
            page_home::HomeAction::OpenCreate => {
                self.nav = NavDestination::Create;
            }
            page_home::HomeAction::OpenSettingsPaths => {
                self.nav = NavDestination::Settings;
                self.settings_screen_state.active_tab = SettingsTab::Paths;
            }
            page_home::HomeAction::CancelDelete => {
                self.home_screen_state.delete_target = None;
            }
            page_home::HomeAction::ConfirmDeleteIntent => {}
            page_home::HomeAction::CancelReinstall => {
                self.home_screen_state.reinstall_target = None;
            }
            page_home::HomeAction::ConfirmReinstallIntent => {
                self.confirm_reinstall_from_home();
            }
            page_home::HomeAction::CancelRename => {
                self.home_screen_state.rename_target = None;
                self.home_screen_state.rename_value.clear();
            }
            page_home::HomeAction::ConfirmRenameIntent => {
                self.rename_home_modlist();
            }
            page_home::HomeAction::CardIntent { modlist_id, action } => match action {
                ModlistCardAction::Resume => {
                    self.nav = NavDestination::Workspace {
                        modlist_id: Some(modlist_id),
                    };
                }
                ModlistCardAction::CopyImportCode => {
                    self.copy_home_import_code(&modlist_id);
                }
                ModlistCardAction::OpenInstallFolder => {
                    self.open_home_install_folder(&modlist_id);
                }
                ModlistCardAction::Rename => {
                    self.open_home_rename_dialog(&modlist_id);
                }
                ModlistCardAction::Delete => {
                    self.home_screen_state.delete_target = Some(modlist_id);
                }
                ModlistCardAction::Reinstall => {
                    self.home_screen_state.reinstall_target = Some(modlist_id);
                }
                ModlistCardAction::Open => {}
            },
        }
    }

    fn copy_home_import_code(&mut self, modlist_id: &str) {
        let Some(entry) = self.registry_entry(modlist_id) else {
            return;
        };
        let code =
            std::fs::read_to_string(entry.destination_folder.join(MODLIST_IMPORT_CODE_FILE_NAME))
                .ok()
                .filter(|code| !code.trim().is_empty())
                .or_else(|| entry.latest_share_code.clone());
        if let Some(code) = code {
            self.pending_clipboard_text = Some(code);
        } else {
            eprintln!("Cannot copy import code for {modlist_id}: no import code is available");
        }
    }

    fn open_home_install_folder(&self, modlist_id: &str) {
        let Some(entry) = self.registry_entry(modlist_id) else {
            return;
        };
        if let Err(err) = open_in_shell(entry.destination_folder.to_string_lossy().as_ref()) {
            eprintln!("Failed to open install folder for {modlist_id}: {err}");
        }
    }

    fn open_home_rename_dialog(&mut self, modlist_id: &str) {
        let Some(entry) = self.registry_entry(modlist_id) else {
            return;
        };
        let entry_id = entry.id.clone();
        let entry_name = entry.name.clone();
        self.home_screen_state.rename_target = Some(entry_id);
        self.home_screen_state.rename_value = entry_name;
    }

    fn rename_home_modlist(&mut self) {
        let Some(modlist_id) = self.home_screen_state.rename_target.take() else {
            return;
        };
        let next_name = self.home_screen_state.rename_value.trim().to_string();
        self.home_screen_state.rename_value.clear();
        if next_name.is_empty() {
            return;
        }
        let (Some(registry), Some(persistence)) =
            (self.registry.as_mut(), self.registry_persistence.as_mut())
        else {
            return;
        };
        let Some(entry) = registry
            .entries
            .iter_mut()
            .find(|entry| entry.id == modlist_id)
        else {
            return;
        };
        entry.name = next_name;
        entry.last_touched_date = Utc::now();
        if let Err(err) = persistence.flush_registry(registry, &self.registry_store) {
            eprintln!("Failed to rename modlist {modlist_id}: {err}");
        }
    }

    fn registry_entry(&self, modlist_id: &str) -> Option<&ModlistEntry> {
        self.registry
            .as_ref()
            .and_then(|registry| registry.entries.iter().find(|entry| entry.id == modlist_id))
    }

    fn confirm_reinstall_from_home(&mut self) {
        let Some(modlist_id) = self.home_screen_state.reinstall_target.take() else {
            return;
        };
        let Some(entry) = self.registry.as_ref().and_then(|registry| {
            registry
                .entries
                .iter()
                .find(|entry| entry.id == modlist_id && entry.state == ModlistState::Installed)
        }) else {
            eprintln!("Cannot reinstall {modlist_id}: installed registry entry not found");
            return;
        };
        let Some(code) = entry.latest_share_code.clone() else {
            eprintln!("Cannot reinstall {modlist_id}: latest share code is missing");
            return;
        };
        let preview = match crate::app::modlist_share::preview_modlist_share_code(&code) {
            Ok(preview) => preview,
            Err(err) => {
                eprintln!("Cannot reinstall {modlist_id}: failed to parse share code: {err}");
                return;
            }
        };

        self.install_screen_state.stage = InstallStage::Preview;
        self.install_screen_state.destination =
            entry.destination_folder.to_string_lossy().to_string();
        self.install_screen_state.import_code = code;
        self.install_screen_state.preview = Some(preview);
        self.install_screen_state.preview_error = None;
        self.install_screen_state.preview_tab = InstallPreviewTab::Summary;
        self.install_screen_state.reinstall_modlist_id = Some(entry.id.clone());
        self.nav = NavDestination::Install;
    }

    fn begin_install_preview_accepted(&mut self) {
        if let Some(modlist_id) = self.install_screen_state.reinstall_modlist_id.as_deref() {
            let (Some(registry), Some(persistence)) =
                (self.registry.as_mut(), self.registry_persistence.as_mut())
            else {
                self.install_screen_state.stage = InstallStage::Installing;
                return;
            };
            if let Some(entry) = registry
                .entries
                .iter_mut()
                .find(|entry| entry.id == modlist_id && entry.state == ModlistState::Installed)
            {
                let now = Utc::now();
                entry.state = ModlistState::InProgress;
                entry.install_date = None;
                entry.last_touched_date = now;
                if let Err(err) = persistence.flush_registry(registry, &self.registry_store) {
                    eprintln!("Failed to mark reinstall in progress for {modlist_id}: {err}");
                }
            }
        }
        self.install_screen_state.stage = InstallStage::Installing;
    }

    fn handle_step1_action(&mut self, action: Step1Action) {
        match action {
            Step1Action::ConnectGitHub => {
                oauth_glue::start_github_flow(
                    &mut self.wizard_state,
                    &mut self.github_auth_rx,
                    false,
                );
            }
            Step1Action::ReconnectGitHub => {
                oauth_glue::start_github_flow(
                    &mut self.wizard_state,
                    &mut self.github_auth_rx,
                    true,
                );
            }
            Step1Action::DisconnectGitHub => {
                oauth_glue::disconnect_github(&mut self.wizard_state, &mut self.github_auth_rx);
            }
            Step1Action::PathsChanged => {}
        }
    }

    fn handle_step5_action(&mut self, action: &Step5Action) {
        match action {
            Step5Action::StartInstall => {
                let result = self
                    .apply_install_start_policies()
                    .and_then(|()| self.write_import_code_before_install_start());
                match result {
                    Ok(()) => {
                        self.wizard_state.step5.start_install_requested = true;
                    }
                    Err(err) => {
                        self.wizard_state.step5.last_status_text =
                            format!("Install start blocked: {err}");
                    }
                }
            }
        }
    }

    fn apply_install_start_policies(&mut self) -> Result<(), String> {
        let destination_folder = self.current_workspace_destination_folder()?;
        crate::install_runtime::start_hooks::on_install_start(
            &mut self.wizard_state,
            &destination_folder,
        )
    }

    fn current_workspace_destination_folder(&self) -> Result<PathBuf, String> {
        let modlist_id = self
            .workspace_view_state
            .loaded_workspace_id
            .as_deref()
            .ok_or_else(|| "no loaded workspace".to_string())?;
        let registry = self
            .registry
            .as_ref()
            .ok_or_else(|| "modlist registry is not loaded".to_string())?;
        registry
            .entries
            .iter()
            .find(|entry| entry.id == modlist_id)
            .map(|entry| entry.destination_folder.clone())
            .ok_or_else(|| format!("modlist registry entry not found: {modlist_id}"))
    }

    fn write_import_code_before_install_start(&self) -> Result<(), String> {
        if self.wizard_state.step5.resume_available {
            return Ok(());
        }

        let modlist_id = self
            .workspace_view_state
            .loaded_workspace_id
            .as_deref()
            .ok_or_else(|| "no loaded workspace".to_string())?;
        let registry = self
            .registry
            .as_ref()
            .ok_or_else(|| "modlist registry is not loaded".to_string())?;
        let entry = registry
            .entries
            .iter()
            .find(|entry| entry.id == modlist_id)
            .ok_or_else(|| format!("modlist registry entry not found: {modlist_id}"))?;
        let code = crate::app::modlist_share::export_modlist_share_code_with_auto_install(
            &self.wizard_state,
            false,
        )?;

        std::fs::create_dir_all(&entry.destination_folder).map_err(|err| {
            format!(
                "failed to create destination folder {}: {err}",
                entry.destination_folder.display()
            )
        })?;
        let target = entry.destination_folder.join(MODLIST_IMPORT_CODE_FILE_NAME);
        std::fs::write(&target, code)
            .map_err(|err| format!("failed to write {}: {err}", target.display()))
    }

    fn handle_workspace_step5_action(&mut self, action: WorkspaceStep5Action) {
        match action {
            WorkspaceStep5Action::Step5(action) => self.handle_step5_action(&action),
            WorkspaceStep5Action::ReturnToHome => {
                self.nav = NavDestination::Home;
            }
            WorkspaceStep5Action::OpenInstallFolder => {
                self.open_current_workspace_install_folder();
            }
        }
    }

    fn open_current_workspace_install_folder(&self) {
        let Some(modlist_id) = self.workspace_view_state.loaded_workspace_id.as_deref() else {
            return;
        };
        let Some(entry) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.entries.iter().find(|entry| entry.id == modlist_id))
        else {
            return;
        };
        if let Err(err) = open_in_shell(entry.destination_folder.to_string_lossy().as_ref()) {
            eprintln!("Failed to open install folder for {modlist_id}: {err}");
        }
    }

    const fn is_install_runtime_busy(&self) -> bool {
        self.wizard_state.step5.prep_running
            || self.wizard_state.step5.install_running
            || self.wizard_state.step5.cancel_pending
    }

    fn is_clean_install_success(&self) -> bool {
        !self.wizard_state.step5.install_running
            && self.wizard_state.step5.last_exit_code == Some(0)
            && !self.wizard_state.step5.last_install_failed
    }

    fn current_workspace_entry_summary(&self) -> CurrentWorkspaceEntrySummary {
        let Some(modlist_id) = self.workspace_view_state.loaded_workspace_id.as_deref() else {
            return CurrentWorkspaceEntrySummary::default();
        };
        let Some(entry) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.entries.iter().find(|entry| entry.id == modlist_id))
        else {
            return CurrentWorkspaceEntrySummary::default();
        };

        CurrentWorkspaceEntrySummary {
            mod_count: entry.mod_count,
            component_count: entry.component_count,
            latest_share_code: entry.latest_share_code.clone(),
        }
    }

    fn record_install_success_if_needed(&mut self) {
        if !self.is_clean_install_success() {
            return;
        }

        self.workspace_view_state.install_complete = true;

        let Some(modlist_id) = self.workspace_view_state.loaded_workspace_id.as_deref() else {
            return;
        };
        let (Some(registry), Some(persistence)) =
            (self.registry.as_mut(), self.registry_persistence.as_mut())
        else {
            return;
        };
        let Some(entry) = registry
            .entries
            .iter_mut()
            .find(|entry| entry.id == modlist_id)
        else {
            return;
        };
        if entry.state != ModlistState::InProgress {
            return;
        }

        let now = Utc::now();
        let latest_share_code =
            crate::app::modlist_share::export_modlist_share_code_with_auto_install(
                &self.wizard_state,
                true,
            );
        entry.state = ModlistState::Installed;
        entry.install_date = Some(now);
        entry.last_touched_date = now;
        match latest_share_code {
            Ok(code) => {
                entry.latest_share_code = Some(code);
            }
            Err(err) => {
                eprintln!("Failed to export verified share code for {modlist_id}: {err}");
            }
        }

        if let Err(err) = persistence.flush_registry(registry, &self.registry_store) {
            eprintln!("Failed to record completed install for {modlist_id}: {err}");
        }
    }

    fn sync_redesign_settings_from_general(&mut self) {
        let next_settings = self.settings_screen_state.current_redesign_settings();
        if next_settings == self.redesign_settings {
            return;
        }

        self.redesign_settings = next_settings;
        self.theme_palette = self.redesign_settings.theme_palette;
        self.dev_mode = self.original_cli_dev_mode || self.redesign_settings.diagnostic_mode;
        if let Err(err) = self.redesign_settings_store.save(&self.redesign_settings) {
            eprintln!("Failed to save redesign settings: {err}");
        }
    }

    fn seed_test_modlist(&mut self) {
        let Some(registry) = self.registry.as_mut() else {
            self.dev_seed_message = Some("Seed failed: registry is not loaded".to_string());
            return;
        };

        match dev_seed::seed_demo_entry(registry, &self.registry_store) {
            Ok(entry) => {
                self.dev_seed_message = Some(format!("Seeded \"{}\"", entry.name));
            }
            Err(err) => {
                self.dev_seed_message = Some(format!("Seed failed: {err}"));
            }
        }
    }

    fn sync_settings_paths_to_step1(&mut self) {
        if self.wizard_state.step1.game_install == "IWDEE" {
            self.wizard_state.step1.bgee_game_folder =
                self.settings_screen_state.iwdee_source_path.clone();
        } else {
            self.wizard_state.step1.bgee_game_folder =
                self.settings_screen_state.bgee_source_path.clone();
            self.wizard_state.step1.eet_bgee_game_folder =
                self.settings_screen_state.bgee_source_path.clone();
        }
        self.wizard_state.step1.bg2ee_game_folder =
            self.settings_screen_state.bg2ee_source_path.clone();
        self.wizard_state.step1.eet_bg2ee_game_folder =
            self.settings_screen_state.bg2ee_source_path.clone();
        self.wizard_state.step1.mods_archive_folder =
            self.settings_screen_state.mods_archive_path.clone();
        self.wizard_state.step1.mods_backup_folder =
            self.settings_screen_state.mods_backup_path.clone();
    }

    fn ensure_workspace_loaded(&mut self, modlist_id: &str) {
        if self.workspace_view_state.loaded_workspace_id.as_deref() == Some(modlist_id) {
            return;
        }

        let Some(entry) = self
            .registry
            .as_ref()
            .and_then(|registry| registry.entries.iter().find(|entry| entry.id == modlist_id))
            .cloned()
        else {
            eprintln!("Workspace load skipped: modlist id not found: {modlist_id}");
            return;
        };

        if entry.state != ModlistState::InProgress {
            eprintln!("Workspace load skipped: modlist is not in progress: {modlist_id}");
            return;
        }

        let store = workspace_store_for_entry(&entry);
        let workspace = match store.load() {
            Ok(workspace) => workspace,
            Err(err) => {
                eprintln!("Workspace load failed for {modlist_id}: {err}");
                return;
            }
        };

        let settings = Step1Settings::from(self.wizard_state.step1.clone());
        workspace_state_loader::populate_wizard_state_from_workspace(
            &workspace,
            &entry,
            &settings,
            &mut self.wizard_state,
        );

        self.workspace_view_state = WorkspaceViewState::new(entry.id.clone(), entry.name.clone());
        self.workspace_view_state.loaded_workspace_id = Some(entry.id);
        self.current_workspace = Some(workspace);
        self.current_workspace_store = Some(store);
    }

    fn flush_current_workspace_state(&mut self) -> Result<(), RegistryError> {
        let Some(modlist_id) = self.workspace_view_state.loaded_workspace_id.as_deref() else {
            return Ok(());
        };
        let Some(existing_workspace) = self.current_workspace.as_ref() else {
            return Ok(());
        };
        let Some(store) = self.current_workspace_store.as_ref() else {
            return Ok(());
        };
        let Some(persistence) = self.registry_persistence.as_mut() else {
            return Ok(());
        };

        let next_workspace = workspace_state_loader::extract_workspace_state_from_wizard(
            existing_workspace,
            &self.wizard_state,
        );
        persistence.flush_workspace(modlist_id, &next_workspace, store)?;
        self.current_workspace = Some(next_workspace);
        Ok(())
    }
}

fn workspace_store_for_entry(entry: &crate::registry::model::ModlistEntry) -> WorkspaceStore {
    if entry.workspace_file_relpath.as_os_str().is_empty() {
        WorkspaceStore::new_for_id(&entry.id)
    } else {
        WorkspaceStore::new(crate::platform_defaults::app_config_file(
            &entry.workspace_file_relpath.to_string_lossy(),
            ".",
        ))
    }
}

fn current_workspace_map(
    state: &WorkspaceViewState,
    workspace: Option<&ModlistWorkspaceState>,
) -> HashMap<String, ModlistWorkspaceState> {
    let Some(modlist_id) = state.loaded_workspace_id.as_ref() else {
        return HashMap::new();
    };
    let Some(workspace) = workspace else {
        return HashMap::new();
    };
    HashMap::from([(modlist_id.clone(), workspace.clone())])
}

fn current_workspace_store_map(
    state: &WorkspaceViewState,
    store: Option<&WorkspaceStore>,
) -> HashMap<String, WorkspaceStore> {
    let Some(modlist_id) = state.loaded_workspace_id.as_ref() else {
        return HashMap::new();
    };
    let Some(store) = store else {
        return HashMap::new();
    };
    HashMap::from([(modlist_id.clone(), store.clone())])
}

const fn workspace_nav_changed_away(previous: &NavDestination, next: &NavDestination) -> bool {
    matches!(
        previous,
        NavDestination::Workspace {
            modlist_id: Some(_)
        }
    ) && !matches!(
        next,
        NavDestination::Workspace {
            modlist_id: Some(_)
        }
    )
}

fn initialize_settings_paths_from_step1(
    settings: &mut SettingsScreenState,
    step1: &crate::app::state::Step1State,
) {
    settings.bgee_source_path =
        first_non_empty(&step1.bgee_game_folder, &step1.eet_bgee_game_folder);
    settings.bg2ee_source_path =
        first_non_empty(&step1.bg2ee_game_folder, &step1.eet_bg2ee_game_folder);
    if step1.game_install == "IWDEE" {
        settings
            .iwdee_source_path
            .clone_from(&step1.bgee_game_folder);
    }
    settings
        .mods_archive_path
        .clone_from(&step1.mods_archive_folder);
    settings
        .mods_backup_path
        .clone_from(&step1.mods_backup_folder);
}

fn first_non_empty(primary: &str, fallback: &str) -> String {
    if primary.trim().is_empty() {
        fallback.to_string()
    } else {
        primary.to_string()
    }
}
