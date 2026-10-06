// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::warn;

use crate::app::app_bootstrap_init;
use crate::app::app_step1_github_oauth::GitHubOAuthFlowResult;
use crate::app::app_step2_update_download::DownloadPoll;
use crate::app::state::{DownloadOrigin, WizardState};
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::app::{
    app_step2_saved_log_flow, app_step2_scan, app_step2_update_check, app_step2_update_download,
    app_step2_update_extract, app_step5_flow,
};
use crate::install_runtime::destination_prep::{DestinationPrepJoinHandle, DestinationPrepWorker};
use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::install_runtime::install_concurrency;
use crate::install_runtime::rail_lock_reason::RailLockReason;
use crate::install_runtime::registry_transition;
use crate::registry::errors::RegistryError;
use crate::registry::model::Game;
use crate::registry::model::ModlistRegistry;
use crate::registry::persistence_cycle::RegistryPersistenceCycle;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::settings::model::AppSettings;
use crate::settings::redesign_fields::{RedesignSettings, ThemeChoice};
use crate::settings::store::SettingsStore;
use crate::ui::create::state_create::CreateScreenState;
use crate::ui::home::state_home::HomeScreenState;
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::orchestrator::left_rail;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_status::{
    PathValidationKind, PathValidationSummary, compute_path_validation_summary,
};
use crate::ui::orchestrator::page_router;
use crate::ui::orchestrator::stubs::home_stub::HomeStubState;
use crate::ui::orchestrator::widgets::NotificationManager;
use crate::ui::orchestrator::widgets::clipboard;
use crate::ui::orchestrator::widgets::help_button;
use crate::ui::settings::oauth_glue;
use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::validate_debounce;
use crate::ui::shared::redesign_tokens::{REDESIGN_NAV_WIDTH_PX, ThemePalette};
use crate::ui::shell::shell_chrome;
use crate::ui::shell::shell_statusbar::RunningInstallStatus;
use crate::ui::step5::state_step5::Step5ConsoleViewState;
use crate::ui::workspace::state_workspace::WorkspaceViewState;
use crate::ui::workspace::step5::state_workspace_step5::WorkspaceStep5State;

const BIO_SETTINGS_DEBOUNCE_MS: u64 = 1000;

#[derive(Debug, Clone, Default)]
pub struct ToolVersionCache {
    pub weidu_version: Option<String>,
    pub mod_installer_version: Option<String>,
}

struct RegistryLoad {
    registry: ModlistRegistry,
    registry_error: Option<RegistryError>,
    registry_backup_path: Option<std::path::PathBuf>,
}

#[derive(Clone, Copy, Default)]
pub struct DirtyFlag(bool);

impl std::ops::Not for DirtyFlag {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostInstallResetGate {
    #[default]
    Idle,
    Pending,
}

impl PostInstallResetGate {
    #[must_use]
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }
}

pub(crate) struct PendingFolderDelete {
    pub(crate) modlist_name: String,
    pub(crate) rx: crate::registry::operations::FolderDeleteReceiver,
}

