// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::mod_downloads;
use crate::app::state::{Step2UpdateAsset, WizardState};

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateDownloadResult {
    pub(crate) downloaded: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) enum Step2UpdateDownloadEvent {
    Progress {
        tp_file: String,
        ok: bool,
        completed: usize,
        total: usize,
    },
    Bytes {
        tp_file: String,
        done: u64,
        total: Option<u64>,
    },
    Finished(Step2UpdateDownloadResult),
}

const BYTES_EVENT_INTERVAL: Duration = Duration::from_millis(250);
const BYTES_EVENT_STEP: u64 = 1024 * 1024;

pub(crate) fn start_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
) {
    start_step2_update_download_scoped(state, step2_update_download_rx, None);
}

pub(crate) fn start_step2_update_download_scoped(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
    scope_tp2: Option<String>,
) {
    if state.step2.update_selected_download_running {
        return;
    }
    if !state.step1.download_archive {
        state.step2.update_selected_download_scope = None;
        state.step2.scan_status = "Download Archive is disabled in Step 1".to_string();
        return;
    }
    let archive_dir = state.step1.mods_archive_folder.trim().to_string();
    if archive_dir.is_empty() {
        state.step2.update_selected_download_scope = None;
        state.step2.scan_status = "Mods Archive folder is empty".to_string();
        return;
    }
    let assets = scoped_assets(state, scope_tp2.as_deref());
    if assets.is_empty() {
        state.step2.update_selected_download_scope = None;
        state.step2.scan_status = "No update archives to download".to_string();
        return;
    }

    let scoped_labels = scope_tp2.as_deref().map(|_| deduped_labels(&assets));
    clear_download_result_buckets(state, scoped_labels.as_deref());

    let archive_dir = PathBuf::from(archive_dir);
    let (tx, rx) = mpsc::channel::<Step2UpdateDownloadEvent>();
    *step2_update_download_rx = Some(rx);
    state.step2.update_selected_download_running = true;
    state.step2.update_selected_download_bytes = None;
    state.step2.update_selected_download_current = None;
    state.step2.update_selected_download_finished.clear();
    state.step2.update_selected_extract_running = false;
    state.step2.update_selected_download_scope = scope_tp2;
    state.step2.scan_status = format!("Downloading updates: 0/{}", assets.len());

    thread::spawn(move || {
        let result = download_update_assets(&archive_dir, &assets, &tx);
        let _ = tx.send(Step2UpdateDownloadEvent::Finished(result));
    });
}

fn scoped_assets(state: &WizardState, scope_tp2: Option<&str>) -> Vec<Step2UpdateAsset> {
    let Some(scope_tp2) = scope_tp2 else {
        return state.step2.update_selected_update_assets.clone();
    };
    state
        .step2
        .update_selected_update_assets
        .iter()
        .filter(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) == scope_tp2)
        .cloned()
        .collect()
}

fn deduped_labels(assets: &[Step2UpdateAsset]) -> Vec<String> {
    let mut labels = assets
        .iter()
        .map(|asset| asset.label.clone())
        .collect::<Vec<_>>();
    labels.dedup();
    labels
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
    step2_update_extract_rx: &mut Option<
        Receiver<super::app_step2_update_extract::Step2UpdateExtractEvent>,
    >,
) {
    let Some(rx) = step2_update_download_rx.as_ref() else {
        return;
    };
    let event = loop {
        match rx.try_recv() {
            Ok(Step2UpdateDownloadEvent::Bytes {
                tp_file,
                done,
                total,
            }) => {
                state.step2.update_selected_download_bytes = Some((done, total));
                state.step2.update_selected_download_current = Some(tp_file);
            }
            Ok(event) => break Some(event),
            Err(TryRecvError::Empty) => break None,
            Err(TryRecvError::Disconnected) => {
                state.step2.update_selected_download_running = false;
                state.step2.update_selected_download_bytes = None;
                state.step2.update_selected_download_current = None;
                state.step2.update_selected_download_finished.clear();
                state.step2.update_selected_download_scope = None;
                state.step2.scan_status =
                    "Download updates failed: worker disconnected".to_string();
                *step2_update_download_rx = None;
                return;
            }
        }
    };
    let Some(event) = event else {
        return;
    };
    let Step2UpdateDownloadEvent::Finished(result) = event else {
        if let Step2UpdateDownloadEvent::Progress {
            tp_file,
            ok,
            completed,
            total,
        } = event
        {
            if ok {
                record_finished_asset(state, &tp_file);
            }
            state.step2.scan_status = format!("Downloading updates: {completed}/{total}");
        }
        return;
    };

    *step2_update_download_rx = None;
    state.step2.update_selected_download_running = false;
    state.step2.update_selected_download_bytes = None;
    state.step2.update_selected_download_current = None;
    state.step2.update_selected_download_finished.clear();
    if state.step2.update_selected_download_scope.is_some() {
        state
            .step2
            .update_selected_downloaded_sources
            .extend(result.downloaded);
        state
            .step2
            .update_selected_download_failed_sources
            .extend(result.failed);
    } else {
        state.step2.update_selected_downloaded_sources = result.downloaded;
        state.step2.update_selected_download_failed_sources = result.failed;
    }
    let downloaded = state.step2.update_selected_downloaded_sources.len();
    let failed = state.step2.update_selected_download_failed_sources.len();
    state.step2.scan_status =
        format!("Download updates finished: {downloaded} downloaded, {failed} failed");
    super::app_step2_update_extract::start_step2_update_extract(state, step2_update_extract_rx);
}

