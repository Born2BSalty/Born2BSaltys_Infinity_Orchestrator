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
use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::app::{
    app_step2_saved_log_flow, app_step2_scan, app_step2_update_check, app_step2_update_download,
    app_step2_update_extract, app_step5_flow,
};
use crate::install_runtime::install_concurrency;
use crate::install_runtime::rail_lock_reason::RailLockReason;
use crate::install_runtime::registry_transition;
use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;
use crate::registry::persistence_cycle::RegistryPersistenceCycle;
use crate::registry::store::RegistryStore;
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::settings::model::AppSettings;
use crate::settings::redesign_fields::{RedesignSettings, ThemeChoice};
use crate::settings::redesign_store::RedesignSettingsStore;
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
use crate::ui::settings::oauth_glue;
use crate::ui::settings::state_settings::SettingsScreenState;
use crate::ui::settings::validate_debounce;
use crate::ui::shared::redesign_tokens::{REDESIGN_NAV_WIDTH_PX, ThemePalette};
use crate::ui::shell::shell_chrome;
use crate::ui::shell::shell_statusbar::RunningInstallStatus;
use crate::ui::step5::state_step5::Step5ConsoleViewState;
use crate::ui::workspace::state_workspace::WorkspaceViewState;
use crate::ui::workspace::step5::state_workspace_step5::WorkspaceStep5State;

const REDESIGN_SETTINGS_DEBOUNCE_MS: u64 = 1000;
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
    pub install_screen_state: InstallScreenState,
    pub create_screen_state: CreateScreenState,

    pub redesign_settings: RedesignSettings,
    pub redesign_settings_store: RedesignSettingsStore,
    pub redesign_settings_dirty: bool,
    pub redesign_settings_last_dirty_at: Option<Instant>,
    pub redesign_settings_last_saved: RedesignSettings,
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

    pub(crate) active_install_modlist_id: Option<String>,

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
    pub(crate) stream_download_rx:
        Option<Receiver<crate::install_runtime::stream_downloader::StreamDownloadEvent>>,

    pub(crate) extract_progress: Arc<std::sync::Mutex<Option<(usize, usize)>>>,

    pub(crate) extract_parallel_rx:
        Option<Receiver<crate::install_runtime::extract_parallel::ExtractAssetEvent>>,

    pub(crate) archive_skip_rx:
        Option<Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>>,

    pub(crate) hash_progress: Arc<std::sync::Mutex<Option<(usize, usize)>>>,
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

fn load_redesign_settings(store: &RedesignSettingsStore) -> RedesignSettings {
    match store.load() {
        Ok(settings) => settings,
        Err(err) => {
            warn!(
                target = "orchestrator",
                "bio_redesign_settings.json load failed: {err}; backing up and using defaults"
            );
            match store.backup_corrupt_file() {
                Ok(backup) => warn!(
                    target = "orchestrator",
                    "backed up corrupt redesign settings to {}",
                    backup.display()
                ),
                Err(backup_err) => warn!(
                    target = "orchestrator",
                    "failed backing up corrupt redesign settings: {backup_err}"
                ),
            }
            RedesignSettings::default()
        }
    }
}

impl OrchestratorApp {
    #[must_use]
    pub fn new(dev_mode: bool) -> Self {
        let bootstrap = app_bootstrap_init::initialize(dev_mode);

        let wizard_state = WizardState {
            step1: bootstrap.step1.clone(),
            github_auth_login: bootstrap.github_auth_login,
            ..Default::default()
        };

        let path_validation = compute_path_validation_summary(&wizard_state);

        let registry_store = RegistryStore::new_default();
        let RegistryLoad {
            registry,
            registry_error,
            registry_backup_path,
        } = load_registry(&registry_store);

        let persistence_cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());

