// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::stream_downloader` — Phase 7 P7.T17 / #1 (the
// user-authorized net-new **parallel streaming downloader**, SPEC §4.3 /
// §13.12a).
//
// **Why this exists (the root cause it fixes).** BIO's archive download
// (`app_step2_update_download::download_update_assets`) is a **serial**
// `ureq` + `io::copy` loop that emits ONLY an aggregate
// `Step2UpdateDownloadEvent::Progress { completed, total }` (a
// `"Downloading updates: N/M"` count). It has **no per-asset byte
// signal** — that absence is the real reason every prior per-mod-progress
// attempt was coarse (the §4.3 grid could only show a determinate phase
// step, not a real fill). The user authorized our own implementation: a
// **bounded parallel pool** that reads each response's `Content-Length`
// and accumulates bytes in a manual read-loop, so the §4.3 grid carries a
// **real per-mod byte fraction**.
//
// **Zero BIO source (the run's hard constraint — SPEC §1 / the six
// carve-outs).** This module:
//
//   - reuses `ureq` (already a `Cargo.toml` dep — the same crate BIO's
//     serial loop uses; no new dependency);
//   - calls BIO's `pub(crate)` `app_step2_update_download::archive_file_
//     name` **read-only** to compute the write path (the SAME fn
//     `archive_store` already calls — established in-crate read-only
//     reuse, not a BIO edit);
//   - writes each archive to **exactly**
//     `PathBuf::from(state.step1.mods_archive_folder.trim())
//       .join(archive_file_name(asset))` — **byte-identical** to the path
//     BIO's reused-unchanged extract re-derives + gates on `.exists()`
//     (`app_step2_update_extract_plan.rs:48-49`). That `.exists()`
//     decoupling seam is *why* a net-new downloader composes without
//     forking extract;
//   - sets the EXACT `state.step2` shapes BIO's extract/ingest expect,
//     copied **verbatim** from the serial loop (see [`run_download`]'s
//     push formats);
//   - on completion calls BIO's `pub(crate)`
//     `app_step2_update_extract::start_step2_update_extract` — the SAME
//     entry BIO's own `poll_step2_update_download` calls — so extract /
//     rescan / install proceed **unchanged**.
//
// **The arming change (user-approved, orchestrator-side only — NOT a BIO
// edit, NOT a SPEC CONFLICT).** BIO's serial worker fires from
// `app_step2_saved_log_flow::advance_pending_saved_log_flow` ONLY when
// `state.step2.pending_saved_log_download == true`. The orchestrator-owned
// `auto_build_driver::arm_auto_build` no longer sets that flag, so BIO's
// serial download never starts (no double-download). This module owns the
// download sub-phase instead: it watches the SAME gate BIO's block uses
// (apply + update-preview done, nothing else running) and, the moment
// resolved assets are available, sets `modlist_auto_build_waiting_for_
// install = true` (the EXACT pub field BIO's own block sets at that point
// — `app_step2_saved_log_flow.rs:103`) and spawns the bounded parallel
// pool. On completion it pushes the result vectors + triggers BIO's
// extract; BIO's unchanged extract → rescan → `start_auto_build_install`
// path (gated on `modlist_auto_build_waiting_for_install`) then carries
// the pipeline to the install hand-off exactly as it does for the serial
// path. The `archive_store` content-addressed staging/dedupe is
// unaffected (extract + `ingest_downloaded_archives` re-derive paths /
// read fs — `stage_known_archives` still drops already-stored assets from
// `update_selected_update_assets` *before* this pool is kicked).
//
// **Concurrency model.** A spawned coordinator thread owns a bounded
// worker pool (`POOL_SIZE` ≈ 4) via a shared work index + a results
// `Mutex`; each worker streams one asset at a time, sending
// `StreamDownloadEvent::AssetProgress` byte deltas + a terminal
// `AssetDone`. The coordinator sends a final `Finished` carrying the
// BIO-shaped `downloaded` / `failed` vectors. The orchestrator owns the
// `Receiver` on `OrchestratorApp` (mirroring `step2_update_download_rx`)
// and drains it every frame in `poll_step2_channels`; on `Finished` it
// writes the `state.step2` vectors + calls `start_step2_update_extract`.
//
// **No-Content-Length → graceful.** The per-row progress is
// `Option<(bytes, Option<total>)>`: `total` is `None` when the server
// sent no `Content-Length` (byte-count progress with an indeterminate
// total — the §4.3 grid renders it as an active-but-unmeasured row).
//
// SPEC: §4.3 (Downloading screen — real per-mod byte fraction), §13.12a
//       (per-install dirs + content-addressed staging + the
//       pipeline-reuse contract), §1 (CRITICAL DIRECTIVE — net-new,
//       zero BIO source).