fn record_finished_asset(state: &mut WizardState, tp_file: &str) {
    let key = mod_downloads::normalize_mod_download_tp2(tp_file);
    let finished = &mut state.step2.update_selected_download_finished;
    if !key.is_empty() && !finished.contains(&key) {
        finished.push(key);
    }
}

fn download_update_assets(
    archive_dir: &Path,
    assets: &[Step2UpdateAsset],
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Step2UpdateDownloadResult {
    let mut result = Step2UpdateDownloadResult {
        downloaded: Vec::new(),
        failed: Vec::new(),
    };
    if let Err(err) = fs::create_dir_all(archive_dir) {
        result.failed.push(format!("Mods Archive: {err}"));
        return result;
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_mins(2))
        .build();
    let mut cached_results = BTreeMap::<String, Result<(), String>>::new();
    let total = assets.len();
    for (index, asset) in assets.iter().enumerate() {
        let file_name = archive_file_name(asset);
        let destination = archive_dir.join(file_name);
        let cache_key = format!("{}|{}", destination.display(), asset.asset_url);
        let download_result = match cached_results.entry(cache_key) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let result = download_one_asset(&agent, asset, &destination, tx);
                entry.insert(result.clone());
                result
            }
        };
        let ok = download_result.is_ok();
        match download_result {
            Ok(()) => {
                result
                    .downloaded
                    .push(format!("{} -> {}", asset.label, destination.display()));
            }
            Err(err) => result.failed.push(format!("{}: {err}", asset.label)),
        }
        let _ = tx.send(Step2UpdateDownloadEvent::Progress {
            tp_file: asset.tp_file.clone(),
            ok,
            completed: index + 1,
            total,
        });
    }
    result
}

fn download_one_asset(
    agent: &ureq::Agent,
    asset: &Step2UpdateAsset,
    destination: &Path,
    tx: &Sender<Step2UpdateDownloadEvent>,
) -> Result<(), String> {
    let response = agent
        .get(&asset.asset_url)
        .set("User-Agent", "BIO-update-download")
        .call()
        .map_err(|err| err.to_string())?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.trim().parse::<u64>().ok());
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination).map_err(|err| err.to_string())?;
    let _ = tx.send(Step2UpdateDownloadEvent::Bytes {
        tp_file: asset.tp_file.clone(),
        done: 0,
        total,
    });
    let (mut done, mut sent_done, mut sent_at) = (0_u64, 0_u64, Instant::now());
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(read) => read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err.to_string()),
        };
        file.write_all(&buffer[..read])
            .map_err(|err| err.to_string())?;
        done = done.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        if done - sent_done >= BYTES_EVENT_STEP || sent_at.elapsed() >= BYTES_EVENT_INTERVAL {
            let _ = tx.send(Step2UpdateDownloadEvent::Bytes {
                tp_file: asset.tp_file.clone(),
                done,
                total,
            });
            (sent_done, sent_at) = (done, Instant::now());
        }
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
        state.step2.update_selected_download_running = true;
        let (tx, rx) = mpsc::channel::<Step2UpdateDownloadEvent>();
        let mut download_rx = Some(rx);
        let mut extract_rx = None;
        for (tp_file, ok) in [("a.tp2", false), ("b.tp2", true)] {
            tx.send(Step2UpdateDownloadEvent::Progress {
                tp_file: tp_file.to_string(),
                ok,
                completed: 1,
                total: 2,
            })
            .unwrap();
            poll_step2_update_download(&mut state, &mut download_rx, &mut extract_rx);
        }

        assert_eq!(
            state.step2.update_selected_download_finished,
            vec!["b".to_string()]
        );
    }

    #[test]
    fn scoped_download_completion_appends_results() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        state.step2.update_selected_downloaded_sources = vec!["Beta -> C:\\b".to_string()];

        let (tx, rx) = mpsc::channel::<Step2UpdateDownloadEvent>();
        tx.send(Step2UpdateDownloadEvent::Finished(
            Step2UpdateDownloadResult {
                downloaded: vec!["Alpha -> C:\\a".to_string()],
                failed: Vec::new(),
            },
        ))
        .unwrap();
        let mut download_rx = Some(rx);
        let mut extract_rx = None;

        poll_step2_update_download(&mut state, &mut download_rx, &mut extract_rx);

        assert_eq!(
            state.step2.update_selected_downloaded_sources,
            vec!["Beta -> C:\\b".to_string(), "Alpha -> C:\\a".to_string()]
        );
    }
}
