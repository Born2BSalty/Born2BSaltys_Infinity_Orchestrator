// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::mod_downloads;
use crate::app::state::{DownloadOrigin, Step2UpdateAsset, WizardState};

pub const DOWNLOAD_POOL_SIZE: usize = 10;

pub type AssetBytes = (u64, Option<u64>);

const BYTES_EVENT_INTERVAL: Duration = Duration::from_millis(250);
const BYTES_EVENT_STEP: u64 = 1024 * 1024;
const READ_CHUNK: usize = 64 * 1024;
const PART_SUFFIX: &str = ".part";
const NOTHING_TO_DOWNLOAD: &str = "No update archives to download";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Step2UpdateDownloadResult {
    pub(crate) downloaded: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) enum Step2UpdateDownloadEvent {
    AssetProgress {
        index: usize,
        bytes: u64,
        total: Option<u64>,
    },
    AssetDone {
        index: usize,
        ok: bool,
        final_bytes: u64,
        total: Option<u64>,
        error: Option<String>,
    },
    Finished(Step2UpdateDownloadResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadRefusal {
    AlreadyRunning,
    ArchiveDisabled,
    ArchiveFolderBlank,
    NothingToDownload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadPoll {
    Idle,
    Finished,
}

struct DownloadGroup {
    leader: usize,
    destination: PathBuf,
    streams: Vec<(String, Vec<usize>)>,
}

struct AssetOutcome {
    index: usize,
    result: Result<PathBuf, String>,
}

struct StreamFailure {
    bytes: u64,
    total: Option<u64>,
    error: String,
}

impl StreamFailure {
    const fn new(bytes: u64, total: Option<u64>, error: String) -> Self {
        Self {
            bytes,
            total,
            error,
        }
    }
}

pub(crate) fn start_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
) -> Result<(), DownloadRefusal> {
    start_step2_update_download_scoped(
        state,
        step2_update_download_rx,
        None,
        &HashSet::new(),
        DownloadOrigin::Workspace,
    )
}

pub(crate) fn start_step2_update_download_scoped(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
    scope_tp2: Option<String>,
    skipped: &HashSet<usize>,
    origin: DownloadOrigin,
) -> Result<(), DownloadRefusal> {
    if state.step2.update_selected_download_running {
        return Err(DownloadRefusal::AlreadyRunning);
    }
    if !state.step1.download_archive {
        refuse_start(state, "Download Archive is disabled in Step 1");
        return Err(DownloadRefusal::ArchiveDisabled);
    }
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        refuse_start(state, "Mods Archive folder is empty");
        return Err(DownloadRefusal::ArchiveFolderBlank);
    }
    let assets = state.step2.update_selected_update_assets.clone();
    let skip = run_skip_set(&assets, scope_tp2.as_deref(), skipped);
    let pending = (0..assets.len())
        .filter(|index| !skip.contains(index))
        .count();
    if origin == DownloadOrigin::Workspace && pending == 0 {
        refuse_start(state, NOTHING_TO_DOWNLOAD);
        return Err(DownloadRefusal::NothingToDownload);
    }
    reset_run_state(state, &assets, scope_tp2, origin, pending);
    if pending == 0 {
        state.step2.scan_status = NOTHING_TO_DOWNLOAD.to_string();
        return Err(DownloadRefusal::NothingToDownload);
    }

    let (tx, rx) = mpsc::channel::<Step2UpdateDownloadEvent>();
    *step2_update_download_rx = Some(rx);
    state.step2.update_selected_download_running = true;
    state.step2.scan_status = format!("Downloading updates: 0/{pending}");

    thread::spawn(move || {
        let result = run_download(&archive_dir, &assets, &skip, &tx);
        let _ = tx.send(Step2UpdateDownloadEvent::Finished(result));
    });
    Ok(())
}

fn refuse_start(state: &mut WizardState, status: &str) {
    state.step2.update_selected_download_scope = None;
    state.step2.scan_status = status.to_string();
}

fn run_skip_set(
    assets: &[Step2UpdateAsset],
    scope_tp2: Option<&str>,
    skipped: &HashSet<usize>,
) -> HashSet<usize> {
    let mut skip = skipped.clone();
    if let Some(scope) = scope_tp2 {
        skip.extend(
            assets
                .iter()
                .enumerate()
                .filter(|(_, asset)| {
                    mod_downloads::normalize_mod_download_tp2(&asset.tp_file) != scope
                })
                .map(|(index, _)| index),
        );
    }
    skip
}

fn reset_run_state(
    state: &mut WizardState,
    assets: &[Step2UpdateAsset],
    scope_tp2: Option<String>,
    origin: DownloadOrigin,
    pending: usize,
) {
    match origin {
        DownloadOrigin::Workspace => {
            let labels = scope_tp2
                .as_deref()
                .map(|scope| scoped_labels(assets, scope));
            clear_download_result_buckets(state, labels.as_deref());
        }
        DownloadOrigin::InstallPipeline => clear_pipeline_result_buckets(state),
    }
    let step2 = &mut state.step2;
    step2.update_selected_download_origin = origin;
    step2.update_selected_download_scope = scope_tp2;
    step2.update_selected_download_bytes.clear();
    step2.update_selected_download_done.clear();
    step2.update_selected_extract_progress = None;
    step2.update_selected_extract_jobs.clear();
    step2.update_selected_download_finished.clear();
    step2.update_selected_download_total = pending;
    step2.update_selected_extract_running = false;
}

fn scoped_labels(assets: &[Step2UpdateAsset], scope: &str) -> Vec<String> {
    let mut labels = assets
        .iter()
        .filter(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) == scope)
        .map(|asset| asset.label.clone())
        .collect::<Vec<_>>();
    labels.dedup();
    labels
}

fn clear_pipeline_result_buckets(state: &mut WizardState) {
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
}

fn clear_download_result_buckets(state: &mut WizardState, scoped_labels: Option<&[String]>) {
    let Some(labels) = scoped_labels else {
        state.step2.update_selected_downloaded_sources.clear();
        state.step2.update_selected_download_failed_sources.clear();
        state.step2.update_selected_extracted_sources.clear();
        state.step2.update_selected_extract_failed_sources.clear();
        return;
    };
    state
        .step2
        .update_selected_downloaded_sources
        .retain(|entry| {
            !labels
                .iter()
                .any(|label| entry.starts_with(&format!("{label} -> ")))
        });
    state
        .step2
        .update_selected_download_failed_sources
        .retain(|entry| {
            !labels
                .iter()
                .any(|label| entry.starts_with(&format!("{label}: ")))
        });
    state
        .step2
        .update_selected_extracted_sources
        .retain(|entry| {
            !labels
                .iter()
                .any(|label| entry.starts_with(&format!("{label} -> ")))
        });
    state
        .step2
        .update_selected_extract_failed_sources
        .retain(|entry| {
            !labels
                .iter()
                .any(|label| entry.starts_with(&format!("{label}: ")))
        });
}

