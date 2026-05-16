// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `OrchestratorApp` — standalone `eframe::App` impl powering the new
// `infinity_orchestrator` binary.
//
// Phase 2 fields (per P2.T3):
//   - `nav` — active destination
//   - `wizard_state` — orchestrator-owned `bio::app::state::WizardState`
//     (constructed directly; field is `pub` and all step substates are `pub`)
//   - `settings_store` — orchestrator-owned `bio::settings::SettingsStore`
//   - `dev_mode` — CLI flag passthrough (OR'd with persisted toggle per M12)
//   - `exe_fingerprint` — populated by bootstrap_init
//   - `path_validation` — derived per frame from `wizard_state.step1`
//   - `theme_palette` — H3: theme state lives on the app, NOT in a global
//     atomic. Read once per frame, passed into render code explicitly.
//
// Phase 3 fields (per P3.T7):
//   - `registry`, `registry_store`, `registry_error`, `registry_backup_path`,
//     `persistence_cycle`, `workspace_state`, `workspace_stores`,
//     `home_stub_state` — see field comments.
//
// Phase 4 fields:
//   - `redesign_settings` + `redesign_settings_store` — sibling
//     `bio_redesign_settings.json` persistence per P4.T10.
//   - `redesign_settings_dirty` — flag flipped by tab_general / tab_paths /
//     tab_advanced edits; settings are persisted on a 1s debounce.
//   - `redesign_settings_last_dirty_at` — debounce timestamp for the
//     redesign settings file.
//   - `redesign_settings_last_saved` — snapshot for change detection.
//   - `settings_screen_state` — per-screen UI state (active tab, name-row
//     edit toggle, debounce timestamps, last validation report).
//   - `github_auth_rx` — owned channel receiver for the GitHub OAuth device
//     flow. Replicates the surface of `WizardApp::step1_github_auth_rx` (per
//     C2 audit, `handle_step1_action` was disqualified from the carve-out
//     because it mutates this field on the WizardApp side).
//   - `tool_version_cache` — `weidu --help` / `mod_installer --version`
//     parsed strings. Phase 4 ships an empty cache; Phase 7 wires the live
//     detection.
//   - `dev_mode_cli_flag` — raw CLI dev_mode (without the OR'd persisted
//     toggle) — preserved so that toggling Diagnostic mode off in Settings
//     doesn't disable dev mode if the user launched with `-d`.
//   - `validate_paths_on_startup` lives on `RedesignSettings` and gates the
//     startup `validate_now::run_now` seeding pass.
//   - `accounts_stub_hint` — last status string shown under Nexus/Mega
//     stub cards.
//
// `OrchestratorApp::new(dev_mode)` per H5: calls
// `bio::app::app_bootstrap_init::initialize(dev_mode)` directly (no inline
// duplicated logic).
//
// **H4 — Persistence on exit.** `eframe::App::on_exit` is the **primary**
// hook (called before `Drop` on normal shutdown). `Drop for OrchestratorApp`
// is the **fallback** (catches panic-unwind / hard exit edge cases). Both
// call `flush_all_now`, which is idempotent.
//
// SPEC: §2.1, §11, §13.1, §13.14, overview "Architecture" section.

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
use crate::app::{
    app_step2_saved_log_flow, app_step2_scan, app_step2_update_check, app_step2_update_download,
    app_step2_update_extract,
};
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
use crate::ui::workspace::state_workspace::WorkspaceViewState;

/// 1-second debounce on the redesign settings file (matches Phase 3's
/// registry cycle cadence).
const REDESIGN_SETTINGS_DEBOUNCE_MS: u64 = 1000;
/// 1-second debounce on the BIO `bio_settings.json` writes (mirrors BIO's
/// existing `app_update_cycle::persist_step1_if_needed` cadence).
const BIO_SETTINGS_DEBOUNCE_MS: u64 = 1000;

/// Cached detected versions for the Tools sub-tab. Phase 4 ships an empty
/// cache; the Tools tab renders blank hints until Phase 7 wires the live
/// detection.
#[derive(Debug, Clone, Default)]
pub struct ToolVersionCache {
    pub weidu_version: Option<String>,
    pub mod_installer_version: Option<String>,
}

