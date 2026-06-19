// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::app::mod_downloads::{self, ModDownloadSource, ModDownloadsLoad};
use crate::app::state::{Step2Selection, WizardState, update_selection_signature};

pub(crate) fn preview_update_selected(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &ModDownloadsLoad,
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let selected_source_ids = state.step2.selected_source_ids.clone();
    state.step2.update_selected_target_game_tab = None;
    state.step2.update_selected_target_tp_file = None;

    let FullUpdatePreviewCollection {
        mut known,
        mut manual,
        mut unknown,
        locked,
        mut update_requests,
        mut queued_tp2,
    } = collect_full_update_preview(state, sources, &selected_source_ids, exact_log_mode);
    let mut pending_preview = PendingLogUpdatePreview {
        known: &mut known,
        manual: &mut manual,
        unknown: &mut unknown,
        update_requests: &mut update_requests,
    };
    extend_log_pending_update_requests(
        state,
        sources,
        &selected_source_ids,
        &mut queued_tp2,
        &mut pending_preview,
    );

    let preview = FullUpdatePreviewResult {
        known,
        manual,
        unknown,
        locked,
        update_requests,
    };
    let update_requests = apply_full_update_preview_state(state, preview, exact_log_mode);
    start_full_update_preview_check(
        state,
        step2_update_check_rx,
        update_requests,
        exact_log_mode,
    );
}

pub(crate) fn preview_update_selected_mod(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &ModDownloadsLoad,
) {
    let Some((game_tab, tp_file)) = target_update_mod(state) else {
        return;
    };
    state.step2.update_selected_target_game_tab = Some(game_tab.clone());
    state.step2.update_selected_target_tp_file = Some(tp_file.clone());
    let selected_source_ids = state.step2.selected_source_ids.clone();
    let mut preview =
        collect_target_update_preview(state, sources, &selected_source_ids, &game_tab, &tp_file);
    if preview.target_label.is_none() {
        collect_pending_target_update_preview(
            state,
            sources,
            &selected_source_ids,
            &game_tab,
            &tp_file,
            &mut preview,
        );
    }
    apply_target_update_preview(state, &game_tab, &tp_file, preview, step2_update_check_rx);
}

struct FullUpdatePreviewCollection {
    known: Vec<String>,
    manual: Vec<String>,
    unknown: Vec<String>,
    locked: Vec<String>,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
    queued_tp2: HashSet<String>,
}

struct FullUpdatePreviewResult {
    known: Vec<String>,
    manual: Vec<String>,
    unknown: Vec<String>,
    locked: Vec<String>,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
}

struct TargetUpdatePreview {
    target_label: Option<String>,
    known: Vec<String>,
    manual: Vec<String>,
    unknown: Vec<String>,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
}

fn collect_full_update_preview(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    exact_log_mode: bool,
) -> FullUpdatePreviewCollection {
    let mut preview = FullUpdatePreviewCollection {
        known: Vec::new(),
        manual: Vec::new(),
        unknown: Vec::new(),
        locked: Vec::new(),
        update_requests: Vec::new(),
        queued_tp2: HashSet::new(),
    };
    let active_tabs = [state.step2.active_game_tab.clone()];
    let all_tabs = ["BGEE".to_string(), "BG2EE".to_string()];
    let game_tabs = if state.step1.game_install == "EET" {
        all_tabs.as_slice()
    } else {
        active_tabs.as_slice()
    };
    for game_tab in game_tabs {
        collect_game_tab_update_preview(
            state,
            sources,
            selected_source_ids,
            exact_log_mode,
            game_tab,
            &mut preview,
        );
    }
    preview
}

fn collect_game_tab_update_preview(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    exact_log_mode: bool,
    game_tab: &str,
    preview: &mut FullUpdatePreviewCollection,
) {
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };
    for mod_state in mods.iter_mut() {
        mod_state.package_marker = None;
        if exact_log_mode || !mod_selected_for_update(mod_state) {
            continue;
        }
        let label = mod_update_label(mod_state);
        if mod_state.update_locked {
            preview.locked.push(label);
            continue;
        }
        queue_mod_update_preview(
            game_tab,
            mod_state,
            &label,
            sources,
            selected_source_ids,
            preview,
        );
    }
}