pub(crate) fn poll_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
) -> DownloadPoll {
    let Some(rx) = step2_update_download_rx.as_ref() else {
        return DownloadPoll::Idle;
    };
    let finished = loop {
        match rx.try_recv() {
            Ok(Step2UpdateDownloadEvent::AssetProgress {
                index,
                bytes,
                total,
            }) => {
                state
                    .step2
                    .update_selected_download_bytes
                    .insert(index, (bytes, total));
            }
            Ok(Step2UpdateDownloadEvent::AssetDone {
                index,
                ok,
                final_bytes,
                total,
                error,
            }) => {
                if !ok {
                    tracing::warn!(
                        target = "orchestrator",
                        index,
                        final_bytes,
                        ?total,
                        error = error.as_deref().unwrap_or("unknown error"),
                        "update asset download failed"
                    );
                }
                record_asset_done(state, index, ok, final_bytes, error.as_deref());
            }
            Ok(Step2UpdateDownloadEvent::Finished(result)) => break Some(result),
            Err(TryRecvError::Empty) => return DownloadPoll::Idle,
            Err(TryRecvError::Disconnected) => break None,
        }
    };

    *step2_update_download_rx = None;
    state.step2.update_selected_download_running = false;
    state.step2.update_selected_download_finished.clear();
    let Some(result) = finished else {
        state.step2.update_selected_download_scope = None;
        state.step2.scan_status = "Download updates failed: worker disconnected".to_string();
        return DownloadPoll::Idle;
    };
    tracing::info!(
        target = "orchestrator",
        downloaded = result.downloaded.len(),
        failed = result.failed.len(),
        "update download finished"
    );
    let downloaded = state.step2.update_selected_downloaded_sources.len();
    let failed = state.step2.update_selected_download_failed_sources.len();
    state.step2.scan_status =
        format!("Download updates finished: {downloaded} downloaded, {failed} failed");
    DownloadPoll::Finished
}

fn record_asset_done(
    state: &mut WizardState,
    index: usize,
    ok: bool,
    final_bytes: u64,
    error: Option<&str>,
) {
    state
        .step2
        .update_selected_download_bytes
        .insert(index, (final_bytes, Some(final_bytes)));
    state.step2.update_selected_download_done.insert(index);
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if let Some(asset) = state
        .step2
        .update_selected_update_assets
        .get(index)
        .cloned()
    {
        if ok {
            let destination = archive_dir.join(archive_file_name(&asset));
            state.step2.update_selected_downloaded_sources.push(format!(
                "{} -> {}",
                asset.label,
                destination.display()
            ));
            record_finished_asset(state, &asset.tp_file);
        } else {
            state
                .step2
                .update_selected_download_failed_sources
                .push(format!(
                    "{}: {}",
                    asset.label,
                    error.unwrap_or("unknown error")
                ));
        }
    }
    let done = state.step2.update_selected_download_done.len();
    let total = state.step2.update_selected_download_total;
    state.step2.scan_status = format!("Downloading updates: {done}/{total}");
}

fn record_finished_asset(state: &mut WizardState, tp_file: &str) {
    let key = mod_downloads::normalize_mod_download_tp2(tp_file);
    let finished = &mut state.step2.update_selected_download_finished;
    if !key.is_empty() && !finished.contains(&key) {
        finished.push(key);
    }
}

fn run_download(
    archive_dir: &Path,
    assets: &[Step2UpdateAsset],
    skipped: &HashSet<usize>,
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Step2UpdateDownloadResult {
    let mut result = Step2UpdateDownloadResult::default();
    if let Err(err) = fs::create_dir_all(archive_dir) {
        let error = format!("Mods Archive: {err}");
        for (index, asset) in assets.iter().enumerate() {
            if skipped.contains(&index) {
                continue;
            }
            let _ = tx.send(Step2UpdateDownloadEvent::AssetDone {
                index,
                ok: false,
                final_bytes: 0,
                total: None,
                error: Some(error.clone()),
            });
            result.failed.push(format!("{}: {error}", asset.label));
        }
        return result;
    }
    sweep_stale_part_files(archive_dir, assets);
    let groups = destination_groups(archive_dir, assets, skipped);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_mins(2))
        .build();
    let next = AtomicUsize::new(0);
    let outcomes = Mutex::new(Vec::<AssetOutcome>::new());
    thread::scope(|scope| {
        for _ in 0..DOWNLOAD_POOL_SIZE.min(groups.len()) {
            scope.spawn(|| {
                while let Some(group) = groups.get(next.fetch_add(1, Ordering::SeqCst)) {
                    let group_outcomes = download_group(&agent, group, tx);
                    outcomes
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend(group_outcomes);
                }
            });
        }
    });
    let mut outcomes = outcomes
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    outcomes.sort_by_key(|outcome| outcome.index);
    for outcome in outcomes {
        let Some(asset) = assets.get(outcome.index) else {
            continue;
        };
        match outcome.result {
            Ok(destination) => {
                result
                    .downloaded
                    .push(format!("{} -> {}", asset.label, destination.display()));
            }
            Err(err) => result.failed.push(format!("{}: {err}", asset.label)),
        }
    }
    result
}

fn sweep_stale_part_files(archive_dir: &Path, assets: &[Step2UpdateAsset]) {
    for asset in assets {
        let part = part_path(&archive_dir.join(archive_file_name(asset)));
        let _ = fs::remove_file(part);
    }
}

fn part_path(destination: &Path) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(PART_SUFFIX);
    PathBuf::from(name)
}

fn destination_groups(
    archive_dir: &Path,
    assets: &[Step2UpdateAsset],
    skipped: &HashSet<usize>,
) -> Vec<DownloadGroup> {
    let mut by_destination = BTreeMap::<PathBuf, Vec<usize>>::new();
    for (index, asset) in assets.iter().enumerate() {
        if skipped.contains(&index) {
            continue;
        }
        by_destination
            .entry(archive_dir.join(archive_file_name(asset)))
            .or_default()
            .push(index);
    }
    let mut groups = by_destination
        .into_iter()
        .filter_map(|(destination, indices)| {
            let leader = *indices.first()?;
            Some(DownloadGroup {
                leader,
                destination,
                streams: url_streams(assets, &indices),
            })
        })
        .collect::<Vec<_>>();
    groups.sort_by_key(|group| group.leader);
    groups
}

fn url_streams(assets: &[Step2UpdateAsset], indices: &[usize]) -> Vec<(String, Vec<usize>)> {
    let mut streams = Vec::<(String, Vec<usize>)>::new();
    for &index in indices {
        let Some(asset) = assets.get(index) else {
            continue;
        };
        if let Some((_, members)) = streams.iter_mut().find(|(url, _)| *url == asset.asset_url) {
            members.push(index);
        } else {
            streams.push((asset.asset_url.clone(), vec![index]));
        }
    }
    streams
}