// rationale: a bounded download pool with a shared work index + results
// mutex is intrinsic to the parallel design (not a smell); `Self`/
// `const fn`/`#[must_use]` churn + the doc-paragraph-length lint add
// noise without behavior value (Cat 3).
#![allow(
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::use_self,
    clippy::too_long_first_doc_paragraph
)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::{Step2UpdateAsset, WizardState};

/// The bounded parallel pool size. ~4 keeps connection pressure modest
/// (mod hosts — GitHub / Weasel / Morpheus — and the user's link) while
/// being meaningfully faster than BIO's serial loop. `> 1` (real
/// parallelism) and `<= POOL_SIZE` in flight at any instant is the
/// contract the runtime trace asserts.
pub const POOL_SIZE: usize = 4;

/// The per-asset read-loop chunk. 64 KiB matches `archive_store::hash_file`
/// — a multi-hundred-MB mod archive streams to disk without buffering
/// wholly in RAM, and byte-delta events arrive often enough for a smooth
/// per-mod bar without flooding the channel.
const READ_CHUNK: usize = 64 * 1024;

/// One asset's live byte progress. `(downloaded_bytes, total)` where
/// `total` is `Some(content_length)` when the server advertised one and
/// `None` for a chunked / no-`Content-Length` response (byte-count with an
/// indeterminate total — SPEC §4.3 "graceful"). Mirrors the `Option<(bytes,
/// Option<total>)>` shape `DownloadProgress` carries per row.
pub type AssetBytes = (u64, Option<u64>);

/// Events the coordinator thread sends to the orchestrator's per-frame
/// drain. Mirrors `bio::app::app_step2_update_download::
/// Step2UpdateDownloadEvent`'s channel discipline, with the per-asset byte
/// granularity BIO's serial loop lacks.
pub enum StreamDownloadEvent {
    /// A byte delta for one asset (`index` into the original asset list).
    /// `bytes` is the running total downloaded so far for that asset;
    /// `total` is its `Content-Length` (`None` ⇒ indeterminate).
    AssetProgress {
        index: usize,
        bytes: u64,
        total: Option<u64>,
    },
    /// One asset reached a terminal state (success or failure). `ok`
    /// distinguishes; `final_bytes` is the byte count at termination.
    AssetDone {
        index: usize,
        ok: bool,
        final_bytes: u64,
        total: Option<u64>,
    },
    /// Every asset finished. Carries the BIO-shaped result vectors the
    /// orchestrator writes verbatim into `state.step2`
    /// (`update_selected_downloaded_sources` /
    /// `update_selected_download_failed_sources`).
    Finished(StreamDownloadResult),
}

/// The terminal result — the EXACT two vectors BIO's serial
/// `Step2UpdateDownloadResult` produces (same element formats, see
/// [`run_download`]). The orchestrator assigns these straight onto
/// `state.step2` then calls `start_step2_update_extract`, identical to
/// BIO's `poll_step2_update_download`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamDownloadResult {
    /// `"{label} -> {dest}"` per success — byte-identical to
    /// `app_step2_update_download.rs:140`.
    pub downloaded: Vec<String>,
    /// `"{label}: {err}"` per failure — byte-identical to
    /// `app_step2_update_download.rs:142`.
    pub failed: Vec<String>,
}

/// Internal per-asset terminal record the workers push into the shared
/// results vec (coordinator orders + formats them at the end).
struct AssetResult {
    index: usize,
    /// `Ok(dest_display)` → success (the destination path's `Display`,
    /// matching BIO's `destination.display()`); `Err(msg)` → the BIO-shaped
    /// error string.
    outcome: Result<String, String>,
}