pub(crate) struct PendingCreateStart {
    pub(crate) token: DestinationPrepToken,
    pub(crate) name: String,
    pub(crate) destination: String,
    pub(crate) game: Game,
    pub(crate) worker: DestinationPrepWorker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestinationPrepFlow {
    CreateScratch,
    InstallPipeline,
    CreateForkDownload,
    WorkspaceStep5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DestinationPrepToken {
    pub(crate) generation: u64,
    pub(crate) flow: DestinationPrepFlow,
    destination_key: String,
    pub(crate) modlist_id: Option<String>,
}

impl DestinationPrepToken {
    #[must_use]
    pub(crate) fn new(
        generation: u64,
        flow: DestinationPrepFlow,
        destination: &str,
        modlist_id: Option<String>,
    ) -> Self {
        Self {
            generation,
            flow,
            destination_key: destination_prep_key(destination),
            modlist_id,
        }
    }

    #[must_use]
    pub(crate) fn matches_context(
        &self,
        current_generation: u64,
        flow: DestinationPrepFlow,
        destination: &str,
        modlist_id: Option<&str>,
    ) -> bool {
        self.generation == current_generation
            && self.flow == flow
            && self.destination_key == destination_prep_key(destination)
            && self.modlist_id.as_deref() == modlist_id
    }
}

#[must_use]
pub(crate) fn destination_prep_key(destination: &str) -> String {
    let mut key = destination.trim().replace('/', "\\");
    while key.len() > 3 && key.ends_with('\\') {
        key.pop();
    }
    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}

fn join_destination_prep_handle(handle: DestinationPrepJoinHandle) {
    if handle.join().is_err() {
        warn!(
            target = "orchestrator",
            "destination prep worker panicked before shutdown join completed"
        );
    }
}

fn hold_destination_prep_worker_for_shutdown(
    workers: &mut Vec<DestinationPrepJoinHandle>,
    worker: DestinationPrepWorker,
) {
    let handle = worker.into_join_handle();
    if handle.is_finished() {
        join_destination_prep_handle(handle);
    } else {
        workers.push(handle);
    }
}

pub struct PendingInstallDestinationPrep {
    pub(crate) token: DestinationPrepToken,
    pub(crate) destination: String,
    pub(crate) game: Game,
    pub(crate) workflow: InstallWorkflow,
    pub(crate) code: String,
    pub(crate) worker: DestinationPrepWorker,
}

impl PendingInstallDestinationPrep {
    #[must_use]
    pub(crate) fn matches_context(
        &self,
        current_generation: u64,
        flow: DestinationPrepFlow,
        destination: &str,
        game: Game,
        workflow: InstallWorkflow,
        code: &str,
    ) -> bool {
        self.token
            .matches_context(current_generation, flow, destination, None)
            && self.game == game
            && self.workflow == workflow
            && self.code == code.trim()
    }
}

pub(crate) struct PendingWorkspaceDestinationPrep {
    pub(crate) token: DestinationPrepToken,
    pub(crate) modlist_id: String,
    pub(crate) worker: DestinationPrepWorker,
}

pub struct OrchestratorApp {
    pub nav: NavDestination,
    pub(crate) last_rendered_nav: NavDestination,
    pub wizard_state: WizardState,
    pub settings_store: SettingsStore,
    pub dev_mode: bool,
    pub dev_mode_cli_flag: bool,
    pub exe_fingerprint: String,
    pub path_validation: PathValidationSummary,
    pub theme_palette: ThemePalette,

    pub registry: ModlistRegistry,
    pub registry_store: RegistryStore,
    pub registry_error: Option<RegistryError>,
    pub registry_backup_path: Option<std::path::PathBuf>,
    pub persistence_cycle: RegistryPersistenceCycle,
    pub workspace_state: HashMap<String, ModlistWorkspaceState>,
    pub workspace_stores: HashMap<String, WorkspaceStore>,
    pub home_stub_state: HomeStubState,

    pub home_screen_state: HomeScreenState,
    pub notification_manager: NotificationManager,
    pub install_screen_state: InstallScreenState,
    pub create_screen_state: CreateScreenState,

    pub redesign_settings: RedesignSettings,
    pub settings_screen_state: SettingsScreenState,
    pub(crate) github_auth_rx: Option<Receiver<GitHubOAuthFlowResult>>,
    pub tool_version_cache: ToolVersionCache,
    pub accounts_stub_hint: Option<String>,
    pub bio_settings_last_saved: AppSettings,
    pub bio_settings_last_dirty_at: Option<Instant>,

    pub workspace_view: WorkspaceViewState,
    pub workspace_state_dirty: DirtyFlag,

    pub workspace_step5: WorkspaceStep5State,

    pub(crate) pending_reinstall_id: Option<String>,

    pub(crate) pending_replaced_entry: Option<crate::install_runtime::replaced_owners::HeldOwners>,

    pub(crate) active_install_modlist_id: Option<String>,

    pub post_install_reset_gate: PostInstallResetGate,

    pub install_running_since: Option<Instant>,

    pub(crate) install_size_worker_rx:
        Option<crate::install_runtime::registry_transition::SizeWorkerReceiver>,

    pub step5_terminal: Option<EmbeddedTerminal>,
    pub step5_terminal_error: Option<String>,
    pub step5_console_view: Step5ConsoleViewState,
    pub(crate) step5_prep_rx: Option<Receiver<Result<TargetPrepResult, String>>>,
    pub(crate) step5_pending_start: Option<PendingInstallStart>,

    pub(crate) step2_scan_rx: Option<Receiver<Step2ScanEvent>>,
    pub(crate) step2_cancel: Option<Arc<AtomicBool>>,
    pub(crate) step2_progress_queue: VecDeque<(usize, usize, String)>,
    pub(crate) step2_update_check_rx:
        Option<Receiver<crate::app::app_step2_update_check_worker::Step2UpdateCheckEvent>>,
    pub(crate) step2_update_download_rx:
        Option<Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent>>,
    pub(crate) step2_update_extract_rx:
        Option<Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>>,
    pub(crate) release_list_rx: Option<crate::app::github_release_list::ReleaseListFetch>,
    pub(crate) forks_rx: Option<crate::app::github_forks_list::ForksFetch>,
    pub(crate) added_mods_seeded_for: Option<String>,

    pub(crate) archive_skip_rx:
        Option<Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>>,
    pub(crate) manual_download_rx:
        Option<Receiver<crate::install_runtime::manual_download_watcher::WatchEvent>>,
    pub(crate) create_destination_prep_rx: Option<PendingCreateStart>,
    pub(crate) install_destination_prep_rx: Option<PendingInstallDestinationPrep>,
    pub(crate) workspace_destination_prep_rx: Option<PendingWorkspaceDestinationPrep>,
    pub(crate) background_destination_prep_workers: Vec<DestinationPrepJoinHandle>,
    pub(crate) destination_prep_generation: u64,

    pub(crate) hash_progress: Arc<std::sync::Mutex<Option<(usize, usize)>>>,

    pub(crate) pending_folder_deletes: Vec<PendingFolderDelete>,

    pub(crate) share_name_buffer: String,

    #[cfg(test)]
    pub(crate) isolated_test_config_root: Option<std::path::PathBuf>,
}

fn load_registry(registry_store: &RegistryStore) -> RegistryLoad {
    match registry_store.load() {
        Ok(registry) => RegistryLoad {
            registry,
            registry_error: None,
            registry_backup_path: None,
        },
        Err(err) => {
            warn!(
                target = "orchestrator",
                "modlists.json load failed: {err}; backing up and entering terminal-error state"
            );
            let registry_backup_path = match registry_store.backup_corrupt_file() {
                Ok(new_path) => Some(new_path),
                Err(backup_err) => {
                    warn!(
                        target = "orchestrator",
                        "backup_corrupt_file failed: {backup_err}"
                    );
                    None
                }
            };
            RegistryLoad {
                registry: ModlistRegistry::default(),
                registry_error: Some(err),
                registry_backup_path,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct AddedModSeedWatch {
    scan_live: bool,
    log_apply_pending: bool,
}

impl OrchestratorApp {
    #[must_use]
    pub fn new(dev_mode: bool) -> Self {
        let bootstrap = app_bootstrap_init::initialize(dev_mode);

        let mut wizard_state = WizardState {
            step1: bootstrap.step1.clone(),
            github_auth_login: bootstrap.github_auth_login,
            ..Default::default()
        };

        crate::app::compat_dlc_source::refresh_source_check(&mut wizard_state.step1);
        let path_validation = compute_path_validation_summary(&wizard_state);

        let registry_store = RegistryStore::new_default();
        let RegistryLoad {
            registry,
            registry_error,
            registry_backup_path,
        } = load_registry(&registry_store);

        let persistence_cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());

        let redesign_settings = bootstrap.general.clone();
        let theme_palette = match redesign_settings.theme_palette {
            ThemeChoice::Light => ThemePalette::Light,
            ThemeChoice::Dark => ThemePalette::Dark,
        };
        let effective_dev_mode = dev_mode || redesign_settings.diagnostic_mode;
        let bio_settings_snapshot = AppSettings {
            exe_fingerprint: bootstrap.exe_fingerprint.clone(),
            step1: bootstrap.step1.clone().into(),
            general: redesign_settings.clone(),
        };

        let mut app = Self {
            nav: NavDestination::default(),
            last_rendered_nav: NavDestination::default(),
            wizard_state,
            settings_store: bootstrap.settings_store,
            dev_mode: effective_dev_mode,
            dev_mode_cli_flag: dev_mode,
            exe_fingerprint: bootstrap.exe_fingerprint,
            path_validation,
            theme_palette,

            registry,
            registry_store,
            registry_error,
            registry_backup_path,
            persistence_cycle,
            workspace_state: HashMap::new(),
            workspace_stores: HashMap::new(),
            home_stub_state: HomeStubState::default(),
            home_screen_state: HomeScreenState::default(),
            notification_manager: NotificationManager::new(),
            install_screen_state: InstallScreenState::default(),
            create_screen_state: CreateScreenState::new(),

            redesign_settings,
            settings_screen_state: SettingsScreenState::default(),
            github_auth_rx: None,
            tool_version_cache: ToolVersionCache::default(),
            accounts_stub_hint: None,
            bio_settings_last_saved: bio_settings_snapshot,
            bio_settings_last_dirty_at: None,

            workspace_view: WorkspaceViewState::default(),
            workspace_state_dirty: DirtyFlag::default(),

            workspace_step5: WorkspaceStep5State::default(),
            pending_reinstall_id: None,
            pending_replaced_entry: None,
            active_install_modlist_id: None,
            post_install_reset_gate: PostInstallResetGate::Idle,
            install_running_since: None,
            install_size_worker_rx: None,
            step5_terminal: None,
            step5_terminal_error: None,
            step5_console_view: Step5ConsoleViewState::default(),
            step5_prep_rx: None,
            step5_pending_start: None,

            step2_scan_rx: None,
            step2_cancel: None,
            step2_progress_queue: VecDeque::new(),
            step2_update_check_rx: None,
            step2_update_download_rx: None,
            step2_update_extract_rx: None,
            release_list_rx: None,
            forks_rx: None,
            added_mods_seeded_for: None,
            archive_skip_rx: None,
            manual_download_rx: None,
            create_destination_prep_rx: None,
            install_destination_prep_rx: None,
            workspace_destination_prep_rx: None,
            background_destination_prep_workers: Vec::new(),
            destination_prep_generation: 0,
            hash_progress: Arc::new(std::sync::Mutex::new(None)),
            pending_folder_deletes: Vec::new(),
            share_name_buffer: String::new(),
            #[cfg(test)]
            isolated_test_config_root: None,
        };

        if app.redesign_settings.validate_paths_on_startup {
            app.settings_screen_state.path_validation_results =
                crate::ui::settings::validate_now::run_now(&app.wizard_state.step1);
        }

        app
    }

    pub const fn mark_workspace_dirty(&mut self) {
        self.workspace_state_dirty = DirtyFlag(true);
    }

    pub(crate) fn next_destination_prep_token(
        &mut self,
        flow: DestinationPrepFlow,
        destination: &str,
        modlist_id: Option<String>,
    ) -> DestinationPrepToken {
        self.destination_prep_generation = self.destination_prep_generation.wrapping_add(1);
        if self.destination_prep_generation == 0 {
            self.destination_prep_generation = 1;
        }
        DestinationPrepToken::new(
            self.destination_prep_generation,
            flow,
            destination,
            modlist_id,
        )
    }

    pub(crate) fn complete_destination_prep_worker(&mut self, worker: DestinationPrepWorker) {
        join_destination_prep_handle(worker.into_join_handle());
        self.drain_finished_destination_prep_workers();
    }

    pub(crate) fn abandon_destination_prep_worker(&mut self, worker: DestinationPrepWorker) {
        hold_destination_prep_worker_for_shutdown(
            &mut self.background_destination_prep_workers,
            worker,
        );
    }

    pub(crate) fn abandon_create_destination_prep(&mut self) {
        if let Some(pending) = self.create_destination_prep_rx.take() {
            self.abandon_destination_prep_worker(pending.worker);
        }
    }

    pub(crate) fn abandon_install_destination_prep(&mut self) {
        if let Some(pending) = self.install_destination_prep_rx.take() {
            self.abandon_destination_prep_worker(pending.worker);
        }
    }

    pub(crate) fn abandon_workspace_destination_prep(&mut self) {
        if let Some(pending) = self.workspace_destination_prep_rx.take() {
            self.abandon_destination_prep_worker(pending.worker);
        }
    }

    fn drain_finished_destination_prep_workers(&mut self) {
        let workers = &mut self.background_destination_prep_workers;
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                let handle = workers.swap_remove(index);
                join_destination_prep_handle(handle);
            } else {
                index += 1;
            }
        }
    }

    fn join_all_destination_prep_workers(&mut self) {
        self.abandon_create_destination_prep();
        self.abandon_install_destination_prep();
        self.abandon_workspace_destination_prep();
        for handle in self.background_destination_prep_workers.drain(..) {
            join_destination_prep_handle(handle);
        }
    }

    pub(crate) fn reset_install_screen_to_gallery(&mut self) {
        reset_install_pipeline_state(InstallPipelineResetSet {
            step2_update_download_rx: &mut self.step2_update_download_rx,
            step2_update_extract_rx: &mut self.step2_update_extract_rx,
            archive_skip_rx: &mut self.archive_skip_rx,
            manual_download_rx: &mut self.manual_download_rx,
            install_destination_prep_rx: &mut self.install_destination_prep_rx,
            background_destination_prep_workers: &mut self.background_destination_prep_workers,
            wizard_state: &mut self.wizard_state,
            install_screen_state: &mut self.install_screen_state,
            hash_progress: &self.hash_progress,
            pending_reinstall_id: &mut self.pending_reinstall_id,
            active_install_modlist_id: &mut self.active_install_modlist_id,
        });
        if let Some(term) = self.step5_terminal.as_mut() {
            term.clear_console();
        }
        self.step5_console_view = Step5ConsoleViewState::default();
    }

    fn maybe_flip_to_installed_on_clean_exit(&mut self) {
        if !crate::ui::workspace::step5::success_banner::clean_exit(&self.wizard_state) {
            return;
        }

        let from_workspace = self.workspace_view.loaded_workspace_id.is_some();
        let Some(id) = self
            .workspace_view
            .loaded_workspace_id
            .clone()
            .or_else(|| self.active_install_modlist_id.clone())
        else {
            warn!(
                target = "orchestrator",
                "clean-exit edge with no loaded workspace id and no \
 active_install_modlist_id; flip_to_installed skipped"
            );
            return;
        };

        let held_code: Option<String> = self
            .registry
            .find(&id)
            .and_then(|e| e.latest_share_code.clone())
            .filter(|c| !c.trim().is_empty());

        if !from_workspace {
            crate::app::app_step2_log::apply_saved_weidu_log_selection(&mut self.wizard_state);
            crate::app::app_step3_sync_flow::sync_step3_from_step2(&mut self.wizard_state);
        }

        let Self {
            registry,
            registry_store,
            wizard_state,
            ..
        } = &mut *self;

        let rx = registry_transition::flip_to_installed(
            &id,
            registry,
            registry_store,
            wizard_state,
            held_code.as_deref(),
        );
        if rx.is_some() {
            self.install_size_worker_rx = rx;
        }

        if !from_workspace {
            self.active_install_modlist_id = None;
        }

        if !from_workspace {
            self.post_install_reset_gate = PostInstallResetGate::Pending;
        }
    }

    fn start_step5_and_check_focus(&mut self) -> bool {
        let was_running = self.wizard_state.step5.install_running;
        let requested = self.start_step5_after_render();
        if !was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
        }
        requested
    }

    const fn slow_workers_active(&self) -> bool {
        self.install_size_worker_rx.is_some()
            || !self.pending_folder_deletes.is_empty()
            || self.manual_download_rx.is_some()
    }

    fn drain_background_workers(&mut self) {
        self.drain_size_worker_result();
        self.drain_folder_deletes();
        self.drain_finished_destination_prep_workers();
    }

    pub(crate) fn drain_folder_deletes(&mut self) {
        use std::sync::mpsc::TryRecvError;

        let mut i = 0;
        while i < self.pending_folder_deletes.len() {
            match self.pending_folder_deletes[i].rx.try_recv() {
                Ok(Ok(())) => {
                    let name = self.pending_folder_deletes.swap_remove(i).modlist_name;
                    self.notification_manager
                        .success(format!("Deleted \"{name}\""));
                }
                Ok(Err(err)) => {
                    let name = self.pending_folder_deletes.swap_remove(i).modlist_name;
                    self.notification_manager.error(format!(
                        "Couldn't remove install folder for \"{name}\": {err}"
                    ));
                }
                Err(TryRecvError::Disconnected) => {
                    let name = self.pending_folder_deletes.swap_remove(i).modlist_name;
                    self.notification_manager.error(format!(
                        "Couldn't remove install folder for \"{name}\": worker disconnected"
                    ));
                }
                Err(TryRecvError::Empty) => {
                    i += 1;
                }
            }
        }
    }

    fn drain_size_worker_result(&mut self) {
        use std::sync::mpsc::TryRecvError;

        let Some(rx) = self.install_size_worker_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok((modlist_id, bytes)) => {
                if let Some(entry) = self.registry.find_mut(&modlist_id) {
                    entry.total_size_bytes = Some(bytes);
                    if let Err(err) = self.registry_store.save(&self.registry) {
                        warn!(
                            target = "orchestrator",
                            "size-fill atomic write for {modlist_id} failed: \
 {err} (in-memory size set; debounced cycle will \
 retry the write — plan )"
                        );
                    }
                } else {
                    tracing::debug!(
                        target = "orchestrator",
                        "size result for {modlist_id} discarded — modlist no \
 longer in registry (deleted)"
                    );
                }
                self.install_size_worker_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                warn!(
                    target = "orchestrator",
                    "install size worker disconnected without a result \
 (thread panicked) — size stays —"
                );
                self.install_size_worker_rx = None;
            }
        }
    }

    fn poll_step2_channels(&mut self) {
        let seed_watch = AddedModSeedWatch {
            scan_live: self.step2_scan_rx.is_some(),
            log_apply_pending: self.wizard_state.step2.pending_saved_log_apply,
        };
        app_step2_scan::poll_step2_scan_events(
            &mut self.wizard_state,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
        );
        app_step2_update_check::poll_step2_update_check(
            &mut self.wizard_state,
            &mut self.step2_update_check_rx,
        );
        if app_step2_update_download::poll_step2_update_download(
            &mut self.wizard_state,
            &mut self.step2_update_download_rx,
        ) == DownloadPoll::Finished
        {
            self.after_download_finished();
        }
        let unpack_failures_before = self
            .wizard_state
            .step2
            .update_selected_extract_failed_sources
            .len();
        app_step2_update_extract::poll_step2_update_extract(
            &mut self.wizard_state,
            &mut self.step2_update_extract_rx,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
        );
        self.toast_new_unpack_failures(unpack_failures_before);
        crate::app::github_release_list::poll_release_list(
            &mut self.wizard_state,
            &mut self.release_list_rx,
        );
        crate::app::github_forks_list::poll_forks_sheet(&mut self.wizard_state, &mut self.forks_rx);
        Self::drain_archive_skip_events(
            &mut self.wizard_state,
            &mut self.archive_skip_rx,
            &mut self.install_screen_state,
            &self.hash_progress,
        );
        if self.nav == crate::ui::orchestrator::nav_destination::NavDestination::Install {
            crate::ui::install::stage_downloading::drain_manual_download_events(self);
        }
        self.start_deferred_extract_once();

        app_step2_saved_log_flow::advance_pending_saved_log_flow(
            &mut self.wizard_state,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
            &mut self.step2_update_check_rx,
            &mut self.step2_update_download_rx,
        );
        crate::ui::workspace::step2_log_glue::advance_pending_weidu_log_reapply(self);
        self.reseed_added_mods_when_settled(seed_watch);
    }

    fn toast_new_unpack_failures(&mut self, before: usize) {
        let entries: Vec<String> = self
            .wizard_state
            .step2
            .update_selected_extract_failed_sources
            .iter()
            .skip(before)
            .cloned()
            .collect();
        for entry in entries {
            self.notification_manager
                .error(format!("Could not unpack {entry}"));
        }
    }

    fn reseed_added_mods_when_settled(&mut self, watch: AddedModSeedWatch) {
        let pending_rebuilt = (watch.scan_live && self.step2_scan_rx.is_none())
            || (watch.log_apply_pending && !self.wizard_state.step2.pending_saved_log_apply);
        if pending_rebuilt {
            self.added_mods_seeded_for = None;
        }
        let NavDestination::Workspace {
            modlist_id: Some(id),
        } = &self.nav
        else {
            return;
        };
        let settled =
            self.step2_scan_rx.is_none() && !self.wizard_state.step2.pending_saved_log_apply;
        if !settled
            || self.workspace_view.loaded_workspace_id.as_deref() != Some(id.as_str())
            || self.added_mods_seeded_for.as_deref() == Some(id.as_str())
        {
            return;
        }
        let id = id.clone();
        crate::app::app_step2_log::reseed_added_mod_pending_downloads(&mut self.wizard_state);
        self.added_mods_seeded_for = Some(id);
    }

    fn start_pipeline_extract(&mut self) {
        let install_ctx_refs_path = self.install_ctx_refs_path();
        let started = app_step2_update_extract::start_step2_update_extract(
            &mut self.wizard_state,
            &mut self.step2_update_extract_rx,
            install_ctx_refs_path.as_deref(),
        );
        tracing::info!(target = "orchestrator", started, "pipeline extract start");
    }

    fn start_deferred_extract_once(&mut self) {
        if !self.install_screen_state.manual_downloads.extract_deferred
            || self
                .install_screen_state
                .manual_downloads
                .manual_hold_active()
        {
            return;
        }
        self.install_screen_state.manual_downloads.extract_deferred = false;
        self.wizard_state.step2.update_selected_extract_running = false;
        self.start_pipeline_extract();
    }

    fn install_ctx_refs_path(&self) -> Option<std::path::PathBuf> {
        self.active_install_modlist_id.as_deref().map(|id| {
            crate::registry::store_workspace::modlist_data_dir(id).join("mod_installed_refs.toml")
        })
    }

    pub(crate) fn after_download_finished(&mut self) {
        match self.wizard_state.step2.update_selected_download_origin {
            DownloadOrigin::InstallPipeline
                if self
                    .install_screen_state
                    .manual_downloads
                    .manual_hold_active() =>
            {
                self.install_screen_state.manual_downloads.extract_deferred = true;
                self.wizard_state.step2.update_selected_extract_running = true;
                tracing::info!(
                    target = "orchestrator",
                    "download finished; extract deferred until manual downloads resolve"
                );
            }
            DownloadOrigin::InstallPipeline => {
                tracing::info!(
                    target = "orchestrator",
                    "download finished; starting parallel extract"
                );
                self.start_pipeline_extract();
            }
            DownloadOrigin::Workspace => {
                let started = app_step2_update_extract::start_step2_update_extract(
                    &mut self.wizard_state,
                    &mut self.step2_update_extract_rx,
                    None,
                );
                if !started {
                    self.wizard_state.step2.pending_weidu_log_reapply = false;
                }
                tracing::info!(target = "orchestrator", started, "workspace extract start");
            }
        }
    }

    fn drain_archive_skip_events(
        wizard_state: &mut WizardState,
        archive_skip_rx: &mut Option<
            Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>,
        >,
        install_screen_state: &mut crate::ui::install::state_install::InstallScreenState,
        hash_progress: &Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    ) {
        use crate::install_runtime::archive_skip_async::ArchiveSkipEvent;
        use std::sync::mpsc::TryRecvError;

        let Some(rx) = archive_skip_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(ArchiveSkipEvent::CandidateEnumerated { total }) => {
                    if let Ok(mut g) = hash_progress.lock() {
                        *g = Some((0, total));
                    }
                }
                Ok(ArchiveSkipEvent::AssetHashStarted { .. }) => {}
                Ok(ArchiveSkipEvent::AssetHashed {
                    index,
                    was_skipped,
                    label,
                    dest_display,
                }) => {
                    if let Ok(mut g) = hash_progress.lock() {
                        let (c, t) = g.unwrap_or((0, 0));
                        *g = Some((c + 1, t));
                    }
                    install_screen_state.hashed_indices.insert(index);
                    if was_skipped && let Some(dest) = dest_display {
                        wizard_state
                            .step2
                            .update_selected_downloaded_sources
                            .push(format!("{label} -> {dest}"));
                    }
                }
                Ok(ArchiveSkipEvent::Finished {
                    summary,
                    skipped_indices,
                }) => {
                    *archive_skip_rx = None;
                    install_screen_state.skip_indices = skipped_indices.into_iter().collect();
                    install_screen_state
                        .pipeline_flags
                        .set_archive_skip_completed(true);
                    tracing::info!(
                        target = "orchestrator",
                        "async archive-skip finished: {} already-present, \
 {} missing (will fetch), {} no-expected-hash, \
 {} candidates hashed ({} persistent-cache hits)",
                        summary.skipped_present,
                        summary.missing_on_disk,
                        summary.no_expected_hash,
                        summary.hashed_candidates,
                        summary.cache_hits,
                    );
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    *archive_skip_rx = None;
                    install_screen_state
                        .pipeline_flags
                        .set_archive_skip_completed(true);
                    tracing::warn!(
                        target = "orchestrator",
                        "async archive-skip worker disconnected without \
 Finished — falling back to download-all"
                    );
                    return;
                }
            }
        }
    }

    fn poll_step5_before_render(&mut self) -> bool {
        let mut step5_requested_repaint = false;
        step5_requested_repaint |= app_step5_flow::poll_step5_terminal(
            &mut self.wizard_state,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
        );
        step5_requested_repaint |= app_step5_flow::poll_step5_prep(
            &mut self.wizard_state,
            &mut self.step5_prep_rx,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
            &mut self.step5_pending_start,
        );
        step5_requested_repaint
    }

    fn start_step5_after_render(&mut self) -> bool {
        app_step5_flow::start_if_requested(
            &mut self.wizard_state,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
            &mut self.step5_prep_rx,
            &mut self.step5_pending_start,
        )
    }

    fn step5_needs_repaint(&self) -> bool {
        self.step5_terminal
            .as_ref()
            .is_some_and(EmbeddedTerminal::has_new_data)
            || self.step5_prep_rx.is_some()
            || self.workspace_destination_prep_rx.is_some()
            || self.wizard_state.step5.prep_running
            || self.wizard_state.step5.install_running
            || self.wizard_state.modlist_auto_build_active
    }

    fn step2_needs_repaint(&self) -> bool {
        self.step2_scan_rx.is_some()
            || self.step2_update_check_rx.is_some()
            || self.step2_update_download_rx.is_some()
            || self.step2_update_extract_rx.is_some()
            || self.archive_skip_rx.is_some()
            || self.create_destination_prep_rx.is_some()
            || self.install_destination_prep_rx.is_some()
            || self.release_list_rx.is_some()
            || self.forks_rx.is_some()
            || self.wizard_state.modlist_auto_build_active
            || !self.step2_progress_queue.is_empty()
            || matches!(
                self.wizard_state.step2.versions_ui.release_list.status,
                crate::app::github_release_list::ReleaseListStatus::Loading
            )
            || matches!(
                self.wizard_state.step2.forks_list.status,
                crate::app::github_forks_list::ForksStatus::Loading
            )
    }

    fn sync_active_workspace_if_dirty(&mut self) {
        if !self.workspace_state_dirty {
            return;
        }
        self.workspace_state_dirty = DirtyFlag(false);

        if crate::ui::orchestrator::page_router::restore_pending(&self.workspace_view.step2) {
            return;
        }

        let Some(id) = self.workspace_view.loaded_workspace_id.clone() else {
            return;
        };

        self.persistence_cycle.note_workspace_extract();

        crate::ui::workspace::workspace_state_loader::sync_step3_from_step2_if_changed(
            &mut self.wizard_state,
        );

        let prior = self.workspace_state.get(&id).cloned().unwrap_or_default();
        let extracted =
            crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
                &self.wizard_state,
                &prior,
            );
        if extracted != prior {
            self.workspace_state.insert(id.clone(), extracted);
            self.persistence_cycle
                .mark_workspace_dirty(&id, Instant::now());
        }
    }

    fn tick_persistence(&mut self) {
        if self.registry_error.is_some() {
            return;
        }
        let now = Instant::now();
        if let Err(err) = self.persistence_cycle.persist_registry_if_needed(
            &self.registry,
            &self.registry_store,
            now,
        ) {
            warn!(
                target = "orchestrator",
                "persist_registry_if_needed failed: {err}"
            );
        }
        for (id, ws) in &self.workspace_state {
            let Some(store) = self.workspace_stores.get(id) else {
                continue;
            };
            if let Err(err) = self
                .persistence_cycle
                .persist_workspace_if_needed(id, ws, store, now)
            {
                warn!(
                    target = "orchestrator",
                    "persist_workspace_if_needed({id}) failed: {err}"
                );
            }
        }

        self.tick_bio_settings(now);
    }

    fn bio_settings_snapshot(&self) -> AppSettings {
        AppSettings {
            exe_fingerprint: self.exe_fingerprint.clone(),
            step1: self.wizard_state.step1.clone().into(),
            general: self.redesign_settings.clone(),
        }
    }

    fn tick_bio_settings(&mut self, now: Instant) {
        let snapshot = self.bio_settings_snapshot();
        if snapshot == self.bio_settings_last_saved {
            self.bio_settings_last_dirty_at = None;
            return;
        }
        self.bio_settings_last_dirty_at.get_or_insert(now);
        if let Some(at) = self.bio_settings_last_dirty_at
            && now.saturating_duration_since(at) >= Duration::from_millis(BIO_SETTINGS_DEBOUNCE_MS)
        {
            match self.settings_store.save(&snapshot) {
                Ok(()) => {
                    self.bio_settings_last_saved = snapshot;
                    self.bio_settings_last_dirty_at = None;
                }
                Err(err) => {
                    warn!(target = "orchestrator", "bio_settings save failed: {err}");
                }
            }
        }
    }

    fn flush_all_now(&mut self) {
        if self.registry_error.is_some() {
            return;
        }
        let errs = self.persistence_cycle.flush_all(
            &self.registry,
            &self.registry_store,
            &self.workspace_state,
            &self.workspace_stores,
        );
        for err in errs {
            warn!(target = "orchestrator", "flush_all error: {err}");
        }
        let bio_snapshot = self.bio_settings_snapshot();
        if bio_snapshot != self.bio_settings_last_saved {
            if let Err(err) = self.settings_store.save(&bio_snapshot) {
                warn!(target = "orchestrator", "bio_settings flush failed: {err}");
            } else {
                self.bio_settings_last_saved = bio_snapshot;
            }
        }
    }

    fn refresh_path_validation_status(&mut self) {
        self.path_validation = compute_path_validation_summary(&self.wizard_state);
        let issue_count = self
            .settings_screen_state
            .path_validation_results
            .issue_count;
        if issue_count > 0 && self.path_validation.kind == PathValidationKind::Ok {
            self.path_validation = PathValidationSummary {
                kind: PathValidationKind::Err(issue_count),
                text: format!("\u{00D7} {issue_count} path issues"),
            };
        }
    }
}