fn download_group(
    agent: &ureq::Agent,
    group: &DownloadGroup,
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Vec<AssetOutcome> {
    let mut outcomes = Vec::new();
    for (url, members) in &group.streams {
        let outcome = stream_to_destination(agent, url, &group.destination, members, tx);
        for &index in members {
            let (final_bytes, total, error) = match &outcome {
                Ok((bytes, total)) => (*bytes, *total, None),
                Err(failure) => (failure.bytes, failure.total, Some(failure.error.clone())),
            };
            let _ = tx.send(Step2UpdateDownloadEvent::AssetDone {
                index,
                ok: error.is_none(),
                final_bytes,
                total,
                error: error.clone(),
            });
            outcomes.push(AssetOutcome {
                index,
                result: error.map_or_else(|| Ok(group.destination.clone()), Err),
            });
        }
    }
    outcomes
}

fn stream_to_destination(
    agent: &ureq::Agent,
    url: &str,
    destination: &Path,
    members: &[usize],
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Result<AssetBytes, StreamFailure> {
    let part = part_path(destination);
    let outcome = stream_into_part(agent, url, &part, members, tx).and_then(|(bytes, total)| {
        fs::rename(&part, destination)
            .map(|()| (bytes, total))
            .map_err(|err| StreamFailure::new(bytes, total, err.to_string()))
    });
    if outcome.is_err() {
        let _ = fs::remove_file(&part);
    }
    outcome
}

fn stream_into_part(
    agent: &ureq::Agent,
    url: &str,
    part: &Path,
    members: &[usize],
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Result<AssetBytes, StreamFailure> {
    let response = agent
        .get(url)
        .set("User-Agent", "BIO-update-download")
        .call()
        .map_err(|err| StreamFailure::new(0, None, err.to_string()))?;
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(StreamFailure::new(0, None, format!("HTTP {status}")));
    }
    let total = response
        .header("Content-Length")
        .and_then(|value| value.trim().parse::<u64>().ok());
    let mut file =
        fs::File::create(part).map_err(|err| StreamFailure::new(0, total, err.to_string()))?;
    send_progress(tx, members, 0, total);
    let mut reader = response.into_reader();
    let mut buffer = vec![0_u8; READ_CHUNK];
    let (mut done, mut sent_done, mut sent_at) = (0_u64, 0_u64, Instant::now());
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(StreamFailure::new(done, total, err.to_string())),
        };
        file.write_all(&buffer[..read])
            .map_err(|err| StreamFailure::new(done, total, err.to_string()))?;
        done = done.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        if done - sent_done >= BYTES_EVENT_STEP || sent_at.elapsed() >= BYTES_EVENT_INTERVAL {
            send_progress(tx, members, done, total);
            (sent_done, sent_at) = (done, Instant::now());
        }
    }
    file.flush()
        .map_err(|err| StreamFailure::new(done, total, err.to_string()))?;
    drop(file);
    send_progress(tx, members, done, total);
    Ok((done, total))
}

fn send_progress(
    tx: &Sender<Step2UpdateDownloadEvent>,
    members: &[usize],
    bytes: u64,
    total: Option<u64>,
) {
    for &index in members {
        let _ = tx.send(Step2UpdateDownloadEvent::AssetProgress {
            index,
            bytes,
            total,
        });
    }
}

pub(crate) fn archive_file_name(asset: &Step2UpdateAsset) -> String {
    let tp2 = safe_archive_segment(&tp2_archive_name(&asset.tp_file));
    let source = safe_archive_segment(&asset.source_id);
    let tag = safe_archive_segment(&asset.tag);
    let ext = archive_extension(&asset.asset_name);
    format!("{tp2}__{source}__{tag}{ext}")
}

pub(crate) fn tp2_archive_name(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let file = replaced.rsplit('/').next().unwrap_or(&replaced).trim();
    let lower = file.to_ascii_lowercase();
    let without_ext = lower.strip_suffix(".tp2").unwrap_or(&lower);
    without_ext.to_string()
}

pub(crate) fn archive_extension(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for ext in [
        ".tar.gz", ".tar.bz2", ".tar.xz", ".zip", ".7z", ".rar", ".tgz", ".tbz2", ".txz",
    ] {
        if lower.ends_with(ext) {
            return ext.to_string();
        }
    }
    ".zip".to_string()
}