/// **Spawn the bounded parallel streaming download (the net-new
/// replacement for BIO's serial loop).** Reads
/// `state.step1.mods_archive_folder` (the global Settings → Paths
/// Mods-archive folder — fed via the established settings-sync) +
/// `state.step2.update_selected_update_assets` (the resolved asset set —
/// read-only input, left intact), sets
/// `state.step2.update_selected_download_running = true` and clears the
/// downloaded/failed vectors (mirroring the serial loop's start state),
/// then spawns a coordinator thread that drives a `POOL_SIZE` worker pool.
///
/// The caller must:
///   1. set `state.step2.modlist_auto_build_waiting_for_install = true`
///      *before* calling this (the pub field BIO's own block sets at this
///      point — `app_step2_saved_log_flow.rs:103` — so BIO's unchanged
///      extract→rescan→install hand-off fires after extract); and
///   2. on the resulting `Receiver`'s `Finished`, write the vectors onto
///      `state.step2` + call `app_step2_update_extract::
///      start_step2_update_extract` (see [`apply_finished`]).
///
/// Idempotent re-entry guard: if `update_selected_download_running` is
/// already set, returns `None` (the pool is already in flight) — matching
/// BIO's serial `start_step2_update_download` early-return.
pub fn start_stream_download(state: &mut WizardState) -> Option<Receiver<StreamDownloadEvent>> {
    if state.step2.update_selected_download_running {
        return None;
    }

    let archive_dir = state.step1.mods_archive_folder.trim().to_string();
    let assets = state.step2.update_selected_update_assets.clone();

    // Mirror the serial loop's start mutations EXACTLY
    // (`app_step2_update_download.rs:50-56`): running flag on,
    // extract-running off, all four download/extract result vectors
    // cleared. `update_selected_update_assets` is left INTACT (read-only
    // input — extract re-reads it; the contract).
    state.step2.update_selected_download_running = true;
    state.step2.update_selected_extract_running = false;
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.scan_status = format!("Downloading updates: 0/{}", assets.len());

    let (tx, rx) = mpsc::channel::<StreamDownloadEvent>();
    let archive_dir = PathBuf::from(archive_dir);
    thread::spawn(move || {
        run_download(&archive_dir, assets, &tx);
    });
    Some(rx)
}

/// The coordinator + bounded worker pool. Runs on a spawned thread (never
/// the egui frame thread). Spawns `min(POOL_SIZE, assets.len())` workers
/// that pull from a shared atomic index; each streams one asset at a time
/// via [`stream_one_asset`], emitting byte deltas + a terminal
/// `AssetDone`. When the index is exhausted and all workers join, the
/// coordinator formats the BIO-shaped result vectors (ordered by original
/// asset index) and sends `Finished`.
///
/// **One deliberate failure does not abort the pool** — a failed asset
/// records an `Err` and the workers keep pulling the rest (matching BIO's
/// serial loop, which `match`es each result and continues). The
/// `auto_build_blocker_before_install` BIO check (unchanged) then stops
/// the auto-build for the failed source after extract, exactly as today.
fn run_download(
    archive_dir: &Path,
    assets: Vec<Step2UpdateAsset>,
    tx: &Sender<StreamDownloadEvent>,
) {
    let total = assets.len();
    let mut result = StreamDownloadResult::default();

    // Create the Mods-archive dir up front (same as the serial loop's
    // `fs::create_dir_all(archive_dir)` precondition). A failure here is
    // the serial loop's `"Mods Archive: {err}"` global failure.
    if let Err(err) = fs::create_dir_all(archive_dir) {
        result.failed.push(format!("Mods Archive: {err}"));
        let _ = tx.send(StreamDownloadEvent::Finished(result));
        return;
    }

    if total == 0 {
        let _ = tx.send(StreamDownloadEvent::Finished(result));
        return;
    }

    // Shared work index + ordered results. `assets` is shared read-only
    // (each worker reads `assets[i]`); `next` hands out indices; `results`
    // collects terminal records.
    let assets = Arc::new(assets);
    let archive_dir = Arc::new(archive_dir.to_path_buf());
    let next = Arc::new(AtomicUsize::new(0));
    let results: Arc<Mutex<Vec<AssetResult>>> = Arc::new(Mutex::new(Vec::with_capacity(total)));

    let worker_count = POOL_SIZE.min(total);
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let assets = Arc::clone(&assets);
        let archive_dir = Arc::clone(&archive_dir);
        let next = Arc::clone(&next);
        let results = Arc::clone(&results);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                if index >= assets.len() {
                    break;
                }
                let asset = &assets[index];
                let dest = archive_dir.join(archive_file_name(asset));
                let outcome = stream_one_asset(asset, &dest, index, &tx);
                let (ok, final_bytes, total_len, rec) = match outcome {
                    Ok((bytes, total_len)) => (
                        true,
                        bytes,
                        total_len,
                        // BIO's exact success format
                        // (`app_step2_update_download.rs:140`):
                        // `"{} -> {}"` with `asset.label` +
                        // `destination.display()`.
                        AssetResult {
                            index,
                            // `destination.display()` rendered to a String
                            // — the coordinator prefixes `"{label} -> "`
                            // (BIO's exact `app_step2_update_download.rs:
                            // 140` format) when ordering the vectors.
                            outcome: Ok(dest.display().to_string()),
                        },
                    ),
                    Err((bytes, total_len, err)) => (
                        false,
                        bytes,
                        total_len,
                        // BIO's exact failure format
                        // (`app_step2_update_download.rs:142`):
                        // `"{}: {err}"` with `asset.label`.
                        AssetResult {
                            index,
                            outcome: Err(err),
                        },
                    ),
                };
                results.lock().expect("download results mutex").push(rec);
                let _ = tx.send(StreamDownloadEvent::AssetDone {
                    index,
                    ok,
                    final_bytes,
                    total: total_len,
                });
            }
        }));
    }

    for h in handles {
        let _ = h.join();
    }

    // Order the terminal records by original asset index and format the
    // two BIO-shaped vectors. `label` is taken from the original asset
    // list (the worker only carried the index) so the strings are
    // byte-identical to BIO's serial loop.
    let mut recs = results.lock().expect("download results mutex");
    recs.sort_by_key(|r| r.index);
    for rec in recs.iter() {
        let label = &assets[rec.index].label;
        match &rec.outcome {
            Ok(dest_display) => result.downloaded.push(format!("{label} -> {dest_display}")),
            Err(err) => result.failed.push(format!("{label}: {err}")),
        }
    }
    drop(recs);

    let _ = tx.send(StreamDownloadEvent::Finished(result));
}

