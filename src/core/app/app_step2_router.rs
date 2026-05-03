// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::controller::util::open_in_shell;
use crate::app::mod_downloads;
use crate::app::state::{Step2Selection, WizardState};
use crate::app::step2_action::Step2Action;
use crate::app::step2_worker::Step2ScanEvent;

pub(crate) fn handle_step2_action(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    step2_update_download_rx: &mut Option<
        Receiver<super::app_step2_update_download::Step2UpdateDownloadResult>,
    >,
    action: Step2Action,
) {
    match action {
        Step2Action::StartScan => super::app_step2_scan::start_step2_scan(
            state,
            step2_scan_rx,
            step2_cancel,
            step2_progress_queue,
        ),
        Step2Action::CancelScan => super::app_step2_scan::cancel_step2_scan(state, step2_cancel),
        Step2Action::OpenUpdatePopup => {
            state.step2.update_selected_target_game_tab = None;
            state.step2.update_selected_target_tp_file = None;
            state.step2.update_selected_popup_open = true;
        }
        Step2Action::CheckExactLogModList => {
            let active_game_tab = state.step2.active_game_tab.clone();
            super::app_step2_saved_log_flow::queue_exact_log_update_preview(
                state,
                &active_game_tab,
                false,
            );
        }
        Step2Action::DownloadUpdates => {
            super::app_step2_update_download::start_step2_update_download(
                state,
                step2_update_download_rx,
            )
        }
        Step2Action::AcceptLatestForExactVersionMisses => {
            let requests = state
                .step2
                .update_selected_exact_version_retry_requests
                .iter()
                .map(
                    |request| super::app_step2_update_check::Step2UpdateCheckRequest {
                        game_tab: request.game_tab.clone(),
                        tp_file: request.tp_file.clone(),
                        label: request.label.clone(),
                        source_id: request.source_id.clone(),
                        repo: request.repo.clone(),
                        exact_github: Vec::new(),
                        source_url: request.source_url.clone(),
                        channel: request.channel.clone(),
                        tag: request.tag.clone(),
                        commit: request.commit.clone(),
                        branch: request.branch.clone(),
                        asset: request.asset.clone(),
                        pkg: request.pkg.clone(),
                        requested_version: None,
                    },
                )
                .collect::<Vec<_>>();
            if requests.is_empty() {
                state.step2.scan_status =
                    "No exact-version misses available for latest fallback".to_string();
                state.step2.update_selected_confirm_latest_fallback_open = false;
                return;
            }
            state.step2.update_selected_merge_latest_fallback = true;
            state.step2.update_selected_confirm_latest_fallback_open = false;
            state
                .step2
                .update_selected_exact_version_retry_requests
                .clear();
            state.step2.update_selected_check_done_count = 0;
            state.step2.update_selected_check_total_count = requests.len();
            state.step2.scan_status =
                format!("Checking latest fallback sources: {}", requests.len());
            super::app_step2_update_check::start_step2_update_check(
                state,
                step2_update_check_rx,
                requests,
            );
        }
        Step2Action::PreviewUpdateSelected => {
            let loaded = mod_downloads::load_mod_download_sources();
            super::app_step2_update_preview::preview_update_selected(
                state,
                step2_update_check_rx,
                &loaded,
            );
        }
        Step2Action::PreviewUpdateSelectedMod => {
            let loaded = mod_downloads::load_mod_download_sources();
            super::app_step2_update_preview::preview_update_selected_mod(
                state,
                step2_update_check_rx,
                &loaded,
            );
        }
        Step2Action::SetSelectedModUpdateLocked(locked) => {
            set_selected_mod_update_locked(state, locked)
        }
        Step2Action::OpenSelectedReadme(path)
        | Step2Action::OpenSelectedTp2Folder(path)
        | Step2Action::OpenSelectedTp2(path)
        | Step2Action::OpenSelectedIni(path)
        | Step2Action::OpenSelectedWeb(path) => {
            if let Err(err) = open_in_shell(&path) {
                state.step2.scan_status = format!("Open failed: {err}");
            }
        }
        Step2Action::OpenModDownloadsUserSource => {
            if let Err(err) = mod_downloads::ensure_mod_downloads_files() {
                state.step2.scan_status = format!("Open failed: {err}");
                return;
            }
            let path = mod_downloads::mod_downloads_user_path();
            if let Err(err) = open_in_shell(path.to_string_lossy().as_ref()) {
                state.step2.scan_status = format!("Open failed: {err}");
            }
        }
        Step2Action::ReloadModDownloadSources => {
            if let Err(err) = mod_downloads::ensure_mod_downloads_files() {
                state.step2.scan_status = format!("Reload sources failed: {err}");
                return;
            }
            let loaded = mod_downloads::load_mod_download_sources();
            if let Some(err) = loaded.error.as_ref() {
                state.step2.scan_status = format!("Reload sources failed: {err}");
            } else {
                state.step2.scan_status =
                    format!("Reloaded mod download sources: {}", loaded.sources.len());
            }
        }
        Step2Action::OpenModDownloadSourceEditor {
            tp2,
            label,
            source_id,
        } => match mod_downloads::load_user_mod_download_source_block(&tp2, &label, &source_id) {
            Ok(text) => {
                state.step2.mod_download_source_editor_open = true;
                state.step2.mod_download_source_editor_tp2 = tp2;
                state.step2.mod_download_source_editor_label = label;
                state.step2.mod_download_source_editor_source_id = source_id;
                state.step2.mod_download_source_editor_text = text;
                state.step2.mod_download_source_editor_error = None;
            }
            Err(err) => {
                state.step2.scan_status = format!("Open source editor failed: {err}");
            }
        },
        Step2Action::SaveModDownloadSourceEditor => {
            let tp2 = state.step2.mod_download_source_editor_tp2.clone();
            let label = state.step2.mod_download_source_editor_label.clone();
            let source_id = state.step2.mod_download_source_editor_source_id.clone();
            let text = state.step2.mod_download_source_editor_text.clone();
            match mod_downloads::save_user_mod_download_source_block(
                &tp2, &label, &source_id, &text,
            ) {
                Ok(()) => {
                    state.step2.mod_download_source_editor_open = false;
                    state.step2.mod_download_source_editor_error = None;
                    invalidate_update_selected_results(state);
                    let loaded = mod_downloads::load_mod_download_sources();
                    if let Some(err) = loaded.error.as_ref() {
                        state.step2.scan_status = format!("Source saved but reload failed: {err}");
                    } else {
                        state.step2.scan_status = format!("Saved source entry for {tp2}");
                    }
                }
                Err(err) => {
                    state.step2.mod_download_source_editor_error = Some(err.clone());
                    state.step2.scan_status = format!("Save source entry failed: {err}");
                }
            }
        }
        Step2Action::SetModDownloadSource { tp2, source_id } => {
            set_mod_download_source(state, &tp2, &source_id)
        }
        Step2Action::OpenCompatForComponent {
            game_tab,
            tp_file,
            component_id,
            component_key,
        } => {
            state.step2.selected = Some(Step2Selection::Component {
                game_tab,
                tp_file,
                component_id,
                component_key,
            });
            state.step2.compat_popup_issue_override = None;
            state.step2.compat_popup_open = true;
        }
        Step2Action::SelectBgeeViaLog | Step2Action::SelectBg2eeViaLog => {}
    }
}

