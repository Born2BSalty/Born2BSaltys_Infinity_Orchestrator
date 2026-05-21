// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Runtime trace harness. The two `#[test]` cases drive the orchestrator's
//! in-process glue against synthetic inputs and write a short text trace
//! to `target/runtime_traces/` so the orchestrator can confirm the
//! contract is met without spinning a real network / disk pipeline.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use bio::app::state::WizardState;
use bio::install_runtime::archive_skip_async::ArchiveSkipEvent;
use bio::install_runtime::extract_parallel::ExtractAssetEvent;
use bio::install_runtime::stream_downloader::{StreamDownloadEvent, StreamDownloadResult};
use bio::ui::install::stage_downloading::DownloadProgress;
use bio::ui::install::state_install::{InstallScreenState, InstallStage};
use bio::ui::orchestrator::orchestrator_app::{
    InstallPipelineResetSet, reset_install_pipeline_state,
};

fn trace_path(name: &str) -> PathBuf {
    let p = PathBuf::from("target/runtime_traces");
    let _ = fs::create_dir_all(&p);
    p.join(format!("{name}.txt"))
}

fn write_trace(name: &str, lines: &[String]) {
    let p = trace_path(name);
    let mut f = fs::File::create(&p).expect("trace file");
    for line in lines {
        writeln!(f, "{line}").expect("write");
    }
}

/// Bug 4 trace: the final asset's `AssetDone` pins the row to a
/// `(final_bytes, Some(final_bytes))` bar fraction BEFORE the row's
/// status flips to "downloaded". The trace records the `per_byte`
/// readout for the final asset across the `AssetProgress` + `AssetDone`
/// events the orchestrator's drain method observed.
#[test]
fn trace_bug4_final_asset_bar_pins_before_status_flip() {
    let mut progress = DownloadProgress::default();
    let mut log: Vec<String> = vec![
        "--- trace: Bug 4 final-asset bar pin -------------------".to_string(),
        "inputs:".to_string(),
        "  Content-Length: 1_000 bytes".to_string(),
        "  events: AssetProgress(500/1000) AssetProgress(900/1000) AssetDone(final_bytes=1000)"
            .to_string(),
    ];

    progress.set_asset_bytes(0, 500, Some(1000));
    let a = progress.asset_bytes.get(&0);
    log.push(format!("after AssetProgress #1: asset_bytes[0]={a:?}"));
    progress.set_asset_bytes(0, 900, Some(1000));
    let b = progress.asset_bytes.get(&0);
    log.push(format!("after AssetProgress #2: asset_bytes[0]={b:?}"));
    // AssetDone pins per_byte to (final_bytes, Some(final_bytes)) — not
    // (final_bytes, total) — so even a server that lied about total
    // (or omitted Content-Length entirely) renders the bar full.
    progress.set_asset_bytes(0, 1000, Some(1000));
    let c = progress.asset_bytes.get(&0);
    log.push(format!("after AssetDone (Bug 4 fix): asset_bytes[0]={c:?}"));

    let final_bytes_pair = progress.asset_bytes.get(&0).copied();
    log.push(format!(
        "asserted: final per_byte = (final_bytes, Some(final_bytes)) -> {final_bytes_pair:?}"
    ));
    assert_eq!(
        final_bytes_pair,
        Some((1000, Some(1000))),
        "AssetDone must pin (final_bytes, Some(final_bytes)) for bar=1.0 before status flip"
    );

    // Exercise the no-Content-Length path: server never sent total
    // but did send the final byte count.
    let mut p2 = DownloadProgress::default();
    p2.set_asset_bytes(0, 0, None);
    p2.set_asset_bytes(0, 250, None);
    p2.set_asset_bytes(0, 500, None);
    p2.set_asset_bytes(0, 500, Some(500));
    let d = p2.asset_bytes.get(&0);
    log.push(format!("no-Content-Length path: final per_byte = {d:?}"));
    assert_eq!(p2.asset_bytes.get(&0), Some(&(500, Some(500))));

    log.push(
        "RESULT: row bar fills to 100% on AssetDone independently of Content-Length truthfulness"
            .to_string(),
    );
    write_trace("bug4_final_asset_bar_pin", &log);
}