pub(crate) fn safe_archive_segment(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '@' => '-',
            _ if ch.is_control() || ch.is_whitespace() => '-',
            _ => ch,
        })
        .collect::<String>();
    let sanitized = sanitized.trim_matches([' ', '.', '-']).trim();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    struct EngineTestRoot {
        path: PathBuf,
        readonly: Vec<(PathBuf, fs::Permissions)>,
    }

    impl EngineTestRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_dl_engine_{}_{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let root = Self {
                path,
                readonly: Vec::new(),
            };
            fs::create_dir_all(&root.path).unwrap();
            root
        }

        fn archive_dir(&self) -> PathBuf {
            self.path.join("archives")
        }

        #[cfg(windows)]
        fn make_readonly(&mut self, file: &Path) {
            let original = fs::metadata(file).unwrap().permissions();
            let mut readonly = original.clone();
            readonly.set_readonly(true);
            self.readonly.push((file.to_path_buf(), original));
            fs::set_permissions(file, readonly).unwrap();
        }
    }

    impl Drop for EngineTestRoot {
        fn drop(&mut self) {
            for (file, original) in self.readonly.drain(..) {
                let _ = fs::set_permissions(&file, original);
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[derive(Default)]
    struct FixtureLog {
        hits: Mutex<Vec<String>>,
        spans: Mutex<Vec<(String, Instant, Instant)>>,
    }

    impl FixtureLog {
        fn span(&self, path: &str) -> (Instant, Instant) {
            self.spans
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .find(|(span_path, _, _)| span_path == path)
                .map(|(_, started, last_write)| (*started, *last_write))
                .expect("request served")
        }

        fn hits(&self) -> Vec<String> {
            self.hits
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        fn hit_count(&self, path: &str) -> usize {
            self.hits().iter().filter(|hit| *hit == path).count()
        }
    }

    struct Fixture {
        base: String,
        log: Arc<FixtureLog>,
    }

    fn serve(bodies: Vec<(&'static str, Vec<u8>)>) -> Fixture {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let log = Arc::new(FixtureLog::default());
        let bodies = Arc::new(bodies);
        let server_log = Arc::clone(&log);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let bodies = Arc::clone(&bodies);
                let log = Arc::clone(&server_log);
                thread::spawn(move || answer(stream, &bodies, &log));
            }
        });
        Fixture { base, log }
    }

    fn answer(mut stream: TcpStream, bodies: &[(&'static str, Vec<u8>)], log: &FixtureLog) {
        let mut request = [0_u8; 2048];
        let read = stream.read(&mut request).unwrap_or(0);
        let path = String::from_utf8_lossy(&request[..read])
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        let started = Instant::now();
        let mut last_write = started;
        log.hits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(path.clone());
        let name = path.rsplit('/').next().unwrap_or_default();
        let body = bodies
            .iter()
            .find(|(body_name, _)| *body_name == name)
            .map(|(_, body)| body.clone())
            .unwrap_or_default();
        let mode = path.split('/').nth(1).unwrap_or_default();
        match mode {
            "404" => {
                let _ = stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
            "nocl" => {
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n");
                let _ = stream.write_all(&body);
            }
            "cut" => {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len() + 4096
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
            }
            "slow" => {
                write_ok_header(&mut stream, body.len());
                for chunk in body.chunks(body.len().div_ceil(4).max(1)) {
                    thread::sleep(Duration::from_millis(40));
                    last_write = Instant::now();
                    let _ = stream.write_all(chunk);
                    let _ = stream.flush();
                }
            }
            _ => {
                write_ok_header(&mut stream, body.len());
                last_write = Instant::now();
                let _ = stream.write_all(&body);
            }
        }
        log.spans
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((path, started, last_write));
        let _ = stream.flush();
    }

    fn write_ok_header(stream: &mut TcpStream, length: usize) {
        let header =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n");
        let _ = stream.write_all(header.as_bytes());
    }

    fn asset(tp_file: &str, label: &str, url: String) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: url,
            installed_source_ref: None,
        }
    }

    fn state_for(root: &EngineTestRoot, assets: Vec<Step2UpdateAsset>) -> WizardState {
        let mut state = WizardState::default();
        state.step1.download_archive = true;
        state.step1.mods_archive_folder = root.archive_dir().to_string_lossy().into_owned();
        state.step2.update_selected_update_assets = assets;
        state
    }

    fn destination(root: &EngineTestRoot, asset: &Step2UpdateAsset) -> PathBuf {
        root.archive_dir().join(archive_file_name(asset))
    }

    fn collect_events(
        rx: Option<&Receiver<Step2UpdateDownloadEvent>>,
    ) -> (Vec<Step2UpdateDownloadEvent>, Step2UpdateDownloadResult) {
        let rx = rx.expect("engine spawned");
        let mut events = Vec::new();
        loop {
            match rx
                .recv_timeout(Duration::from_secs(30))
                .expect("engine finishes")
            {
                Step2UpdateDownloadEvent::Finished(result) => return (events, result),
                event => events.push(event),
            }
        }
    }

    fn start_and_collect(
        state: &mut WizardState,
        skipped: &HashSet<usize>,
        origin: DownloadOrigin,
    ) -> (Vec<Step2UpdateDownloadEvent>, Step2UpdateDownloadResult) {
        let mut rx = None;
        start_step2_update_download_scoped(state, &mut rx, None, skipped, origin)
            .expect("engine starts");
        collect_events(rx.as_ref())
    }

    fn poll_until_finished(
        state: &mut WizardState,
        rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
    ) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while poll_step2_update_download(state, rx) != DownloadPoll::Finished {
            assert!(rx.is_some(), "engine disconnected before Finished");
            assert!(Instant::now() < deadline, "engine did not finish in time");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn done_events(events: &[Step2UpdateDownloadEvent]) -> Vec<(usize, bool, Option<String>)> {
        events
            .iter()
            .filter_map(|event| match event {
                Step2UpdateDownloadEvent::AssetDone {
                    index, ok, error, ..
                } => Some((*index, *ok, error.clone())),
                _ => None,
            })
            .collect()
    }

    fn event_indices(events: &[Step2UpdateDownloadEvent]) -> Vec<usize> {
        events
            .iter()
            .filter_map(|event| match event {
                Step2UpdateDownloadEvent::AssetProgress { index, .. }
                | Step2UpdateDownloadEvent::AssetDone { index, .. } => Some(*index),
                Step2UpdateDownloadEvent::Finished(_) => None,
            })
            .collect()
    }

    fn queued(
        events: Vec<Step2UpdateDownloadEvent>,
    ) -> (
        Sender<Step2UpdateDownloadEvent>,
        Option<Receiver<Step2UpdateDownloadEvent>>,
    ) {
        let (tx, rx) = mpsc::channel();
        for event in events {
            tx.send(event).unwrap();
        }
        (tx, Some(rx))
    }

    fn done(index: usize, ok: bool, error: Option<&str>) -> Step2UpdateDownloadEvent {
        Step2UpdateDownloadEvent::AssetDone {
            index,
            ok,
            final_bytes: 10,
            total: Some(10),
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn download_pool_size_is_ten() {
        assert_eq!(DOWNLOAD_POOL_SIZE, 10);
    }

    #[test]
    fn writes_exact_path_and_bio_shaped_vectors_with_content_length() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![
            ("A", b"AAA-bytes-payload".to_vec()),
            ("B", vec![7_u8; 4096]),
        ]);
        let a = asset("AMOD/AMOD.TP2", "AMOD", format!("{}/cl/A", fixture.base));
        let b = asset("BMOD/BMOD.TP2", "BMOD", format!("{}/cl/B", fixture.base));
        let mut state = state_for(&root, vec![a.clone(), b.clone()]);

        let (_, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        let (dest_a, dest_b) = (destination(&root, &a), destination(&root, &b));
        assert_eq!(fs::read(&dest_a).unwrap(), b"AAA-bytes-payload");
        assert_eq!(fs::read(&dest_b).unwrap(), vec![7_u8; 4096]);
        assert!(!part_path(&dest_a).exists() && !part_path(&dest_b).exists());
        assert!(result.failed.is_empty());
        assert_eq!(
            result.downloaded,
            vec![
                format!("AMOD -> {}", dest_a.display()),
                format!("BMOD -> {}", dest_b.display()),
            ]
        );
    }

    #[test]
    fn no_content_length_is_graceful_indeterminate_total() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("N", b"no-content-length-body".to_vec())]);
        let a = asset("NMOD/NMOD.TP2", "NMOD", format!("{}/nocl/N", fixture.base));
        let mut state = state_for(&root, vec![a.clone()]);

        let (events, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        assert!(events.iter().all(|event| match event {
            Step2UpdateDownloadEvent::AssetProgress { total, .. }
            | Step2UpdateDownloadEvent::AssetDone { total, .. } => total.is_none(),
            Step2UpdateDownloadEvent::Finished(_) => true,
        }));
        assert!(result.failed.is_empty());
        assert_eq!(
            fs::read(destination(&root, &a)).unwrap(),
            b"no-content-length-body"
        );
    }

    #[test]
    fn failure_records_bio_shaped_failed_and_does_not_abort_pool() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("C", b"C-good-bytes".to_vec())]);
        let good = asset("CMOD/CMOD.TP2", "CMOD", format!("{}/cl/C", fixture.base));
        let bad = asset("XMOD/XMOD.TP2", "XMOD", format!("{}/404/X", fixture.base));
        let mut state = state_for(&root, vec![good.clone(), bad]);

        let (_, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        let dest_good = destination(&root, &good);
        assert!(dest_good.exists());
        assert_eq!(
            result.downloaded,
            vec![format!("CMOD -> {}", dest_good.display())]
        );
        assert_eq!(result.failed.len(), 1);
        assert!(result.failed[0].starts_with("XMOD: "));
    }

    #[test]
    fn empty_asset_set_finishes_cleanly() {
        let root = EngineTestRoot::new();
        let mut state = state_for(&root, Vec::new());
        let mut rx = None;

        let started = start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            None,
            &HashSet::new(),
            DownloadOrigin::InstallPipeline,
        );

        assert_eq!(started, Err(DownloadRefusal::NothingToDownload));
        assert!(rx.is_none());
        assert!(!state.step2.update_selected_download_running);
    }

    #[test]
    fn re_entry_guard_refuses_when_already_running() {
        let root = EngineTestRoot::new();
        let mut state = state_for(&root, Vec::new());
        state.step2.update_selected_download_running = true;
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        state.step2.scan_status = "busy".to_string();
        let mut rx = None;

        assert_eq!(
            start_step2_update_download(&mut state, &mut rx),
            Err(DownloadRefusal::AlreadyRunning)
        );
        assert!(rx.is_none());
        assert_eq!(
            state.step2.update_selected_download_scope.as_deref(),
            Some("alpha")
        );
        assert_eq!(state.step2.scan_status, "busy");
    }

    #[test]
    fn finished_flips_flag_and_sets_status_without_bulk_assigning_vectors() {
        let mut state = WizardState::default();
        state.step2.update_selected_downloaded_sources =
            vec!["MyMod -> C:\\arch\\MyMod.zip".to_string()];
        state.step2.update_selected_download_failed_sources = vec!["BadMod: HTTP 404".to_string()];
        state.step2.update_selected_download_running = true;
        let (_tx, mut rx) = queued(vec![Step2UpdateDownloadEvent::Finished(
            Step2UpdateDownloadResult {
                downloaded: vec!["Other -> C:\\x.zip".to_string()],
                failed: vec!["Other: boom".to_string()],
            },
        )]);

        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Finished
        );

        assert!(!state.step2.update_selected_download_running);
        assert!(rx.is_none());
        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec!["MyMod -> C:\\arch\\MyMod.zip".to_string()]
        );
        assert_eq!(
            state.step2.update_selected_download_failed_sources,
            vec!["BadMod: HTTP 404".to_string()]
        );
        assert_eq!(
            state.step2.scan_status,
            "Download updates finished: 1 downloaded, 1 failed"
        );
    }

    #[test]
    fn asset_done_carries_error_string_for_per_asset_failed_push() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("D", b"D-good".to_vec())]);
        let good = asset("DMOD/DMOD.TP2", "DMOD", format!("{}/cl/D", fixture.base));
        let bad = asset("EMOD/EMOD.TP2", "EMOD", format!("{}/404/E", fixture.base));
        let mut state = state_for(&root, vec![good, bad]);

        let (events, _) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        let mut dones = done_events(&events);
        dones.sort_by_key(|(index, _, _)| *index);
        assert_eq!(dones.len(), 2);
        assert_eq!(dones[0], (0, true, None));
        assert!(!dones[1].1);
        assert!(dones[1].2.as_deref().is_some_and(|err| !err.is_empty()));
    }

    #[test]
    fn scoped_download_keeps_other_mods_results() {
        let mut state = WizardState::default();
        state.step2.update_selected_downloaded_sources =
            vec!["Alpha -> C:\\a".to_string(), "Beta -> C:\\b".to_string()];
        state.step2.update_selected_download_failed_sources = vec!["Alpha: err".to_string()];
        state.step2.update_selected_extracted_sources = vec!["Alpha -> C:\\a2".to_string()];
        state.step2.update_selected_extract_failed_sources = vec!["Alpha: err2".to_string()];

        clear_download_result_buckets(&mut state, Some(&["Alpha".to_string()]));

        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec!["Beta -> C:\\b".to_string()]
        );
        assert!(
            state
                .step2
                .update_selected_download_failed_sources
                .is_empty()
        );
        assert!(state.step2.update_selected_extracted_sources.is_empty());
        assert!(
            state
                .step2
                .update_selected_extract_failed_sources
                .is_empty()
        );
    }

    #[test]
    fn full_download_still_clears_every_result_bucket() {
        let mut state = WizardState::default();
        state.step2.update_selected_downloaded_sources = vec!["Alpha -> C:\\a".to_string()];
        state.step2.update_selected_download_failed_sources = vec!["Beta: err".to_string()];
        state.step2.update_selected_extracted_sources = vec!["Gamma -> C:\\g".to_string()];
        state.step2.update_selected_extract_failed_sources = vec!["Delta: err".to_string()];

        clear_download_result_buckets(&mut state, None);

        assert!(state.step2.update_selected_downloaded_sources.is_empty());
        assert!(
            state
                .step2
                .update_selected_download_failed_sources
                .is_empty()
        );
        assert!(state.step2.update_selected_extracted_sources.is_empty());
        assert!(
            state
                .step2
                .update_selected_extract_failed_sources
                .is_empty()
        );
    }

    #[test]
    fn failed_asset_is_not_recorded_as_finished() {
        let mut state = WizardState::default();
        state.step2.update_selected_update_assets = vec![
            asset("a.tp2", "A", String::new()),
            asset("b.tp2", "B", String::new()),
        ];
        state.step2.update_selected_download_running = true;
        let (_tx, mut rx) = queued(vec![done(0, false, Some("HTTP 404")), done(1, true, None)]);

        poll_step2_update_download(&mut state, &mut rx);

        assert_eq!(
            state.step2.update_selected_download_finished,
            vec!["b".to_string()]
        );
    }

    #[test]
    fn scoped_download_completion_appends_results() {
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = "C:\\arch".to_string();
        state.step2.update_selected_update_assets =
            vec![asset("alpha.tp2", "Alpha", String::new())];
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        state.step2.update_selected_downloaded_sources = vec!["Beta -> C:\\b".to_string()];
        let dest = PathBuf::from("C:\\arch").join(archive_file_name(
            &state.step2.update_selected_update_assets[0],
        ));
        let (_tx, mut rx) = queued(vec![
            done(0, true, None),
            Step2UpdateDownloadEvent::Finished(Step2UpdateDownloadResult::default()),
        ]);

        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Finished
        );

        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec![
                "Beta -> C:\\b".to_string(),
                format!("Alpha -> {}", dest.display())
            ]
        );
    }

    #[test]
    fn same_destination_same_url_downloads_once_and_reports_every_index() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("S", b"shared-archive".to_vec())]);
        let url = format!("{}/slow/S", fixture.base);
        let first = asset("SMOD/SMOD.TP2", "SMOD", url.clone());
        let second = Step2UpdateAsset {
            game_tab: "BG2EE".to_string(),
            ..asset("SMOD/SMOD.TP2", "SMOD", url)
        };
        let dest = destination(&root, &first);
        let mut state = state_for(&root, vec![first, second]);
        let mut rx = None;

        start_step2_update_download(&mut state, &mut rx).expect("engine starts");
        poll_until_finished(&mut state, &mut rx);

        assert_eq!(fixture.log.hit_count("/slow/S"), 1);
        assert_eq!(fs::read(&dest).unwrap(), b"shared-archive");
        assert_eq!(
            state.step2.update_selected_download_done,
            [0, 1].into_iter().collect()
        );
        let line = format!("SMOD -> {}", dest.display());
        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec![line.clone(), line]
        );
    }

    #[test]
    fn same_destination_different_urls_stream_in_order_never_concurrently() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("P", vec![1_u8; 2048]), ("Q", vec![2_u8; 2048])]);
        let first = asset("TMOD/TMOD.TP2", "TMOD", format!("{}/slow/P", fixture.base));
        let second = Step2UpdateAsset {
            game_tab: "BG2EE".to_string(),
            ..asset("TMOD/TMOD.TP2", "TMOD", format!("{}/slow/Q", fixture.base))
        };
        let dest = destination(&root, &first);
        let mut state = state_for(&root, vec![first, second]);

        let (events, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::Workspace);

        assert_eq!(
            fixture.log.hits(),
            vec!["/slow/P".to_string(), "/slow/Q".to_string()]
        );
        let (_, first_last_write) = fixture.log.span("/slow/P");
        let (second_started, _) = fixture.log.span("/slow/Q");
        assert!(
            second_started >= first_last_write,
            "the second URL of a group is requested only after the first stream completes"
        );
        assert_eq!(fs::read(&dest).unwrap(), vec![2_u8; 2048]);
        assert_eq!(done_events(&events), vec![(0, true, None), (1, true, None)]);
        assert_eq!(result.downloaded.len(), 2);
    }

    #[test]
    fn failed_stream_removes_its_part_file_and_leaves_the_archive_path_alone() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("X", vec![9_u8; 64])]);
        let a = asset("XMOD/XMOD.TP2", "XMOD", format!("{}/cut/X", fixture.base));
        let dest = destination(&root, &a);
        fs::create_dir_all(root.archive_dir()).unwrap();
        fs::write(&dest, b"old-archive").unwrap();
        let mut state = state_for(&root, vec![a]);

        let (events, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        assert!(matches!(
            done_events(&events).as_slice(),
            [(0, false, Some(_))]
        ));
        assert!(!part_path(&dest).exists());
        assert_eq!(fs::read(&dest).unwrap(), b"old-archive");
        assert!(result.downloaded.is_empty());
        assert_eq!(result.failed.len(), 1);
    }

    #[test]
    fn failed_request_keeps_an_existing_archive() {
        let root = EngineTestRoot::new();
        let fixture = serve(Vec::new());
        let a = asset(
            "DEAD/DEAD.TP2",
            "DEAD",
            format!("{}/404/DEAD", fixture.base),
        );
        let dest = destination(&root, &a);
        fs::create_dir_all(root.archive_dir()).unwrap();
        fs::write(&dest, b"kept").unwrap();
        let mut state = state_for(&root, vec![a]);

        let (events, _) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        assert!(matches!(
            done_events(&events).as_slice(),
            [(0, false, Some(_))]
        ));
        assert_eq!(fs::read(&dest).unwrap(), b"kept");
        assert!(!part_path(&dest).exists());
    }

    #[test]
    fn later_url_failure_in_a_group_keeps_the_earlier_archive() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("G", b"good-first".to_vec())]);
        let first = asset("GMOD/GMOD.TP2", "GMOD", format!("{}/cl/G", fixture.base));
        let second = Step2UpdateAsset {
            game_tab: "BG2EE".to_string(),
            ..asset("GMOD/GMOD.TP2", "GMOD", format!("{}/404/G", fixture.base))
        };
        let dest = destination(&root, &first);
        let mut state = state_for(&root, vec![first, second]);

        let (events, _) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        let dones = done_events(&events);
        assert_eq!(dones.len(), 2);
        assert_eq!(dones[0], (0, true, None));
        assert!(!dones[1].1 && dones[1].0 == 1);
        assert_eq!(fs::read(&dest).unwrap(), b"good-first");
        assert!(!part_path(&dest).exists());
    }

    #[cfg(windows)]
    #[test]
    fn rename_failure_reports_the_asset_as_failed_and_keeps_the_old_archive() {
        let mut root = EngineTestRoot::new();
        let fixture = serve(vec![("R", b"new-archive-bytes".to_vec())]);
        let a = asset("RMOD/RMOD.TP2", "RMOD", format!("{}/cl/R", fixture.base));
        let dest = destination(&root, &a);
        fs::create_dir_all(root.archive_dir()).unwrap();
        fs::write(&dest, b"old-archive").unwrap();
        root.make_readonly(&dest);
        let mut state = state_for(&root, vec![a]);

        let (events, result) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        assert_eq!(fixture.log.hit_count("/cl/R"), 1);
        assert!(matches!(
            done_events(&events).as_slice(),
            [(0, false, Some(_))]
        ));
        assert!(!part_path(&dest).exists());
        assert_eq!(fs::read(&dest).unwrap(), b"old-archive");
        assert!(result.downloaded.is_empty());
        assert_eq!(result.failed.len(), 1);
    }

    #[test]
    fn start_sweeps_stale_part_files_for_the_run() {
        let root = EngineTestRoot::new();
        let fixture = serve(Vec::new());
        let skipped_asset = asset("KMOD/KMOD.TP2", "KMOD", format!("{}/404/K", fixture.base));
        let fetched = asset("LMOD/LMOD.TP2", "LMOD", format!("{}/404/L", fixture.base));
        fs::create_dir_all(root.archive_dir()).unwrap();
        let stale_skipped = part_path(&destination(&root, &skipped_asset));
        let stale_fetched = part_path(&destination(&root, &fetched));
        let unrelated = root.archive_dir().join("someone_else.zip.part");
        for stale in [&stale_skipped, &stale_fetched, &unrelated] {
            fs::write(stale, b"stale").unwrap();
        }
        let mut state = state_for(&root, vec![skipped_asset, fetched]);

        start_and_collect(
            &mut state,
            &HashSet::from([0]),
            DownloadOrigin::InstallPipeline,
        );

        assert!(!stale_skipped.exists());
        assert!(!stale_fetched.exists());
        assert!(unrelated.exists());
    }

    #[test]
    fn pipeline_nothing_to_download_refusal_leaves_origin_scope_and_cleared_buckets_in_place() {
        let root = EngineTestRoot::new();
        let mut state = state_for(&root, vec![asset("a.tp2", "A", "http://x/a".to_string())]);
        state.step2.update_selected_download_scope = Some("leftover".to_string());
        state.step2.update_selected_downloaded_sources = vec!["A -> C:\\a.zip".to_string()];
        state.step2.update_selected_download_failed_sources = vec!["B: err".to_string()];
        state.step2.update_selected_extracted_sources = vec!["C -> C:\\c".to_string()];
        state.step2.update_selected_extract_failed_sources = vec!["D: err".to_string()];
        state
            .step2
            .update_selected_download_bytes
            .insert(4, (1, None));
        state.step2.update_selected_download_done.insert(4);
        let mut rx = None;

        let started = start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            None,
            &HashSet::from([0]),
            DownloadOrigin::InstallPipeline,
        );

        assert_eq!(started, Err(DownloadRefusal::NothingToDownload));
        assert!(rx.is_none());
        let step2 = &state.step2;
        assert_eq!(
            step2.update_selected_download_origin,
            DownloadOrigin::InstallPipeline
        );
        assert_eq!(step2.update_selected_download_scope, None);
        assert_eq!(
            step2.update_selected_downloaded_sources,
            vec!["A -> C:\\a.zip".to_string()]
        );
        assert!(step2.update_selected_download_failed_sources.is_empty());
        assert!(step2.update_selected_extracted_sources.is_empty());
        assert!(step2.update_selected_extract_failed_sources.is_empty());
        assert!(step2.update_selected_download_bytes.is_empty());
        assert!(step2.update_selected_download_done.is_empty());
        assert!(!step2.update_selected_download_running);
        assert!(!root.archive_dir().exists());
    }

    #[test]
    fn workspace_nothing_to_download_refusal_leaves_every_bucket_untouched() {
        let root = EngineTestRoot::new();
        let mut state = state_for(&root, vec![asset("a.tp2", "A", "http://x/a".to_string())]);
        state.step2.update_selected_downloaded_sources = vec!["A -> C:\\a.zip".to_string()];
        state.step2.update_selected_download_failed_sources = vec!["B: err".to_string()];
        state.step2.update_selected_extracted_sources = vec!["A -> C:\\mods\\a".to_string()];
        state.step2.update_selected_extract_failed_sources = vec!["D: err".to_string()];
        state
            .step2
            .update_selected_download_bytes
            .insert(0, (5, Some(5)));
        state.step2.update_selected_download_done.insert(0);
        state
            .step2
            .update_selected_download_finished
            .push("a".to_string());
        state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        state.step2.update_selected_extract_progress = Some((11, 11));
        state
            .step2
            .update_selected_extract_jobs
            .entry("a".to_string())
            .or_default()
            .done = 1;
        let before = state.step2.clone();
        let mut rx = None;

        let started = start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            Some("zzz".to_string()),
            &HashSet::new(),
            DownloadOrigin::Workspace,
        );

        assert_eq!(started, Err(DownloadRefusal::NothingToDownload));
        assert!(rx.is_none());
        let step2 = &state.step2;
        assert_eq!(step2.update_selected_download_scope, None);
        assert_eq!(step2.scan_status, "No update archives to download");
        assert_eq!(
            step2.update_selected_downloaded_sources,
            before.update_selected_downloaded_sources
        );
        assert_eq!(
            step2.update_selected_download_failed_sources,
            before.update_selected_download_failed_sources
        );
        assert_eq!(
            step2.update_selected_extracted_sources,
            before.update_selected_extracted_sources
        );
        assert_eq!(
            step2.update_selected_extract_failed_sources,
            before.update_selected_extract_failed_sources
        );
        assert_eq!(
            step2.update_selected_download_bytes,
            before.update_selected_download_bytes
        );
        assert_eq!(
            step2.update_selected_download_done,
            before.update_selected_download_done
        );
        assert_eq!(
            step2.update_selected_download_finished,
            before.update_selected_download_finished
        );
        assert_eq!(
            step2.update_selected_download_origin,
            DownloadOrigin::InstallPipeline
        );
        assert_eq!(step2.update_selected_extract_progress, Some((11, 11)));
        assert_eq!(
            step2.update_selected_extract_jobs,
            before.update_selected_extract_jobs
        );
    }

    #[test]
    fn stream_start_emits_a_zero_byte_progress_event() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("Z", vec![3_u8; 1024])]);
        let a = asset("ZMOD/ZMOD.TP2", "ZMOD", format!("{}/slow/Z", fixture.base));
        let mut state = state_for(&root, vec![a]);

        let (events, _) =
            start_and_collect(&mut state, &HashSet::new(), DownloadOrigin::InstallPipeline);

        assert!(matches!(
            events.first(),
            Some(Step2UpdateDownloadEvent::AssetProgress {
                index: 0,
                bytes: 0,
                total: Some(1024),
            })
        ));
        assert!(matches!(
            events.iter().rev().nth(1),
            Some(Step2UpdateDownloadEvent::AssetProgress {
                index: 0,
                bytes: 1024,
                ..
            })
        ));
    }

    #[test]
    fn skipped_indices_emit_no_events() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("A", b"a".to_vec()), ("B", b"b".to_vec())]);
        let a = asset("AMOD/AMOD.TP2", "AMOD", format!("{}/cl/A", fixture.base));
        let b = asset("BMOD/BMOD.TP2", "BMOD", format!("{}/cl/B", fixture.base));
        let mut state = state_for(&root, vec![a, b]);

        let (events, result) = start_and_collect(
            &mut state,
            &HashSet::from([0]),
            DownloadOrigin::InstallPipeline,
        );

        assert!(!events.is_empty());
        assert!(event_indices(&events).iter().all(|index| *index == 1));
        assert_eq!(fixture.log.hits(), vec!["/cl/B".to_string()]);
        assert_eq!(result.downloaded.len(), 1);
        assert!(result.downloaded[0].starts_with("BMOD -> "));
    }

    #[test]
    fn scoped_start_reports_absolute_indices() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("M7", b"seven".to_vec())]);
        let assets = (0..11)
            .map(|n| {
                asset(
                    &format!("mod{n}/setup-mod{n}.tp2"),
                    &format!("Mod{n}"),
                    format!("{}/cl/M{n}", fixture.base),
                )
            })
            .collect::<Vec<_>>();
        let dest = destination(&root, &assets[7]);
        let mut state = state_for(&root, assets);
        state.step2.update_selected_extract_progress = Some((11, 11));
        state
            .step2
            .update_selected_extract_jobs
            .entry("mod7".to_string())
            .or_default()
            .done = 1;
        let mut rx = None;

        start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            Some("mod7".to_string()),
            &HashSet::new(),
            DownloadOrigin::Workspace,
        )
        .expect("engine starts");
        assert_eq!(state.step2.scan_status, "Downloading updates: 0/1");
        assert_eq!(state.step2.update_selected_extract_progress, None);
        assert!(state.step2.update_selected_extract_jobs.is_empty());
        let (events, result) = collect_events(rx.as_ref());

        assert!(!events.is_empty());
        assert!(event_indices(&events).iter().all(|index| *index == 7));
        assert_eq!(
            result.downloaded,
            vec![format!("Mod7 -> {}", dest.display())]
        );
        let (_tx, mut rx) = queued(events);
        poll_step2_update_download(&mut state, &mut rx);
        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec![format!("Mod7 -> {}", dest.display())]
        );
        assert_eq!(
            state.step2.update_selected_download_finished,
            vec!["mod7".to_string()]
        );
    }

    #[test]
    fn pipeline_start_keeps_skip_hit_downloaded_entries() {
        let root = EngineTestRoot::new();
        let fixture = serve(vec![("B", b"b-bytes".to_vec())]);
        let a = asset("AMOD/AMOD.TP2", "AMOD", format!("{}/cl/A", fixture.base));
        let b = asset("BMOD/BMOD.TP2", "BMOD", format!("{}/cl/B", fixture.base));
        let seeded = format!("AMOD -> {}", destination(&root, &a).display());
        let dest_b = destination(&root, &b);
        let mut state = state_for(&root, vec![a, b]);
        state.step2.update_selected_downloaded_sources = vec![seeded.clone()];
        let mut rx = None;

        start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            None,
            &HashSet::from([0]),
            DownloadOrigin::InstallPipeline,
        )
        .expect("engine starts");
        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec![seeded.clone()]
        );
        poll_until_finished(&mut state, &mut rx);

        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec![seeded, format!("BMOD -> {}", dest_b.display())]
        );
    }

    #[test]
    fn pipeline_start_clears_a_leftover_scope() {
        let root = EngineTestRoot::new();
        let fixture = serve(Vec::new());
        let a = asset("AMOD/AMOD.TP2", "AMOD", format!("{}/404/A", fixture.base));
        let mut state = state_for(&root, vec![a]);
        state.step2.update_selected_download_scope = Some("amod".to_string());
        state.step2.update_selected_extract_progress = Some((11, 11));
        state
            .step2
            .update_selected_extract_jobs
            .entry("amod".to_string())
            .or_default()
            .done = 1;
        let mut rx = None;

        start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            None,
            &HashSet::new(),
            DownloadOrigin::InstallPipeline,
        )
        .expect("engine starts");
        assert_eq!(state.step2.update_selected_download_scope, None);
        assert_eq!(state.step2.update_selected_extract_progress, None);
        assert!(state.step2.update_selected_extract_jobs.is_empty());
        assert_eq!(
            state.step2.update_selected_download_origin,
            DownloadOrigin::InstallPipeline
        );
        poll_until_finished(&mut state, &mut rx);
        assert_eq!(state.step2.update_selected_download_scope, None);
    }

    #[test]
    fn reset_run_state_clears_the_extract_tally() {
        let root = EngineTestRoot::new();
        let fixture = serve(Vec::new());
        let a = asset("AMOD/AMOD.TP2", "AMOD", format!("{}/404/A", fixture.base));
        let mut state = state_for(&root, vec![a]);
        state
            .step2
            .update_selected_extract_jobs
            .entry("amod".to_string())
            .or_default()
            .total = 2;
        let mut rx = None;

        start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            Some("amod".to_string()),
            &HashSet::new(),
            DownloadOrigin::Workspace,
        )
        .expect("engine starts");

        assert!(state.step2.update_selected_extract_jobs.is_empty());
        poll_until_finished(&mut state, &mut rx);
        assert!(state.step2.update_selected_extract_jobs.is_empty());
    }

    #[test]
    fn poll_returns_finished_once_and_starts_no_extract() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_running = true;
        state
            .step2
            .update_selected_download_bytes
            .insert(0, (4, Some(4)));
        state.step2.update_selected_download_done.insert(0);
        let (_tx, mut rx) = queued(vec![Step2UpdateDownloadEvent::Finished(
            Step2UpdateDownloadResult::default(),
        )]);

        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Finished
        );
        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Idle
        );
        assert!(!state.step2.update_selected_extract_running);
        assert_eq!(
            state.step2.scan_status,
            "Download updates finished: 0 downloaded, 0 failed"
        );
        assert_eq!(
            state.step2.update_selected_download_bytes.get(&0),
            Some(&(4, Some(4)))
        );
        assert!(state.step2.update_selected_download_done.contains(&0));
    }

    #[test]
    fn failed_asset_enters_the_done_set() {
        let mut state = WizardState::default();
        state.step2.update_selected_update_assets = vec![asset("a.tp2", "A", String::new())];
        state.step2.update_selected_download_running = true;
        state.step2.update_selected_download_total = 1;
        let (_tx, mut rx) = queued(vec![done(0, false, None)]);

        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Idle
        );

        assert!(state.step2.update_selected_download_done.contains(&0));
        assert_eq!(
            state.step2.update_selected_download_bytes.get(&0),
            Some(&(10, Some(10)))
        );
        assert!(state.step2.update_selected_download_finished.is_empty());
        assert_eq!(
            state.step2.update_selected_download_failed_sources,
            vec!["A: unknown error".to_string()]
        );
        assert_eq!(state.step2.scan_status, "Downloading updates: 1/1");
    }

    #[test]
    fn archive_folder_failure_marks_every_pending_asset_failed() {
        let root = EngineTestRoot::new();
        fs::write(root.archive_dir(), b"not-a-folder").unwrap();
        let skipped_asset = asset("KMOD/KMOD.TP2", "KMOD", "http://127.0.0.1:9/K".to_string());
        let pending = asset("LMOD/LMOD.TP2", "LMOD", "http://127.0.0.1:9/L".to_string());
        let mut state = state_for(&root, vec![skipped_asset, pending]);
        let mut rx = None;

        start_step2_update_download_scoped(
            &mut state,
            &mut rx,
            None,
            &HashSet::from([0]),
            DownloadOrigin::InstallPipeline,
        )
        .expect("engine starts");
        let (events, result) = collect_events(rx.as_ref());

        let dones = done_events(&events);
        assert_eq!(dones.len(), 1);
        assert_eq!((dones[0].0, dones[0].1), (1, false));
        let error = dones[0].2.clone().expect("error carried");
        assert!(error.starts_with("Mods Archive: "));
        assert_eq!(result.failed, vec![format!("LMOD: {error}")]);

        let (_tx, mut queued_rx) = queued(events);
        assert_eq!(
            poll_step2_update_download(&mut state, &mut queued_rx),
            DownloadPoll::Idle
        );
        assert_eq!(state.step2.scan_status, "Downloading updates: 1/1");
        assert_eq!(
            state.step2.update_selected_download_done,
            std::collections::BTreeSet::from([1])
        );
        assert_eq!(
            state.step2.update_selected_download_failed_sources,
            vec![format!("LMOD: {error}")]
        );

        let (_tx, mut finished_rx) = queued(vec![Step2UpdateDownloadEvent::Finished(result)]);
        assert_eq!(
            poll_step2_update_download(&mut state, &mut finished_rx),
            DownloadPoll::Finished
        );
        assert_eq!(
            state.step2.scan_status,
            "Download updates finished: 0 downloaded, 1 failed"
        );
        assert!(state.step2.update_selected_downloaded_sources.is_empty());
    }

    #[test]
    fn disconnected_worker_keeps_the_byte_map_and_clears_the_scope() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_running = true;
        state.step2.update_selected_download_scope = Some("a".to_string());
        let (tx, rx) = mpsc::channel();
        tx.send(Step2UpdateDownloadEvent::AssetProgress {
            index: 0,
            bytes: 3,
            total: Some(9),
        })
        .unwrap();
        drop(tx);
        let mut rx = Some(rx);

        assert_eq!(
            poll_step2_update_download(&mut state, &mut rx),
            DownloadPoll::Idle
        );

        assert!(rx.is_none());
        assert!(!state.step2.update_selected_download_running);
        assert_eq!(state.step2.update_selected_download_scope, None);
        assert_eq!(
            state.step2.update_selected_download_bytes.get(&0),
            Some(&(3, Some(9)))
        );
        assert_eq!(
            state.step2.scan_status,
            "Download updates failed: worker disconnected"
        );
    }
}
