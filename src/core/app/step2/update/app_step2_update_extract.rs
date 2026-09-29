// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::{DownloadOrigin, WizardState};
use crate::app::step2_worker::Step2ScanEvent;

#[path = "app_step2_update_extract_archive.rs"]
pub mod archive;
#[path = "app_step2_update_extract_plan.rs"]
pub mod plan;

pub const EXTRACT_POOL_SIZE: usize = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Step2UpdateExtractResult {
    pub(crate) extracted: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) enum Step2UpdateExtractEvent {
    AssetStarted {
        tp_file: String,
    },
    AssetDone {
        index: usize,
        ok: bool,
        label: String,
        tp_file: String,
        target_or_err: String,
    },
    Finished(Step2UpdateExtractResult),
}

struct ExtractOutcome {
    index: usize,
    label: String,
    result: Result<String, String>,
}

pub(crate) fn start_step2_update_extract(
    state: &mut WizardState,
    step2_update_extract_rx: &mut Option<Receiver<Step2UpdateExtractEvent>>,
    install_ctx_installed_refs_path: Option<&Path>,
) -> bool {
    if state.step2.update_selected_extract_running {
        return false;
    }
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        state.step2.update_selected_download_scope = None;
        return false;
    }

    let scope = match state.step2.update_selected_download_origin {
        DownloadOrigin::InstallPipeline => None,
        DownloadOrigin::Workspace => state.step2.update_selected_download_scope.clone(),
    };
    let jobs = plan::build_extract_jobs(
        state,
        &archive_dir,
        install_ctx_installed_refs_path,
        scope.as_deref(),
    );
    if jobs.is_empty() {
        state.step2.update_selected_download_scope = None;
        let failed = state.step2.update_selected_extract_failed_sources.len();
        if failed > 0 {
            state.step2.scan_status =
                format!("Extract updates finished: 0 updated, {failed} failed");
        }
        tracing::info!(
            target = "orchestrator",
            failed,
            archive_dir = %archive_dir.display(),
            "update extract not started: no extract jobs"
        );
        return false;
    }

    let total = jobs.len();
    state.step2.update_selected_extract_jobs.clear();
    for job in &jobs {
        state
            .step2
            .update_selected_extract_jobs
            .entry(normalize_mod_download_tp2(&job.tp_file))
            .or_default()
            .total += 1;
    }
    let (tx, rx) = mpsc::channel::<Step2UpdateExtractEvent>();
    *step2_update_extract_rx = Some(rx);
    state.step2.update_selected_extract_progress = Some((0, total));
    state.step2.update_selected_extract_running = true;
    state.step2.scan_status = format!("Extracting updates: 0/{total}");
    tracing::info!(
        target = "orchestrator",
        total,
        archive_dir = %archive_dir.display(),
        "update extract starting"
    );

    thread::spawn(move || run_parallel_extract(&jobs, &tx));
    true
}

fn run_parallel_extract(
    jobs: &[plan::Step2UpdateExtractJob],
    tx: &Sender<Step2UpdateExtractEvent>,
) {
    let next = AtomicUsize::new(0);
    let outcomes = Mutex::new(Vec::<ExtractOutcome>::with_capacity(jobs.len()));
    thread::scope(|scope| {
        for _ in 0..EXTRACT_POOL_SIZE.min(jobs.len()) {
            scope.spawn(|| {
                while let Some(index) = claim_job(&next, jobs.len()) {
                    let outcome = extract_job(index, &jobs[index], tx);
                    outcomes
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(outcome);
                }
            });
        }
    });
    let outcomes = outcomes
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    let _ = tx.send(Step2UpdateExtractEvent::Finished(extract_result(outcomes)));
}

fn claim_job(next: &AtomicUsize, total: usize) -> Option<usize> {
    let index = next.fetch_add(1, Ordering::SeqCst);
    (index < total).then_some(index)
}

fn extract_job(
    index: usize,
    job: &plan::Step2UpdateExtractJob,
    tx: &Sender<Step2UpdateExtractEvent>,
) -> ExtractOutcome {
    let _ = tx.send(Step2UpdateExtractEvent::AssetStarted {
        tp_file: job.tp_file.clone(),
    });
    let result = archive::extract_one_archive(job).map(|target| target.display().to_string());
    let (ok, target_or_err) = match &result {
        Ok(target) => (true, target.clone()),
        Err(err) => (false, err.clone()),
    };
    let _ = tx.send(Step2UpdateExtractEvent::AssetDone {
        index,
        ok,
        label: job.label.clone(),
        tp_file: job.tp_file.clone(),
        target_or_err,
    });
    ExtractOutcome {
        index,
        label: job.label.clone(),
        result,
    }
}