/// Stream a single asset to `dest` with `ureq` + a manual byte-accumulating
/// read-loop (the per-asset byte signal BIO's `io::copy` serial loop does
/// not expose). Reads `Content-Length` into `total` (`None` ⇒ chunked /
/// indeterminate — graceful per SPEC §4.3). Emits an `AssetProgress` byte
/// delta as bytes accumulate.
///
/// Errors map via `err.to_string()` exactly as BIO's serial
/// `download_one_asset` does (so the failed-source string is byte-identical
/// to BIO's). A non-2xx status is mapped to a readable error (ureq 2.x
/// `.call()` already `Err`s on 4xx/5xx — the explicit status guard is
/// defense in depth + a clear message).
///
/// Returns `Ok((bytes, total))` on success / `Err((bytes, total, msg))` on
/// failure (the byte count at the point of failure feeds the terminal
/// `AssetDone`).
fn stream_one_asset(
    asset: &Step2UpdateAsset,
    dest: &Path,
    index: usize,
    tx: &Sender<StreamDownloadEvent>,
) -> Result<(u64, Option<u64>), (u64, Option<u64>, String)> {
    // Same agent construction as BIO's serial loop
    // (`app_step2_update_download.rs:119-122`): a 20s connect / 120s read
    // timeout. (Built per-asset so a slow asset's read timeout cannot
    // wedge a shared agent across the pool.)
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120))
        .build();

    let response = match agent
        .get(&asset.asset_url)
        .set("User-Agent", "BIO-update-download")
        .call()
    {
        Ok(r) => r,
        Err(err) => return Err((0, None, err.to_string())),
    };

    let status = response.status();
    if !(200..300).contains(&status) {
        return Err((0, None, format!("HTTP {status}")));
    }

    // `Content-Length` → the determinate total (SPEC §4.3 real per-mod
    // fraction). Absent / unparseable ⇒ `None` (byte-count with an
    // indeterminate total — the graceful path).
    let total: Option<u64> = response
        .header("Content-Length")
        .and_then(|v| v.trim().parse::<u64>().ok());

    let mut reader = response.into_reader();
    let mut file = match fs::File::create(dest) {
        Ok(f) => f,
        Err(err) => return Err((0, total, err.to_string())),
    };

    let mut buf = vec![0u8; READ_CHUNK];
    let mut downloaded: u64 = 0;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(err) => return Err((downloaded, total, err.to_string())),
        };
        if let Err(err) = std::io::Write::write_all(&mut file, &buf[..n]) {
            return Err((downloaded, total, err.to_string()));
        }
        downloaded += n as u64;
        // A byte delta per chunk — the real per-mod fraction the §4.3 grid
        // renders (monotonic by construction; reaches `total` on success).
        let _ = tx.send(StreamDownloadEvent::AssetProgress {
            index,
            bytes: downloaded,
            total,
        });
    }
    // Ensure the bytes are on disk before extract re-derives the path +
    // gates on `.exists()` (BIO's serial loop relies on the same
    // file-closed-after-`io::copy` ordering; an explicit flush removes any
    // doubt under the parallel pool).
    if let Err(err) = std::io::Write::flush(&mut file) {
        return Err((downloaded, total, err.to_string()));
    }
    Ok((downloaded, total))
}