pub struct InstallPipelineResetSet<'a> {
    pub(crate) step2_update_download_rx:
        &'a mut Option<Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent>>,
    pub(crate) step2_update_extract_rx:
        &'a mut Option<Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>>,
    pub archive_skip_rx:
        &'a mut Option<Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>>,
    pub manual_download_rx:
        &'a mut Option<Receiver<crate::install_runtime::manual_download_watcher::WatchEvent>>,
    pub install_destination_prep_rx: &'a mut Option<PendingInstallDestinationPrep>,
    pub background_destination_prep_workers: &'a mut Vec<DestinationPrepJoinHandle>,
    pub wizard_state: &'a mut WizardState,
    pub install_screen_state: &'a mut InstallScreenState,
    pub hash_progress: &'a Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    pub pending_reinstall_id: &'a mut Option<String>,
    pub active_install_modlist_id: &'a mut Option<String>,
}

pub fn reset_install_pipeline_state(set: InstallPipelineResetSet<'_>) {
    let InstallPipelineResetSet {
        step2_update_download_rx,
        step2_update_extract_rx,
        archive_skip_rx,
        manual_download_rx,
        install_destination_prep_rx,
        background_destination_prep_workers,
        wizard_state,
        install_screen_state,
        hash_progress,
        pending_reinstall_id,
        active_install_modlist_id,
    } = set;

    if wizard_state.step2.update_selected_download_origin == DownloadOrigin::InstallPipeline {
        *step2_update_download_rx = None;
        *step2_update_extract_rx = None;
    }
    *archive_skip_rx = None;
    *manual_download_rx = None;
    if let Some(pending) = install_destination_prep_rx.take() {
        hold_destination_prep_worker_for_shutdown(
            background_destination_prep_workers,
            pending.worker,
        );
    }

    wizard_state.modlist_auto_build_active = false;
    wizard_state.modlist_auto_build_waiting_for_install = false;
    wizard_state.step2.pending_saved_log_apply = false;
    wizard_state.step2.pending_saved_log_update_preview = false;
    wizard_state.step2.pending_saved_log_download = false;
    wizard_state.step2.update_selected_download_running = false;
    wizard_state.step2.update_selected_download_bytes.clear();
    wizard_state.step2.update_selected_download_done.clear();
    wizard_state.step2.update_selected_download_finished.clear();
    wizard_state.step2.update_selected_extract_running = false;

    install_screen_state.clear_preview();
    install_screen_state.pipeline_kind = crate::ui::install::state_install::PipelineKind::Install;
    install_screen_state.stage = crate::ui::install::state_install::InstallStage::Gallery;
    install_screen_state.manual_downloads =
        crate::ui::install::state_install::ManualDownloadsState::default();

    if let Ok(mut g) = hash_progress.lock() {
        *g = None;
    }
    wizard_state.step2.update_selected_extract_progress = None;
    wizard_state.step2.update_selected_extract_jobs.clear();

    *pending_reinstall_id = None;
    *active_install_modlist_id = None;
}