fn extract_result(mut outcomes: Vec<ExtractOutcome>) -> Step2UpdateExtractResult {
    outcomes.sort_by_key(|outcome| outcome.index);
    let mut result = Step2UpdateExtractResult::default();
    for outcome in outcomes {
        match outcome.result {
            Ok(target) => result
                .extracted
                .push(format!("{} -> {target}", outcome.label)),
            Err(err) => result.failed.push(format!("{}: {err}", outcome.label)),
        }
    }
    result
}

pub(crate) fn poll_step2_update_extract(
    state: &mut WizardState,
    step2_update_extract_rx: &mut Option<Receiver<Step2UpdateExtractEvent>>,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    let Some(rx) = step2_update_extract_rx.as_ref() else {
        return;
    };
    let finished = loop {
        match rx.try_recv() {
            Ok(Step2UpdateExtractEvent::AssetStarted { tp_file }) => {
                state
                    .step2
                    .update_selected_extract_jobs
                    .entry(normalize_mod_download_tp2(&tp_file))
                    .or_default()
                    .started += 1;
            }
            Ok(Step2UpdateExtractEvent::AssetDone {
                index,
                ok,
                label,
                tp_file,
                target_or_err,
            }) => record_asset_done(state, index, ok, &label, &tp_file, &target_or_err),
            Ok(Step2UpdateExtractEvent::Finished(result)) => break Some(result),
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => break None,
        }
    };

    *step2_update_extract_rx = None;
    state.step2.update_selected_extract_running = false;
    let Some(result) = finished else {
        state.step2.update_selected_download_scope = None;
        state.step2.scan_status = "Extract updates failed: worker disconnected".to_string();
        return;
    };
    tracing::info!(
        target = "orchestrator",
        extracted = result.extracted.len(),
        failed = result.failed.len(),
        "update extract finished"
    );
    if state.step2.update_selected_download_scope.is_some() {
        state
            .step2
            .update_selected_extracted_sources
            .extend(result.extracted);
    } else {
        state.step2.update_selected_extracted_sources = result.extracted;
    }
    remove_extracted_update_entries(state);
    state
        .step2
        .update_selected_extract_failed_sources
        .extend(result.failed);
    state.step2.update_selected_download_scope = None;

    let extracted = state.step2.update_selected_extracted_sources.len();
    let failed = state.step2.update_selected_extract_failed_sources.len();
    if extracted > 0 {
        state.step1_mods_folder_has_tp2 = Some(true);
        state.step2.log_pending_downloads.clear();
        state.step2.scan_status = format!("Extracted {extracted} updates; rescanning Mods Folder");
        if state.step2.update_selected_download_origin == DownloadOrigin::InstallPipeline {
            state.step2.pending_saved_log_apply = true;
        }
        super::app_step2_scan::start_step2_scan(
            state,
            step2_scan_rx,
            step2_cancel,
            step2_progress_queue,
        );
    } else {
        state.step2.scan_status =
            format!("Extract updates finished: {extracted} updated, {failed} failed");
    }
}

fn record_asset_done(
    state: &mut WizardState,
    index: usize,
    ok: bool,
    label: &str,
    tp_file: &str,
    target_or_err: &str,
) {
    let (completed, total) = state
        .step2
        .update_selected_extract_progress
        .unwrap_or((0, 0));
    let completed = completed + 1;
    state.step2.update_selected_extract_progress = Some((completed, total));
    let key = normalize_mod_download_tp2(tp_file);
    if ok {
        state
            .step2
            .update_selected_extract_jobs
            .entry(key)
            .or_default()
            .done += 1;
    } else {
        state.step2.update_selected_extract_jobs.remove(&key);
    }
    state.step2.scan_status = format!("Extracting updates: {completed}/{total}");
    tracing::info!(
        target = "orchestrator",
        index,
        ok,
        label,
        tp_file,
        target_or_err,
        completed,
        total,
        "update extract asset done"
    );
}