fn set_selected_mod_update_locked(state: &mut WizardState, locked: bool) {
    let Some(Step2Selection::Mod { game_tab, tp_file }) = state.step2.selected.clone() else {
        return;
    };
    let mod_name;
    let update_entry;
    let had_cached_update_entry = !locked && popup_has_cached_update_entry(state, &tp_file);
    {
        let mods = if game_tab == "BGEE" {
            &mut state.step2.bgee_mods
        } else {
            &mut state.step2.bg2ee_mods
        };
        let Some(mod_state) = mods
            .iter_mut()
            .find(|mod_state| mod_state.tp_file == tp_file)
        else {
            return;
        };
        if let Err(err) = super::mod_update_locks::set_mod_update_lock(&mod_state.tp_file, locked) {
            state.step2.scan_status = format!("Update lock failed: {err}");
            return;
        }
        mod_state.update_locked = locked;
        if locked {
            mod_state.package_marker = None;
        } else if had_cached_update_entry {
            mod_state.package_marker = Some('+');
        }
        mod_name = mod_state.name.clone();
        update_entry = mod_update_entry_text(mod_state);
    }
    sync_cached_popup_update_lock(state, &game_tab, &tp_file, update_entry.as_deref(), locked);
    let verb = if locked { "Locked" } else { "Unlocked" };
    state.step2.scan_status = format!("{verb} updates for {mod_name}");
}

fn set_mod_download_source(state: &mut WizardState, tp2: &str, source_id: &str) {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp2);
    let source_id = source_id.trim();
    if tp2_key.is_empty() || source_id.is_empty() {
        return;
    }
    if state
        .step2
        .selected_source_ids
        .get(&tp2_key)
        .map(String::as_str)
        == Some(source_id)
    {
        return;
    }
    state
        .step2
        .selected_source_ids
        .insert(tp2_key.clone(), source_id.to_string());
    invalidate_update_selected_results(state);
    let label = selected_source_label(state, &tp2_key).unwrap_or(tp2_key);
    state.step2.scan_status = format!("Source changed for {label}. Run Check Updates again.");
}

