// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;

pub(crate) fn queue_exact_log_update_preview(
    state: &mut WizardState,
    active_game_tab: &str,
    auto_download: bool,
) {
    state.step2.active_game_tab = active_game_tab.to_string();
    state.step2.pending_saved_log_apply = true;
    state.step2.pending_saved_log_update_preview = true;
    state.step2.pending_saved_log_download = auto_download;
    state.step2.scan_status = if auto_download {
        "Preparing missing mod download...".to_string()
    } else {
        "Preparing missing mod check...".to_string()
    };
}

pub(crate) fn advance_pending_saved_log_flow(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    step2_update_download_rx: &mut Option<
        Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>,
    >,
) {
    if state.step2.is_scanning || step2_scan_rx.is_some() {
        return;
    }

    let preflight_blocker = if state.modlist_auto_build_active
        && (state.step2.pending_saved_log_apply || state.step2.pending_saved_log_update_preview)
    {
        auto_build_preflight_blocker(state)
    } else {
        None
    };
    if let Some(reason) = preflight_blocker {
        stop_auto_build(state, &reason);
        return;
    }

    if state.step2.pending_saved_log_apply || state.step2.pending_saved_log_update_preview {
        if scan_failed(state) {
            clear_pending(state);
            return;
        }
        if state.step2.last_scan_report.is_none() {
            super::app_step2_scan::start_step2_scan(
                state,
                step2_scan_rx,
                step2_cancel,
                step2_progress_queue,
            );
            return;
        }
        if state.step2.pending_saved_log_apply {
            state.step2.pending_saved_log_apply = false;
            super::app_step2_log::apply_saved_weidu_log_selection(state);
        }
        if state.step2.pending_saved_log_update_preview {
            state.step2.pending_saved_log_update_preview = false;
            let loaded = crate::app::mod_downloads::load_mod_download_sources();
            super::app_step2_update_preview::preview_update_selected(
                state,
                step2_update_check_rx,
                &loaded,
            );
        }
    }

    if state.step2.pending_saved_log_download
        && !state.step2.pending_saved_log_apply
        && !state.step2.pending_saved_log_update_preview
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
    {
        if state.modlist_auto_build_active {
            if let Some(reason) = auto_build_blocker_before_download(state) {
                stop_auto_build(state, &reason);
                return;
            }
            if state.step2.update_selected_update_assets.is_empty() {
                state.step2.pending_saved_log_download = false;
                finish_auto_build_at_step5(state);
                return;
            }
            state.modlist_auto_build_waiting_for_install = true;
        }
        state.step2.pending_saved_log_download = false;
        super::app_step2_update_download::start_step2_update_download(
            state,
            step2_update_download_rx,
        );
    }

    if state.modlist_auto_build_active
        && state.modlist_auto_build_waiting_for_install
        && !state.step2.is_scanning
        && step2_scan_rx.is_none()
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
    {
        if scan_failed(state) {
            stop_auto_build(state, "scan failed after extraction");
            return;
        }
        if let Some(reason) = auto_build_blocker_before_install(state) {
            stop_auto_build(state, &reason);
            return;
        }
        finish_auto_build_at_step5(state);
    }
}

const fn clear_pending(state: &mut WizardState) {
    state.step2.pending_saved_log_apply = false;
    state.step2.pending_saved_log_update_preview = false;
    state.step2.pending_saved_log_download = false;
}

fn stop_auto_build(state: &mut WizardState, reason: &str) {
    clear_pending(state);
    state.modlist_auto_build_active = false;
    state.modlist_auto_build_waiting_for_install = false;
    let message = format!("Auto Build stopped: {reason}");
    state.step2.scan_status = message.clone();
    state.step5.last_status_text = message;
}

fn finish_auto_build_at_step5(state: &mut WizardState) {
    state.modlist_auto_build_active = false;
    state.modlist_auto_build_waiting_for_install = false;
    state.step2.update_selected_popup_open = false;
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.mod_download_forks_popup_open = false;
    state.current_step = 4;
    state.step5.start_install_requested = false;
    state.step5.last_status_text = "Auto Build: ready to install".to_string();
}

fn auto_build_blocker_before_download(state: &WizardState) -> Option<String> {
    if let Some(reason) = auto_build_preflight_blocker(state) {
        return Some(reason);
    }
    if !state
        .step2
        .update_selected_exact_version_failed_sources
        .is_empty()
        || !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
    {
        return Some("exact pinned version unavailable".to_string());
    }
    if let Some(source) = state.step2.update_selected_manual_sources.first() {
        return Some(format!("manual-only source required: {source}"));
    }
    if let Some(source) = state.step2.update_selected_unknown_sources.first() {
        return Some(format!("failed source resolution: {source}"));
    }
    if let Some(source) = state.step2.update_selected_failed_sources.first() {
        return Some(format!("failed source check: {source}"));
    }
    if state.step2.update_selected_update_assets.is_empty()
        && (!state.step2.update_selected_missing_sources.is_empty()
            || !state.step2.update_selected_update_sources.is_empty())
    {
        return Some("downloadable sources have no resolved archive".to_string());
    }
    None
}

