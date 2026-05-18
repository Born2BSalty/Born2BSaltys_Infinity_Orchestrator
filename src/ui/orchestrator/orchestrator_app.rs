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
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::app::{
    app_step2_saved_log_flow, app_step2_scan, app_step2_update_check, app_step2_update_download,
    app_step2_update_extract, app_step5_flow,
};
use crate::install_runtime::install_concurrency;
use crate::install_runtime::rail_lock_reason::RailLockReason;
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
    /// Per-screen Create UI state (active stage, typed name, chosen game,
    /// destination + `DestChoice`, Load Draft dialog open). Added in Phase 6
    /// Run 3 alongside the real Create screen (P6.T13). The stage machine is
    /// whole (`Choose | Fork*`); Run 3 implements `Choose` + the Load Draft
    /// dialog, the `Fork*` stages render the Run-4 deferred placeholder.
    /// Built via `CreateScreenState::new` so the game ComboBox defaults to
    /// `EET` (SPEC §5.1 — `Game::default()` is `BGEE`).
    pub create_screen_state: CreateScreenState,

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

    // ---------- Phase 7 fields (P7.T1 — Step-5 install runtime) ----------
    /// Per-modlist Step-5 chrome state (the success-banner / post-install
    /// action-row wrap-around state + the Run-1 install-clicked marker that
    /// drives the P7.T8 `← Previous` lock). Net-new orchestrator chrome
    /// state — BIO's Step-5 state is untouched. Run 1 only uses the
    /// install-clicked marker; the dialog/post-install fields are declared
    /// inert (Run 3 behavior), mirroring the established staged-field
    /// pattern (`WorkspaceViewState`'s Phase-7 fields).
    pub workspace_step5: WorkspaceStep5State,

    /// **P7.T9 / T9b / T14 — install-start monotonic anchor.** Set to
    /// `Some(Instant::now())` the frame `wizard_state.step5.install_running`
    /// transitions `false → true`, cleared the frame it goes `true →
    /// false`. The statusbar's `<elapsed>` segment (P7.T14) and the C5
    /// rail-lock reason (P7.T9b) both tick against this monotonic clock —
    /// the persisted `ModlistEntry.install_started_at` is a wall-clock
    /// `DateTime<Utc>` (recoverable across runs) and cannot be subtracted
    /// monotonically for a live "+MM:SS" readout, so this process-local
    /// `Instant` is the UI clock. `None` ⇒ no install running.
    pub install_running_since: Option<Instant>,

    // The Step-5 install-runtime fields the orchestrator owns exactly as
    // `WizardApp` owns them (`src/ui/app.rs:53-57`). The orchestrator's
    // `update` loop drives them through the SAME `bio::app::*` call
    // sequence `bio::ui::app::update_loop::run` uses (the H3 read-only
    // reference path — `poll_step5_terminal` + `poll_step5_prep` before the
    // render, `start_if_requested` after; see `poll_step5_before_render` /
    // `start_step5_after_render` below). The orchestrator never invokes
    // that private `update_loop` module — it replicates the sequence.
    //
    // Visibility: `step5_terminal` / `step5_terminal_error` /
    // `step5_console_view` are `pub` (their types — `EmbeddedTerminal` /
    // `String` / `Step5ConsoleViewState` — are all `pub`, and
    // `page_workspace_step5::render` reads them by `&mut`).
    // `step5_prep_rx` / `step5_pending_start` are `pub(crate)` because
    // `PendingInstallStart` is a BIO `pub(crate)` struct (reachable
    // same-crate via the carve-out-#3 lib+bin split); `pub` would trip
    // `private_interfaces` — the same precedent as the Step-2 receivers
    // and `github_auth_rx`.
    /// The embedded WeiDU terminal (child process + stdout capture +
    /// prompt detection). `None` until the first install starts; `None`
    /// here makes `page_step5::render` render the pre-install panel
    /// (Command card, Summary card, console box, prompt input) with no
    /// live child — the Run-1 breakpoint state.
    pub step5_terminal: Option<EmbeddedTerminal>,
    /// Last terminal-construction error (surfaced inside BIO's panel).
    pub step5_terminal_error: Option<String>,
    /// Step-5 console UI state (filter selection, auto-scroll, prompt
    /// answers panel open). One per-process instance, mirroring
    /// `WizardApp.step5_console_view`. Passed `&mut` into
    /// `page_step5::render`.
    pub step5_console_view: Step5ConsoleViewState,
    /// The target-prep worker channel BIO's `start_if_requested` populates
    /// (and `poll_step5_prep` drains). BIO type, identical to
    /// `WizardApp.step5_prep_rx`.
    pub(crate) step5_prep_rx: Option<Receiver<Result<TargetPrepResult, String>>>,
    /// BIO's pending-install handle, held between target-prep start and
    /// completion. BIO type, identical to `WizardApp.step5_pending_start`.
    /// (The plan's P7.T1 prose says `bool`; the binding requirement is to
    /// mirror `WizardApp`'s field set so the SAME `bio::app::*` call
    /// sequence type-checks — `bio::app::app_step5_flow::start_if_requested`
    /// takes `&mut Option<PendingInstallStart>`. Reported as a PLAN GAP:
    /// prose simplification, not a behavior change.)
    pub(crate) step5_pending_start: Option<PendingInstallStart>,

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
            // SPEC §5.1: the game ComboBox defaults to `EET`. `new()` forces
            // it (the bare `Default` would be `Game::default()` == `BGEE`).
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

            // Phase 6 — workspace spine. Same init shape as WizardApp's Step 2
            // channels (`src/ui/app.rs:80-86`): all `None` / empty; the
            // `bio::app::app_step2_*` action handlers populate them when a
            // background task starts.
            workspace_view: WorkspaceViewState::default(),
            workspace_state_dirty: false,

            // Phase 7 — Step-5 install runtime. Same init shape as
            // WizardApp's Step-5 fields (`src/ui/app.rs:87-91`): no
            // terminal / no error / fresh console view / no prep channel /
            // no pending start. `step5_terminal == None` is the pre-install
            // state — `page_step5::render` renders the Command/Summary
            // cards + console box + prompt input with no live child.
            workspace_step5: WorkspaceStep5State::default(),
            // No install running at construction (a force-quit-mid-install
            // relaunch has a dead process ⇒ `install_running == false` ⇒
            // the rail is unlocked from launch; the edge-detect in
            // `update` arms this if/when an install starts this run).
            install_running_since: None,
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

    /// **P7.T1 — drive the Step-5 install runtime, pre-render portion.**
    ///
    /// Mirrors the Step-5 lines of `bio::app::app_update_cycle::
    /// poll_before_render` (`app_update_cycle.rs:66-76`) — the exact two
    /// calls `bio::ui::app::update_loop::run` makes before rendering Step 5
    /// (the H3 read-only reference path; that private `update_loop` module
    /// is never invoked — the sequence is replicated). Same rationale as
    /// `poll_step2_channels`: `poll_before_render` is monolithic (it also
    /// requires the Step-5 args and unconditionally calls these two
    /// functions), so the orchestrator calls the **same narrower
    /// `bio::app::app_step5_flow` functions `poll_before_render` itself
    /// calls** (`poll_step5_terminal` then `poll_step5_prep`), in the same
    /// order, with the orchestrator's owned Step-5 fields. Both callees are
    /// `pub(crate) fn`, same-crate reachable via the carve-out-#3 lib+bin
    /// split (the same reachability `poll_step2_channels` already relies
    /// on). Returns whether Step 5 wants a repaint this frame.
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

    /// **P7.T1 — drive the Step-5 install runtime, post-render portion.**
    ///
    /// Mirrors `bio::app::app_update_cycle::start_after_render`
    /// (`app_update_cycle.rs:79-93`), which is the single
    /// `app_step5_flow::start_if_requested` call `bio::ui::app::update_loop::
    /// run` makes *after* rendering Step 5 (so a `StartInstall` action
    /// dispatched this frame — which flips `state.step5.start_install_
    /// requested = true` — is picked up on the next poll, exactly as the
    /// legacy wizard does). `start_if_requested` is `pub(crate) fn`,
    /// same-crate reachable. Returns whether Step 5 wants a repaint.
    fn start_step5_after_render(&mut self) -> bool {
        app_step5_flow::start_if_requested(
            &mut self.wizard_state,
            &mut self.step5_terminal,
            &mut self.step5_terminal_error,
            &mut self.step5_prep_rx,
            &mut self.step5_pending_start,
        )
    }

    /// True when the Step-5 install runtime needs a repaint next frame
    /// (terminal has new data, prep channel live, or an install is in
    /// flight). Mirrors the Step-5 subset of `bio::app::app_update_cycle::
    /// needs_repaint` (`app_update_cycle.rs:137-144`).
    fn step5_needs_repaint(&self) -> bool {
        self.step5_terminal
            .as_ref()
            .map(EmbeddedTerminal::has_new_data)
            .unwrap_or(false)
            || self.step5_prep_rx.is_some()
            || self.wizard_state.step5.prep_running
            || self.wizard_state.step5.install_running
            || self.wizard_state.modlist_auto_build_active
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

    /// **P6.T11 — dirty-bit-gated workspace extract (the H1 gate).** Called
    /// once per frame *before* `tick_persistence`. The contract:
    ///
    /// - `workspace_state_dirty == false` ⇒ **return immediately** — no
    ///   `extract_workspace_state_from_wizard`, no compare, no map touch.
    ///   This is the H1 "zero per-frame extract overhead for an idle
    ///   workspace" property: an idle frame never reaches the extract, so
    ///   `persistence_cycle.workspace_extract_debug_count` stays flat
    ///   (observable in the P6.T11 acceptance).
    /// - `workspace_state_dirty == true` and a workspace is loaded ⇒ perform
    ///   exactly one extract: `note_workspace_extract()` (bumps the debug
    ///   counter), read the prior persisted state for the loaded id, call
    ///   `workspace_state_loader::extract_workspace_state_from_wizard`
    ///   (carrying `prior`'s egui-only fields through — `expand_state` /
    ///   `prompt_overrides` / `dev_scanned_mods_folder` / `last_share_code`),
    ///   and if it differs from `prior` write it into `self.workspace_state`
    ///   + `mark_workspace_dirty(id)` on the cycle so the existing per-id
    ///   debounce (`persist_workspace_if_needed`, called from
    ///   `tick_persistence`) flushes it ~`debounce_ms` later. If it does not
    ///   differ, nothing is queued (the cadence's own diff is the second
    ///   guard).
    ///
    /// The flag is **always cleared** once consumed (dirty or not) so the
    /// extract runs at most once per dirtying burst, not every frame —
    /// re-dirtying (the next mutating action / Step-3 fingerprint change)
    /// re-arms it. Rename never sets this flag (it sets the *registry*
    /// dirty bit), so a rename does not trigger a workspace extract.
    ///
    /// Per C5 the loader (and so this extract) is never reached mid-install
    /// — the rail-nav lock (Phase 7) prevents nav-into-a-different-workspace
    /// while an install runs.
    fn sync_active_workspace_if_dirty(&mut self) {
        // H1 gate: idle ⇒ zero work. No extract, no compare, no allocation.
        if !self.workspace_state_dirty {
            return;
        }
        // Consume the flag now (at most one extract per dirtying burst).
        self.workspace_state_dirty = false;

        // Fix-Run 4 (Part 2) — restore-pending save guard. While a
        // cold-resume restore is pending/unreconciled for the active modlist
        // (`rescan_snapshot` set OR `resume_pending`), the in-memory
        // `WizardState` Step-2/3 set is the empty post-`populate` shell — the
        // resume-triggered scan + reconcile have not landed yet. Extracting
        // it would overwrite the in-memory `workspace_state` map with that
        // empty/poisoned value, which the debounce cadence (and the on-exit
        // `flush_all` that writes the map) would then persist over the real,
        // correct per-modlist `workspace.json`. The on-disk file is already
        // correct and there is nothing legitimate to persist until the
        // restore reconciles. The dirty flag is **consumed** (early return
        // without extract): the resume reconcile rebuilds Step 3, and any
        // genuine post-restore user edit re-dirties via `step_action_
        // dispatch`, so nothing legitimate is lost — only the poisoning
        // extract is skipped. (Fix-Run-3's `order_for_tab` guard still
        // covers the production/never-refilled path; this covers the dev
        // fast-scan window where the scanned set *will* be refilled.)
        if crate::ui::orchestrator::page_router::restore_pending(&self.workspace_view.step2) {
            return;
        }

        // Only the currently-loaded workspace has live `WizardState` to
        // extract. No loaded id (e.g. dirtied then navigated away before
        // this tick — the nav-away flush already wrote it) ⇒ nothing to do.
        let Some(id) = self.workspace_view.loaded_workspace_id.clone() else {
            return;
        };

        // One dirty-gated extract performed → record it (H1 observability).
        self.persistence_cycle.note_workspace_extract();

        // Fix-Run 1 (Bug A) — sync the live Step-2 selection into Step 3
        // before extracting. The Step-2 checkbox path now marks the
        // workspace dirty (so this debounced write fires), but the toggle
        // only mutated `step2.<tab>_mods`; `extract` reads `step3.<tab>_
        // items`. This BIO-faithful sync (no-op when only Step 3 was
        // reordered — preserves the user's drag order) makes the debounced
        // write capture the toggle.
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
            // Queue the debounced per-id write (the existing Phase-3
            // cadence in `persist_workspace_if_needed` does the actual
            // throttled disk write from `tick_persistence`).
            self.persistence_cycle
                .mark_workspace_dirty(&id, Instant::now());
        }
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

        // P7.T1 — drive the Step-5 install runtime BEFORE the render, in
        // the SAME order `bio::ui::app::update_loop::run` does (the H3
        // read-only reference path): `bio::app::app_update_cycle::
        // poll_before_render` polls Step 2 first, then Step 5. The
        // orchestrator polls Step 2 above (`poll_step2_channels`) and the
        // Step-5 portion here (`poll_step5_before_render` = the exact
        // `poll_step5_terminal` + `poll_step5_prep` lines of
        // `poll_before_render`). BIO requests an input-focus when an
        // install transitions to running across the poll boundary
        // (`update_loop.rs:139-141`) — replicated so the prompt input
        // grabs focus the frame the install starts.
        let install_was_running = self.wizard_state.step5.install_running;
        let mut step5_requested_repaint = self.poll_step5_before_render();
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
            // P7.T9b/T14 — install just started: anchor the monotonic
            // clock the rail-lock reason + the statusbar `<elapsed>` tick
            // against. (The persisted wall-clock start time is
            // `ModlistEntry.install_started_at`, written by
            // `start_hooks::on_install_start`; this is the live UI clock.)
            self.install_running_since = Some(Instant::now());
        }
        // Install just ended (clean exit / cancel / failure): drop the
        // anchor so the rail unlocks + the statusbar resets to
        // `0 jobs running` on the next frame.
        if install_was_running && !self.wizard_state.step5.install_running {
            self.install_running_since = None;
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

        // P7.T9 / T9b / T14 — derive the single install-concurrency state
        // ONCE per frame (SPEC §13.15: only one install at a time). It
        // powers BOTH the C5 rail-nav lock (`RailLockReason`, P7.T9b) and
        // the statusbar's `1 job running · <modlist> · <elapsed>` readout
        // (P7.T14). The running modlist's display **name** (for the
        // verbatim SPEC §13.15 tooltip + the statusbar) is resolved from
        // the registry here (the rail/statusbar have no registry handle).
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

        // Phase 4 P4.T9 — overlay the OAuth popup over the active destination
        // when the wizard state has it open. Must run **after** the shell so
        // the popup floats above the rail / page chrome.
        oauth_glue::render_github_popup_if_open(self, ctx);

        // P7.T1 — drive the Step-5 install runtime AFTER the render, exactly
        // as `bio::ui::app::update_loop::run` does: `bio::app::
        // app_update_cycle::start_after_render` runs post-render so a
        // `Step5Action::StartInstall` dispatched this frame by
        // `page_workspace_step5::render` (which flips
        // `state.step5.start_install_requested = true`) is picked up here
        // and kicks off the install — identical to the legacy wizard. The
        // same install-transition input-focus edge BIO applies
        // (`update_loop.rs:152-154`). When Step 5 wants a repaint (terminal
        // streaming / prep in flight / install running) request one, since
        // egui paints lazily and the child process reports off-thread.
        let install_was_running = self.wizard_state.step5.install_running;
        step5_requested_repaint |= self.start_step5_after_render();
        if !install_was_running && self.wizard_state.step5.install_running {
            self.step5_console_view.request_input_focus = true;
        }
        if step5_requested_repaint || self.step5_needs_repaint() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        // P6.T11 — dirty-bit-gated workspace extract (the H1 gate). MUST run
        // before `tick_persistence` so a just-dirtied workspace's extracted
        // state is in `self.workspace_state` before the debounce cadence
        // diffs it. Idle (flag `false`) ⇒ this is a single bool check + an
        // early return (no extract, the H1 property).
        self.sync_active_workspace_if_dirty();

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