pub struct OrchestratorApp {
    /// Active destination router.
    pub nav: NavDestination,
    /// Orchestrator-owned `WizardState`. Independent of `WizardApp`'s state.
    pub wizard_state: WizardState,
    /// Persistence store for `bio_settings.json`. Owned per-process; the
    /// orchestrator instantiates its own.
    pub settings_store: SettingsStore,
    /// Dev-mode toggle (OR of CLI flag + Settings → General toggle).
    pub dev_mode: bool,
    /// Raw CLI dev-mode flag — held so we can OR with the persisted toggle
    /// across runtime updates without losing the launch-time enable.
    pub dev_mode_cli_flag: bool,
    /// Executable fingerprint produced by `bootstrap_init`.
    pub exe_fingerprint: String,
    /// Per-frame cached path-validation summary used by the left rail's
    /// bottom status row.
    pub path_validation: PathValidationSummary,
    /// Active theme palette. H3: per-frame propagation, NOT a global atomic.
    pub theme_palette: ThemePalette,

    // ---------- Phase 3 fields ----------
    pub registry: ModlistRegistry,
    pub registry_store: RegistryStore,
    pub registry_error: Option<RegistryError>,
    pub registry_backup_path: Option<std::path::PathBuf>,
    pub persistence_cycle: RegistryPersistenceCycle,
    pub workspace_state: HashMap<String, ModlistWorkspaceState>,
    pub workspace_stores: HashMap<String, WorkspaceStore>,
    /// Phase 3 home-stub state (dev seed toast). Retained for the dev-only
    /// stub path; Phase 5's real Home uses `home_screen_state` instead.
    pub home_stub_state: HomeStubState,

    // ---------- Phase 5 fields ----------
    /// Per-screen Home UI state (active filter chip; Run-2 delete/reinstall/
    /// toast fields are declared but inert this run). Added in Phase 5 P5.T4
    /// alongside the real Home screen (P5.T15).
    pub home_screen_state: HomeScreenState,
    /// Per-screen Install Modlist UI state (active stage, destination,
    /// `DestChoice`, pasted code). Added in Phase 5 / Run 3 alongside the real
    /// Install screen (P5.T14). The 4-stage machine is whole; Run 3
    /// implements Paste + the stage-4 stub.
    pub install_screen_state: InstallScreenState,

    // ---------- Phase 4 fields ----------
    pub redesign_settings: RedesignSettings,
    pub redesign_settings_store: RedesignSettingsStore,
    pub redesign_settings_dirty: bool,
    pub redesign_settings_last_dirty_at: Option<Instant>,
    pub redesign_settings_last_saved: RedesignSettings,
    pub settings_screen_state: SettingsScreenState,
    pub(crate) github_auth_rx: Option<Receiver<GitHubOAuthFlowResult>>,
    pub tool_version_cache: ToolVersionCache,
    pub accounts_stub_hint: Option<String>,
    /// BIO `bio_settings.json` snapshot + debounce timestamp. The
    /// orchestrator persists Step1 settings whenever the in-memory copy
    /// drifts from the on-disk snapshot.
    pub bio_settings_last_saved: AppSettings,
    pub bio_settings_last_dirty_at: Option<Instant>,

    // ---------- Phase 6 fields ----------
    /// Per-modlist workspace view state (active step, completed steps,
    /// loaded-modlist tracking, rename/fork/flash state). Replaces the
    /// Phase-2 workspace stub once a modlist is opened (P6.T1 / P6.T12).
    pub workspace_view: WorkspaceViewState,
    /// Dirty bit for the per-modlist workspace state (`workspace.json`). Set
    /// by `step_action_dispatch` on mutating Step 2/4 variants (and, in
    /// Run 4, by the Step 3 fingerprint detector). The debounced workspace
    /// write that consumes this flag lands in **Run 4 (P6.T11)** — Run 1
    /// only adds the flag + setter; nothing reads it yet.
    pub workspace_state_dirty: bool,