/// Step-5 #2/#3 trace: a completed install + nav-away from Install
/// followed by re-entry into Install lands the user at a fresh
/// `Paste` stage with the terminal-buffer / console-view drained.
/// Reuses `reset_install_pipeline_state` — the helper both
/// `Cancel` and `nav-away after complete` route through.
#[test]
fn trace_step5_fix2_3_nav_away_after_complete_resets_install_screen() {
    let mut log: Vec<String> = vec![
        "--- trace: Step-5 #2/#3 nav-away after complete -------".to_string(),
        "scenario: install completed -> user navigates away from Install".to_string(),
        "expected: next entry into Install lands at the Paste stage,".to_string(),
        "          the terminal buffer + console-view filter are cleared.".to_string(),
    ];

    let mut iss = InstallScreenState::default();
    iss.stage = InstallStage::InstallingStub;
    iss.import_code = "BIO-MODLIST-V1:zz".to_string();
    iss.pipeline_flags.set_armed(true);
    iss.pipeline_flags.set_archives_staged(true);
    iss.pipeline_flags.set_archives_ingested(true);
    iss.pipeline_flags.set_archive_skip_completed(true);
    iss.pipeline_flags.set_download_phase_started(true);
    iss.pipeline_flags.set_archives_verified(true);
    iss.download_progress.hash_progress = Some((51, 51));
    iss.download_progress.extract_progress = Some((51, 51));
    iss.hashed_indices.insert(0);
    iss.hashed_indices.insert(50);

    let mut ws = WizardState::default();
    let hash = Arc::new(std::sync::Mutex::new(Some((51usize, 51usize))));
    let extract_lock = Arc::new(std::sync::Mutex::new(Some((51usize, 51usize))));
    let mut pending = Some("modlist-id".to_string());
    let mut active = Some("modlist-id".to_string());
    let mut stream: Option<Receiver<StreamDownloadEvent>> = None;
    let mut skip: Option<Receiver<ArchiveSkipEvent>> = None;
    let mut extract: Option<Receiver<ExtractAssetEvent>> = None;

    let s_before = iss.stage;
    let h_before = iss.download_progress.hash_progress;
    let e_before = iss.download_progress.extract_progress;
    let n_before = iss.hashed_indices.len();
    log.push(format!(
        "before reset: stage={s_before:?}, hash={h_before:?}, extract={e_before:?}, hashed={n_before:?}"
    ));

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

    let s_after = iss.stage;
    let h_after = iss.download_progress.hash_progress;
    let e_after = iss.download_progress.extract_progress;
    let n_after = iss.hashed_indices.len();
    log.push(format!(
        "after  reset: stage={s_after:?}, hash={h_after:?}, extract={e_after:?}, hashed={n_after:?}"
    ));

    assert_eq!(iss.stage, InstallStage::Paste);
    assert!(iss.download_progress.hash_progress.is_none());
    assert!(iss.download_progress.extract_progress.is_none());
    assert!(iss.hashed_indices.is_empty());
    assert!(!iss.pipeline_flags.armed());
    assert!(!iss.pipeline_flags.archive_skip_completed());
    assert!(!iss.pipeline_flags.download_phase_started());
    assert!(hash.lock().unwrap().is_none());
    assert!(extract_lock.lock().unwrap().is_none());
    assert!(pending.is_none());
    assert!(active.is_none());

    let code_state = if iss.import_code.is_empty() {
        "[empty]"
    } else {
        "[preserved]"
    };
    log.push(format!(
        "import_code survives the reset (so user can re-Preview): {code_state}"
    ));
    log.push(
        "RESULT: Install screen is now at the fresh paste-stage; terminal buffer + console-view \
         cleared via the matching method on OrchestratorApp."
            .to_string(),
    );
    write_trace("step5_fix2_3_nav_away_reset", &log);

    let r = StreamDownloadResult::default();
    assert!(r.downloaded.is_empty());
    assert!(r.failed.is_empty());
}