fn auto_build_blocker_before_install(state: &WizardState) -> Option<String> {
    if let Some(reason) = auto_build_preflight_blocker(state) {
        return Some(reason);
    }
    if let Some(source) = state.step2.update_selected_download_failed_sources.first() {
        return Some(format!("failed download: {source}"));
    }
    if let Some(source) = state.step2.update_selected_extract_failed_sources.first() {
        return Some(format!("failed extraction/config restore: {source}"));
    }
    if !state.step2.update_selected_update_assets.is_empty()
        || !state.step2.update_selected_missing_sources.is_empty()
        || !state.step2.update_selected_update_sources.is_empty()
    {
        return Some("unresolved downloads remain".to_string());
    }
    None
}

pub(crate) fn unresolved_required_mods_blocker(state: &WizardState) -> Option<String> {
    if !state
        .step2
        .update_selected_exact_version_failed_sources
        .is_empty()
        || !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
    {
        return Some("exact pinned version unavailable".to_string());
    }
    if let Some(source) = state.step2.update_selected_manual_sources.first() {
        return Some(format!("manual-only source required: {source}"));
    }
    if let Some(source) = state.step2.update_selected_unknown_sources.first() {
        return Some(format!("failed source resolution: {source}"));
    }
    if let Some(source) = state.step2.update_selected_failed_sources.first() {
        return Some(format!("failed source check: {source}"));
    }
    if let Some(source) = state.step2.update_selected_download_failed_sources.first() {
        return Some(format!("failed download: {source}"));
    }
    if let Some(source) = state.step2.update_selected_extract_failed_sources.first() {
        return Some(format!("failed extraction/config restore: {source}"));
    }
    if state.step2.update_selected_update_assets.is_empty()
        && (!state.step2.update_selected_missing_sources.is_empty()
            || !state.step2.update_selected_update_sources.is_empty())
    {
        return Some("downloadable sources have no resolved archive".to_string());
    }
    None
}

fn auto_build_preflight_blocker(state: &WizardState) -> Option<String> {
    let (ok, message) = crate::app::state_validation::run_path_check(&state.step1);
    if ok {
        None
    } else {
        Some(format!("local path/tool preflight failed: {message}"))
    }
}

fn scan_failed(state: &WizardState) -> bool {
    state.step2.scan_status.starts_with("Scan failed:")
        || state.step2.scan_status == "Scan canceled"
        || state.step2.scan_status == "Scan worker disconnected"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_auto_build_routes_to_step5_without_auto_starting_install() {
        let mut state = WizardState {
            modlist_auto_build_active: true,
            modlist_auto_build_waiting_for_install: true,
            ..Default::default()
        };

        finish_auto_build_at_step5(&mut state);

        assert!(!state.modlist_auto_build_active);
        assert!(!state.modlist_auto_build_waiting_for_install);
        assert_eq!(state.current_step, 4);
        assert!(
            !state.step5.start_install_requested,
            "auto-build must stop on Step 5 and wait for the user to click Install"
        );
        assert_eq!(state.step5.last_status_text, "Auto Build: ready to install");
    }

    #[test]
    fn unresolved_required_mods_blocker_maps_each_bucket_and_honors_precedence() {
        use crate::app::state::Step2State;

        type Setup = fn(&mut Step2State);

        let blocker_for = |setup: Setup| -> Option<String> {
            let mut state = WizardState::default();
            setup(&mut state.step2);
            unresolved_required_mods_blocker(&state)
        };

        assert!(
            blocker_for(|_| {}).is_none(),
            "no unresolved sources -> no blocker"
        );

        let cases: &[(Setup, &str)] = &[
            (
                |s| s.update_selected_exact_version_failed_sources = vec!["ISNF".to_string()],
                "exact pinned version",
            ),
            (
                |s| s.update_selected_manual_sources = vec!["ManualMod".to_string()],
                "manual-only source",
            ),
            (
                |s| s.update_selected_unknown_sources = vec!["UnknownMod".to_string()],
                "failed source resolution",
            ),
            (
                |s| s.update_selected_failed_sources = vec!["TNT".to_string()],
                "failed source check",
            ),
            (
                |s| s.update_selected_download_failed_sources = vec!["DlMod".to_string()],
                "failed download",
            ),
            (
                |s| s.update_selected_extract_failed_sources = vec!["ExMod".to_string()],
                "failed extraction",
            ),
            (
                |s| s.update_selected_missing_sources = vec!["MissMod".to_string()],
                "no resolved archive",
            ),
        ];
        for (setup, expected) in cases {
            let reason = blocker_for(*setup);
            assert!(
                reason.as_deref().is_some_and(|r| r.contains(expected)),
                "bucket must halt with '{expected}', got {reason:?}"
            );
        }

        let precedence = blocker_for(|s| {
            s.update_selected_exact_version_failed_sources = vec!["ISNF".to_string()];
            s.update_selected_manual_sources = vec!["ManualMod".to_string()];
        });
        assert!(
            precedence
                .as_deref()
                .is_some_and(|r| r.contains("exact pinned version")),
            "exact-version bucket must win precedence, got {precedence:?}"
        );
    }

    #[test]
    fn gate_failure_bucket_halts_auto_build() {
        let mut state = WizardState {
            modlist_auto_build_active: true,
            modlist_auto_build_waiting_for_install: true,
            step2: crate::app::state::Step2State {
                update_selected_failed_sources: vec!["TNT".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let reason = unresolved_required_mods_blocker(&state);
        assert!(reason.is_some(), "blocker must fire for failed source");
        stop_auto_build(&mut state, reason.as_deref().unwrap_or(""));
        assert!(!state.modlist_auto_build_active);
        assert!(!state.modlist_auto_build_waiting_for_install);
        assert!(state.step2.scan_status.contains("Auto Build stopped"));
    }
}