    // The six Step 2 channel receivers the Step-2 background tasks use.
    // Owned here exactly as `WizardApp` owns them (`src/ui/app.rs:46-52`):
    // all start `None` / empty; `bio::app::app_step2_*` channel-creation
    // functions populate them when the relevant background task starts (the
    // action handlers take them by `&mut`, mirroring the BIO pattern — see
    // `step_action_dispatch::dispatch_step2`). They are **drained every
    // frame** by `poll_step2_channels` (P6.T2c — the narrower-call mirror
    // of `bio::app::app_update_cycle::poll_before_render`'s Step-2 portion,
    // since `poll_before_render` is monolithic and also requires Step-5
    // runtime args the orchestrator does not own pre-Phase-7).
    //
    // Visibility: `pub(crate)` (matching the sibling `github_auth_rx`
    // channel field) — the update-event types are BIO `pub(crate)` enums
    // reachable same-crate per the carve-out-#3 lib+bin split. `pub` would
    // trip `private_interfaces`; the orchestrator binary is same-crate so
    // `pub(crate)` is both correct and sufficient (`step_action_dispatch`
    // and the Run-4 poll wiring are same-crate).
    /// ① Step 2 TP2-scan worker event channel.
    pub(crate) step2_scan_rx: Option<Receiver<Step2ScanEvent>>,
    /// ② Step 2 scan cancellation flag.
    pub(crate) step2_cancel: Option<Arc<AtomicBool>>,
    /// ③ Step 2 scan progress queue `(done, total, label)`.
    pub(crate) step2_progress_queue: VecDeque<(usize, usize, String)>,
    /// ④ Step 2 update-check worker event channel.
    pub(crate) step2_update_check_rx:
        Option<Receiver<crate::app::app_step2_update_check_worker::Step2UpdateCheckEvent>>,
    /// ⑤ Step 2 update-download worker event channel.
    pub(crate) step2_update_download_rx:
        Option<Receiver<crate::app::app_step2_update_download::Step2UpdateDownloadEvent>>,
    /// ⑥ Step 2 update-extract worker event channel. (The 6th — the
    /// historically-missed `_extract_rx`.) Drained every frame by
    /// `poll_step2_channels` (P6.T2c) via
    /// `bio::app::app_step2_update_extract::poll_step2_update_extract` —
    /// the same callee `bio::app::app_update_cycle::poll_before_render`
    /// uses for the extract receiver. (Run 1 owned it inert behind an
    /// `#[allow(dead_code)]`; Run 1b wires the poll so it is now read.)
    pub(crate) step2_update_extract_rx:
        Option<Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>>,
}