pub(crate) fn source_probe_deferred(debounce: &HashMap<&'static str, Instant>) -> bool {
    debounce
        .keys()
        .any(|field| crate::ui::settings::validate_now::is_game_folder_field(field))
}

fn refresh_source_compatibility(app: &mut OrchestratorApp) {
    if source_probe_deferred(&app.settings_screen_state.path_edit_debounce) {
        return;
    }
    if !crate::app::compat_dlc_source::refresh_source_check(&mut app.wizard_state.step1) {
        return;
    }
    if let Some(err) = crate::app::compat_logic::apply_step2_compat_rules(
        &app.wizard_state.step1,
        &mut app.wizard_state.step2.bgee_mods,
        &mut app.wizard_state.step2.bg2ee_mods,
    ) {
        app.wizard_state.step2.scan_status = format!("Compat rules load failed: {err}");
    }
    crate::ui::install::page_install::refresh_source_compat_issue(
        &mut app.install_screen_state,
        &app.wizard_state.step1,
    );
}

impl eframe::App for OrchestratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = self.theme_palette;
        ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(palette));
        self.sync_diagnostic_mode(ctx);

        validate_debounce::tick(self, Instant::now());
        refresh_source_compatibility(self);
        if let Some(next_due_in) = next_debounce_due_in(self) {
            ctx.request_repaint_after(next_due_in);
        }

        oauth_glue::poll_github_oauth_flow(self);

        self.poll_step2_channels();
        crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(self);
        if self.step2_needs_repaint() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        let install_was_running = self.wizard_state.step5.install_running;
        let mut step5_requested_repaint = self.poll_step5_before_render();
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
            self.install_running_since = Some(Instant::now());
        }
        if install_was_running && !self.wizard_state.step5.install_running {
            self.install_running_since = None;
            self.maybe_flip_to_installed_on_clean_exit();
        }

        self.refresh_path_validation_status();

        let modlist_count = self.registry.entries.len();

        let running = install_concurrency::install_in_progress(self);
        let rail_lock: Option<RailLockReason> = running.as_ref().map(|r| {
            let modlist_label = self
                .registry
                .find(&r.modlist_id)
                .map_or_else(|| r.modlist_id.clone(), |e| e.name.clone());
            RailLockReason::InstallRunning {
                modlist_id: r.modlist_id.clone(),
                modlist_label,
                started_at: r.started_at,
            }
        });
        let running_status: Option<RunningInstallStatus> = running.as_ref().map(|r| {
            let modlist_name = self
                .registry
                .find(&r.modlist_id)
                .map_or_else(|| r.modlist_id.clone(), |e| e.name.clone());
            RunningInstallStatus {
                modlist_name,
                elapsed: r.started_at.elapsed(),
            }
        });

        let history_has_items = self.notification_manager.has_history();
        let history_open = self.notification_manager.history_open;
        let history_clicked = shell_chrome::render_shell(
            ctx,
            palette,
            modlist_count,
            running_status.as_ref(),
            history_has_items,
            history_open,
            |ui| {
                egui::SidePanel::left("orchestrator_left_rail")
                    .exact_width(REDESIGN_NAV_WIDTH_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        left_rail::render(
                            ui,
                            palette,
                            &mut self.nav,
                            self.dev_mode,
                            &self.path_validation,
                            rail_lock.as_ref(),
                        );
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                        left: 28,
                        right: 28,
                        top: 24,
                        bottom: 24,
                    }))
                    .show_inside(ui, |ui| {
                        page_router::render(ui, self, ctx);
                    });
            },
        );
        self.drive_notifications(ctx, palette, history_clicked);

        oauth_glue::render_github_popup_if_open(self, ctx);

        step5_requested_repaint |= self.start_step5_and_check_focus();
        schedule_repaint_if_needed(
            ctx,
            step5_requested_repaint,
            self.step5_needs_repaint(),
            self.slow_workers_active(),
        );

        self.drain_background_workers();

        self.sync_active_workspace_if_dirty();

        self.tick_persistence();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.join_all_destination_prep_workers();
        self.flush_all_now();
    }
}