fn remove_extracted_update_entries(state: &mut WizardState) {
    let extracted_labels = state
        .step2
        .update_selected_extracted_sources
        .iter()
        .filter_map(|entry| entry.split_once(" -> ").map(|(label, _)| label.trim()))
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if extracted_labels.is_empty() {
        return;
    }
    state.step2.update_selected_missing_sources.retain(|entry| {
        !extracted_labels
            .iter()
            .any(|label| entry.starts_with(&format!("{label} (")))
    });
    state.step2.update_selected_update_sources.retain(|entry| {
        !extracted_labels
            .iter()
            .any(|label| entry.starts_with(&format!("{label} (")))
    });
    state
        .step2
        .update_selected_update_assets
        .retain(|asset| !extracted_labels.contains(&asset.label));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use crate::app::app_step2_update_download::archive_file_name;
    use crate::app::mod_downloads::AMBIENT_TEST_LOCK;
    use crate::app::state::Step2UpdateAsset;

    struct ExtractEngineRoot(PathBuf);

    impl ExtractEngineRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_extract_engine_{}_{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let root = Self(path);
            fs::create_dir_all(&root.0).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(root.0.clone()));
            root
        }

        fn subdir(&self, name: &str) -> PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn state(&self) -> WizardState {
            let mut state = WizardState::default();
            state.step1.mods_archive_folder =
                self.subdir("archives").to_string_lossy().into_owned();
            state.step1.mods_folder = self.subdir("mods").to_string_lossy().into_owned();
            state.step1.mods_backup_folder = self.subdir("backup").to_string_lossy().into_owned();
            state
        }
    }

    impl Drop for ExtractEngineRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn ambient_lock() -> std::sync::MutexGuard<'static, ()> {
        AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn asset(index: usize) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("MOD{index}/MOD{index}.TP2"),
            label: format!("MOD{index}"),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("MOD{index}-v1.zip"),
            asset_url: format!("https://example/MOD{index}-v1.zip"),
            installed_source_ref: None,
        }
    }

    fn seed_assets(state: &mut WizardState, count: usize, on_disk: usize) {
        let archive_dir = PathBuf::from(&state.step1.mods_archive_folder);
        let assets = (0..count).map(asset).collect::<Vec<_>>();
        for asset in assets.iter().take(on_disk) {
            fs::write(
                archive_dir.join(archive_file_name(asset)),
                b"fake-archive-body",
            )
            .unwrap();
        }
        state.step2.update_selected_update_assets = assets;
    }

    fn job(root: &ExtractEngineRoot, label: &str) -> plan::Step2UpdateExtractJob {
        plan::Step2UpdateExtractJob {
            label: label.to_string(),
            tp_file: format!("{label}/{label}.tp2"),
            aliases: Vec::new(),
            tp2_rename: None,
            subdir_require: None,
            archive_path: root.subdir("archives").join(format!("{label}.zip")),
            mods_root: root.subdir("mods"),
            backup_root: root.subdir("backup"),
            target_root: None,
            backup_version_tag: "v1".to_string(),
            installed_source_ref: None,
            installed_source_id: None,
            installed_refs_path: root.0.join("refs.toml"),
        }
    }

    fn wait_for_finished(
        rx: Option<&Receiver<Step2UpdateExtractEvent>>,
    ) -> (Vec<Step2UpdateExtractEvent>, Step2UpdateExtractResult) {
        let rx = rx.expect("extractor spawned");
        let mut events = Vec::new();
        loop {
            match rx
                .recv_timeout(Duration::from_secs(30))
                .expect("extractor finishes")
            {
                Step2UpdateExtractEvent::Finished(result) => return (events, result),
                event @ Step2UpdateExtractEvent::AssetDone { .. } => events.push(event),
                Step2UpdateExtractEvent::AssetStarted { .. } => {}
            }
        }
    }

    fn queued(
        events: Vec<Step2UpdateExtractEvent>,
    ) -> (
        Sender<Step2UpdateExtractEvent>,
        Option<Receiver<Step2UpdateExtractEvent>>,
    ) {
        let (tx, rx) = mpsc::channel();
        for event in events {
            tx.send(event).unwrap();
        }
        (tx, Some(rx))
    }

    fn poll(state: &mut WizardState, rx: &mut Option<Receiver<Step2UpdateExtractEvent>>) {
        let mut scan_rx = None;
        let mut cancel = None;
        let mut progress_queue = VecDeque::new();
        poll_step2_update_extract(state, rx, &mut scan_rx, &mut cancel, &mut progress_queue);
    }

    fn finished_with(extracted: &[&str]) -> Step2UpdateExtractEvent {
        Step2UpdateExtractEvent::Finished(Step2UpdateExtractResult {
            extracted: extracted.iter().map(ToString::to_string).collect(),
            failed: Vec::new(),
        })
    }

    fn tally(state: &WizardState) -> Vec<(&str, (usize, usize, usize))> {
        state
            .step2
            .update_selected_extract_jobs
            .iter()
            .map(|(key, job)| (key.as_str(), (job.total, job.started, job.done)))
            .collect()
    }

    #[test]
    fn extract_pool_size_is_ten() {
        assert_eq!(EXTRACT_POOL_SIZE, 10);
    }

    #[test]
    fn empty_archive_dir_early_returns_false() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        let mut rx = None;

        assert!(!start_step2_update_extract(&mut state, &mut rx, None));

        assert!(rx.is_none());
        assert!(!state.step2.update_selected_extract_running);
        assert_eq!(state.step2.update_selected_extract_progress, None);
        assert_eq!(state.step2.update_selected_download_scope, None);
    }

    #[test]
    fn reentry_guard_returns_false_when_already_running() {
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_extract_progress = Some((1, 4));
        let mut rx = None;

        assert!(!start_step2_update_extract(&mut state, &mut rx, None));

        assert!(rx.is_none());
        assert_eq!(state.step2.update_selected_extract_progress, Some((1, 4)));
    }

    #[test]
    fn empty_jobs_early_return_after_archive_dir_check() {
        let _lock = ambient_lock();
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        let mut rx = None;

        assert!(!start_step2_update_extract(&mut state, &mut rx, None));

        assert!(rx.is_none());
        assert!(!state.step2.update_selected_extract_running);
        assert_eq!(state.step2.update_selected_extract_progress, None);
    }

    #[test]
    fn scope_clears_when_the_extract_has_no_jobs() {
        let _lock = ambient_lock();
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        let mut rx = None;

        start_step2_update_extract(&mut state, &mut rx, None);

        assert!(state.step2.update_selected_download_scope.is_none());
    }

    #[test]
    fn run_parallel_extract_with_zero_total_sends_only_finished() {
        let (tx, rx) = mpsc::channel::<Step2UpdateExtractEvent>();

        run_parallel_extract(&[], &tx);

        let events = rx.try_iter().collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Step2UpdateExtractEvent::Finished(result)
                if result.extracted.is_empty() && result.failed.is_empty()
        ));
    }

    #[test]
    fn asset_done_carries_label_and_target_or_err() {
        let root = ExtractEngineRoot::new();
        let jobs = vec![job(&root, "ALPHA"), job(&root, "BETA")];
        for job in &jobs {
            fs::write(&job.archive_path, b"not-a-zip").unwrap();
        }
        let (tx, rx) = mpsc::channel::<Step2UpdateExtractEvent>();

        run_parallel_extract(&jobs, &tx);

        let mut dones = rx
            .try_iter()
            .filter_map(|event| match event {
                Step2UpdateExtractEvent::AssetDone {
                    index,
                    ok,
                    label,
                    tp_file,
                    target_or_err,
                } => Some((index, ok, label, tp_file, target_or_err)),
                Step2UpdateExtractEvent::AssetStarted { .. }
                | Step2UpdateExtractEvent::Finished(_) => None,
            })
            .collect::<Vec<_>>();
        dones.sort_by_key(|(index, ..)| *index);
        assert_eq!(dones.len(), 2);
        assert_eq!(
            (
                dones[0].0,
                dones[0].1,
                dones[0].2.as_str(),
                dones[0].3.as_str()
            ),
            (0, false, "ALPHA", "ALPHA/ALPHA.tp2")
        );
        assert_eq!(
            (
                dones[1].0,
                dones[1].1,
                dones[1].2.as_str(),
                dones[1].3.as_str()
            ),
            (1, false, "BETA", "BETA/BETA.tp2")
        );
        assert!(dones.iter().all(|(.., err)| !err.is_empty()));
    }

    #[test]
    fn extract_result_entries_are_bio_shaped() {
        let outcomes = vec![
            ExtractOutcome {
                index: 2,
                label: "BadMod".to_string(),
                result: Err("archive corrupt".to_string()),
            },
            ExtractOutcome {
                index: 0,
                label: "MyMod".to_string(),
                result: Ok("C:\\Mods\\MyMod".to_string()),
            },
            ExtractOutcome {
                index: 1,
                label: "Other".to_string(),
                result: Ok("C:\\Mods\\Other".to_string()),
            },
        ];

        let result = extract_result(outcomes);

        assert_eq!(
            result.extracted,
            vec![
                "MyMod -> C:\\Mods\\MyMod".to_string(),
                "Other -> C:\\Mods\\Other".to_string()
            ]
        );
        assert_eq!(result.failed, vec!["BadMod: archive corrupt".to_string()]);
    }

    #[test]
    fn extract_progress_total_tracks_actual_jobs_not_asset_count() {
        let _lock = ambient_lock();
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        seed_assets(&mut state, 3, 2);
        let mut rx = None;

        assert!(start_step2_update_extract(&mut state, &mut rx, None));
        assert_eq!(state.step2.update_selected_extract_progress, Some((0, 2)));
        assert!(state.step2.update_selected_extract_running);
        assert_eq!(state.step2.scan_status, "Extracting updates: 0/2");

        let (events, result) = wait_for_finished(rx.as_ref());
        assert_eq!(events.len(), 2);
        assert_eq!(result.failed.len(), 2);
    }

    #[test]
    fn unscoped_extract_plan_holds_every_asset_despite_a_leftover_scope() {
        let _lock = ambient_lock();
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        seed_assets(&mut state, 2, 2);
        state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        state.step2.update_selected_download_scope = Some("mod0".to_string());
        let refs_path = root.0.join("mod_installed_refs.toml");
        let mut rx = None;

        assert!(start_step2_update_extract(
            &mut state,
            &mut rx,
            Some(&refs_path)
        ));

        assert_eq!(
            state.step2.update_selected_extract_progress,
            Some((0, 2)),
            "the pipeline extract plans both assets, ignoring the leftover scope"
        );
        wait_for_finished(rx.as_ref());
    }

    #[test]
    fn asset_done_advances_progress_and_status() {
        let mut state = WizardState::default();
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_extract_progress = Some((0, 3));
        let (_tx, mut rx) = queued(vec![
            Step2UpdateExtractEvent::AssetDone {
                index: 1,
                ok: true,
                label: "A".to_string(),
                tp_file: "A/A.TP2".to_string(),
                target_or_err: "C:\\a".to_string(),
            },
            Step2UpdateExtractEvent::AssetDone {
                index: 0,
                ok: false,
                label: "B".to_string(),
                tp_file: "B/B.TP2".to_string(),
                target_or_err: "boom".to_string(),
            },
        ]);

        poll(&mut state, &mut rx);

        assert!(rx.is_some());
        assert_eq!(state.step2.update_selected_extract_progress, Some((2, 3)));
        assert_eq!(state.step2.scan_status, "Extracting updates: 2/3");
    }

    #[test]
    fn extract_start_tallies_jobs_per_mod() {
        let _lock = ambient_lock();
        let root = ExtractEngineRoot::new();
        let mut state = root.state();
        seed_assets(&mut state, 3, 2);
        state
            .step2
            .update_selected_extract_jobs
            .entry("stale".to_string())
            .or_default()
            .done = 4;
        let mut rx = None;

        assert!(start_step2_update_extract(&mut state, &mut rx, None));

        assert_eq!(
            tally(&state),
            vec![("mod0", (1, 0, 0)), ("mod1", (1, 0, 0))]
        );
        wait_for_finished(rx.as_ref());

        state.step2.update_selected_extract_running = false;
        let second_archive = Step2UpdateAsset {
            game_tab: "BG2EE".to_string(),
            tag: "v2".to_string(),
            asset_name: "MOD0-v2.zip".to_string(),
            ..asset(0)
        };
        fs::write(
            PathBuf::from(&state.step1.mods_archive_folder)
                .join(archive_file_name(&second_archive)),
            b"fake-archive-body",
        )
        .unwrap();
        state
            .step2
            .update_selected_update_assets
            .push(second_archive);
        let mut rx = None;

        assert!(start_step2_update_extract(&mut state, &mut rx, None));

        assert_eq!(
            tally(&state),
            vec![("mod0", (2, 0, 0)), ("mod1", (1, 0, 0))]
        );
        wait_for_finished(rx.as_ref());
    }

    #[test]
    fn asset_started_and_done_advance_the_mods_tally() {
        let mut state = WizardState::default();
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_extract_progress = Some((0, 2));
        for key in ["a", "b"] {
            state
                .step2
                .update_selected_extract_jobs
                .entry(key.to_string())
                .or_default()
                .total = 1;
        }
        let (tx, mut rx) = queued(vec![Step2UpdateExtractEvent::AssetStarted {
            tp_file: "A\\Setup-A.TP2".to_string(),
        }]);

        poll(&mut state, &mut rx);

        assert_eq!(tally(&state), vec![("a", (1, 1, 0)), ("b", (1, 0, 0))]);
        assert_eq!(state.step2.update_selected_extract_progress, Some((0, 2)));

        tx.send(Step2UpdateExtractEvent::AssetDone {
            index: 0,
            ok: true,
            label: "A".to_string(),
            tp_file: "A/A.tp2".to_string(),
            target_or_err: "C:\\a".to_string(),
        })
        .unwrap();

        poll(&mut state, &mut rx);

        assert_eq!(tally(&state), vec![("a", (1, 1, 1)), ("b", (1, 0, 0))]);
        assert_eq!(state.step2.update_selected_extract_progress, Some((1, 2)));
        assert_eq!(state.step2.scan_status, "Extracting updates: 1/2");
        assert!(rx.is_some());
    }

    #[test]
    fn failed_unpack_drops_the_mods_tally_entry() {
        let mut state = WizardState::default();
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_extract_progress = Some((0, 2));
        for key in ["a", "b"] {
            let jobs = state
                .step2
                .update_selected_extract_jobs
                .entry(key.to_string())
                .or_default();
            jobs.total = 1;
            jobs.started = 1;
        }
        let (_tx, mut rx) = queued(vec![Step2UpdateExtractEvent::AssetDone {
            index: 0,
            ok: false,
            label: "A".to_string(),
            tp_file: "A/A.tp2".to_string(),
            target_or_err: "corrupt archive".to_string(),
        }]);

        poll(&mut state, &mut rx);

        assert_eq!(tally(&state), vec![("b", (1, 1, 0))]);
        assert_eq!(state.step2.update_selected_extract_progress, Some((1, 2)));
    }

    #[test]
    fn extract_finished_with_results_rearms_saved_log_apply_for_the_pipeline_origin() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_extract_progress = Some((1, 1));
        let (tx, mut rx) = queued(vec![finished_with(&[
            "EEFIXPACK -> C:\\dest\\mods\\eefixpack",
        ])]);

        poll(&mut state, &mut rx);

        assert!(state.step2.pending_saved_log_apply);
        assert!(rx.is_none());
        assert!(!state.step2.update_selected_extract_running);
        assert_eq!(state.step2.update_selected_extract_progress, Some((1, 1)));
        drop(tx);
    }

    #[test]
    fn extract_finished_with_zero_extracted_does_not_rearm_apply() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_origin = DownloadOrigin::InstallPipeline;
        state.step2.update_selected_extract_running = true;
        let (tx, mut rx) = queued(vec![finished_with(&[])]);

        poll(&mut state, &mut rx);

        assert!(!state.step2.pending_saved_log_apply);
        assert!(rx.is_none());
        assert_eq!(
            state.step2.scan_status,
            "Extract updates finished: 0 updated, 0 failed"
        );
        drop(tx);
    }

    #[test]
    fn workspace_origin_extract_finish_does_not_rearm_saved_log_apply() {
        let mut state = WizardState::default();
        state.step2.update_selected_download_origin = DownloadOrigin::Workspace;
        state.step2.update_selected_extract_running = true;
        let (tx, mut rx) = queued(vec![finished_with(&[
            "EEFIXPACK -> C:\\dest\\mods\\eefixpack",
        ])]);

        poll(&mut state, &mut rx);

        assert!(!state.step2.pending_saved_log_apply);
        assert!(rx.is_none());
        assert!(state.step2.is_scanning, "the rescan still starts");
        assert_eq!(state.step1_mods_folder_has_tp2, Some(true));
        drop(tx);
    }

    #[test]
    fn disconnected_worker_clears_running_and_scope() {
        let mut state = WizardState::default();
        state.step2.update_selected_extract_running = true;
        state.step2.update_selected_download_scope = Some("alpha".to_string());
        let (tx, mut rx) = queued(Vec::new());
        drop(tx);

        poll(&mut state, &mut rx);

        assert!(rx.is_none());
        assert!(!state.step2.update_selected_extract_running);
        assert_eq!(state.step2.update_selected_download_scope, None);
        assert_eq!(
            state.step2.scan_status,
            "Extract updates failed: worker disconnected"
        );
    }
}