impl OrchestratorApp {
    /// Construct an orchestrator app instance.
    pub fn new(dev_mode: bool) -> Self {
        let bootstrap = app_bootstrap_init::initialize(dev_mode);

        let wizard_state = WizardState {
            step1: bootstrap.step1.clone(),
            github_auth_login: bootstrap.github_auth_login,
            ..Default::default()
        };

        let path_validation = compute_path_validation_summary(&wizard_state);

        // ---------- Registry init ----------
        let registry_store = RegistryStore::new_default();
        let mut registry_error: Option<RegistryError> = None;
        let mut registry_backup_path: Option<std::path::PathBuf> = None;
        let registry = match registry_store.load() {
            Ok(reg) => reg,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "modlists.json load failed: {err}; backing up and entering terminal-error state"
                );
                match registry_store.backup_corrupt_file() {
                    Ok(new_path) => registry_backup_path = Some(new_path),
                    Err(backup_err) => warn!(
                        target = "orchestrator",
                        "backup_corrupt_file failed: {backup_err}"
                    ),
                }
                registry_error = Some(err);
                ModlistRegistry::default()
            }
        };

        let persistence_cycle = RegistryPersistenceCycle::new_with_baseline(registry.clone());

        // ---------- Redesign settings init (Phase 4) ----------
        let redesign_settings_store = RedesignSettingsStore::new_default();
        let redesign_settings = match redesign_settings_store.load() {
            Ok(s) => s,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "bio_redesign_settings.json load failed: {err}; backing up and using defaults"
                );
                // SPEC §13.14: redesign settings are reconstructable UI
                // preferences, so they get backup-and-default (not the
                // registry's terminal-error block). Move the bad file aside
                // so the next debounced write can't silently overwrite it.
                match redesign_settings_store.backup_corrupt_file() {
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
        };
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

            // Phase 6 — workspace spine. Same init shape as WizardApp's Step 2
            // channels (`src/ui/app.rs:80-86`): all `None` / empty; the
            // `bio::app::app_step2_*` action handlers populate them when a
            // background task starts.
            workspace_view: WorkspaceViewState::default(),
            workspace_state_dirty: false,
            step2_scan_rx: None,
            step2_cancel: None,
            step2_progress_queue: VecDeque::new(),
            step2_update_check_rx: None,
            step2_update_download_rx: None,
            step2_update_extract_rx: None,
        };

        // NOTE: do NOT call `oauth_glue::load_persisted_login` here.
        // `app_bootstrap_init::initialize` (above) already reads the GitHub
        // token from the OS keychain once and the resolved login is already
        // in `bootstrap.github_auth_login` → `wizard_state.github_auth_login`.
        // A second `keyring::Entry::get_password()` triggers a SECOND macOS
        // keychain authorization prompt for unsigned binaries (rebuilds
        // invalidate the keychain ACL's signature trust each time), which
        // shows up to the user as the keychain prompt firing in a loop on
        // startup. One bootstrap-time read is enough.

        // Run per-field validation once at startup so any prefilled paths
        // (loaded from bio_settings.json) show their inline status the moment
        // the user opens Settings → Paths. Gated on the persisted
        // `validate_paths_on_startup` toggle (SPEC §11.1 / §11.2): when off,
        // the seeding pass is skipped and inline status only appears once
        // the user edits a field (which kicks off the debounce cycle).
        if app.redesign_settings.validate_paths_on_startup {
            app.settings_screen_state.path_validation_results =
                crate::ui::settings::validate_now::run_now(&app.wizard_state.step1);
        }

        app
    }

    /// Mark the active modlist's workspace state dirty so the debounced
    /// workspace write picks it up. Called by `step_action_dispatch` on
    /// every mutating Step 2/4 variant (and, in Run 4, the Step 3 fingerprint
    /// detector). **Run 1 only sets the flag** — the debounced write that
    /// consumes it is wired in Run 4 (P6.T11); nothing drains it yet.
    pub fn mark_workspace_dirty(&mut self) {
        self.workspace_state_dirty = true;
    }

    /// Drain the 6 Step-2 background-thread receivers every frame (P6.T2c —
    /// fixes the scan-hang / Cancel-stuck defects).
    ///
    /// **Why the narrower `poll_step2_*` calls, not
    /// `bio::app::app_update_cycle::poll_before_render`.** The H3 real path
    /// (`bio::ui::app::update_loop::run` → `app_update_cycle::
    /// poll_before_render`) is **monolithic**: `poll_before_render`'s
    /// signature additionally requires `step5_terminal` /
    /// `step5_terminal_error` / `step5_prep_rx` / `step5_pending_start`, and
    /// its body unconditionally calls `app_step5_flow::poll_step5_terminal`
    /// + `poll_step5_prep` (`src/core/app/navigation/app_update_cycle.rs:
    /// 33-76`). The orchestrator does not own the Step-5 install runtime
    /// pre-Phase-7 (that's Phase 7's `install_runtime`), so it cannot
    /// satisfy those args. Per the brief's explicit instruction for the
    /// monolithic case, this calls the **same narrower `bio::app::
    /// app_step2_*` functions `poll_before_render` calls for the Step-2
    /// portion** (`app_update_cycle.rs:38-64`), in the same order, with the
    /// orchestrator's owned receivers — draining scan / cancel / progress /
    /// update-check / update-download / update-extract / saved-log-flow
    /// exactly as `WizardApp` does, minus only the Step-5 lines. Every
    /// callee is `pub(crate) fn`, same-crate reachable via the carve-out-#3
    /// lib+bin split.
    fn poll_step2_channels(&mut self) {
        // Mirrors `poll_before_render` lines 38-64 (the Step-2 portion).
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
        app_step2_saved_log_flow::advance_pending_saved_log_flow(
            &mut self.wizard_state,
            &mut self.step2_scan_rx,
            &mut self.step2_cancel,
            &mut self.step2_progress_queue,
            &mut self.step2_update_check_rx,
            &mut self.step2_update_download_rx,
        );
    }

    /// True when a Step-2 background task is in flight, so the orchestrator
    /// must keep repainting (the worker reports on a thread; without a
    /// repaint request egui paints lazily and the scan/cancel would appear
    /// to hang until the next user input). Mirrors the Step-2 subset of
    /// `bio::app::app_update_cycle::needs_repaint`.
    fn step2_needs_repaint(&self) -> bool {
        self.step2_scan_rx.is_some()
            || self.step2_update_check_rx.is_some()
            || self.step2_update_download_rx.is_some()
            || self.step2_update_extract_rx.is_some()
            || !self.step2_progress_queue.is_empty()
    }

    /// Per-frame poll: try to flush pending registry / workspace writes if
    /// their debounce has elapsed.
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

        // Redesign settings debounce.
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

        // BIO settings debounce — persists Step1 edits made via the Settings
        // → Paths / Tools / Advanced tabs.
        self.tick_bio_settings(now);
    }

    /// Build the `AppSettings` snapshot written to `bio_settings.json`.
    ///
    /// `game_install` is masked to the value loaded from disk at startup. In
    /// the orchestrator the game is a **per-modlist** choice held in
    /// `wizard_state.step1.game_install` (Phase 6's workspace loader sets it
    /// from the modlist's `entry.game`). Writing that per-modlist value into
    /// the global settings file would conflate per-modlist state with global
    /// state (plan P4.T3). Masking it in every snapshot — the one used for
    /// the dirty comparison **and** the one written — means a per-modlist
    /// game switch never marks `bio_settings` dirty and never reaches disk;
    /// the global `game_install` loaded at startup is preserved verbatim.
    fn bio_settings_snapshot(&self) -> AppSettings {
        let mut step1: crate::settings::model::Step1Settings =
            self.wizard_state.step1.clone().into();
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

    /// Synchronous full flush. Called from both `on_exit` (primary) and
    /// `Drop` (fallback). Idempotent.
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
}

