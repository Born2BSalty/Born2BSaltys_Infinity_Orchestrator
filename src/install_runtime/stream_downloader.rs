// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::hash::BuildHasher;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::{Step2UpdateAsset, WizardState};

pub const POOL_SIZE: usize = 4;

const READ_CHUNK: usize = 64 * 1024;
const READ_TIMEOUT_SECS: u64 = 120;

pub type AssetBytes = (u64, Option<u64>);
type AssetStreamError = (u64, Option<u64>, String);

pub enum StreamDownloadEvent {
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

    Finished(StreamDownloadResult),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamDownloadResult {
    pub downloaded: Vec<String>,

    pub failed: Vec<String>,
}

struct AssetResult {
    index: usize,

    outcome: Result<String, String>,
}

pub fn start_stream_download<S>(
    state: &mut WizardState,
    skipped: &std::collections::HashSet<usize, S>,
) -> Option<Receiver<StreamDownloadEvent>>
where
    S: BuildHasher,
{
    if state.step2.update_selected_download_running {
        return None;
    }

    let archive_dir = state.step1.mods_archive_folder.trim().to_string();
    let assets = state.step2.update_selected_update_assets.clone();

    state.step2.update_selected_download_running = true;
    state.step2.update_selected_extract_running = false;

    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.scan_status = format!("Downloading updates: 0/{}", assets.len());

    let (tx, rx) = mpsc::channel::<StreamDownloadEvent>();
    let archive_dir = PathBuf::from(archive_dir);
    let skipped_owned: Arc<std::collections::HashSet<usize>> =
        Arc::new(skipped.iter().copied().collect());
    thread::spawn(move || {
        run_download(&archive_dir, assets, &tx, &skipped_owned);
    });
    Some(rx)
}

fn run_download(
    archive_dir: &Path,
    assets: Vec<Step2UpdateAsset>,
    tx: &Sender<StreamDownloadEvent>,
    skipped: &Arc<std::collections::HashSet<usize>>,
) {
    let total = assets.len();
    let mut result = StreamDownloadResult::default();

    if let Err(err) = fs::create_dir_all(archive_dir) {
        result.failed.push(format!("Mods Archive: {err}"));
        let _ = tx.send(StreamDownloadEvent::Finished(result));
        return;
    }

    if total == 0 {
        let _ = tx.send(StreamDownloadEvent::Finished(result));
        return;
    }

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
        let skipped = Arc::clone(skipped);
        handles.push(thread::spawn(move || {
            loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                if index >= assets.len() {
                    break;
                }
                if skipped.contains(&index) {
                    continue;
                }
                let asset = &assets[index];
                let dest = archive_dir.join(archive_file_name(asset));
                let outcome = stream_one_asset(asset, &dest, index, &tx);

                let (ok, final_bytes, total_len, error_msg, rec) = match outcome {
                    Ok((bytes, total_len)) => (
                        true,
                        bytes,
                        total_len,
                        None,
                        AssetResult {
                            index,

                            outcome: Ok(dest.display().to_string()),
                        },
                    ),
                    Err((bytes, total_len, err)) => (
                        false,
                        bytes,
                        total_len,
                        Some(err.clone()),
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
                    error: error_msg,
                });
            }
        }));
    }

    for h in handles {
        let _ = h.join();
    }

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

fn stream_one_asset(
    asset: &Step2UpdateAsset,
    dest: &Path,
    index: usize,
    tx: &Sender<StreamDownloadEvent>,
) -> Result<AssetBytes, AssetStreamError> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(READ_TIMEOUT_SECS))
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

        let _ = tx.send(StreamDownloadEvent::AssetProgress {
            index,
            bytes: downloaded,
            total,
        });
    }

    if let Err(err) = std::io::Write::flush(&mut file) {
        return Err((downloaded, total, err.to_string()));
    }
    Ok((downloaded, total))
}

#[must_use]
pub fn deterministic_dest(asset: &Step2UpdateAsset, archive_dir: &Path) -> PathBuf {
    archive_dir.join(archive_file_name(asset))
}