fn invalidate_update_selected_results(state: &mut WizardState) {
    state.step2.update_selected_has_run = false;
    state.step2.update_selected_last_selection_signature = None;
    state.step2.update_selected_last_was_full_selection = false;
    state.step2.update_selected_check_done_count = 0;
    state.step2.update_selected_check_total_count = 0;
    state.step2.update_selected_update_assets.clear();
    state.step2.update_selected_update_sources.clear();
    state.step2.update_selected_locked_update_assets.clear();
    state.step2.update_selected_locked_update_sources.clear();
    state.step2.update_selected_missing_sources.clear();
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.update_selected_known_sources.clear();
    state.step2.update_selected_manual_sources.clear();
    state.step2.update_selected_unknown_sources.clear();
    state
        .step2
        .update_selected_exact_version_failed_sources
        .clear();
    state.step2.update_selected_failed_sources.clear();
    state.step2.update_selected_check_requests.clear();
    state
        .step2
        .update_selected_exact_version_retry_requests
        .clear();
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.exact_log_mod_list_checked = false;
}

fn selected_source_label(state: &WizardState, tp2_key: &str) -> Option<String> {
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        if mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file) == tp2_key {
            return Some(if mod_state.name.trim().is_empty() {
                mod_state.tp_file.clone()
            } else {
                mod_state.name.clone()
            });
        }
    }
    state
        .step2
        .log_pending_downloads
        .iter()
        .find(|pending| mod_downloads::normalize_mod_download_tp2(&pending.tp_file) == tp2_key)
        .map(|pending| pending.label.clone())
}

fn sync_cached_popup_update_lock(
    state: &mut WizardState,
    game_tab: &str,
    tp_file: &str,
    update_entry: Option<&str>,
    locked: bool,
) {
    if locked {
        move_cached_assets_to_locked(state, game_tab, tp_file);
        move_cached_update_entry_to_locked(state, update_entry);
    } else {
        restore_cached_assets_from_locked(state, game_tab, tp_file);
        restore_cached_update_entry_from_locked(state, update_entry);
    }
}

fn move_cached_assets_to_locked(state: &mut WizardState, game_tab: &str, tp_file: &str) {
    let mut keep = Vec::new();
    for asset in state.step2.update_selected_update_assets.drain(..) {
        if asset.game_tab == game_tab && asset.tp_file == tp_file {
            state.step2.update_selected_locked_update_assets.push(asset);
        } else {
            keep.push(asset);
        }
    }
    state.step2.update_selected_update_assets = keep;
}

fn restore_cached_assets_from_locked(state: &mut WizardState, game_tab: &str, tp_file: &str) {
    let mut keep = Vec::new();
    for asset in state.step2.update_selected_locked_update_assets.drain(..) {
        if asset.game_tab == game_tab && asset.tp_file == tp_file {
            state.step2.update_selected_update_assets.push(asset);
        } else {
            keep.push(asset);
        }
    }
    state.step2.update_selected_locked_update_assets = keep;
}

fn move_cached_update_entry_to_locked(state: &mut WizardState, update_entry: Option<&str>) {
    let Some(update_entry) = update_entry else {
        return;
    };
    let mut keep = Vec::new();
    for entry in state.step2.update_selected_update_sources.drain(..) {
        if entry == update_entry {
            state
                .step2
                .update_selected_locked_update_sources
                .push(entry);
        } else {
            keep.push(entry);
        }
    }
    state.step2.update_selected_update_sources = keep;
}

fn restore_cached_update_entry_from_locked(state: &mut WizardState, update_entry: Option<&str>) {
    let Some(update_entry) = update_entry else {
        return;
    };
    let mut keep = Vec::new();
    for entry in state.step2.update_selected_locked_update_sources.drain(..) {
        if entry == update_entry {
            state.step2.update_selected_update_sources.push(entry);
        } else {
            keep.push(entry);
        }
    }
    state.step2.update_selected_locked_update_sources = keep;
}

fn popup_has_cached_update_entry(state: &WizardState, tp_file: &str) -> bool {
    state
        .step2
        .update_selected_update_assets
        .iter()
        .any(|asset| asset.tp_file == tp_file)
        || state
            .step2
            .update_selected_locked_update_assets
            .iter()
            .any(|asset| asset.tp_file == tp_file)
}

fn mod_update_entry_text(mod_state: &crate::app::state::Step2ModState) -> Option<String> {
    let latest = mod_state.latest_checked_version.as_deref()?;
    let label = if mod_state.name.trim().is_empty() {
        mod_state.tp_file.as_str()
    } else {
        mod_state.name.trim()
    };
    Some(format!("{label} ({latest})"))
}