        let redesign_settings_store = RedesignSettingsStore::new_default();
        let redesign_settings = load_redesign_settings(&redesign_settings_store);
        let theme_palette = match redesign_settings.theme_palette {
            ThemeChoice::Light => ThemePalette::Light,
            ThemeChoice::Dark => ThemePalette::Dark,
        };
        let effective_dev_mode = dev_mode || redesign_settings.diagnostic_mode;
        let bio_settings_snapshot = AppSettings {
            exe_fingerprint: bootstrap.exe_fingerprint.clone(),
            step1: bootstrap.step1.clone().into(),
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
            install_screen_state: InstallScreenState::default(),
            create_screen_state: CreateScreenState::new(),

            redesign_settings_last_saved: redesign_settings.clone(),
            redesign_settings,
            redesign_settings_store,
            redesign_settings_dirty: false,
            redesign_settings_last_dirty_at: None,
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
            active_install_modlist_id: None,
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
            stream_download_rx: None,
            extract_progress: Arc::new(std::sync::Mutex::new(None)),
            extract_parallel_rx: None,
            archive_skip_rx: None,
            hash_progress: Arc::new(std::sync::Mutex::new(None)),
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

    pub(crate) fn reset_install_screen_to_paste(&mut self) {
        reset_install_pipeline_state(InstallPipelineResetSet {
            stream_download_rx: &mut self.stream_download_rx,
            archive_skip_rx: &mut self.archive_skip_rx,
            extract_parallel_rx: &mut self.extract_parallel_rx,
            wizard_state: &mut self.wizard_state,
            install_screen_state: &mut self.install_screen_state,
            hash_progress: &self.hash_progress,
            extract_progress: &self.extract_progress,
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

        let share_code_override: Option<String> = if from_workspace {
            None
        } else {
            self.registry
                .find(&id)
                .and_then(|e| e.latest_share_code.clone())
                .filter(|c| !c.trim().is_empty())
        };

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
            share_code_override.as_deref(),
        );
        if rx.is_some() {
            self.install_size_worker_rx = rx;
        }

        if !from_workspace {
            self.active_install_modlist_id = None;
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
        app_step2_update_download::poll_step2_update_download(
            &mut self.wizard_state,
            &mut self.step2_update_download_rx,
            &mut self.step2_update_extract_rx,
        );
        app_step2_update_extract::poll_step2_update_extract(
            &mut self.wizard_state,
            &mut self.step2_update_extract_rx,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
        );
        Self::drain_archive_skip_events(
            &mut self.wizard_state,
            &mut self.archive_skip_rx,
            &mut self.install_screen_state,
            &self.hash_progress,
        );
        Self::drain_stream_download(
            &mut self.wizard_state,
            &mut self.stream_download_rx,
            &mut self.extract_parallel_rx,
            &mut self.install_screen_state.download_progress,
            &self.extract_progress,
        );
        Self::drain_extract_parallel(
            &mut self.wizard_state,
            &mut self.extract_parallel_rx,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
            &self.extract_progress,
        );

        app_step2_saved_log_flow::advance_pending_saved_log_flow(
            &mut self.wizard_state,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
            &mut self.step2_update_check_rx,
            &mut self.step2_update_download_rx,
        );
    }

    fn drain_stream_download(
        wizard_state: &mut WizardState,
        stream_download_rx: &mut Option<
            Receiver<crate::install_runtime::stream_downloader::StreamDownloadEvent>,
        >,
        extract_parallel_rx: &mut Option<
            Receiver<crate::install_runtime::extract_parallel::ExtractAssetEvent>,
        >,
        progress: &mut crate::ui::install::stage_downloading::DownloadProgress,
        extract_progress: &Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    ) {
        use crate::install_runtime::extract_parallel::start_parallel_extract;
        use crate::install_runtime::stream_downloader::{
            StreamDownloadEvent, apply_result_state, deterministic_dest,
        };
        use std::path::PathBuf;
        use std::sync::mpsc::TryRecvError;

        let Some(rx) = stream_download_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(StreamDownloadEvent::AssetProgress {
                    index,
                    bytes,
                    total,
                }) => {
                    progress.set_asset_bytes(index, bytes, total);
                }
                Ok(StreamDownloadEvent::AssetDone {
                    index,
                    ok,
                    final_bytes,
                    total: _,
                    error,
                }) => {
                    progress.set_asset_bytes(index, final_bytes, Some(final_bytes));
                    let archive_dir = PathBuf::from(wizard_state.step1.mods_archive_folder.trim());
                    if let Some(asset) = wizard_state.step2.update_selected_update_assets.get(index)
                    {
                        if ok {
                            let dest = deterministic_dest(asset, &archive_dir);
                            wizard_state
                                .step2
                                .update_selected_downloaded_sources
                                .push(format!("{} -> {}", asset.label, dest.display()));
                        } else {
                            let err_str = error.as_deref().unwrap_or("unknown error").to_string();
                            wizard_state
                                .step2
                                .update_selected_download_failed_sources
                                .push(format!("{}: {}", asset.label, err_str));
                        }
                    }
                }
                Ok(StreamDownloadEvent::Finished(result)) => {
                    let downloaded = result.downloaded.len();
                    let failed = result.failed.len();
                    tracing::info!(
                        target = "orchestrator",
                        downloaded,
                        failed,
                        "stream download Finished drained; starting parallel extract"
                    );
                    *stream_download_rx = None;
                    apply_result_state(wizard_state, result);
                    if let Some(rx) = start_parallel_extract(wizard_state, extract_progress) {
                        *extract_parallel_rx = Some(rx);
                        tracing::info!(
                            target = "orchestrator",
                            "parallel extract receiver installed"
                        );
                    } else {
                        tracing::info!(
                            target = "orchestrator",
                            "parallel extract receiver not installed"
                        );
                    }
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    *stream_download_rx = None;
                    wizard_state.step2.update_selected_download_running = false;
                    wizard_state.step2.scan_status =
                        "Download updates failed: worker disconnected".to_string();
                    return;
                }
            }
        }
    }

    fn drain_extract_parallel(
        wizard_state: &mut WizardState,
        extract_parallel_rx: &mut Option<
            Receiver<crate::install_runtime::extract_parallel::ExtractAssetEvent>,
        >,
        step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
        step2_cancel: &mut Option<Arc<AtomicBool>>,
        step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
        extract_progress: &Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    ) {
        use crate::install_runtime::extract_parallel::ExtractAssetEvent;
        use std::collections::HashSet;
        use std::sync::mpsc::TryRecvError;

        let Some(rx) = extract_parallel_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(ExtractAssetEvent::AssetDone {
                    index,
                    ok,
                    label,
                    target_or_err,
                }) => {
                    Self::record_extract_asset_done(
                        index,
                        ok,
                        &label,
                        &target_or_err,
                        extract_progress,
                    );
                }
                Ok(ExtractAssetEvent::Finished(result)) => {
                    Self::log_extract_finished(&result, extract_progress);
                    *extract_parallel_rx = None;
                    wizard_state.step2.update_selected_extract_running = false;
                    wizard_state.step2.update_selected_extracted_sources = result.extracted;

                    let extracted_labels: HashSet<String> = wizard_state
                        .step2
                        .update_selected_extracted_sources
                        .iter()
                        .filter_map(|e| e.split_once(" -> ").map(|(l, _)| l.trim().to_string()))
                        .filter(|l| !l.is_empty())
                        .collect();
                    if !extracted_labels.is_empty() {
                        wizard_state
                            .step2
                            .update_selected_missing_sources
                            .retain(|e| {
                                !extracted_labels
                                    .iter()
                                    .any(|l| e.starts_with(&format!("{l} (")))
                            });
                        wizard_state
                            .step2
                            .update_selected_update_sources
                            .retain(|e| {
                                !extracted_labels
                                    .iter()
                                    .any(|l| e.starts_with(&format!("{l} (")))
                            });
                        wizard_state
                            .step2
                            .update_selected_update_assets
                            .retain(|a| !extracted_labels.contains(&a.label));
                    }

                    wizard_state
                        .step2
                        .update_selected_extract_failed_sources
                        .extend(result.failed);

                    let extracted = wizard_state.step2.update_selected_extracted_sources.len();
                    let failed = wizard_state
                        .step2
                        .update_selected_extract_failed_sources
                        .len();

                    if extracted > 0 {
                        wizard_state.step1_mods_folder_has_tp2 = Some(true);
                        wizard_state.step2.log_pending_downloads.clear();
                        wizard_state.step2.scan_status =
                            format!("Extracted {extracted} updates; rescanning Mods Folder");
                        app_step2_scan::start_step2_scan(
                            wizard_state,
                            step2_scan_rx,
                            step2_cancel,
                            step2_progress_queue,
                        );
                    } else {
                        wizard_state.step2.scan_status = format!(
                            "Extract updates finished: {extracted} updated, \
 {failed} failed"
                        );
                    }
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    let progress_before = extract_progress.lock().ok().and_then(|g| *g);
                    tracing::info!(
                        target = "orchestrator",
                        progress_before = ?progress_before,
                        "extract drain: channel disconnected"
                    );
                    *extract_parallel_rx = None;
                    wizard_state.step2.update_selected_extract_running = false;
                    wizard_state.step2.scan_status =
                        "Extract updates failed: worker disconnected".to_string();
                    return;
                }
            }
        }
    }

    fn record_extract_asset_done(
        index: usize,
        ok: bool,
        label: &str,
        target_or_err: &str,
        extract_progress: &Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    ) {
        let mut progress_after = None;
        let progress_lock_ok = extract_progress.lock().is_ok_and(|mut g| {
            let (c, t) = g.unwrap_or((0, 0));
            let next = (c + 1, t);
            *g = Some(next);
            progress_after = Some(next);
            true
        });
        tracing::info!(
            target = "orchestrator",
            index,
            ok,
            label,
            target_or_err,
            progress_lock_ok,
            progress_after = ?progress_after,
            "extract drain: AssetDone received"
        );
    }

    fn log_extract_finished(
        result: &crate::install_runtime::extract_parallel::ExtractResult,
        extract_progress: &Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    ) {
        let progress_before = extract_progress.lock().ok().and_then(|g| *g);
        tracing::info!(
            target = "orchestrator",
            extracted_count = result.extracted.len(),
            failed_count = result.failed.len(),
            progress_before = ?progress_before,
            "extract drain: Finished received"
        );
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
            || self.wizard_state.step5.prep_running
            || self.wizard_state.step5.install_running
            || self.wizard_state.modlist_auto_build_active
    }

    fn step2_needs_repaint(&self) -> bool {
        self.step2_scan_rx.is_some()
            || self.step2_update_check_rx.is_some()
            || self.step2_update_download_rx.is_some()
            || self.step2_update_extract_rx.is_some()
            || self.stream_download_rx.is_some()
            || self.extract_parallel_rx.is_some()
            || self.archive_skip_rx.is_some()
            || self.wizard_state.modlist_auto_build_active
            || !self.step2_progress_queue.is_empty()
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

        if self.redesign_settings_dirty
            && self.redesign_settings != self.redesign_settings_last_saved
        {
            self.redesign_settings_last_dirty_at.get_or_insert(now);
            if let Some(at) = self.redesign_settings_last_dirty_at
                && now.saturating_duration_since(at)
                    >= Duration::from_millis(REDESIGN_SETTINGS_DEBOUNCE_MS)
            {
                match self.redesign_settings_store.save(&self.redesign_settings) {
                    Ok(()) => {
                        self.redesign_settings_last_saved = self.redesign_settings.clone();
                        self.redesign_settings_dirty = false;
                        self.redesign_settings_last_dirty_at = None;
                    }
                    Err(err) => {
                        warn!(
                            target = "orchestrator",
                            "redesign settings save failed: {err}"
                        );
                    }
                }
            }
        }

        self.tick_bio_settings(now);
    }

    fn bio_settings_snapshot(&self) -> AppSettings {
        let mut step1_clone = self.wizard_state.step1.clone();
        crate::install_runtime::settings_sanitizer::sanitize_step1_for_settings_persistence(
            &mut step1_clone,
            &self.bio_settings_last_saved.step1,
        );
        let mut step1: crate::settings::model::Step1Settings = step1_clone.into();
        step1
            .game_install
            .clone_from(&self.bio_settings_last_saved.step1.game_install);
        AppSettings {
            exe_fingerprint: self.exe_fingerprint.clone(),
            step1,
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
        if self.redesign_settings != self.redesign_settings_last_saved {
            if let Err(err) = self.redesign_settings_store.save(&self.redesign_settings) {
                warn!(
                    target = "orchestrator",
                    "redesign settings flush failed: {err}"
                );
            } else {
                self.redesign_settings_last_saved = self.redesign_settings.clone();
            }
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
    pub stream_download_rx:
        &'a mut Option<Receiver<crate::install_runtime::stream_downloader::StreamDownloadEvent>>,
    pub archive_skip_rx:
        &'a mut Option<Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>>,
    pub extract_parallel_rx:
        &'a mut Option<Receiver<crate::install_runtime::extract_parallel::ExtractAssetEvent>>,
    pub wizard_state: &'a mut WizardState,
    pub install_screen_state: &'a mut InstallScreenState,
    pub hash_progress: &'a Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    pub extract_progress: &'a Arc<std::sync::Mutex<Option<(usize, usize)>>>,
    pub pending_reinstall_id: &'a mut Option<String>,
    pub active_install_modlist_id: &'a mut Option<String>,
}

pub fn reset_install_pipeline_state(set: InstallPipelineResetSet<'_>) {
    let InstallPipelineResetSet {
        stream_download_rx,
        archive_skip_rx,
        extract_parallel_rx,
        wizard_state,
        install_screen_state,
        hash_progress,
        extract_progress,
        pending_reinstall_id,
        active_install_modlist_id,
    } = set;

    *stream_download_rx = None;
    *archive_skip_rx = None;
    *extract_parallel_rx = None;

    wizard_state.modlist_auto_build_active = false;
    wizard_state.modlist_auto_build_waiting_for_install = false;
    wizard_state.step2.pending_saved_log_apply = false;
    wizard_state.step2.pending_saved_log_update_preview = false;
    wizard_state.step2.pending_saved_log_download = false;
    wizard_state.step2.update_selected_download_running = false;
    wizard_state.step2.update_selected_extract_running = false;

    install_screen_state.clear_preview();
    install_screen_state.stage = crate::ui::install::state_install::InstallStage::Paste;

    if let Ok(mut g) = hash_progress.lock() {
        *g = None;
    }
    if let Ok(mut g) = extract_progress.lock() {
        *g = None;
    }

    *pending_reinstall_id = None;
    *active_install_modlist_id = None;
}

impl eframe::App for OrchestratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = self.theme_palette;

        validate_debounce::tick(self, Instant::now());
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

        shell_chrome::render_shell(ctx, palette, modlist_count, running_status.as_ref(), |ui| {
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
        });

        oauth_glue::render_github_popup_if_open(self, ctx);

        let install_was_running = self.wizard_state.step5.install_running;
        step5_requested_repaint |= self.start_step5_after_render();
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
        }
        if step5_requested_repaint || self.step5_needs_repaint() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.install_size_worker_rx.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        self.drain_size_worker_result();

        self.sync_active_workspace_if_dirty();

        self.tick_persistence();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_all_now();
    }
}