#[cfg(test)]
impl OrchestratorApp {
    pub(crate) fn new_isolated_for_test(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ISOLATED_TEST_SEQ: AtomicU64 = AtomicU64::new(0);
        let stem = format!(
            "bio_{tag}_{}_{}",
            std::process::id(),
            ISOLATED_TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir();
        let mut app = Self::new(false);
        app.registry_store =
            RegistryStore::new_with_path(dir.join(format!("{stem}_registry.json")));
        app.registry = ModlistRegistry::default();
        app.settings_store = crate::settings::store::SettingsStore::new_with_path(
            dir.join(format!("{stem}_settings.json")),
        );
        app.wizard_state.step1 = crate::app::state::Step1State::default();
        app.path_validation = compute_path_validation_summary(&app.wizard_state);
        app.bio_settings_last_saved = AppSettings {
            exe_fingerprint: app.exe_fingerprint.clone(),
            step1: app.wizard_state.step1.clone().into(),
            general: app.redesign_settings.clone(),
        };
        let config_root = dir.join(format!("{stem}_config"));
        crate::platform_defaults::set_config_dir_override(Some(config_root.clone()));
        app.isolated_test_config_root = Some(config_root);
        app
    }

    fn cleanup_isolated_test_config_root(&mut self) {
        if let Some(root) = self.isolated_test_config_root.take() {
            let _ = std::fs::remove_dir_all(&root);
            crate::platform_defaults::clear_config_dir_override_if(&root);
            let _ = std::fs::remove_file(self.registry_store.path());
            let _ = std::fs::remove_file(self.settings_store.path());
        }
    }
}

impl Drop for OrchestratorApp {
    fn drop(&mut self) {
        self.join_all_destination_prep_workers();
        self.flush_all_now();
        #[cfg(test)]
        self.cleanup_isolated_test_config_root();
    }
}

impl OrchestratorApp {
    fn drive_notifications(
        &mut self,
        ctx: &egui::Context,
        palette: ThemePalette,
        history_clicked: bool,
    ) {
        for msg in clipboard::take_pending_toasts(ctx) {
            self.notification_manager.success(msg);
        }
        for warning in crate::app::modlist_share::take_pending_warnings() {
            self.notification_manager.warn(warning);
        }
        if help_button::take_export_request(ctx) {
            self.export_diagnostics_from_help();
        }
        if history_clicked {
            self.notification_manager.history_open = !self.notification_manager.history_open;
        }
        self.notification_manager.show(ctx, palette);
        self.notification_manager
            .render_history_popup(ctx, palette, !history_clicked);
    }

    fn sync_diagnostic_mode(&mut self, ctx: &egui::Context) {
        help_button::publish_diagnostic_mode(ctx, self.dev_mode);
        crate::ui::step5::service_diagnostics_support_step5::apply_diagnostic_log_level(
            &mut self.wizard_state.step1,
            self.dev_mode,
            self.dev_mode_cli_flag,
        );
    }