impl eframe::App for OrchestratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = self.theme_palette;

        // Phase 4 P4.T11b — per-edit debounced path validation. Runs once per
        // frame; fields whose debounce window has elapsed get re-validated.
        validate_debounce::tick(self, Instant::now());
        // egui paints lazily — without an explicit repaint request, the tick
        // wouldn't fire again until the next user event, so a debounce set
        // during typing could hang for several seconds before the user moves
        // the mouse. Request a frame exactly when the soonest pending
        // debounce window will elapse.
        if let Some(next_due_in) = next_debounce_due_in(self) {
            ctx.request_repaint_after(next_due_in);
        }

        // Drive the OAuth flow's receiver (if any). Mutates wizard_state
        // (`github_auth_*` fields) per BIO's existing `poll_github_oauth_flow`.
        oauth_glue::poll_github_oauth_flow(self);

        // Drain the 6 Step-2 background-thread receivers BEFORE the render
        // (exactly the order `bio::ui::app::update_loop::run` polls them —
        // the H3 real path; see `poll_step2_channels`). Without this the
        // scan worker starts but never reports → the UI hangs and Cancel
        // never completes (P6.T2c — fixes the Run-1 follow-up's defect #1:
        // scan-hang / Cancel-stuck). A Step-2 task in flight needs an
        // explicit repaint request because egui paints lazily and the
        // worker reports off-thread.
        self.poll_step2_channels();
        // SPEC §6.3 (the #2 fix) — rescan is non-destructive. The drain
        // above has just landed the freshly-scanned mod set if the scan
        // completed this frame; re-apply the pre-scan selection snapshot
        // onto it (preserving `selected_order`), dropping only mods /
        // components no longer present + surfacing the missing-mods
        // warning. No-op unless `is_scanning` just transitioned `true →
        // false` after a *successful* scan with a snapshot pending — must
        // run AFTER `poll_step2_channels` so the fresh mods are in place.
        crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(self);
        if self.step2_needs_repaint() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // Per-frame path validation summary (left rail bottom).
        self.path_validation = compute_path_validation_summary(&self.wizard_state);
        // If the screen's last full-validation report disagrees with the live
        // state, layer that into the rail summary so the rail reflects the
        // most recent edit.
        if self
            .settings_screen_state
            .path_validation_results
            .issue_count
            > 0
            && self.path_validation.kind == PathValidationKind::Ok
        {
            self.path_validation = PathValidationSummary {
                kind: PathValidationKind::Err(
                    self.settings_screen_state
                        .path_validation_results
                        .issue_count,
                ),
                text: format!(
                    "\u{00D7} {} path issues",
                    self.settings_screen_state
                        .path_validation_results
                        .issue_count
                ),
            };
        }

        let modlist_count = self.registry.entries.len();
        let jobs_running = 0usize;

        shell_chrome::render_shell(ctx, palette, modlist_count, jobs_running, |ui| {
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
                        None,
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

        // Phase 4 P4.T9 — overlay the OAuth popup over the active destination
        // when the wizard state has it open. Must run **after** the shell so
        // the popup floats above the rail / page chrome.
        oauth_glue::render_github_popup_if_open(self, ctx);

        // Per-frame persistence tick.
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

/// Returns the soonest `Duration` from now at which any field's debounce
/// window will elapse, or `None` if no field is currently pending. Used to
/// request a precisely-timed repaint so the next `validate_debounce::tick`
/// runs even without intervening user input.
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