/// The deterministic on-disk destination for `asset` in `archive_dir` —
/// **exactly** `archive_dir.join(archive_file_name(asset))**, the
/// byte-identical path BIO's reused-unchanged extract re-derives + gates
/// on `.exists()` (`app_step2_update_extract_plan.rs:48-49`). Public
/// because it is this module's documented write-path *contract* (the
/// runtime trace + any future caller verifies against it); it composes
/// BIO's `archive_file_name` read-only — zero BIO edit.
#[must_use]
pub fn deterministic_dest(asset: &Step2UpdateAsset, archive_dir: &Path) -> PathBuf {
    archive_dir.join(archive_file_name(asset))
}

/// **Apply the BIO-shaped result vectors + running flag + status — the
/// state-mutation half of BIO's `poll_step2_update_download` tail**
/// (`app_step2_update_download.rs:94-101`): assign the two vectors onto
/// `state.step2`, clear the running flag, set the BIO-verbatim finished
/// status line. Pure `state` mutation (no BIO `pub(crate)` type in the
/// signature) so it is `pub` + observable by the runtime trace. The
/// caller (`apply_finished`, in-crate) then makes the single
/// `start_step2_update_extract` call. Split out so the trace can prove
/// contract (4) end-to-end via the public surface while the
/// extract-trigger (5) stays an in-crate `pub(crate)` BIO call.
pub fn apply_result_state(state: &mut WizardState, result: StreamDownloadResult) {
    state.step2.update_selected_download_running = false;
    state.step2.update_selected_downloaded_sources = result.downloaded;
    state.step2.update_selected_download_failed_sources = result.failed;
    let downloaded = state.step2.update_selected_downloaded_sources.len();
    let failed = state.step2.update_selected_download_failed_sources.len();
    // BIO-verbatim finished status (`app_step2_update_download.rs:100-101`).
    state.step2.scan_status =
        format!("Download updates finished: {downloaded} downloaded, {failed} failed");
}