fn mod_selected_for_update(mod_state: &crate::app::state::Step2ModState) -> bool {
    mod_state.checked
        || mod_state
            .components
            .iter()
            .any(|component| component.checked)
}

fn mod_update_label(mod_state: &crate::app::state::Step2ModState) -> String {
    if mod_state.name.trim().is_empty() {
        mod_state.tp_file.clone()
    } else {
        mod_state.name.clone()
    }
}

fn queue_mod_update_preview(
    game_tab: &str,
    mod_state: &mut crate::app::state::Step2ModState,
    label: &str,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut FullUpdatePreviewCollection,
) {
    let source = resolve_selected_source(sources, selected_source_ids, &mod_state.tp_file);
    if let Some(source) = source {
        if mod_downloads::source_is_auto_resolvable(&source) {
            let tp2_key = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
            if preview.queued_tp2.insert(tp2_key) {
                queue_source_request(
                    game_tab,
                    &mod_state.tp_file,
                    label,
                    None,
                    &source,
                    &mut preview.update_requests,
                );
            }
            preview.known.push(label.to_string());
        } else {
            mod_state.package_marker = Some('!');
            preview.manual.push(label.to_string());
        }
    } else {
        mod_state.package_marker = Some('!');
        preview.unknown.push(label.to_string());
    }
}

fn apply_full_update_preview_state(
    state: &mut WizardState,
    preview: FullUpdatePreviewResult,
    exact_log_mode: bool,
) -> Vec<super::app_step2_update_check::Step2UpdateCheckRequest> {
    let known_count = preview.known.len();
    let manual_count = preview.manual.len();
    let unknown_count = preview.unknown.len();
    let locked_count = preview.locked.len();
    let request_count = preview.update_requests.len();
    state.step2.update_selected_known_sources = preview.known;
    state.step2.update_selected_manual_sources = preview.manual;
    state.step2.update_selected_unknown_sources = preview.unknown;
    clear_full_update_preview_results(state);
    state.step2.update_selected_check_total_count = request_count;
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
    state.step2.update_selected_last_selection_signature =
        Some(update_selection_signature(&state.step2));
    state.step2.update_selected_last_was_full_selection = true;
    if exact_log_mode {
        state.step2.exact_log_mod_list_checked = true;
    }
    state.step2.scan_status = full_update_preview_status(
        exact_log_mode,
        known_count,
        manual_count,
        unknown_count,
        locked_count,
    );
    preview.update_requests
}

fn clear_full_update_preview_results(state: &mut WizardState) {
    state.step2.update_selected_update_assets.clear();
    state.step2.update_selected_update_sources.clear();
    state.step2.update_selected_locked_update_assets.clear();
    state.step2.update_selected_locked_update_sources.clear();
    state.step2.update_selected_missing_sources.clear();
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
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
    state.step2.update_selected_check_done_count = 0;
}

fn full_update_preview_status(
    exact_log_mode: bool,
    known_count: usize,
    manual_count: usize,
    unknown_count: usize,
    locked_count: usize,
) -> String {
    if exact_log_mode {
        let missing_count = known_count + manual_count + unknown_count;
        format!(
            "Check mod list: {missing_count} missing, {known_count} auto, {manual_count} manual, {unknown_count} no source"
        )
    } else {
        format!(
            "Compare versions: {known_count} auto, {manual_count} manual, {unknown_count} missing, {locked_count} locked"
        )
    }
}

fn start_full_update_preview_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
    exact_log_mode: bool,
) {
    if update_requests.is_empty() {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    } else {
        state.step2.scan_status = if exact_log_mode {
            format!("Checking missing mod sources: {}", update_requests.len())
        } else {
            format!("Checking version sources: {}", update_requests.len())
        };
        super::app_step2_update_check::start_step2_update_check(
            state,
            step2_update_check_rx,
            update_requests,
        );
    }
}

fn target_update_mod(state: &WizardState) -> Option<(String, String)> {
    if let (Some(game_tab), Some(tp_file)) = (
        state.step2.update_selected_target_game_tab.clone(),
        state.step2.update_selected_target_tp_file.clone(),
    ) {
        Some((game_tab, tp_file))
    } else if let Some(Step2Selection::Mod { game_tab, tp_file }) = state.step2.selected.clone() {
        Some((game_tab, tp_file))
    } else {
        None
    }
}

