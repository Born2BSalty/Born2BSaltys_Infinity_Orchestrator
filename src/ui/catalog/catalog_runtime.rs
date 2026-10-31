use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant, SystemTime};

use crate::app::app_step2_update_check::{Step2UpdateCheckOutcome, Step2UpdateCheckRequest};
use crate::app::app_step2_update_check_worker::{self, Step2UpdateCheckEvent};
use crate::app::app_step2_update_download::{self, Step2UpdateDownloadEvent};
use crate::app::app_step2_update_extract::{archive, plan};
use crate::app::app_step2_update_source_refs::{self, RemoteFileFacts};
use crate::app::github_forks_list::{self, ForksFetch, ForksStatus};
use crate::app::github_release_list::{
    self, ReleaseListEvent, ReleaseListState, ReleaseListStatus,
};
use crate::app::mod_downloads;
use crate::app::state::{Step2DiscoveredFork, Step2UpdateAsset, VersionsSheet, WizardState};
use crate::ui::workspace::versions::versions_sheets;

use super::state_catalog::{
    CatalogDestination, CatalogScreenState, CatalogTransferSnapshot, LocalPresence,
};

type ScanResult = (u64, PathBuf, BTreeMap<String, LocalPresence>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CatalogItemOutcome {
    Completed(String),
    Failed(String),
    Canceled,
}

#[derive(Debug, Clone)]
pub(crate) enum CatalogTransferEvent {
    Progress(String),
    Item {
        tp2: String,
        outcome: CatalogItemOutcome,
    },
    Finished,
}

#[derive(Default)]
pub(crate) struct CatalogRuntime {
    pub(crate) loaded: bool,
    pub(crate) running: bool,
    pub(crate) run_id: u64,
    pub(crate) snapshot: Option<CatalogTransferSnapshot>,
    pub(crate) outcomes: BTreeMap<String, CatalogItemOutcome>,
    pub(crate) cancel_token: Option<Arc<AtomicBool>>,
    pub(crate) scan_rx: Option<Receiver<ScanResult>>,
    pub(crate) transfer_rx: Option<Receiver<(u64, CatalogTransferEvent)>>,
    fork_fetch: Option<ForksFetch>,
    release_rx: Option<Receiver<ReleaseListEvent>>,
    save_state: WizardState,
    source_stamp: Option<(u64, Option<SystemTime>)>,
    pending_source_choice: Option<(String, String)>,
    checked_at: Option<Instant>,
    pub(crate) refresh_scan: bool,
}

impl CatalogRuntime {
    pub(crate) fn mark_loaded(&mut self) {
        self.loaded = true;
        self.checked_at = Some(Instant::now());
        self.source_stamp = std::fs::metadata(mod_downloads::mod_downloads_user_path())
            .ok()
            .map(|metadata| (metadata.len(), metadata.modified().ok()));
    }

    pub(crate) fn ensure_loaded(
        &mut self,
        state: &mut CatalogScreenState,
        destination: &CatalogDestination,
    ) {
        if !self.loaded
            || !state.loaded
            || self
                .checked_at
                .is_none_or(|time| time.elapsed() >= Duration::from_secs(1))
        {
            self.checked_at = Some(Instant::now());
            let path = mod_downloads::mod_downloads_user_path();
            let stamp = std::fs::metadata(&path)
                .ok()
                .map(|m| (m.len(), m.modified().ok()));
            if !state.loaded
                || !self.loaded
                || stamp != self.source_stamp
                || (self.pending_source_choice.is_some() && state.error.is_some())
            {
                match std::fs::read_to_string(&path) {
                    Ok(user) => {
                        state.replace_from_texts(
                            mod_downloads::default_mod_downloads_content(),
                            &user,
                        );
                        if let Some((tp2, source_id)) = self.pending_source_choice.take()
                            && state.error.is_none()
                        {
                            state.choose_source(&tp2, &source_id);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        self.pending_source_choice = None;
                        state
                            .replace_from_texts(mod_downloads::default_mod_downloads_content(), "");
                    }
                    Err(error) => {
                        state.error = Some(format!("Could not read My default sources: {error}"));
                        state.loaded = true;
                    }
                }
                self.mark_loaded();
                self.refresh_scan = true;
            }
        }
        if state.scan_destination != destination.mods_root || self.refresh_scan {
            self.start_scan(state, &destination.mods_root);
            self.refresh_scan = false;
        }
    }

    pub(crate) fn start_scan(&mut self, state: &mut CatalogScreenState, destination: &Path) {
        state.scan_destination = destination.to_path_buf();
        if destination.as_os_str().is_empty() {
            state.local_presence.clear();
            self.scan_rx = None;
            return;
        }
        let generation = state.generation;
        let root = destination.to_path_buf();
        let sources = state
            .rows
            .iter()
            .map(|row| (row.tp2.clone(), row.sources.clone()))
            .collect::<Vec<_>>();
        state.local_presence = state
            .rows
            .iter()
            .map(|row| (row.tp2.clone(), LocalPresence::Scanning))
            .collect();
        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);
        std::thread::spawn(move || {
            let result = scan_local_sources(&root, &sources);
            let _ = tx.send((generation, root, result));
        });
    }

    pub(crate) fn start(
        &mut self,
        state: &mut CatalogScreenState,
        keys: &BTreeSet<String>,
        destination: CatalogDestination,
        wizard_busy: bool,
    ) -> Result<u64, String> {
        if self.running {
            return Err("A Catalog download is already running".to_string());
        }
        let snapshot = state.snapshot(keys, destination, wizard_busy)?;
        let lease = app_step2_update_download::reserve_catalog_destination(
            &snapshot.destination.mods_root,
            &snapshot.destination.archives,
            &snapshot.destination.backup_root,
        )?;
        self.run_id = self.run_id.wrapping_add(1);
        let run_id = self.run_id;
        tracing::info!(
            run_id,
            generation = snapshot.generation,
            "Catalog download starting"
        );
        self.running = true;
        self.outcomes.clear();
        let token = Arc::new(AtomicBool::new(false));
        self.cancel_token = Some(Arc::clone(&token));
        self.snapshot = Some(snapshot.clone());
        let (tx, rx) = mpsc::channel();
        self.transfer_rx = Some(rx);
        state.status = format!("Resolving {} selected mods\u{2026}", snapshot.sources.len());
        std::thread::spawn(move || {
            let _lease = lease;
            run_transfer(run_id, &snapshot, &token, &tx);
        });
        Ok(run_id)
    }

    pub(crate) const fn pending(&self) -> bool {
        self.running
            || self.scan_rx.is_some()
            || self.fork_fetch.is_some()
            || self.release_rx.is_some()
    }

    pub(crate) fn cancel(&self) {
        if let Some(token) = &self.cancel_token {
            token.store(true, Ordering::Relaxed);
        }
    }

    pub(crate) fn accept_transfer(
        &mut self,
        state: &mut CatalogScreenState,
        run_id: u64,
        event: CatalogTransferEvent,
    ) -> bool {
        if run_id != self.run_id || !self.running {
            return false;
        }
        match event {
            CatalogTransferEvent::Progress(status) => state.status = status,
            CatalogTransferEvent::Item { tp2, outcome } => {
                self.outcomes
                    .insert(mod_downloads::normalize_mod_download_tp2(&tp2), outcome);
            }
            CatalogTransferEvent::Finished => {
                self.running = false;
                self.cancel_token = None;
                self.refresh_scan = true;
                let completed = self
                    .outcomes
                    .values()
                    .filter(|o| matches!(o, CatalogItemOutcome::Completed(_)))
                    .count();
                let failed = self
                    .outcomes
                    .values()
                    .filter(|o| matches!(o, CatalogItemOutcome::Failed(_)))
                    .count();
                let canceled = self
                    .outcomes
                    .values()
                    .filter(|o| matches!(o, CatalogItemOutcome::Canceled))
                    .count();
                state.status = format!(
                    "Finished: {completed} completed, {failed} failed, {canceled} canceled"
                );
            }
        }
        true
    }

    pub(crate) fn poll(&mut self, state: &mut CatalogScreenState) {
        if let Some(rx) = &self.scan_rx {
            match rx.try_recv() {
                Ok((generation, root, result)) => {
                    state.accept_scan(generation, &root, result);
                    self.scan_rx = None;
                }
                Err(TryRecvError::Disconnected) => {
                    for presence in state.local_presence.values_mut() {
                        if *presence == LocalPresence::Scanning {
                            *presence = LocalPresence::Error("The local scan stopped".to_string());
                        }
                    }
                    self.scan_rx = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.transfer_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        for (run_id, event) in events {
            self.accept_transfer(state, run_id, event);
        }
        if disconnected && self.running {
            let keys = self
                .snapshot
                .as_ref()
                .map(|s| s.sources.iter().map(|s| s.tp2.clone()).collect::<Vec<_>>())
                .unwrap_or_default();
            for key in keys {
                if !self
                    .outcomes
                    .contains_key(&mod_downloads::normalize_mod_download_tp2(&key))
                {
                    self.accept_transfer(
                        state,
                        self.run_id,
                        CatalogTransferEvent::Item {
                            tp2: key,
                            outcome: CatalogItemOutcome::Failed(
                                "The download worker stopped".to_string(),
                            ),
                        },
                    );
                }
            }
            self.accept_transfer(state, self.run_id, CatalogTransferEvent::Finished);
        }
        if !self.running {
            self.transfer_rx = None;
        }
        let was_loading = state.editor_step2.forks_list.status == ForksStatus::Loading;
        github_forks_list::poll_forks(&mut state.editor_step2.forks_list, &mut self.fork_fetch);
        let repo = state.editor_step2.forks_list.repo.clone();
        if was_loading {
            match state.editor_step2.forks_list.status.clone() {
                ForksStatus::Ready(forks) => {
                    state.accept_forks(&repo, Ok(forks));
                }
                ForksStatus::Failed(error) => {
                    state.accept_forks(&repo, Err(error));
                }
                ForksStatus::Idle | ForksStatus::Loading => {}
            }
        }
        if state
            .editor_step2
            .versions_ui
            .source_form
            .as_ref()
            .is_some_and(|form| form.repo != state.editor_step2.versions_ui.release_list.repo)
        {
            self.release_rx = None;
            state.editor_step2.versions_ui.release_list = ReleaseListState::default();
        }
        if let Some(rx) = &self.release_rx {
            match rx.try_recv() {
                Ok(ReleaseListEvent::Finished { repo, result }) => {
                    state.accept_releases(&repo, result);
                    self.release_rx = None;
                }
                Err(TryRecvError::Disconnected) => {
                    let repo = state.editor_step2.versions_ui.release_list.repo.clone();
                    state.accept_releases(&repo, Err("The release lookup stopped".to_string()));
                    self.release_rx = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    pub(crate) fn open_forks(&mut self, state: &mut CatalogScreenState, tp2: &str) {
        let Some(source) = state.source(tp2).cloned() else {
            return;
        };
        let Some(repo) = source
            .github
            .filter(|repo| repo.split('/').count() == 2 && !repo.trim().is_empty())
        else {
            return;
        };
        state.editor_step2.mod_download_forks.clear();
        state.editor_step2.mod_download_forks_popup_error = None;
        state
            .editor_step2
            .mod_download_forks_popup_tp2
            .clone_from(&source.tp2);
        state
            .editor_step2
            .mod_download_forks_popup_label
            .clone_from(&source.name);
        state.editor_step2.forks_list.repo = repo;
        state.editor_step2.forks_list.status = ForksStatus::Loading;
        state.editor_step2.versions_ui.sheet = Some(VersionsSheet::Forks);
        self.fork_fetch = None;
    }

    pub(crate) fn request_releases(&mut self, state: &mut CatalogScreenState, repo: String) {
        state
            .editor_step2
            .versions_ui
            .release_list
            .repo
            .clone_from(&repo);
        state.editor_step2.versions_ui.release_list.status = ReleaseListStatus::Loading;
        self.release_rx = Some(github_release_list::spawn_release_list_fetch(repo, false));
    }

    pub(crate) fn use_fork(
        &mut self,
        state: &mut CatalogScreenState,
        discovered: &Step2DiscoveredFork,
    ) {
        let mut form = versions_sheets::seed_source_form_from_fork(
            &state.editor_step2.mod_download_forks_popup_tp2,
            &state.editor_step2.mod_download_forks_popup_label,
            discovered,
        );
        form.card_key = format!(
            "catalog:{}",
            mod_downloads::normalize_mod_download_tp2(&form.tp2)
        );
        state.editor_step2.versions_ui.sheet_tp2 = Some(form.card_key.clone());
        state.editor_step2.versions_ui.source_form = Some(form);
        state.editor_step2.versions_ui.sheet = Some(VersionsSheet::EditSource);
        self.fork_fetch = None;
    }

    pub(crate) fn dismiss_editor(&mut self, state: &mut CatalogScreenState) {
        state.editor_step2.versions_ui.source_form = None;
        state.editor_step2.versions_ui.sheet = None;
        state.editor_step2.forks_list.status = ForksStatus::Idle;
        state.editor_step2.versions_ui.release_list = ReleaseListState::default();
        self.fork_fetch = None;
        self.release_rx = None;
    }

    pub(crate) fn save_editor(&mut self, state: &mut CatalogScreenState) -> Result<(), String> {
        let Some(form) = state.editor_step2.versions_ui.source_form.clone() else {
            return Err("No source is open".to_string());
        };
        self.save_state.step2.scan_status.clear();
        match crate::app::app_step2_router::save_global_source_form(&mut self.save_state, &form) {
            Ok(()) => {
                let source = crate::app::mod_source_history::normalize_source(
                    &form.tp2,
                    &crate::app::source_form::to_source(&form),
                );
                self.pending_source_choice = Some((source.tp2, source.source_id));
                self.dismiss_editor(state);
                self.loaded = false;
                self.checked_at = None;
                state.status = if self.save_state.step2.scan_status.contains("not saved") {
                    format!(
                        "Source saved to My default. {}",
                        self.save_state.step2.scan_status
                    )
                } else {
                    "Saved to My default".to_string()
                };
                Ok(())
            }
            Err(error) => {
                if let Some(form) = state.editor_step2.versions_ui.source_form.as_mut() {
                    form.error = Some(error.clone());
                }
                Err(error)
            }
        }
    }
}

pub(crate) fn scan_local_sources(
    root: &Path,
    sources: &[(String, Vec<mod_downloads::ModDownloadSource>)],
) -> BTreeMap<String, LocalPresence> {
    let mut names = BTreeMap::new();
    let error = if root.is_dir() {
        let mut error = None;
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            match entry {
                Ok(entry)
                    if entry.file_type().is_file()
                        && entry
                            .path()
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("tp2")) =>
                {
                    names
                        .entry(mod_downloads::normalize_mod_download_tp2(
                            &entry.file_name().to_string_lossy(),
                        ))
                        .or_insert_with(|| entry.path().to_path_buf());
                }
                Err(err) => {
                    error.get_or_insert_with(|| err.to_string());
                }
                Ok(_) => {}
            }
        }
        error
    } else {
        Some("Mods folder is unavailable".to_string())
    };
    sources
        .iter()
        .map(|(key, sources)| {
            let found = sources
                .iter()
                .flat_map(mod_downloads::ModDownloadSource::declared_tp2_names)
                .find_map(|name| names.get(&mod_downloads::normalize_mod_download_tp2(&name)));
            let status = found.map_or_else(
                || {
                    error.as_ref().map_or(LocalPresence::NotFound, |error| {
                        LocalPresence::Error(error.clone())
                    })
                },
                |path| LocalPresence::Found(path.clone()),
            );
            (key.clone(), status)
        })
        .collect()
}

pub(crate) fn resolver_request(
    source: &mod_downloads::ModDownloadSource,
) -> Step2UpdateCheckRequest {
    Step2UpdateCheckRequest {
        game_tab: String::new(),
        tp_file: source.tp2.clone(),
        label: source.name.clone(),
        source_id: source.source_id.clone(),
        repo: source.github.clone().unwrap_or_default(),
        exact_github: source.exact_github.clone(),
        source_url: source.url.clone(),
        channel: source.channel.clone(),
        tag: source.tag.clone(),
        commit: source.commit.clone(),
        branch: source.branch.clone(),
        release: source.release.clone(),
        asset: source.asset.clone(),
        pkg: mod_downloads::preferred_pkg_for_current_platform(source),
        requested_version: None,
    }
}

fn emit(tx: &Sender<(u64, CatalogTransferEvent)>, run_id: u64, event: CatalogTransferEvent) {
    let _ = tx.send((run_id, event));
}

fn run_transfer(
    run_id: u64,
    snapshot: &CatalogTransferSnapshot,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<(u64, CatalogTransferEvent)>,
) {
    let mut settled = BTreeSet::new();
    if !cancel.load(Ordering::Relaxed) {
        let rx = app_step2_update_check_worker::spawn_update_check_worker(
            snapshot.sources.iter().map(resolver_request).collect(),
        );
        let outcomes = loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Step2UpdateCheckEvent::Finished(outcomes)) => break outcomes,
                Ok(Step2UpdateCheckEvent::Progress(progress)) => emit(
                    tx,
                    run_id,
                    CatalogTransferEvent::Progress(format!(
                        "Resolving sources: {}/{}",
                        progress.completed, progress.total
                    )),
                ),
                Err(mpsc::RecvTimeoutError::Disconnected) => break Vec::new(),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if cancel.load(Ordering::Relaxed) {
                break Vec::new();
            }
        };
        if !cancel.load(Ordering::Relaxed) {
            transfer_resolved(run_id, snapshot, outcomes, cancel, tx, &mut settled);
        }
    }
    for source in &snapshot.sources {
        let key = mod_downloads::normalize_mod_download_tp2(&source.tp2);
        if !settled.contains(&key) {
            let outcome = if cancel.load(Ordering::Relaxed) {
                CatalogItemOutcome::Canceled
            } else {
                CatalogItemOutcome::Failed("Source lookup stopped without a result".to_string())
            };
            emit(tx, run_id, CatalogTransferEvent::Item { tp2: key, outcome });
        }
    }
    emit(tx, run_id, CatalogTransferEvent::Finished);
}

fn transfer_resolved(
    run_id: u64,
    snapshot: &CatalogTransferSnapshot,
    outcomes: Vec<Step2UpdateCheckOutcome>,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<(u64, CatalogTransferEvent)>,
    settled: &mut BTreeSet<String>,
) {
    let mut resolved = collect_resolved_assets(run_id, outcomes, tx, settled);
    if archive_destinations_conflict(&snapshot.destination.archives, &resolved.assets) {
        for asset in &resolved.assets {
            let key = mod_downloads::normalize_mod_download_tp2(&asset.tp_file);
            settled.insert(key.clone());
            emit(tx, run_id, CatalogTransferEvent::Item { tp2: key, outcome: CatalogItemOutcome::Failed("Selected sources use conflicting archive paths. Download these mods separately.".to_string()) });
        }
        return;
    }
    let success = download_resolved_assets(run_id, snapshot, &resolved.assets, cancel, tx, settled);
    extract_resolved_assets(
        run_id,
        snapshot,
        &mut resolved,
        success,
        cancel,
        tx,
        settled,
    );
}

pub(crate) fn archive_destinations_conflict(
    archive_root: &Path,
    assets: &[Step2UpdateAsset],
) -> bool {
    let mut destinations = BTreeMap::<String, (PathBuf, &str)>::new();
    for asset in assets {
        let path = archive_root.join(app_step2_update_download::archive_file_name(asset));
        let key = app_step2_update_download::destination_key(&path);
        match destinations.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((path, asset.asset_url.as_str()));
            }
            std::collections::btree_map::Entry::Occupied(entry)
                if entry.get().0 != path || entry.get().1 != asset.asset_url =>
            {
                return true;
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    false
}

struct ResolvedCatalogAssets {
    assets: Vec<Step2UpdateAsset>,
    remote: BTreeMap<String, RemoteFileFacts>,
}

fn collect_resolved_assets(
    run_id: u64,
    outcomes: Vec<Step2UpdateCheckOutcome>,
    tx: &Sender<(u64, CatalogTransferEvent)>,
    settled: &mut BTreeSet<String>,
) -> ResolvedCatalogAssets {
    let mut assets = Vec::new();
    let mut remote = BTreeMap::<String, RemoteFileFacts>::new();
    for outcome in outcomes {
        let key = mod_downloads::normalize_mod_download_tp2(&outcome.tp_file);
        if let Some(facts) = outcome.remote_file {
            remote.insert(key.clone(), facts);
        }
        if let (Some(name), Some(url), None) =
            (&outcome.asset_name, &outcome.asset_url, &outcome.error)
        {
            assets.push(Step2UpdateAsset {
                game_tab: outcome.game_tab,
                tp_file: outcome.tp_file,
                label: outcome.label,
                source_id: outcome.source_id,
                tag: outcome.tag.unwrap_or_default(),
                asset_name: name.clone(),
                asset_url: url.clone(),
                installed_source_ref: outcome.source_ref,
            });
        } else {
            settled.insert(key.clone());
            emit(
                tx,
                run_id,
                CatalogTransferEvent::Item {
                    tp2: key,
                    outcome: CatalogItemOutcome::Failed(outcome.error.unwrap_or_else(|| {
                        "This source needs a manual download. Open the selected source link."
                            .to_string()
                    })),
                },
            );
        }
    }
    ResolvedCatalogAssets { assets, remote }
}

fn download_resolved_assets(
    run_id: u64,
    snapshot: &CatalogTransferSnapshot,
    assets: &[Step2UpdateAsset],
    cancel: &Arc<AtomicBool>,
    tx: &Sender<(u64, CatalogTransferEvent)>,
    settled: &mut BTreeSet<String>,
) -> BTreeSet<usize> {
    let rx = app_step2_update_download::spawn_global_download(
        snapshot.destination.archives.clone(),
        assets.to_vec(),
        Arc::clone(cancel),
    );
    let mut success = BTreeSet::new();
    while let Ok(event) = rx.recv() {
        match event {
            Step2UpdateDownloadEvent::AssetProgress {
                index,
                bytes,
                total,
            } => {
                if let Some(asset) = assets.get(index) {
                    emit(
                        tx,
                        run_id,
                        CatalogTransferEvent::Progress(format!(
                            "Downloading {}: {} / {} bytes",
                            asset.label,
                            bytes,
                            total.map_or_else(|| "?".to_string(), |n| n.to_string())
                        )),
                    );
                }
            }
            Step2UpdateDownloadEvent::AssetDone {
                index, ok, error, ..
            } => {
                if let Some(asset) = assets.get(index) {
                    if ok {
                        success.insert(index);
                    } else {
                        let key = mod_downloads::normalize_mod_download_tp2(&asset.tp_file);
                        settled.insert(key.clone());
                        let outcome = if cancel.load(Ordering::Relaxed) {
                            CatalogItemOutcome::Canceled
                        } else {
                            CatalogItemOutcome::Failed(
                                error.unwrap_or_else(|| "Download failed".to_string()),
                            )
                        };
                        emit(tx, run_id, CatalogTransferEvent::Item { tp2: key, outcome });
                    }
                }
            }
            Step2UpdateDownloadEvent::Finished(_) => break,
        }
    }
    success
}

fn extract_resolved_assets(
    run_id: u64,
    snapshot: &CatalogTransferSnapshot,
    resolved: &mut ResolvedCatalogAssets,
    success: BTreeSet<usize>,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<(u64, CatalogTransferEvent)>,
    settled: &mut BTreeSet<String>,
) {
    for index in success {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let asset = &resolved.assets[index];
        let key = mod_downloads::normalize_mod_download_tp2(&asset.tp_file);
        let Some(source) = snapshot
            .sources
            .iter()
            .find(|source| mod_downloads::normalize_mod_download_tp2(&source.tp2) == key)
        else {
            continue;
        };
        emit(
            tx,
            run_id,
            CatalogTransferEvent::Progress(format!("Extracting {}\u{2026}", asset.label)),
        );
        let job = plan::build_global_extract_job(
            asset,
            source,
            &snapshot.destination.archives,
            &snapshot.destination.mods_root,
            &snapshot.destination.backup_root,
            resolved.remote.remove(&key),
        );
        let result = archive::extract_one_archive_global(&job).and_then(|(root, record)| {
            let folder = snapshot.destination.mods_root.to_string_lossy();
            let path = app_step2_update_source_refs::mods_folder_refs_path(&folder)
                .ok_or_else(|| "No global folder source-reference path is available".to_string())?;
            app_step2_update_source_refs::save_folder_installed_records(&[record], &path, &folder)
                .map_err(|error| {
                    format!(
                        "Extracted to {}, but could not save folder source references: {error}",
                        root.display()
                    )
                })?;
            Ok(root)
        });
        let outcome = match result {
            Ok(root) => CatalogItemOutcome::Completed(root.display().to_string()),
            Err(error) => CatalogItemOutcome::Failed(error),
        };
        settled.insert(key.clone());
        emit(tx, run_id, CatalogTransferEvent::Item { tp2: key, outcome });
    }
}