/// **Apply the coordinator's `Finished` result the EXACT way BIO's own
/// `poll_step2_update_download` does** (`app_step2_update_download.rs:
/// 94-103`): the [`apply_result_state`] vectors/flag/status, then call
/// `app_step2_update_extract::start_step2_update_extract` (BIO's
/// `pub(crate)` extract entry — the SAME one BIO's poll invokes; reused
/// **unchanged**, no fork). After this, BIO's unchanged extract → rescan →
/// `start_auto_build_install` path (gated on the
/// `modlist_auto_build_waiting_for_install` the caller set before the
/// download) carries the pipeline to the install hand-off — identical to
/// the serial path's continuation. `pub(crate)` because its signature
/// names BIO's `pub(crate)` `Step2UpdateExtractEvent` (same visibility
/// rule as the `OrchestratorApp` receiver fields); the in-crate
/// `drain_stream_download` is the only caller.
pub(crate) fn apply_finished(
    state: &mut WizardState,
    result: StreamDownloadResult,
    step2_update_extract_rx: &mut Option<
        Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>,
    >,
) {
    apply_result_state(state, result);
    // BIO's `pub(crate)` extract entry — the SAME call BIO's
    // `poll_step2_update_download` makes (`app_step2_update_download.rs:
    // 102`). Reused unchanged: only the *download* mechanism is replaced;
    // extract stays BIO's.
    crate::app::app_step2_update_extract::start_step2_update_extract(
        state,
        step2_update_extract_rx,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
        // DATA-LOSS-safe: a unique temp dir; this module never binds the
        // real `%APPDATA%\bio\` (it operates only on an arbitrary
        // Mods-archive dir + temp HTTP fixtures).
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_stream_dl_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn asset(tp: &str, src: &str, tag: &str, name: &str, url: String) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp.to_string(),
            label: tp.to_string(),
            source_id: src.to_string(),
            tag: tag.to_string(),
            asset_name: name.to_string(),
            asset_url: url,
            installed_source_ref: None,
        }
    }

    /// A throwaway in-process HTTP server (`127.0.0.1:0`) for the
    /// temp-path tests. Routes by path: `/cl/<name>` serves a fixed body
    /// WITH `Content-Length`; `/nocl/<name>` serves a body with NO
    /// `Content-Length` (closes the connection to delimit it); `/404`
    /// returns 404. NEVER a real network host (DATA-LOSS / directive).
    ///
    /// `expected_requests` is the EXACT number of HTTP requests the test
    /// will make (== asset count, including any 404 asset) — NOT
    /// `bodies.len()`. The 404 asset has no `bodies` entry but still
    /// consumes a request, so counting by `bodies.len()` would stop the
    /// server early and leave a worker's connection refused (a flaky
    /// race). Each connection is served on **its own thread** so the
    /// bounded pool's workers genuinely overlap (a single-threaded
    /// `accept`-then-serve loop would serialize them).
    fn spawn_fixture(
        bodies: Vec<(String, Vec<u8>)>,
        with_cl: bool,
        expected_requests: usize,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let base = format!("http://{addr}");
        let bodies = std::sync::Arc::new(bodies);
        let handle = thread::spawn(move || {
            let mut conns = Vec::new();
            let mut accepted = 0usize;
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let bodies = std::sync::Arc::clone(&bodies);
                conns.push(thread::spawn(move || {
                    let mut buf = [0u8; 2048];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_string();
                    if path == "/404" {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    } else {
                        let name = path.rsplit('/').next().unwrap_or("");
                        let body = bodies
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, b)| b.clone())
                            .unwrap_or_default();
                        if with_cl {
                            let header = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(header.as_bytes());
                            let _ = stream.write_all(&body);
                        } else {
                            // No Content-Length: body then close the
                            // connection to delimit it (ureq reads to EOF).
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                            let _ = stream.write_all(&body);
                        }
                    }
                    let _ = stream.flush();
                }));
                accepted += 1;
                if accepted >= expected_requests {
                    break;
                }
            }
            for c in conns {
                let _ = c.join();
            }
        });
        (base, handle)
    }

    #[test]
    fn writes_exact_path_and_bio_shaped_vectors_with_content_length() {
        let archive_dir = td();
        let payloads = vec![
            ("A".to_string(), b"AAA-bytes-payload".to_vec()),
            ("B".to_string(), vec![7u8; 4096]),
        ];
        // 2 assets ⇒ 2 requests.
        let (base, h) = spawn_fixture(payloads.clone(), true, 2);

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset(
            "AMOD/AMOD.TP2",
            "github",
            "v1",
            "A.zip",
            format!("{base}/cl/A"),
        );
        let b = asset(
            "BMOD/BMOD.TP2",
            "weasel",
            "v2",
            "B.zip",
            format!("{base}/cl/B"),
        );
        state.step2.update_selected_update_assets = vec![a.clone(), b.clone()];

        let rx = start_stream_download(&mut state).expect("pool spawns");
        assert!(
            state.step2.update_selected_download_running,
            "running flag set true at start (BIO serial-loop parity)"
        );

        let mut finished: Option<StreamDownloadResult> = None;
        while let Ok(ev) = rx.recv() {
            if let StreamDownloadEvent::Finished(r) = ev {
                finished = Some(r);
                break;
            }
        }
        let r = finished.expect("Finished arrives");
        h.join().unwrap();

        // (a) EXACT write path = mods_archive_folder / archive_file_name.
        let dest_a = archive_dir.join(archive_file_name(&a));
        let dest_b = archive_dir.join(archive_file_name(&b));
        assert!(
            dest_a.exists() && dest_b.exists(),
            "archives at the deterministic path"
        );
        assert_eq!(std::fs::read(&dest_a).unwrap(), b"AAA-bytes-payload");
        assert_eq!(std::fs::read(&dest_b).unwrap(), vec![7u8; 4096]);

        // (b) BIO-verbatim success vector: "{label} -> {dest.display()}".
        assert_eq!(r.failed, Vec::<String>::new());
        assert!(
            r.downloaded
                .contains(&format!("{} -> {}", a.label, dest_a.display()))
        );
        assert!(
            r.downloaded
                .contains(&format!("{} -> {}", b.label, dest_b.display()))
        );
        // update_selected_update_assets left INTACT (read-only input).
        assert_eq!(state.step2.update_selected_update_assets.len(), 2);

        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn no_content_length_is_graceful_indeterminate_total() {
        let archive_dir = td();
        let payloads = vec![("N".to_string(), b"no-content-length-body".to_vec())];
        // 1 asset ⇒ 1 request.
        let (base, h) = spawn_fixture(payloads, false, 1);

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset(
            "NMOD/NMOD.TP2",
            "github",
            "v1",
            "N.zip",
            format!("{base}/nocl/N"),
        );
        state.step2.update_selected_update_assets = vec![a.clone()];

        let rx = start_stream_download(&mut state).expect("pool spawns");
        let mut saw_indeterminate = false;
        let mut finished: Option<StreamDownloadResult> = None;
        while let Ok(ev) = rx.recv() {
            match ev {
                StreamDownloadEvent::AssetProgress { total, bytes, .. } => {
                    if total.is_none() {
                        saw_indeterminate = true;
                    }
                    assert!(bytes > 0);
                }
                StreamDownloadEvent::Finished(r) => {
                    finished = Some(r);
                    break;
                }
                StreamDownloadEvent::AssetDone { total, .. } => {
                    assert!(total.is_none(), "no Content-Length ⇒ indeterminate total");
                }
            }
        }
        h.join().unwrap();
        assert!(
            saw_indeterminate,
            "byte progress with an indeterminate total"
        );
        let r = finished.unwrap();
        assert_eq!(r.failed, Vec::<String>::new());
        let dest = archive_dir.join(archive_file_name(&a));
        assert_eq!(std::fs::read(&dest).unwrap(), b"no-content-length-body");
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn failure_records_bio_shaped_failed_and_does_not_abort_pool() {
        let archive_dir = td();
        // C succeeds; the 404 asset fails — the pool must keep C.
        // 2 assets (good + 404) ⇒ 2 requests (the 404 has no `payloads`
        // entry but still consumes a request — counting by payloads.len()
        // was the flaky-race bug).
        let payloads = vec![("C".to_string(), b"C-good-bytes".to_vec())];
        let (base, h) = spawn_fixture(payloads, true, 2);

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let good = asset(
            "CMOD/CMOD.TP2",
            "github",
            "v1",
            "C.zip",
            format!("{base}/cl/C"),
        );
        let bad = asset(
            "XMOD/XMOD.TP2",
            "github",
            "v9",
            "X.zip",
            format!("{base}/404"),
        );
        state.step2.update_selected_update_assets = vec![good.clone(), bad.clone()];

        let rx = start_stream_download(&mut state).expect("pool spawns");
        let mut finished: Option<StreamDownloadResult> = None;
        while let Ok(ev) = rx.recv() {
            if let StreamDownloadEvent::Finished(r) = ev {
                finished = Some(r);
                break;
            }
        }
        h.join().unwrap();
        let r = finished.unwrap();

        // Good asset survived (pool not aborted by the failure).
        let dest_good = archive_dir.join(archive_file_name(&good));
        assert!(dest_good.exists());
        assert!(
            r.downloaded
                .iter()
                .any(|e| e == &format!("{} -> {}", good.label, dest_good.display()))
        );
        // Failure recorded in the BIO-verbatim "{label}: {err}" shape.
        assert_eq!(r.failed.len(), 1);
        assert!(
            r.failed[0].starts_with(&format!("{}: ", bad.label)),
            "failed entry is BIO-shaped \"{{label}}: {{err}}\": {:?}",
            r.failed
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn empty_asset_set_finishes_cleanly() {
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let rx = start_stream_download(&mut state).expect("pool spawns even with 0 assets");
        let mut got_finished = false;
        while let Ok(ev) = rx.recv() {
            if let StreamDownloadEvent::Finished(r) = ev {
                assert!(r.downloaded.is_empty() && r.failed.is_empty());
                got_finished = true;
                break;
            }
        }
        assert!(got_finished);
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn re_entry_guard_returns_none_when_already_running() {
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = td().to_string_lossy().into_owned();
        state.step2.update_selected_download_running = true; // already in flight
        assert!(
            start_stream_download(&mut state).is_none(),
            "a second start while running is a no-op (BIO serial-loop parity)"
        );
    }

    #[test]
    fn deterministic_dest_is_byte_identical_to_bio_extract_path() {
        // The write-path contract: EXACTLY the path BIO's reused-unchanged
        // extract re-derives + gates on `.exists()`
        // (`app_step2_update_extract_plan.rs:48-49` =
        // `archive_dir.join(archive_file_name(asset))`).
        let archive_dir = td();
        let a = asset(
            "MOD/MOD.TP2",
            "github",
            "v1",
            "MOD-v1.zip",
            "http://x".into(),
        );
        assert_eq!(
            deterministic_dest(&a, &archive_dir),
            archive_dir.join(archive_file_name(&a)),
            "deterministic_dest MUST equal BIO's extract-derived archive path"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn apply_finished_sets_bio_shaped_state_and_triggers_bio_extract() {
        // **(5) — the extract trigger.** `apply_finished` is the EXACT
        // BIO `poll_step2_update_download` tail: BIO-shaped vectors +
        // running false + verbatim status, THEN BIO's `pub(crate)`
        // `start_step2_update_extract`. With an empty per-install Mods
        // folder BIO's extract honestly early-returns (no spawn) — its
        // observable post-state proves the call site ran exactly once
        // through `apply_finished`. (In-crate because `apply_finished` /
        // `start_step2_update_extract` are `pub(crate)` — the runtime
        // trace proves the out-of-crate-observable contract; this proves
        // the `pub(crate)` linkage. Same split as the orchestrator's
        // acceptance-driver hygiene precedent.)
        let mut st = WizardState::default();
        // mods_archive_folder set so start_step2_update_extract passes its
        // own empty-archive-dir guard and reaches build_extract_jobs (which
        // then early-returns on the empty Mods folder — the honest path).
        st.step1.mods_archive_folder = td().to_string_lossy().into_owned();
        let result = StreamDownloadResult {
            downloaded: vec!["MyMod -> C:\\arch\\MyMod.zip".to_string()],
            failed: vec!["BadMod: HTTP 404".to_string()],
        };
        st.step2.update_selected_download_running = true;

        // First prove apply_result_state's status in isolation (the
        // BIO-verbatim download-finished line — the state-mutation half).
        let mut st_state_only = st.clone();
        apply_result_state(&mut st_state_only, result.clone());
        assert_eq!(
            st_state_only.step2.scan_status, "Download updates finished: 1 downloaded, 1 failed",
            "apply_result_state sets BIO's verbatim download-finished status"
        );

        // Now the full apply_finished = apply_result_state + BIO's
        // start_step2_update_extract.
        let mut extract_rx: Option<
            Receiver<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>,
        > = None;
        apply_finished(&mut st, result.clone(), &mut extract_rx);

        // BIO-shaped vectors assigned verbatim + running cleared (the
        // apply_result_state half — unaffected by the extract tail).
        assert!(!st.step2.update_selected_download_running);
        assert_eq!(
            st.step2.update_selected_downloaded_sources,
            result.downloaded
        );
        assert_eq!(
            st.step2.update_selected_download_failed_sources,
            result.failed
        );
        // **The extract-trigger proof:** BIO's start_step2_update_extract
        // RAN — its `build_extract_jobs` saw the empty per-install Mods
        // folder, pushed "Mods Folder is empty" to
        // `update_selected_extract_failed_sources`, and (failed > 0)
        // OVERWROTE scan_status with its own verbatim
        // "Extract updates finished: 0 updated, N failed". That overwrite
        // (vs the download-finished line proven above) is the observable
        // evidence the `pub(crate)` extract entry was invoked exactly once
        // via apply_finished — not left as apply_result_state's status.
        assert_eq!(
            st.step2.scan_status, "Extract updates finished: 0 updated, 1 failed",
            "BIO's start_step2_update_extract ran via apply_finished and \
             set its own status (the extract-trigger proof)"
        );
        assert!(
            st.step2
                .update_selected_extract_failed_sources
                .iter()
                .any(|e| e == "Mods Folder is empty"),
            "BIO's build_extract_jobs ran (its empty-Mods-folder honest path)"
        );
        // The extract is NOT left running (it honestly early-returned with
        // no jobs — no worker spawned for an empty Mods folder).
        assert!(
            !st.step2.update_selected_extract_running,
            "start_step2_update_extract early-returned (no jobs) — not left running"
        );
    }
}