fn collect_target_update_preview(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    game_tab: &str,
    tp_file: &str,
) -> TargetUpdatePreview {
    let mut preview = TargetUpdatePreview {
        target_label: None,
        known: Vec::new(),
        manual: Vec::new(),
        unknown: Vec::new(),
        update_requests: Vec::new(),
    };
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };
    if let Some(mod_state) = mods
        .iter_mut()
        .find(|mod_state| mod_state.tp_file == tp_file)
    {
        mod_state.package_marker = None;
        let label = mod_update_label(mod_state);
        preview.target_label = Some(label.clone());
        if !mod_state.update_locked {
            queue_target_mod_update_preview(
                game_tab,
                mod_state,
                &label,
                sources,
                selected_source_ids,
                &mut preview,
            );
        }
    }
    preview
}

fn queue_target_mod_update_preview(
    game_tab: &str,
    mod_state: &mut crate::app::state::Step2ModState,
    label: &str,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut TargetUpdatePreview,
) {
    let source = resolve_selected_source(sources, selected_source_ids, &mod_state.tp_file);
    if let Some(source) = source {
        if mod_downloads::source_is_auto_resolvable(&source) {
            queue_source_request(
                game_tab,
                &mod_state.tp_file,
                label,
                None,
                &source,
                &mut preview.update_requests,
            );
            preview.known.push(label.to_string());
        } else {
            mod_state.package_marker = Some('!');
            preview.manual.push(label.to_string());
        }
    } else {
        mod_state.package_marker = Some('!');
        preview.unknown.push(label.to_string());
    }
}

fn collect_pending_target_update_preview(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    game_tab: &str,
    tp_file: &str,
    preview: &mut TargetUpdatePreview,
) {
    let target_tp2 = mod_downloads::normalize_mod_download_tp2(tp_file);
    for pending in &state.step2.log_pending_downloads {
        if pending.game_tab != game_tab
            || mod_downloads::normalize_mod_download_tp2(&pending.tp_file) != target_tp2
        {
            continue;
        }
        queue_pending_target_update_preview(state, sources, selected_source_ids, pending, preview);
        break;
    }
}

fn queue_pending_target_update_preview(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    pending: &crate::app::state::Step2LogPendingDownload,
    preview: &mut TargetUpdatePreview,
) {
    let source = resolve_selected_source(sources, selected_source_ids, &pending.tp_file);
    if let Some(source) = source {
        if mod_downloads::source_is_auto_resolvable(&source) {
            queue_source_request(
                &pending.game_tab,
                &pending.tp_file,
                &pending.label,
                state
                    .step1
                    .installs_exactly_from_weidu_logs()
                    .then_some(pending.requested_version.as_deref())
                    .flatten(),
                &source,
                &mut preview.update_requests,
            );
            preview.known.push(pending.label.clone());
        } else {
            preview.manual.push(pending.label.clone());
        }
    } else {
        preview.unknown.push(pending.label.clone());
    }
    preview.target_label = Some(pending.label.clone());
}

fn apply_target_update_preview(
    state: &mut WizardState,
    game_tab: &str,
    tp_file: &str,
    preview: TargetUpdatePreview,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
) {
    let update_requests = preview.update_requests;
    if let Some(label) = preview.target_label.as_deref() {
        replace_label_entries(
            &mut state.step2.update_selected_known_sources,
            label,
            preview.known,
        );
        replace_label_entries(
            &mut state.step2.update_selected_manual_sources,
            label,
            preview.manual,
        );
        replace_label_entries(
            &mut state.step2.update_selected_unknown_sources,
            label,
            preview.unknown,
        );
        super::app_step2_update_check::clear_update_check_result_for_mod(
            state, game_tab, tp_file, label,
        );
    }
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.update_selected_check_done_count = 0;
    state.step2.update_selected_check_total_count = update_requests.len();
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
    state.step2.update_selected_last_selection_signature = None;
    state.step2.update_selected_last_was_full_selection = false;
    start_target_update_preview_check(state, step2_update_check_rx, update_requests);
}

fn start_target_update_preview_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
) {
    if update_requests.is_empty() {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    } else {
        state.step2.scan_status = "Checking version source: 1".to_string();
        super::app_step2_update_check::start_step2_update_check(
            state,
            step2_update_check_rx,
            update_requests,
        );
    }
}