    fn export_diagnostics_from_help(&mut self) {
        let exe_fingerprint = self.exe_fingerprint.clone();
        let result = crate::ui::step5::service_diagnostics_support_step5::export_diagnostics(
            &self.wizard_state,
            self.step5_terminal.as_ref(),
            self.dev_mode,
            &exe_fingerprint,
        );
        match result {
            Ok(path) => {
                self.notification_manager
                    .success(format!("Diagnostics exported: {}", path.display()));
            }
            Err(err) => {
                self.notification_manager
                    .error(format!("Diagnostics export failed: {err}"));
            }
        }
    }
}

fn schedule_repaint_if_needed(
    ctx: &egui::Context,
    step5_repaint: bool,
    step5_needs: bool,
    slow_worker_active: bool,
) {
    if step5_repaint || step5_needs {
        ctx.request_repaint_after(Duration::from_millis(16));
    } else if slow_worker_active {
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

fn next_debounce_due_in(app: &OrchestratorApp) -> Option<std::time::Duration> {
    let threshold =
        std::time::Duration::from_millis(crate::ui::settings::validate_debounce::DEBOUNCE_MS);
    let now = Instant::now();
    app.settings_screen_state
        .path_edit_debounce
        .values()
        .map(|at| {
            let elapsed = now.saturating_duration_since(*at);
            threshold.saturating_sub(elapsed)
        })
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::TryRecvError;

    #[test]
    fn source_probe_waits_for_the_bgee_source_debounce() {
        let empty = HashMap::new();
        assert!(!source_probe_deferred(&empty));

        let mut with_bgee_source = HashMap::new();
        with_bgee_source.insert(
            crate::ui::settings::validate_now::FIELD_BGEE_GAME_FOLDER,
            Instant::now(),
        );
        assert!(source_probe_deferred(&with_bgee_source));

        let mut with_other_field = HashMap::new();
        with_other_field.insert(
            crate::ui::settings::validate_now::FIELD_GLOBAL_MODS_FOLDER,
            Instant::now(),
        );
        assert!(!source_probe_deferred(&with_other_field));

        let mut with_eet_bgee_source = HashMap::new();
        with_eet_bgee_source.insert(
            crate::ui::settings::validate_now::FIELD_EET_BGEE_GAME_FOLDER,
            Instant::now(),
        );
        assert!(source_probe_deferred(&with_eet_bgee_source));

        for field in [
            crate::ui::settings::validate_now::FIELD_BG2EE_GAME_FOLDER,
            crate::ui::settings::validate_now::FIELD_IWDEE_GAME_FOLDER,
            crate::ui::settings::validate_now::FIELD_EET_BG2EE_GAME_FOLDER,
        ] {
            let mut with_field = HashMap::new();
            with_field.insert(field, Instant::now());
            assert!(source_probe_deferred(&with_field), "{field}");
        }
    }

    #[test]
    fn isolated_test_app_flushes_settings_to_temp_not_the_config_dir() {
        let probe = format!("isolation-probe-{}", std::process::id());
        let real_before = SettingsStore::new_default()
            .load()
            .map(|s| s.general.user_name)
            .unwrap_or_default();
        let mut app = OrchestratorApp::new_isolated_for_test("isolationtest");
        app.redesign_settings.user_name.clone_from(&probe);
        app.flush_all_now();
        let isolated = app.settings_store.load().expect("temp store loads");
        assert_eq!(isolated.general.user_name, probe);
        drop(app);
        let real_after = SettingsStore::new_default()
            .load()
            .map(|s| s.general.user_name)
            .unwrap_or_default();
        assert_eq!(real_after, real_before);
        assert_ne!(real_after, probe);
    }

    #[test]
    fn isolated_app_carries_no_machine_paths() {
        let app = OrchestratorApp::new_isolated_for_test("no_machine_paths");
        assert_eq!(app.wizard_state.step1.bgee_game_folder.len(), 0);
        assert_eq!(app.wizard_state.step1.bg2ee_game_folder.len(), 0);
        assert_eq!(app.wizard_state.step1.eet_pre_dir.len(), 0);
        assert_eq!(app.wizard_state.step1.eet_new_dir.len(), 0);
        assert_eq!(app.wizard_state.step1.mods_folder.len(), 0);
        assert_eq!(
            app.wizard_state.step1.prepare_target_dirs_before_install,
            crate::app::state::Step1State::default().prepare_target_dirs_before_install
        );
    }

    #[test]
    fn dropping_an_isolated_app_clears_its_config_root() {
        let app = OrchestratorApp::new_isolated_for_test("root-clear");
        let root = app
            .isolated_test_config_root
            .clone()
            .expect("isolated app carries a config root");
        assert!(
            crate::registry::store_workspace::modlist_data_dir("X").starts_with(&root),
            "the root is active while the app is alive"
        );

        drop(app);

        assert!(
            !crate::registry::store_workspace::modlist_data_dir("X").starts_with(&root),
            "the root is cleared once the app that owns it drops"
        );
        assert!(!root.exists());
    }

    fn dirty_ws() -> WizardState {
        let mut ws = WizardState {
            modlist_auto_build_active: true,
            modlist_auto_build_waiting_for_install: true,
            ..Default::default()
        };
        ws.step2.pending_saved_log_apply = true;
        ws.step2.pending_saved_log_update_preview = true;
        ws.step2.pending_saved_log_download = true;
        ws.step2.update_selected_download_running = true;
        ws.step2.update_selected_extract_running = true;
        ws.step2
            .update_selected_download_bytes
            .insert(0, (10, Some(20)));
        ws.step2.update_selected_download_done.insert(0);
        ws.step2.update_selected_extract_progress = Some((5, 51));
        ws.step2
            .update_selected_extract_jobs
            .entry("mod".to_string())
            .or_default()
            .done = 1;
        ws
    }

    fn dirty_iss() -> InstallScreenState {
        let mut iss = InstallScreenState {
            stage: crate::ui::install::state_install::InstallStage::Downloading,
            pipeline_kind: crate::ui::install::state_install::PipelineKind::Fork,
            ..Default::default()
        };
        iss.pipeline_flags.set_armed(true);
        iss.pipeline_flags.set_archives_staged(true);
        iss.pipeline_flags.set_archive_skip_completed(true);
        iss.pipeline_flags.set_download_phase_started(true);
        iss.pipeline_flags.set_archives_verified(true);
        iss.download_progress.hash_progress = Some((10, 51));
        iss.download_progress.extract_progress = Some((5, 51));
        iss.hashed_indices.insert(0);
        iss.hashed_indices.insert(3);
        iss
    }

    fn blocking_destination_prep_worker() -> (
        crate::install_runtime::destination_prep::DestinationPrepWorker,
        std::sync::mpsc::Sender<()>,
    ) {
        let (result_sender, result_receiver) = std::sync::mpsc::channel::<
            crate::install_runtime::destination_prep::DestinationPrepResult,
        >();
        let (release_worker, worker_released) = std::sync::mpsc::channel::<()>();
        let worker =
            crate::install_runtime::destination_prep::DestinationPrepWorker::from_parts_for_test(
                result_receiver,
                std::thread::spawn(move || {
                    let _ = worker_released.recv();
                    let _ = result_sender.send(Ok(
                        crate::install_runtime::destination_prep::DestinationPrepReport::Skipped {
                            reason: crate::install_runtime::destination_prep::SkipReason::Continue,
                        },
                    ));
                }),
            );
        (worker, release_worker)
    }

    fn assert_pipeline_channels_closed(
        s_dl: &std::sync::mpsc::Sender<
            crate::app::app_step2_update_download::Step2UpdateDownloadEvent,
        >,
        s_sk: &std::sync::mpsc::Sender<
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent,
        >,
        s_ex: &std::sync::mpsc::Sender<
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent,
        >,
    ) {
        assert!(
            s_dl.send(
                crate::app::app_step2_update_download::Step2UpdateDownloadEvent::Finished(
                    crate::app::app_step2_update_download::Step2UpdateDownloadResult::default()
                )
            )
            .is_err()
        );
        assert!(
            s_sk.send(
                crate::install_runtime::archive_skip_async::ArchiveSkipEvent::CandidateEnumerated {
                    total: 1
                }
            )
            .is_err()
        );
        assert!(
            s_ex.send(
                crate::app::app_step2_update_extract::Step2UpdateExtractEvent::AssetDone {
                    index: 0,
                    ok: true,
                    label: "MOD".to_string(),
                    tp_file: "MOD/MOD.TP2".to_string(),
                    target_or_err: "C:/x".to_string(),
                }
            )
            .is_err()
        );
    }

    #[test]
    fn reset_install_pipeline_state_drops_all_receivers_and_clears_wizard_latches() {
        let (s_dl, r_dl) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_download::Step2UpdateDownloadEvent,
        >();
        let (s_sk, r_sk) = std::sync::mpsc::channel::<
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent,
        >();
        let (s_ex, r_ex) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent,
        >();
        let (destination_prep_worker, release_worker) = blocking_destination_prep_worker();
        let mut stream = Some(r_dl);
        let mut skip = Some(r_sk);
        let mut extract = Some(r_ex);
        let mut manual_dl: Option<
            Receiver<crate::install_runtime::manual_download_watcher::WatchEvent>,
        > = None;
        let mut background_destination_prep_workers = Vec::new();
        let mut dest_prep = Some(PendingInstallDestinationPrep {
            token: DestinationPrepToken::new(
                1,
                DestinationPrepFlow::InstallPipeline,
                r"D:\target",
                None,
            ),
            destination: r"D:\target".to_string(),
            game: Game::BGEE,
            workflow: InstallWorkflow::PasteAndInstall,
            code: "BIO-MODLIST-V1:test".to_string(),
            worker: destination_prep_worker,
        });
        let mut ws = dirty_ws();
        ws.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        let mut iss = dirty_iss();
        let hash = Arc::new(std::sync::Mutex::new(Some((10usize, 51usize))));
        let mut pending = Some("modlist-id".to_string());
        let mut active = Some("modlist-id".to_string());

        reset_install_pipeline_state(InstallPipelineResetSet {
            step2_update_download_rx: &mut stream,
            step2_update_extract_rx: &mut extract,
            archive_skip_rx: &mut skip,
            manual_download_rx: &mut manual_dl,
            install_destination_prep_rx: &mut dest_prep,
            background_destination_prep_workers: &mut background_destination_prep_workers,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            pending_reinstall_id: &mut pending,
            active_install_modlist_id: &mut active,
        });

        assert!(stream.is_none(), "step2_update_download_rx dropped");
        assert!(skip.is_none(), "archive_skip_rx dropped");
        assert!(extract.is_none(), "step2_update_extract_rx dropped");
        assert!(dest_prep.is_none(), "install_destination_prep_rx dropped");
        assert_eq!(
            background_destination_prep_workers.len(),
            1,
            "running destination prep worker retained for shutdown join"
        );

        assert_pipeline_channels_closed(&s_dl, &s_sk, &s_ex);
        drop((s_dl, s_sk, s_ex));
        release_worker
            .send(())
            .expect("release destination prep worker");
        for handle in background_destination_prep_workers {
            join_destination_prep_handle(handle);
        }

        assert!(!ws.modlist_auto_build_active);
        assert!(!ws.modlist_auto_build_waiting_for_install);
        assert!(!ws.step2.pending_saved_log_apply);
        assert!(!ws.step2.pending_saved_log_update_preview);
        assert!(!ws.step2.pending_saved_log_download);
        assert!(!ws.step2.update_selected_download_running);
        assert!(!ws.step2.update_selected_extract_running);
        assert_eq!(ws.step2.update_selected_download_bytes.len(), 0);
        assert_eq!(ws.step2.update_selected_download_done.len(), 0);
        assert_eq!(ws.step2.update_selected_extract_jobs.len(), 0);

        assert!(pending.is_none());
        assert!(active.is_none());
    }

    #[test]
    fn reset_install_pipeline_state_clears_screen_state_and_shared_progress_mutexes() {
        let mut stream: Option<
            Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent>,
        > = None;
        let mut skip: Option<
            Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>,
        > = None;
        let mut extract: Option<
            Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>,
        > = None;
        let mut manual_dl: Option<
            Receiver<crate::install_runtime::manual_download_watcher::WatchEvent>,
        > = None;
        let mut dest_prep: Option<PendingInstallDestinationPrep> = None;
        let mut background_destination_prep_workers = Vec::new();
        let mut ws = dirty_ws();
        let mut iss = dirty_iss();
        let hash = Arc::new(std::sync::Mutex::new(Some((10usize, 51usize))));
        let mut pending: Option<String> = None;
        let mut active: Option<String> = None;

        reset_install_pipeline_state(InstallPipelineResetSet {
            step2_update_download_rx: &mut stream,
            step2_update_extract_rx: &mut extract,
            archive_skip_rx: &mut skip,
            manual_download_rx: &mut manual_dl,
            install_destination_prep_rx: &mut dest_prep,
            background_destination_prep_workers: &mut background_destination_prep_workers,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            pending_reinstall_id: &mut pending,
            active_install_modlist_id: &mut active,
        });

        assert!(!iss.pipeline_flags.armed());
        assert!(!iss.pipeline_flags.archives_staged());
        assert!(!iss.pipeline_flags.archive_skip_completed());
        assert!(!iss.pipeline_flags.download_phase_started());
        assert!(!iss.pipeline_flags.archives_verified());
        assert!(iss.download_progress.hash_progress.is_none());
        assert!(iss.download_progress.extract_progress.is_none());
        assert_eq!(iss.hashed_indices.len(), 0);
        assert_eq!(
            iss.stage,
            crate::ui::install::state_install::InstallStage::Gallery
        );
        assert_eq!(
            iss.pipeline_kind,
            crate::ui::install::state_install::PipelineKind::Install,
            "a cancelled or completed run must not leave the screen armed as a fork"
        );

        assert!(hash.lock().unwrap().is_none(), "shared hash mutex blanked");
        assert_eq!(
            ws.step2.update_selected_extract_progress, None,
            "the extract progress field is blanked"
        );
        assert!(
            ws.step2.update_selected_extract_jobs.is_empty(),
            "the per-mod extract tally is blanked"
        );
    }

    #[test]
    fn composed_cancel_drains_all_three_event_streams_after_reset() {
        let (s_dl, r_dl) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_download::Step2UpdateDownloadEvent,
        >();
        let (s_sk, r_sk) = std::sync::mpsc::channel::<
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent,
        >();
        let (s_ex, r_ex) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent,
        >();
        let _ = s_dl.send(
            crate::app::app_step2_update_download::Step2UpdateDownloadEvent::AssetProgress {
                index: 0,
                bytes: 100,
                total: Some(1000),
            },
        );
        let _ = s_sk.send(
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent::AssetHashStarted {
                index: 0,
            },
        );
        let _ = s_ex.send(
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent::AssetDone {
                index: 0,
                ok: true,
                label: "MOD".to_string(),
                tp_file: "MOD/MOD.TP2".to_string(),
                target_or_err: "C:/x".to_string(),
            },
        );
        let mut stream = Some(r_dl);
        let mut skip = Some(r_sk);
        let mut extract = Some(r_ex);
        let mut manual_dl: Option<
            Receiver<crate::install_runtime::manual_download_watcher::WatchEvent>,
        > = None;
        let mut dest_prep: Option<PendingInstallDestinationPrep> = None;
        let mut background_destination_prep_workers = Vec::new();
        let mut ws = WizardState::default();
        ws.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        let mut iss = InstallScreenState::default();
        let hash = Arc::new(std::sync::Mutex::new(None));
        let mut pending = None;
        let mut active = None;
        reset_install_pipeline_state(InstallPipelineResetSet {
            step2_update_download_rx: &mut stream,
            step2_update_extract_rx: &mut extract,
            archive_skip_rx: &mut skip,
            manual_download_rx: &mut manual_dl,
            install_destination_prep_rx: &mut dest_prep,
            background_destination_prep_workers: &mut background_destination_prep_workers,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            pending_reinstall_id: &mut pending,
            active_install_modlist_id: &mut active,
        });
        assert!(stream.is_none());
        assert!(skip.is_none());
        assert!(extract.is_none());
        assert!(
            s_dl.send(
                crate::app::app_step2_update_download::Step2UpdateDownloadEvent::Finished(
                    crate::app::app_step2_update_download::Step2UpdateDownloadResult::default()
                )
            )
            .is_err()
        );
        let _ = TryRecvError::Empty;
    }

    #[test]
    fn drain_runs_from_orchestrator_frame_path() {
        struct DrainTestRoot {
            path: std::path::PathBuf,
        }
        impl DrainTestRoot {
            fn new() -> Self {
                use std::sync::atomic::{AtomicU64, Ordering};
                static COUNTER: AtomicU64 = AtomicU64::new(0);
                let path = std::env::temp_dir().join(format!(
                    "bio_manualdl_{}_{}_orchdrain",
                    std::process::id(),
                    COUNTER.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self { path }
            }
        }
        impl Drop for DrainTestRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }

        let root = DrainTestRoot::new();
        let dropped = root.path.join("Ascension.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app = OrchestratorApp::new_isolated_for_test("manualdl-drain-frame");
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![crate::app::state::ManualDownloadRequest {
                game_tab: "BGEE".to_string(),
                tp_file: "ascension/setup-ascension.tp2".to_string(),
                label: "Ascension".to_string(),
                source_id: String::new(),
                page_url: String::new(),
                reason: crate::app::state::ManualDownloadReason::NotAutoResolvable,
                aliases: Vec::new(),
                display_name: String::new(),
            }];
        app.install_screen_state.manual_downloads.rows =
            vec![crate::ui::install::state_install::ManualDownloadRow {
                label: "Ascension".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: crate::ui::install::state_install::ManualRowStatus::Waiting,
            }];

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(
            crate::install_runtime::manual_download_watcher::WatchEvent::Candidate(
                crate::install_runtime::manual_archive_probe::ArchiveProbe {
                    path: dropped.clone(),
                    file_name: "Ascension.zip".to_string(),
                    size: std::fs::metadata(&dropped).unwrap().len(),
                    hash: None,
                    tp2_names: vec!["setup-ascension.tp2".to_string()],
                    format: crate::install_runtime::manual_archive_probe::ProbeFormat::Zip,
                },
            ),
        )
        .expect("send candidate");
        drop(tx);
        app.manual_download_rx = Some(rx);
        app.nav = crate::ui::orchestrator::nav_destination::NavDestination::Install;

        app.poll_step2_channels();

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].status,
            crate::ui::install::state_install::ManualRowStatus::Found,
            "the orchestrator's per-frame drain entry matches a seeded Candidate"
        );
    }

    #[test]
    fn drain_waits_while_not_on_install_page() {
        struct DrainTestRoot {
            path: std::path::PathBuf,
        }
        impl DrainTestRoot {
            fn new() -> Self {
                use std::sync::atomic::{AtomicU64, Ordering};
                static COUNTER: AtomicU64 = AtomicU64::new(0);
                let path = std::env::temp_dir().join(format!(
                    "bio_manualdl_{}_{}_orchdrainwaits",
                    std::process::id(),
                    COUNTER.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path).unwrap();
                Self { path }
            }
        }
        impl Drop for DrainTestRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }

        let root = DrainTestRoot::new();
        let dropped = root.path.join("Ascension.zip");
        std::fs::write(&dropped, b"content").unwrap();
        let archive_dir = root.path.join("archives");

        let mut app = OrchestratorApp::new_isolated_for_test("manualdl-drain-waits");
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![crate::app::state::ManualDownloadRequest {
                game_tab: "BGEE".to_string(),
                tp_file: "ascension/setup-ascension.tp2".to_string(),
                label: "Ascension".to_string(),
                source_id: String::new(),
                page_url: String::new(),
                reason: crate::app::state::ManualDownloadReason::NotAutoResolvable,
                aliases: Vec::new(),
                display_name: String::new(),
            }];
        app.install_screen_state.manual_downloads.rows =
            vec![crate::ui::install::state_install::ManualDownloadRow {
                label: "Ascension".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: crate::ui::install::state_install::ManualRowStatus::Waiting,
            }];

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(
            crate::install_runtime::manual_download_watcher::WatchEvent::Candidate(
                crate::install_runtime::manual_archive_probe::ArchiveProbe {
                    path: dropped.clone(),
                    file_name: "Ascension.zip".to_string(),
                    size: std::fs::metadata(&dropped).unwrap().len(),
                    hash: None,
                    tp2_names: vec!["setup-ascension.tp2".to_string()],
                    format: crate::install_runtime::manual_archive_probe::ProbeFormat::Zip,
                },
            ),
        )
        .expect("send candidate");
        drop(tx);
        app.manual_download_rx = Some(rx);
        app.nav = crate::ui::orchestrator::nav_destination::NavDestination::Home;

        app.poll_step2_channels();

        assert_eq!(
            app.install_screen_state.manual_downloads.rows[0].status,
            crate::ui::install::state_install::ManualRowStatus::Waiting,
            "a match landing while another page is open waits in the channel"
        );
        assert!(
            app.manual_download_rx.is_some(),
            "the receiver stays open while the Install page is not showing"
        );
    }

    #[test]
    fn stream_finish_during_hold_defers_extract() {
        let mut app = OrchestratorApp::new_isolated_for_test("manualdl-defer-finish");
        app.install_screen_state.manual_downloads.rows =
            vec![crate::ui::install::state_install::ManualDownloadRow {
                label: "Ascension".to_string(),
                from: "nexusmods.com".to_string(),
                page_url: String::new(),
                status: crate::ui::install::state_install::ManualRowStatus::Waiting,
            }];

        app.step2_update_download_rx = Some(finished_download_channel());
        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        app.wizard_state.modlist_auto_build_active = true;
        app.wizard_state.modlist_auto_build_waiting_for_install = true;

        app.poll_step2_channels();
        app.poll_step2_channels();

        assert!(
            app.install_screen_state.manual_downloads.extract_deferred,
            "the deferred mark is set while the hold is active"
        );
        assert!(
            app.wizard_state.step2.update_selected_extract_running,
            "the deferred window reads as extraction busy"
        );
        assert!(
            app.wizard_state.modlist_auto_build_active,
            "the auto build does not finish or stop while extraction is deferred"
        );
        assert_eq!(
            app.wizard_state.current_step, 0,
            "the pipeline does not route to the install step before extraction"
        );
        assert!(
            app.step2_update_download_rx.is_none(),
            "the download receiver is consumed on Finished"
        );
        assert!(
            app.step2_update_extract_rx.is_none(),
            "extraction is not started while a manual row still waits"
        );

        app.start_deferred_extract_once();
        assert!(
            app.install_screen_state.manual_downloads.extract_deferred,
            "the mark stays set while the row still waits"
        );

        app.install_screen_state.manual_downloads.rows[0].status =
            crate::ui::install::state_install::ManualRowStatus::Found;
        app.start_deferred_extract_once();
        assert!(
            !app.install_screen_state.manual_downloads.extract_deferred,
            "the mark clears once the row is found"
        );
        assert!(
            !app.wizard_state.step2.update_selected_extract_running,
            "with nothing to extract the busy flag is released with the mark"
        );
    }

    #[test]
    fn stream_finish_without_hold_never_defers() {
        let mut app = OrchestratorApp::new_isolated_for_test("manualdl-no-hold-finish");

        app.step2_update_download_rx = Some(finished_download_channel());
        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;

        app.poll_step2_channels();

        assert!(
            !app.install_screen_state.manual_downloads.extract_deferred,
            "no manual rows means no hold, so extraction is not deferred"
        );
        assert!(app.step2_update_download_rx.is_none());
        assert!(!app.wizard_state.step2.update_selected_extract_running);
    }

    fn finished_download_channel()
    -> Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(
            crate::app::app_step2_update_download::Step2UpdateDownloadEvent::Finished(
                crate::app::app_step2_update_download::Step2UpdateDownloadResult::default(),
            ),
        )
        .expect("send Finished");
        rx
    }

    struct EngineFinishRoot {
        path: std::path::PathBuf,
    }

    impl EngineFinishRoot {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = Self {
                path: std::env::temp_dir().join(format!(
                    "bio_dl_engine_{}_{}_orchfinish",
                    std::process::id(),
                    COUNTER.fetch_add(1, Ordering::Relaxed)
                )),
            };
            std::fs::create_dir_all(&root.path).unwrap();
            root
        }
    }

    impl Drop for EngineFinishRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn waiting_manual_row() -> crate::ui::install::state_install::ManualDownloadRow {
        crate::ui::install::state_install::ManualDownloadRow {
            label: "Ascension".to_string(),
            from: "nexusmods.com".to_string(),
            page_url: String::new(),
            status: crate::ui::install::state_install::ManualRowStatus::Waiting,
        }
    }

    #[test]
    fn workspace_origin_finish_consumes_the_scope_and_never_defers() {
        let root = EngineFinishRoot::new();
        let mut app = OrchestratorApp::new_isolated_for_test("dl-engine-workspace-finish");
        app.wizard_state.step1.mods_archive_folder =
            root.path.join("archives").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::Workspace;
        app.wizard_state.step2.update_selected_download_scope = Some("alpha".to_string());
        app.install_screen_state.manual_downloads.rows = vec![waiting_manual_row()];
        app.step2_update_download_rx = Some(finished_download_channel());

        app.poll_step2_channels();

        assert!(app.step2_update_download_rx.is_none());
        assert_eq!(
            app.wizard_state.step2.update_selected_download_scope, None,
            "the workspace extract start consumed the drawer scope"
        );
        assert!(
            !app.install_screen_state.manual_downloads.extract_deferred,
            "the workspace origin never waits on the pipeline's manual hold"
        );
        assert!(
            app.step2_update_extract_rx.is_none(),
            "a blank Mods Folder plans no job, so no extractor starts"
        );
        assert!(!app.wizard_state.step2.update_selected_extract_running);
    }

    #[test]
    fn reset_keeps_a_workspace_run_receiver() {
        let mut app = OrchestratorApp::new_isolated_for_test("reset-keeps-workspace-rx");
        let (_workspace_tx, workspace_rx) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent,
        >();
        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::Workspace;
        app.step2_update_extract_rx = Some(workspace_rx);

        app.reset_install_screen_to_gallery();

        assert!(app.step2_update_extract_rx.is_some());

        let (_pipeline_tx, pipeline_rx) = std::sync::mpsc::channel::<
            crate::app::app_step2_update_extract::Step2UpdateExtractEvent,
        >();
        app.wizard_state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        app.step2_update_extract_rx = Some(pipeline_rx);

        app.reset_install_screen_to_gallery();

        assert!(app.step2_update_extract_rx.is_none());
    }

    #[test]
    fn pipeline_nothing_to_download_refusal_kicks_the_pipeline_extract() {
        let root = EngineFinishRoot::new();
        let archive_dir = root.path.join("archives");
        std::fs::create_dir_all(&archive_dir).unwrap();
        let asset = crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: "cached/setup-cached.tp2".to_string(),
            label: "Cached".to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: "cached.zip".to_string(),
            asset_url: "http://127.0.0.1:9/cached.zip".to_string(),
            installed_source_ref: None,
        };
        std::fs::write(
            archive_dir.join(crate::app::app_step2_update_download::archive_file_name(
                &asset,
            )),
            b"not-a-real-archive",
        )
        .unwrap();

        let mut app = OrchestratorApp::new_isolated_for_test("dl-engine-pipeline-all-cached");
        app.wizard_state.step1.download_archive = true;
        app.wizard_state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        app.wizard_state.step1.mods_folder = root.path.join("mods").to_string_lossy().into_owned();
        app.wizard_state.step1.mods_backup_folder =
            root.path.join("backup").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_update_assets = vec![asset];
        app.wizard_state.step2.update_selected_download_scope = Some("leftover".to_string());
        app.wizard_state.modlist_auto_build_active = true;
        app.active_install_modlist_id = Some("all-cached".to_string());
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_archive_skip_completed(true);
        app.install_screen_state.skip_indices.insert(0);

        crate::ui::install::stage_downloading::kick_streaming_downloader_once(&mut app);

        assert!(
            app.step2_update_download_rx.is_none(),
            "no download worker is spawned when every asset is skipped"
        );
        assert_eq!(
            app.wizard_state.step2.update_selected_download_origin,
            DownloadOrigin::InstallPipeline
        );
        assert_eq!(app.wizard_state.step2.update_selected_download_scope, None);
        assert!(
            app.install_screen_state
                .pipeline_flags
                .download_phase_started()
        );
        let extract_rx = app
            .step2_update_extract_rx
            .take()
            .expect("the pipeline extractor started at once");
        assert_eq!(
            app.wizard_state.step2.update_selected_extract_progress,
            Some((0, 1)),
            "the pipeline extract planned the cached archive"
        );
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match extract_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(crate::app::app_step2_update_extract::Step2UpdateExtractEvent::Finished(_))
                | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    assert!(Instant::now() < deadline, "extract did not finish in time");
                }
            }
        }
    }

    #[test]
    fn a_new_unpack_failure_raises_one_error_toast() {
        let mut app = OrchestratorApp::new_isolated_for_test("unpack_toast");
        let before = app.notification_manager.history().len();
        app.wizard_state
            .step2
            .update_selected_extract_failed_sources
            .push("Old: stale".to_string());
        app.wizard_state
            .step2
            .update_selected_extract_failed_sources
            .push("d0questpack: matching mod folder not found".to_string());
        app.toast_new_unpack_failures(1);
        let errors = app
            .notification_manager
            .history()
            .iter()
            .skip(before)
            .filter(|record| record.kind == egui_toast::ToastKind::Error)
            .map(|record| record.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            errors,
            vec!["Could not unpack d0questpack: matching mod folder not found".to_string()]
        );
    }
}