pub fn apply_result_state(state: &mut WizardState, result: StreamDownloadResult) {
    let StreamDownloadResult { downloaded, failed } = result;
    drop((downloaded, failed));
    state.step2.update_selected_download_running = false;

    let downloaded = state.step2.update_selected_downloaded_sources.len();
    let failed = state.step2.update_selected_download_failed_sources.len();

    state.step2.scan_status =
        format!("Download updates finished: {downloaded} downloaded, {failed} failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
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
                let Ok(mut stream) = stream else { break };
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
                        } else {
                            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                        }
                        let _ = stream.write_all(&body);
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

        let (base, h) = spawn_fixture(payloads, true, 2);

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

        let rx = start_stream_download(&mut state, &std::collections::HashSet::new())
            .expect("pool spawns");
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

        let dest_a = archive_dir.join(archive_file_name(&a));
        let dest_b = archive_dir.join(archive_file_name(&b));
        assert!(
            dest_a.exists() && dest_b.exists(),
            "archives at the deterministic path"
        );
        assert_eq!(std::fs::read(&dest_a).unwrap(), b"AAA-bytes-payload");
        assert_eq!(std::fs::read(&dest_b).unwrap(), vec![7u8; 4096]);

        assert_eq!(r.failed, Vec::<String>::new());
        assert!(
            r.downloaded
                .contains(&format!("{} -> {}", a.label, dest_a.display()))
        );
        assert!(
            r.downloaded
                .contains(&format!("{} -> {}", b.label, dest_b.display()))
        );

        assert_eq!(state.step2.update_selected_update_assets.len(), 2);

        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn no_content_length_is_graceful_indeterminate_total() {
        let archive_dir = td();
        let payloads = vec![("N".to_string(), b"no-content-length-body".to_vec())];

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

        let rx = start_stream_download(&mut state, &std::collections::HashSet::new())
            .expect("pool spawns");
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

        let rx = start_stream_download(&mut state, &std::collections::HashSet::new())
            .expect("pool spawns");
        let mut finished: Option<StreamDownloadResult> = None;
        while let Ok(ev) = rx.recv() {
            if let StreamDownloadEvent::Finished(r) = ev {
                finished = Some(r);
                break;
            }
        }
        h.join().unwrap();
        let r = finished.unwrap();

        let dest_good = archive_dir.join(archive_file_name(&good));
        assert!(dest_good.exists());
        assert!(
            r.downloaded
                .iter()
                .any(|e| e == &format!("{} -> {}", good.label, dest_good.display()))
        );

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
        let rx = start_stream_download(&mut state, &std::collections::HashSet::new())
            .expect("pool spawns even with 0 assets");
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
        state.step2.update_selected_download_running = true;
        assert!(
            start_stream_download(&mut state, &std::collections::HashSet::new()).is_none(),
            "a second start while running is a no-op (BIO serial-loop parity)"
        );
    }

    #[test]
    fn deterministic_dest_is_byte_identical_to_bio_extract_path() {
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
    fn apply_result_state_flips_flag_and_sets_status_without_bulk_assigning_vectors() {
        let mut st = WizardState::default();

        st.step2.update_selected_downloaded_sources =
            vec!["MyMod -> C:\\arch\\MyMod.zip".to_string()];
        st.step2.update_selected_download_failed_sources = vec!["BadMod: HTTP 404".to_string()];
        st.step2.update_selected_download_running = true;

        let result = StreamDownloadResult::default();
        apply_result_state(&mut st, result);

        assert!(!st.step2.update_selected_download_running);

        assert_eq!(
            st.step2.update_selected_downloaded_sources,
            vec!["MyMod -> C:\\arch\\MyMod.zip".to_string()],
            "downloaded vector NOT bulk-overwritten (per-asset pushes survive)"
        );
        assert_eq!(
            st.step2.update_selected_download_failed_sources,
            vec!["BadMod: HTTP 404".to_string()],
            "failed vector NOT bulk-overwritten (per-asset pushes survive)"
        );

        assert_eq!(
            st.step2.scan_status, "Download updates finished: 1 downloaded, 1 failed",
            "apply_result_state sets BIO's verbatim download-finished status \
             (reading the already-populated vectors)"
        );
    }

    #[test]
    fn fix_1b_asset_done_carries_error_string_for_per_asset_failed_push() {
        let archive_dir = td();
        let payloads = vec![("D".to_string(), b"D-good".to_vec())];
        let (base, h) = spawn_fixture(payloads, true, 2);

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let good = asset(
            "DMOD/DMOD.TP2",
            "github",
            "v1",
            "D.zip",
            format!("{base}/cl/D"),
        );
        let bad = asset(
            "EMOD/EMOD.TP2",
            "github",
            "v9",
            "E.zip",
            format!("{base}/404"),
        );
        state.step2.update_selected_update_assets = vec![good, bad];

        let rx = start_stream_download(&mut state, &std::collections::HashSet::new())
            .expect("pool spawns");
        let mut dones: Vec<(usize, bool, Option<String>)> = Vec::new();
        while let Ok(ev) = rx.recv() {
            match ev {
                StreamDownloadEvent::AssetDone {
                    index, ok, error, ..
                } => {
                    dones.push((index, ok, error));
                }
                StreamDownloadEvent::Finished(_) => break,
                StreamDownloadEvent::AssetProgress { .. } => {}
            }
        }
        h.join().unwrap();

        assert_eq!(dones.len(), 2, "one AssetDone per asset");

        let good_done = dones.iter().find(|d| d.1).expect("one ok asset");
        assert_eq!(good_done.2, None, "ok ⇒ error None");

        let bad_done = dones.iter().find(|d| !d.1).expect("one failed asset");
        assert!(
            bad_done.2.is_some(),
            "failed ⇒ error carries the BIO-shaped error string"
        );
        assert!(
            !bad_done.2.as_deref().unwrap().is_empty(),
            "error string is non-empty (BIO-shaped \"{{err}}\")"
        );

        let _ = std::fs::remove_dir_all(&archive_dir);
    }
}