fn replace_label_entries(target: &mut Vec<String>, label: &str, replacement: Vec<String>) {
    target.retain(|entry| entry != label);
    target.extend(replacement);
}

fn queue_source_request(
    game_tab: &str,
    tp_file: &str,
    label: &str,
    requested_version: Option<&str>,
    source: &ModDownloadSource,
    update_requests: &mut Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
) {
    if let Some(repo) = source.github.as_deref() {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: repo.to_string(),
            exact_github: source.exact_github.clone(),
            source_url: String::new(),
            channel: source.channel.clone(),
            tag: source.tag.clone(),
            commit: source.commit.clone(),
            branch: source.branch.clone(),
            asset: if source.commit.is_none() && source.tag.is_none() && source.branch.is_none() {
                source.asset.clone()
            } else {
                None
            },
            pkg: if source.commit.is_none() && source.tag.is_none() && source.branch.is_none() {
                mod_downloads::preferred_pkg_for_current_platform(source)
            } else {
                None
            },
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::source_is_page_archive_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::source_is_sentrizeal_download_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::is_direct_archive_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    }
}

struct PendingLogUpdatePreview<'a> {
    known: &'a mut Vec<String>,
    manual: &'a mut Vec<String>,
    unknown: &'a mut Vec<String>,
    update_requests: &'a mut Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
}

fn extend_log_pending_update_requests(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    queued_tp2: &mut HashSet<String>,
    pending_preview: &mut PendingLogUpdatePreview<'_>,
) {
    let include_all_tabs = state.step1.game_install == "EET";
    let game_tab = state.step2.active_game_tab.as_str();
    for pending in &state.step2.log_pending_downloads {
        if !include_all_tabs && pending.game_tab != game_tab {
            continue;
        }
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
        if tp2_key.is_empty() || !queued_tp2.insert(tp2_key) {
            continue;
        }
        let source = resolve_selected_source(sources, selected_source_ids, &pending.tp_file);
        if let Some(source) = source {
            if mod_downloads::source_is_auto_resolvable(&source) {
                queue_source_request(
                    &pending.game_tab,
                    &pending.tp_file,
                    &pending.label,
                    forwarded_version(state, pending.requested_version.as_deref()),
                    &source,
                    pending_preview.update_requests,
                );
                pending_preview.known.push(pending.label.clone());
            } else {
                pending_preview.manual.push(pending.label.clone());
            }
        } else {
            pending_preview.unknown.push(pending.label.clone());
        }
    }
}

#[must_use]
fn forwarded_version<'a>(state: &WizardState, pending_version: Option<&'a str>) -> Option<&'a str> {
    if state.step1.installs_exactly_from_weidu_logs()
        || super::app_step2_update_check::reproduce_exact_gate(state)
    {
        pending_version
    } else {
        None
    }
}

fn resolve_selected_source(
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    tp2: &str,
) -> Option<ModDownloadSource> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp2);
    sources.resolve_source(tp2, selected_source_ids.get(&tp2_key).map(String::as_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_full_update_preview_results_clears_extracted_and_failed_lists() {
        let mut state = WizardState::default();
        state.step2.update_selected_extracted_sources = vec!["EET -> C:/m/EET".to_string()];
        state.step2.update_selected_extract_failed_sources =
            vec!["BadMod: unpack failed".to_string()];

        clear_full_update_preview_results(&mut state);

        assert!(
            state.step2.update_selected_extracted_sources.is_empty(),
            "the post-extract Extracted list must clear on a fresh full update check"
        );
        assert!(
            state
                .step2
                .update_selected_extract_failed_sources
                .is_empty(),
            "the extract-failed list must clear alongside it"
        );
    }

    #[test]
    fn reproduce_mode_forwards_pinned_version() {
        let state = WizardState {
            modlist_auto_build_active: true,
            reproduce_exact: true,
            ..Default::default()
        };
        assert_eq!(forwarded_version(&state, Some("8.39")), Some("8.39"));
    }

    #[test]
    fn non_reproduce_non_exact_drops_version() {
        let state = WizardState::default();
        assert_eq!(forwarded_version(&state, Some("8.39")), None);
    }
}