impl Drop for OrchestratorApp {
    fn drop(&mut self) {
        self.flush_all_now();
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
        ws
    }

    fn dirty_iss() -> InstallScreenState {
        let mut iss = InstallScreenState {
            stage: crate::ui::install::state_install::InstallStage::Downloading,
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

    #[test]
    fn reset_install_pipeline_state_drops_all_receivers_and_clears_wizard_latches() {
        let (s_dl, r_dl) = std::sync::mpsc::channel::<
            crate::install_runtime::stream_downloader::StreamDownloadEvent,
        >();
        let (s_sk, r_sk) = std::sync::mpsc::channel::<
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent,
        >();
        let (s_ex, r_ex) = std::sync::mpsc::channel::<
            crate::install_runtime::extract_parallel::ExtractAssetEvent,
        >();
        let mut stream = Some(r_dl);
        let mut skip = Some(r_sk);
        let mut extract = Some(r_ex);
        let mut ws = dirty_ws();
        let mut iss = dirty_iss();
        let hash = Arc::new(std::sync::Mutex::new(Some((10usize, 51usize))));
        let extract_lock = Arc::new(std::sync::Mutex::new(Some((5usize, 51usize))));
        let mut pending = Some("modlist-id".to_string());
        let mut active = Some("modlist-id".to_string());

        reset_install_pipeline_state(InstallPipelineResetSet {
            stream_download_rx: &mut stream,
            archive_skip_rx: &mut skip,
            extract_parallel_rx: &mut extract,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            extract_progress: &extract_lock,
            pending_reinstall_id: &mut pending,
            active_install_modlist_id: &mut active,
        });

        assert!(stream.is_none(), "stream_download_rx dropped");
        assert!(skip.is_none(), "archive_skip_rx dropped");
        assert!(extract.is_none(), "extract_parallel_rx dropped");

        assert!(
            s_dl.send(
                crate::install_runtime::stream_downloader::StreamDownloadEvent::Finished(
                    crate::install_runtime::stream_downloader::StreamDownloadResult::default()
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
                crate::install_runtime::extract_parallel::ExtractAssetEvent::AssetDone {
                    index: 0,
                    ok: true,
                    label: "MOD".to_string(),
                    target_or_err: "C:/x".to_string(),
                }
            )
            .is_err()
        );
        drop((s_dl, s_sk, s_ex));

        assert!(!ws.modlist_auto_build_active);
        assert!(!ws.modlist_auto_build_waiting_for_install);
        assert!(!ws.step2.pending_saved_log_apply);
        assert!(!ws.step2.pending_saved_log_update_preview);
        assert!(!ws.step2.pending_saved_log_download);
        assert!(!ws.step2.update_selected_download_running);
        assert!(!ws.step2.update_selected_extract_running);

        assert!(pending.is_none());
        assert!(active.is_none());
    }

    #[test]
    fn reset_install_pipeline_state_clears_screen_state_and_shared_progress_mutexes() {
        let mut stream: Option<
            Receiver<crate::install_runtime::stream_downloader::StreamDownloadEvent>,
        > = None;
        let mut skip: Option<
            Receiver<crate::install_runtime::archive_skip_async::ArchiveSkipEvent>,
        > = None;
        let mut extract: Option<
            Receiver<crate::install_runtime::extract_parallel::ExtractAssetEvent>,
        > = None;
        let mut ws = dirty_ws();
        let mut iss = dirty_iss();
        let hash = Arc::new(std::sync::Mutex::new(Some((10usize, 51usize))));
        let extract_lock = Arc::new(std::sync::Mutex::new(Some((5usize, 51usize))));
        let mut pending: Option<String> = None;
        let mut active: Option<String> = None;

        reset_install_pipeline_state(InstallPipelineResetSet {
            stream_download_rx: &mut stream,
            archive_skip_rx: &mut skip,
            extract_parallel_rx: &mut extract,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            extract_progress: &extract_lock,
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
        assert!(iss.hashed_indices.is_empty());
        assert_eq!(
            iss.stage,
            crate::ui::install::state_install::InstallStage::Paste
        );

        assert!(hash.lock().unwrap().is_none(), "shared hash mutex blanked");
        assert!(
            extract_lock.lock().unwrap().is_none(),
            "shared extract mutex blanked"
        );
    }

    #[test]
    fn composed_cancel_drains_all_three_event_streams_after_reset() {
        let (s_dl, r_dl) = std::sync::mpsc::channel::<
            crate::install_runtime::stream_downloader::StreamDownloadEvent,
        >();
        let (s_sk, r_sk) = std::sync::mpsc::channel::<
            crate::install_runtime::archive_skip_async::ArchiveSkipEvent,
        >();
        let (s_ex, r_ex) = std::sync::mpsc::channel::<
            crate::install_runtime::extract_parallel::ExtractAssetEvent,
        >();
        let _ = s_dl.send(
            crate::install_runtime::stream_downloader::StreamDownloadEvent::AssetProgress {
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
            crate::install_runtime::extract_parallel::ExtractAssetEvent::AssetDone {
                index: 0,
                ok: true,
                label: "MOD".to_string(),
                target_or_err: "C:/x".to_string(),
            },
        );
        let mut stream = Some(r_dl);
        let mut skip = Some(r_sk);
        let mut extract = Some(r_ex);
        let mut ws = WizardState::default();
        let mut iss = InstallScreenState::default();
        let hash = Arc::new(std::sync::Mutex::new(None));
        let extract_lock = Arc::new(std::sync::Mutex::new(None));
        let mut pending = None;
        let mut active = None;
        reset_install_pipeline_state(InstallPipelineResetSet {
            stream_download_rx: &mut stream,
            archive_skip_rx: &mut skip,
            extract_parallel_rx: &mut extract,
            wizard_state: &mut ws,
            install_screen_state: &mut iss,
            hash_progress: &hash,
            extract_progress: &extract_lock,
            pending_reinstall_id: &mut pending,
            active_install_modlist_id: &mut active,
        });
        assert!(stream.is_none());
        assert!(skip.is_none());
        assert!(extract.is_none());
        assert!(
            s_dl.send(
                crate::install_runtime::stream_downloader::StreamDownloadEvent::Finished(
                    crate::install_runtime::stream_downloader::StreamDownloadResult::default()
                )
            )
            .is_err()
        );
        let _ = TryRecvError::Empty;
    }
}
