// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::controller::util::open_in_shell;
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_downloads;
use crate::app::mod_source_history;
use crate::app::state::{
    DownloadOrigin, Step2Selection, Step2State, Step2UpdateAsset, WizardState,
};
use crate::app::step2_action::{ModSourceEditDestination, Step2Action};
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
        Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>,
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
        Step2Action::CancelScan => {
            super::app_step2_scan::cancel_step2_scan(state, step2_cancel.as_ref());
        }
        Step2Action::OpenUpdatePopup => open_update_popup(state),
        Step2Action::DownloadUpdates => {
            start_workspace_download(state, step2_update_download_rx, None);
        }
        Step2Action::DownloadUpdateFor { tp2 } => {
            if !crate::app::state::update_pipeline_busy(&state.step2) {
                let promoted = promote_in_sync_assets(&mut state.step2, &tp2);
                if !start_workspace_download(state, step2_update_download_rx, Some(tp2)) {
                    demote_in_sync_assets(&mut state.step2, &promoted);
                }
            }
        }
        Step2Action::AcceptLatestForExactVersionMisses => {
            if crate::app::state::update_pipeline_busy(&state.step2) {
                state.step2.scan_status = "Wait for the current check to finish".to_string();
            } else {
                accept_latest_for_exact_version_misses(state, step2_update_check_rx);
            }
        }
        Step2Action::PreviewUpdateSelected => {
            let loaded = mod_downloads::load_mod_download_sources();
            super::app_step2_update_preview::preview_update_selected(
                state,
                step2_update_check_rx,
                &loaded,
                super::app_step2_update_preview::UpdateCheckScope::WholeFolder,
            );
        }
        Step2Action::PreviewUpdateSelectedMod => {
            let target = open_drawer_focused_on_selected_mod(state);
            preview_update_target_mod(state, step2_update_check_rx, target);
        }
        Step2Action::SetSelectedModUpdateLocked(locked) => {
            set_selected_mod_update_locked(state, locked);
        }
        Step2Action::SetModUpdateLocked { tp2, locked } => {
            set_mod_update_locked(state, &tp2, locked);
        }
        Step2Action::OpenSelectedReadme(path)
        | Step2Action::OpenSelectedTp2Folder(path)
        | Step2Action::OpenSelectedTp2(path)
        | Step2Action::OpenSelectedIni(path)
        | Step2Action::OpenSelectedWeb(path) => open_selected_path(state, &path),
        Step2Action::OpenCompatForComponent {
            game_tab,
            tp_file,
            component_id,
            component_key,
        } => open_compat_for_component(state, game_tab, tp_file, component_id, component_key),
        Step2Action::SelectBgeeViaLog
        | Step2Action::SelectBg2eeViaLog
        | Step2Action::ImportWeiduLogs
        | Step2Action::DownloadUpdatesAndApplyLogs => {}
        other => handle_step2_download_source_action(state, step2_update_check_rx, other),
    }
}

fn start_workspace_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<
        Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>,
    >,
    scope_tp2: Option<String>,
) -> bool {
    match super::app_step2_update_download::start_step2_update_download_scoped(
        state,
        step2_update_download_rx,
        scope_tp2,
        &HashSet::new(),
        DownloadOrigin::Workspace,
    ) {
        Ok(()) => true,
        Err(refusal) => {
            tracing::info!(
                target = "orchestrator",
                ?refusal,
                "workspace update download not started"
            );
            false
        }
    }
}

fn in_sync_source_line(asset: &Step2UpdateAsset) -> String {
    format!("{} ({})", asset.label, asset.tag)
}

pub(crate) fn promote_in_sync_assets(step2: &mut Step2State, tp2: &str) -> Vec<Step2UpdateAsset> {
    let matches_tp2 =
        |asset: &Step2UpdateAsset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file) == tp2;
    if step2.update_selected_update_assets.iter().any(matches_tp2) {
        return Vec::new();
    }
    let (promoted, kept): (Vec<_>, Vec<_>) =
        std::mem::take(&mut step2.update_selected_in_sync_assets)
            .into_iter()
            .partition(matches_tp2);
    step2.update_selected_in_sync_assets = kept;
    for asset in &promoted {
        step2
            .update_selected_update_sources
            .push(in_sync_source_line(asset));
        step2.update_selected_update_assets.push(asset.clone());
    }
    promoted
}

fn demote_in_sync_assets(step2: &mut Step2State, promoted: &[Step2UpdateAsset]) {
    for asset in promoted {
        step2
            .update_selected_update_assets
            .retain(|candidate| candidate != asset);
        let line = in_sync_source_line(asset);
        if let Some(index) = step2
            .update_selected_update_sources
            .iter()
            .position(|source| *source == line)
        {
            step2.update_selected_update_sources.remove(index);
        }
        step2.update_selected_in_sync_assets.push(asset.clone());
    }
}

fn handle_step2_download_source_action(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    action: Step2Action,
) {
    match action {
        Step2Action::DiscoverModDownloadForks { tp2, label, repo } => {
            if crate::app::state::update_pipeline_busy(&state.step2) {
                state.step2.scan_status = "Wait for the current check to finish".to_string();
            } else {
                discover_mod_download_forks(state, tp2, label, &repo);
            }
        }
        Step2Action::OpenModDownloadsUserSource => open_mod_downloads_user_source(state),
        Step2Action::ReloadModDownloadSources => reload_mod_download_sources(state),
        Step2Action::SaveSourceForm => {
            if crate::app::state::update_pipeline_busy(&state.step2) {
                state.step2.scan_status = "Wait for the current check to finish".to_string();
            } else {
                save_source_form(state, step2_update_check_rx);
            }
        }
        Step2Action::RequestReleaseList { repo } => request_release_list(state, &repo),
        Step2Action::SetModDownloadSource { tp2, source_id } => {
            if crate::app::state::update_pipeline_busy(&state.step2) {
                state.step2.scan_status = "Wait for the current check to finish".to_string();
            } else {
                set_mod_download_source(state, step2_update_check_rx, &tp2, &source_id);
            }
        }
        Step2Action::UseKnownSource {
            tp2,
            card_key,
            block,
            save_to,
            who,
        } => use_known_source(
            state,
            step2_update_check_rx,
            &tp2,
            &card_key,
            &block,
            save_to,
            &who,
        ),
        Step2Action::SaveSourceNote {
            tp2,
            signature,
            text,
            who,
        } => save_source_note(state, &tp2, &signature, &text, &who),
        Step2Action::BookmarkOnDisk { tp2, card_key } => bookmark_on_disk(state, &tp2, &card_key),
        _ => {}
    }
}

fn preview_update_target_mod(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    target: Option<(String, String)>,
) {
    if let Some(target) = target {
        let loaded = mod_downloads::load_mod_download_sources();
        super::app_step2_update_preview::preview_update_selected_mod(
            state,
            step2_update_check_rx,
            &loaded,
            target,
        );
    }
}

fn open_update_popup(state: &mut WizardState) {
    state.step2.update_selected_target_game_tab = None;
    state.step2.update_selected_target_tp_file = None;
    state.step2.update_selected_refresh_target_game_tab = None;
    state.step2.update_selected_refresh_target_tp_file = None;
    let auto_check_pending = !state.step2.update_selected_has_run
        || crate::app::state::update_selection_stale(&state.step2);
    state.step2.versions_ui = crate::app::state::VersionsDrawerUi::default();
    state.step2.versions_ui.auto_check_pending = auto_check_pending;
    state.step2.update_selected_popup_open = true;
}

fn open_drawer_focused_on_selected_mod(state: &mut WizardState) -> Option<(String, String)> {
    let target = super::app_step2_update_preview::selected_mod_target(state)?;
    open_update_popup(state);
    state.step2.versions_ui.auto_check_pending = false;
    state
        .step2
        .versions_ui
        .focus_card(mod_downloads::normalize_mod_download_tp2(&target.1));
    Some(target)
}

fn accept_latest_for_exact_version_misses(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
) {
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
                release: None,
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
    state.step2.scan_status = format!("Checking latest fallback sources: {}", requests.len());
    super::app_step2_update_check::start_step2_update_check(state, step2_update_check_rx, requests);
}

fn open_selected_path(state: &mut WizardState, path: &str) {
    if let Err(err) = open_in_shell(path) {
        state.step2.scan_status = format!("Open failed: {err}");
    }
}

fn discover_mod_download_forks(state: &mut WizardState, tp2: String, label: String, repo: &str) {
    state.step2.mod_download_forks_popup_open = true;
    state.step2.mod_download_forks_popup_title = format!("Forks for {label}");
    state.step2.mod_download_forks_popup_tp2 = tp2;
    state.step2.mod_download_forks_popup_label = label;
    state.step2.mod_download_forks.clear();
    state.step2.mod_download_forks_popup_error = None;
    state.step2.forks_list = super::github_forks_list::ForksListState {
        repo: repo.to_string(),
        status: super::github_forks_list::ForksStatus::Loading,
    };
}

fn open_mod_downloads_user_source(state: &mut WizardState) {
    if let Err(err) = mod_downloads::ensure_mod_downloads_files() {
        state.step2.scan_status = format!("Open failed: {err}");
        return;
    }
    let path = mod_downloads::mod_downloads_user_path();
    if let Err(err) = open_in_shell(path.to_string_lossy().as_ref()) {
        state.step2.scan_status = format!("Open failed: {err}");
    }
}

fn reload_mod_download_sources(state: &mut WizardState) {
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

fn save_source_form(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
) {
    let Some(mut form) = state.step2.versions_ui.source_form.clone() else {
        return;
    };
    let adding_new_mod = is_new_mod_form(&form);
    if adding_new_mod && !apply_new_mod_tp2(&mut form) {
        if let Some(open_form) = state.step2.versions_ui.source_form.as_mut() {
            open_form.error = Some("Enter the mod's TP2 name first".to_string());
        }
        return;
    }
    if adding_new_mod && !route_new_mod_to_this_modlist(state, &mut form) {
        return;
    }
    let full_text = super::source_form::source_form_save_text(&form);
    let target_path = match form.save_to {
        ModSourceEditDestination::GlobalDefault => None,
        ModSourceEditDestination::ThisModlist => mod_downloads::active_modlist_downloads_path(),
    };
    let card_key = if adding_new_mod {
        mod_downloads::normalize_mod_download_tp2(&form.tp2)
    } else {
        form.card_key.clone()
    };
    let new_source =
        mod_source_history::normalize_source(&form.tp2, &super::source_form::to_source(&form));
    let request = SourceSaveRequest {
        tp2: &form.tp2,
        name: &form.label,
        source_id: &form.source_id,
        allow_id_change: form.identity.may_change_id,
        text: &full_text,
        target_path: target_path.as_deref(),
        new_source: Some(&new_source),
    };
    let saved_to = history_saved_to(form.save_to, &form.note_who);

    match save_source_block_recording_history(state, &request, form.save_to, saved_to, None) {
        Ok(()) => {
            state.step2.versions_ui.sheet = None;
            state.step2.versions_ui.source_form = None;
            save_form_note(state, &form, &new_source);
            let added_list_error = if adding_new_mod {
                push_new_mod_pending_download(state, &form).err()
            } else {
                None
            };
            finish_saving_mod_download_source_editor(state, step2_update_check_rx, &card_key);
            if let Some(err) = added_list_error {
                state.step2.scan_status = format!(
                    "Saved source entry for {card_key}, but the added-mods list was not updated: {err}"
                );
            }
        }
        Err(err) => {
            if let Some(form) = state.step2.versions_ui.source_form.as_mut() {
                form.error = Some(err.clone());
            }
            state.step2.scan_status = format!("Save source entry failed: {err}");
        }
    }
}

pub(crate) const NEW_MOD_CARD_KEY: &str = "new-mod";

pub(crate) fn is_new_mod_form(form: &super::source_form::SourceForm) -> bool {
    form.identity.is_new_mod && form.card_key == NEW_MOD_CARD_KEY
}

fn strip_suffix_ignore_case<'a>(value: &'a str, suffix: &str) -> &'a str {
    value
        .len()
        .checked_sub(suffix.len())
        .and_then(|at| {
            value
                .get(at..)
                .filter(|tail| tail.eq_ignore_ascii_case(suffix))
                .and_then(|_| value.get(..at))
        })
        .unwrap_or(value)
}

fn strip_prefix_ignore_case<'a>(value: &'a str, prefix: &str) -> &'a str {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .and_then(|_| value.get(prefix.len()..))
        .unwrap_or(value)
}

fn clean_new_mod_tp2(raw: &str) -> Option<String> {
    let without_ext = strip_suffix_ignore_case(raw.trim(), ".tp2");
    let name = strip_prefix_ignore_case(without_ext, "setup-");
    let unsafe_char = |c: char| {
        c.is_whitespace() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    };
    (!name.is_empty() && !name.chars().any(unsafe_char)).then(|| name.to_string())
}

fn apply_new_mod_tp2(form: &mut super::source_form::SourceForm) -> bool {
    let Some(tp2) = clean_new_mod_tp2(&form.tp2) else {
        return false;
    };
    if form.label.trim().is_empty() {
        form.label.clone_from(&tp2);
    }
    form.tp2 = tp2;
    true
}

fn route_new_mod_to_this_modlist(
    state: &mut WizardState,
    form: &mut super::source_form::SourceForm,
) -> bool {
    if mod_downloads::active_modlist_downloads_path().is_none() {
        if let Some(open_form) = state.step2.versions_ui.source_form.as_mut() {
            open_form.error = Some("Open a modlist first".to_string());
        }
        return false;
    }
    form.save_to = ModSourceEditDestination::ThisModlist;
    true
}

fn push_new_mod_pending_download(
    state: &mut WizardState,
    form: &super::source_form::SourceForm,
) -> Result<(), String> {
    let key = mod_downloads::normalize_mod_download_tp2(&form.tp2);
    let step2 = &mut state.step2;
    let scanned = step2
        .bgee_mods
        .iter()
        .chain(step2.bg2ee_mods.iter())
        .any(|mod_state| mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file) == key);
    if scanned {
        return Ok(());
    }
    let recorded = super::added_mods::record_added_mod(&key);
    let already_pending = step2
        .log_pending_downloads
        .iter()
        .any(|pending| mod_downloads::normalize_mod_download_tp2(&pending.tp_file) == key);
    if already_pending {
        return recorded;
    }
    let label = if form.name.trim().is_empty() {
        form.tp2.clone()
    } else {
        form.name.trim().to_string()
    };
    step2
        .log_pending_downloads
        .push(crate::app::state::Step2LogPendingDownload {
            game_tab: step2.active_game_tab.clone(),
            tp_file: format!("{}.tp2", form.tp2),
            label,
            requested_version: None,
        });
    recorded
}

fn save_form_note(
    state: &mut WizardState,
    form: &super::source_form::SourceForm,
    new_source: &mod_downloads::ModDownloadSource,
) {
    if form.note == form.note_seed {
        return;
    }
    let signature = mod_source_history::rule_signature(new_source);
    let key = mod_source_history::note_key(&form.tp2, &signature);
    let who = if form.note_who.trim().is_empty() {
        destination_label(form.save_to)
    } else {
        form.note_who.as_str()
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match mod_source_history::with_writable_store(|store| {
        mod_source_history::set_note(store, &key, &form.note, who, &today);
    }) {
        Ok(()) => state.step2.versions_ui.known = None,
        Err(()) => {
            state.step2.scan_status = "Source history file is unreadable; not saved".to_string();
        }
    }
}

const fn destination_label(destination: ModSourceEditDestination) -> &'static str {
    match destination {
        ModSourceEditDestination::GlobalDefault => "My default",
        ModSourceEditDestination::ThisModlist => "This modlist",
    }
}

fn read_replaced_user_source(
    tp2: &str,
    source_id: &str,
    destination: ModSourceEditDestination,
) -> Option<mod_downloads::ModDownloadSource> {
    let path = match destination {
        ModSourceEditDestination::GlobalDefault => Some(mod_downloads::mod_downloads_user_path()),
        ModSourceEditDestination::ThisModlist => mod_downloads::active_modlist_downloads_path(),
    };
    let path = path?;
    let old_text = std::fs::read_to_string(&path).ok()?;
    let loaded = mod_downloads::load_mod_download_sources_from_texts("", &old_text, "");
    let target_key = mod_downloads::normalize_source_id(source_id);
    loaded
        .find_sources(tp2)
        .into_iter()
        .find(|source| mod_downloads::normalize_source_id(&source.source_id) == target_key)
}

const fn history_saved_to(save_to: ModSourceEditDestination, who: &str) -> &str {
    match save_to {
        ModSourceEditDestination::ThisModlist if !who.trim_ascii().is_empty() => who,
        other => destination_label(other),
    }
}

struct SourceSaveRequest<'a> {
    tp2: &'a str,
    name: &'a str,
    source_id: &'a str,
    allow_id_change: bool,
    text: &'a str,
    target_path: Option<&'a Path>,
    new_source: Option<&'a mod_downloads::ModDownloadSource>,
}

fn save_source_block_recording_history(
    state: &mut WizardState,
    request: &SourceSaveRequest<'_>,
    destination: ModSourceEditDestination,
    saved_to: &str,
    replaced: Option<&mod_downloads::ModDownloadSource>,
) -> Result<(), String> {
    let old_source = replaced
        .cloned()
        .or_else(|| read_replaced_user_source(request.tp2, request.source_id, destination));
    mod_downloads::save_user_mod_download_source_block(
        request.tp2,
        request.name,
        request.source_id,
        request.allow_id_change,
        request.text,
        request.target_path,
    )?;
    let new_source = request
        .new_source
        .cloned()
        .or_else(|| read_replaced_user_source(request.tp2, request.source_id, destination));
    if let Some(new_source) = new_source {
        record_history_if_changed(
            state,
            request.tp2,
            old_source.as_ref(),
            &new_source,
            saved_to,
        );
    }
    state.step2.versions_ui.known = None;
    Ok(())
}

fn record_history_if_changed(
    state: &mut WizardState,
    tp2: &str,
    old_source: Option<&mod_downloads::ModDownloadSource>,
    new_source: &mod_downloads::ModDownloadSource,
    saved_to: &str,
) {
    let Some(old_source) = old_source else {
        return;
    };
    if mod_source_history::rule_signature(old_source)
        == mod_source_history::rule_signature(new_source)
    {
        return;
    }
    let block = mod_downloads::complete_source_block(old_source);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match mod_source_history::with_writable_store(|store| {
        mod_source_history::record_history(store, tp2, block, saved_to, &today);
    }) {
        Ok(()) => state.step2.versions_ui.known = None,
        Err(()) => {
            state.step2.scan_status = "Source history file is unreadable; not saved".to_string();
        }
    }
}

fn use_known_source(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    tp2: &str,
    card_key: &str,
    block: &str,
    save_to: ModSourceEditDestination,
    who: &str,
) {
    if crate::app::state::update_pipeline_busy(&state.step2) {
        state.step2.scan_status = "Wait for the current check to finish".to_string();
        return;
    }
    let loaded = mod_downloads::load_mod_download_sources();
    let selected_source_id = state.step2.selected_source_ids.get(card_key).cloned();
    let Some(current) = loaded.resolve_source(tp2, selected_source_id.as_deref()) else {
        state.step2.scan_status =
            "Save source entry failed: no current source for this mod".to_string();
        return;
    };
    let Some(parsed) = mod_source_history::source_from_block(tp2, block) else {
        state.step2.scan_status =
            "Save source entry failed: could not read the known source".to_string();
        return;
    };

    let mut new_source = current.clone();
    new_source.github = parsed.github;
    new_source.url = parsed.url;
    new_source.kind = parsed.kind;
    new_source.commit = parsed.commit;
    new_source.tag = parsed.tag;
    new_source.branch = parsed.branch;
    new_source.release = parsed.release;
    new_source.channel = parsed.channel;
    new_source.asset = parsed.asset;
    new_source.pkg_windows = parsed.pkg_windows;
    new_source.pkg_linux = parsed.pkg_linux;
    new_source.pkg_macos = parsed.pkg_macos;

    let full_text = format!(
        "{}\n\n{}\n",
        mod_downloads::template_mod_header(&current.tp2, &current.name),
        mod_downloads::complete_source_block(&new_source)
    );
    let target_path = match save_to {
        ModSourceEditDestination::GlobalDefault => None,
        ModSourceEditDestination::ThisModlist => mod_downloads::active_modlist_downloads_path(),
    };

    let normalized_new = mod_source_history::normalize_source(&current.tp2, &new_source);
    let request = SourceSaveRequest {
        tp2: &current.tp2,
        name: &current.name,
        source_id: &current.source_id,
        allow_id_change: false,
        text: &full_text,
        target_path: target_path.as_deref(),
        new_source: Some(&normalized_new),
    };

    match save_source_block_recording_history(
        state,
        &request,
        save_to,
        history_saved_to(save_to, who),
        Some(&current),
    ) {
        Ok(()) => {
            let toast = format!(
                "{} \u{b7} {} \u{b7} saved to {}",
                current.name,
                super::versions_view::rule_words(&normalized_new),
                destination_label(save_to)
            );
            state.step2.versions_ui.pending_toast = Some(toast);
            finish_saving_mod_download_source_editor(state, step2_update_check_rx, card_key);
        }
        Err(err) => {
            state.step2.scan_status = format!("Save source entry failed: {err}");
        }
    }
}

fn save_source_note(state: &mut WizardState, tp2: &str, signature: &str, text: &str, who: &str) {
    let key = mod_source_history::note_key(tp2, signature);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match mod_source_history::with_writable_store(|store| {
        mod_source_history::set_note(store, &key, text, who, &today);
    }) {
        Ok(()) => {
            state.step2.versions_ui.known = None;
            state.step2.versions_ui.sheet = None;
            state.step2.versions_ui.sheet_tp2 = None;
            state.step2.versions_ui.sheet_error = None;
            state.step2.versions_ui.pending_toast = Some("Note saved".to_string());
        }
        Err(()) => {
            state.step2.versions_ui.sheet_error =
                Some("Source history file is unreadable; not saved".to_string());
        }
    }
}

fn bookmark_on_disk(state: &mut WizardState, tp2: &str, card_key: &str) {
    if crate::app::state::update_pipeline_busy(&state.step2) {
        state.step2.scan_status = "Wait for the current check to finish".to_string();
        return;
    }
    let loaded = mod_downloads::load_mod_download_sources();
    let selected_source_id = state.step2.selected_source_ids.get(card_key).cloned();
    let Some(current) = loaded.resolve_source(tp2, selected_source_id.as_deref()) else {
        state.step2.scan_status = "Nothing to bookmark".to_string();
        return;
    };
    if current.github.is_none() {
        state.step2.scan_status = "Nothing to bookmark".to_string();
        return;
    }
    let installed = super::app_step2_update_source_refs::InstalledRefLookup::load(
        state.step1.mods_folder.trim(),
    )
    .source_id_and_ref(tp2);
    let Some((bookmarked_source, label)) = mod_source_history::bookmark_block(
        &current,
        installed.as_ref().map(|(source_id, _)| source_id.as_str()),
        installed
            .as_ref()
            .map(|(_, source_ref)| source_ref.as_str()),
    ) else {
        state.step2.scan_status = "Nothing to bookmark".to_string();
        return;
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match mod_source_history::with_writable_store(|store| {
        mod_source_history::add_bookmark(store, &current.tp2, &bookmarked_source, &label, &today);
    }) {
        Ok(()) => {
            state.step2.versions_ui.known = None;
            state.step2.versions_ui.pending_toast = Some(format!("Bookmarked {label}"));
            state.step2.scan_status = format!("Bookmarked {label}");
        }
        Err(()) => {
            state.step2.scan_status = "Source history file is unreadable; not saved".to_string();
        }
    }
}

fn request_release_list(state: &mut WizardState, repo: &str) {
    let repo = repo.trim();
    if repo.is_empty() || state.github_auth_login.trim().is_empty() {
        return;
    }
    state.step2.versions_ui.release_list = super::github_release_list::ReleaseListState {
        repo: repo.to_string(),
        status: super::github_release_list::ReleaseListStatus::Loading,
    };
}

fn finish_saving_mod_download_source_editor(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    tp2: &str,
) {
    state.step2.mod_download_source_editor_open = false;
    state.step2.mod_download_source_editor_error = None;
    let loaded = mod_downloads::load_mod_download_sources();
    if let Some(err) = loaded.error.as_ref() {
        invalidate_update_selected_results(state);
        state.step2.scan_status = format!("Source saved but reload failed: {err}");
    } else if state.step2.update_selected_has_run
        && refresh_update_result_for_tp2(state, step2_update_check_rx, &loaded, tp2)
    {
        state.step2.scan_status = format!("Saved source entry for {tp2}; refreshing update result");
    } else {
        invalidate_update_selected_results(state);
        state.step2.scan_status = format!("Saved source entry for {tp2}");
    }
}

fn open_compat_for_component(
    state: &mut WizardState,
    game_tab: String,
    tp_file: String,
    component_id: String,
    component_key: String,
) {
    state.step2.selected = Some(Step2Selection::Component {
        game_tab,
        tp_file,
        component_id,
        component_key,
    });
    state.step2.compat_popup_issue_override = None;
    state.step2.compat_popup_open = true;
}

fn set_selected_mod_update_locked(state: &mut WizardState, locked: bool) {
    let Some(Step2Selection::Mod { game_tab, tp_file }) = state.step2.selected.clone() else {
        return;
    };
    let had_cached_update_entry = !locked && popup_has_cached_update_entry(state, &tp_file);

    let mod_name: String;
    let update_entry: Option<String>;
    {
        let selected_mods = if game_authority::slot_for_tab(&game_tab) == GameSlot::First {
            &mut state.step2.bgee_mods
        } else {
            &mut state.step2.bg2ee_mods
        };
        let Some(selected_mod) = selected_mods.iter_mut().find(|m| m.tp_file == tp_file) else {
            return;
        };
        if let Err(err) =
            super::mod_update_locks::set_mod_update_lock(&selected_mod.tp_file, locked)
        {
            state.step2.scan_status = format!("Update lock failed: {err}");
            return;
        }
        mod_name = selected_mod.name.clone();
        update_entry = mod_update_entry_text(selected_mod);
    }

    let tp2_key = mod_downloads::normalize_mod_download_tp2(&tp_file);
    sync_update_locked_by_tp2(state, &tp2_key, locked, had_cached_update_entry);
    sync_cached_popup_update_lock(state, &game_tab, &tp_file, update_entry.as_deref(), locked);
    let verb = if locked { "Locked" } else { "Unlocked" };
    state.step2.scan_status = format!("{verb} updates for {mod_name}");
}

fn set_mod_update_locked(state: &mut WizardState, tp2: &str, locked: bool) {
    if let Err(err) = super::mod_update_locks::set_mod_update_lock(tp2, locked) {
        state.step2.scan_status = format!("Update lock failed: {err}");
        return;
    }
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp2);
    let had_cached = !locked
        && state
            .step2
            .bgee_mods
            .iter()
            .chain(state.step2.bg2ee_mods.iter())
            .any(|m| {
                mod_downloads::normalize_mod_download_tp2(&m.tp_file) == tp2_key
                    && popup_has_cached_update_entry(state, &m.tp_file)
            });
    let (canonical_tp_file, mod_name) = state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .find(|m| mod_downloads::normalize_mod_download_tp2(&m.tp_file) == tp2_key)
        .map_or_else(
            || (tp2.to_string(), tp2.to_string()),
            |m| (m.tp_file.clone(), m.name.clone()),
        );
    sync_update_locked_by_tp2(state, &tp2_key, locked, had_cached);
    let update_entry = state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .find(|m| mod_downloads::normalize_mod_download_tp2(&m.tp_file) == tp2_key)
        .and_then(mod_update_entry_text);
    let first_tab = game_authority::first_slot_tab(&state.step1.game_install);
    sync_cached_popup_update_lock(
        state,
        first_tab,
        &canonical_tp_file,
        update_entry.as_deref(),
        locked,
    );
    let verb = if locked { "Locked" } else { "Unlocked" };
    let label = if mod_name.trim().is_empty() {
        &canonical_tp_file
    } else {
        &mod_name
    };
    state.step2.scan_status = format!("{verb} updates for {label}");
}

fn sync_update_locked_by_tp2(
    state: &mut WizardState,
    tp2_key: &str,
    locked: bool,
    had_cached_update_entry: bool,
) {
    for mod_state in state
        .step2
        .bgee_mods
        .iter_mut()
        .chain(state.step2.bg2ee_mods.iter_mut())
    {
        if mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file) != tp2_key {
            continue;
        }
        mod_state.update_locked = locked;
        if locked {
            mod_state.package_marker = None;
        } else if had_cached_update_entry {
            mod_state.package_marker = Some('+');
        }
    }
}

fn set_mod_download_source(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    tp2: &str,
    source_id: &str,
) {
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
    let label = selected_source_label(state, &tp2_key).unwrap_or(tp2_key);
    let loaded = mod_downloads::load_mod_download_sources();
    if state.step2.update_selected_has_run
        && refresh_update_result_for_tp2(state, step2_update_check_rx, &loaded, tp2)
    {
        state.step2.scan_status = format!("Source changed for {label}; refreshing update result");
    } else {
        invalidate_update_selected_results(state);
        state.step2.scan_status = format!("Source changed for {label}. Run Check Updates again.");
    }
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
    state.step2.update_selected_in_sync_assets.clear();
    state.step2.update_selected_missing_sources.clear();
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.update_selected_known_sources.clear();
    state.step2.update_selected_manual_sources.clear();
    state.step2.update_selected_manual_downloads.clear();
    state.step2.skipped_manual_downloads.clear();
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
    state.step2.update_selected_refresh_target_game_tab = None;
    state.step2.update_selected_refresh_target_tp_file = None;
    state.step2.exact_log_mod_list_checked = false;
}

fn refresh_update_result_for_tp2(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &mod_downloads::ModDownloadsLoad,
    tp2: &str,
) -> bool {
    let Some((game_tab, tp_file)) = update_target_for_tp2(state, tp2) else {
        return false;
    };
    state.step2.update_selected_refresh_target_game_tab = Some(game_tab.clone());
    state.step2.update_selected_refresh_target_tp_file = Some(tp_file.clone());
    let previous_target_game_tab = state.step2.update_selected_target_game_tab.clone();
    let previous_target_tp_file = state.step2.update_selected_target_tp_file.clone();
    super::app_step2_update_preview::preview_update_selected_mod(
        state,
        step2_update_check_rx,
        sources,
        (game_tab, tp_file),
    );
    state.step2.update_selected_target_game_tab = previous_target_game_tab;
    state.step2.update_selected_target_tp_file = previous_target_tp_file;
    true
}

fn update_target_for_tp2(state: &WizardState, tp2: &str) -> Option<(String, String)> {
    let target = mod_downloads::normalize_mod_download_tp2(tp2);
    if target.is_empty() {
        return None;
    }
    let first_tab = game_authority::first_slot_tab(&state.step1.game_install);
    for (game_tab, mods) in [
        (first_tab, state.step2.bgee_mods.as_slice()),
        ("BG2EE", state.step2.bg2ee_mods.as_slice()),
    ] {
        for mod_state in mods {
            if mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file) == target {
                return Some((game_tab.to_string(), mod_state.tp_file.clone()));
            }
        }
    }
    for pending in &state.step2.log_pending_downloads {
        if mod_downloads::normalize_mod_download_tp2(&pending.tp_file) == target {
            return Some((pending.game_tab.clone(), pending.tp_file.clone()));
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::mod_downloads::AMBIENT_TEST_LOCK;
    use crate::app::state::{Step2ModState, Step2Selection, WizardState};

    struct AmbientGuard(Option<PathBuf>);

    impl AmbientGuard {
        fn acquire() -> Self {
            Self(crate::app::mod_downloads::active_modlist_dir())
        }
    }

    impl Drop for AmbientGuard {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    fn unique_tmp_dir(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_router_test_{}_{}_{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn make_mod_state(tp_file: &str) -> Step2ModState {
        Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("{tp_file}.tp2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: Vec::new(),
        }
    }

    #[test]
    fn eet_dual_tab_lock_sync() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("eet_lock");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        crate::app::mod_downloads::set_active_modlist_dir(Some(tmp_dir.clone()));

        let tp_file = "cdtweaks/cdtweaks.tp2";
        let mut state = WizardState::default();
        state.step2.bgee_mods.push(make_mod_state(tp_file));
        state.step2.bg2ee_mods.push(make_mod_state(tp_file));
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
        });

        super::set_selected_mod_update_locked(&mut state, true);
        assert!(
            state.step2.bgee_mods[0].update_locked,
            "bgee instance must be locked"
        );
        assert!(
            state.step2.bg2ee_mods[0].update_locked,
            "bg2ee instance must be locked after locking via bgee tab"
        );

        super::set_selected_mod_update_locked(&mut state, false);
        assert!(
            !state.step2.bgee_mods[0].update_locked,
            "bgee instance must be unlocked"
        );
        assert!(
            !state.step2.bg2ee_mods[0].update_locked,
            "bg2ee instance must be unlocked after unlocking via bgee tab"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn update_target_misses_an_alias_key() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(make_mod_state("setup-bg1npcmusic.tp2"));

        assert!(super::update_target_for_tp2(&state, "bg1npcmusic").is_some());
        assert!(super::update_target_for_tp2(&state, "BG1NPC").is_none());
    }

    struct SourceFormConfigDirGuard(PathBuf);

    impl SourceFormConfigDirGuard {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_router_source_form_config_dir_test_{}_{}_{label}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }

    impl Drop for SourceFormConfigDirGuard {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct TargetDirGuard(PathBuf);

    impl TargetDirGuard {
        fn create(label: &str) -> Self {
            let path = unique_tmp_dir(label);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TargetDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn first_save_of_an_existing_mod_keeps_its_name() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("first_save_keeps_name");
        let target_guard = TargetDirGuard::create("first_save_target");
        let target_dir = target_guard.0.clone();
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_dir.clone()));

        let source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS Mod".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("owner/scs".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let form = crate::app::source_form::from_source(
            &source,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: false,
                is_new_mod: false,
            },
            crate::app::step2_action::ModSourceEditDestination::ThisModlist,
            "stratagems",
        );

        let mut state = WizardState::default();
        state.step2.versions_ui.source_form = Some(form);
        let mut step2_update_check_rx = None;

        super::save_source_form(&mut state, &mut step2_update_check_rx);

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "save must not fail: {}",
            state.step2.scan_status
        );

        let written = std::fs::read_to_string(target_dir.join("mod_downloads_user.toml")).unwrap();
        assert!(
            written.contains("name = \"SCS Mod\""),
            "written file must keep the mod's own name:\n{written}"
        );
        assert!(
            !written.contains("name = \"Gibberlings3\""),
            "written file must not use the source label as the mod name:\n{written}"
        );
    }

    #[test]
    fn open_update_popup_arms_auto_check_only_when_stale() {
        let mut state = WizardState::default();

        super::open_update_popup(&mut state);
        assert!(state.step2.versions_ui.auto_check_pending);

        state.step2.update_selected_has_run = true;
        state.step2.update_selected_last_was_full_selection = true;
        state.step2.update_selected_last_selection_signature =
            Some(crate::app::state::update_selection_signature(&state.step2));

        super::open_update_popup(&mut state);
        assert!(!state.step2.versions_ui.auto_check_pending);

        state
            .step2
            .selected_source_ids
            .insert("mod".to_string(), "primary".to_string());

        super::open_update_popup(&mut state);
        assert!(state.step2.versions_ui.auto_check_pending);
    }

    #[test]
    fn check_this_mod_focuses_the_selected_mods_card() {
        use crate::app::state::VersionsChip;

        let mut state = WizardState::default();
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: "BGEE".to_string(),
            tp_file: "Setup-Ascension.tp2".to_string(),
        });
        state.step2.versions_ui.search = "zzz".to_string();
        state.step2.versions_ui.chip = VersionsChip::Fetch;
        state.step2.update_selected_has_run = false;

        let target = super::open_drawer_focused_on_selected_mod(&mut state);

        assert_eq!(
            target,
            Some(("BGEE".to_string(), "Setup-Ascension.tp2".to_string()))
        );
        assert_eq!(
            state.step2.versions_ui.focused_tp2.as_deref(),
            Some("ascension")
        );
        assert!(state.step2.versions_ui.focus_scroll_pending);
        assert!(!state.step2.versions_ui.auto_check_pending);
        assert_eq!(state.step2.versions_ui.chip, VersionsChip::All);
        assert!(state.step2.versions_ui.search.is_empty());
        assert!(state.step2.update_selected_popup_open);
    }

    #[test]
    fn check_this_mod_with_no_selection_changes_nothing() {
        let mut state = WizardState::default();
        state.step2.selected = None;
        state.step2.update_selected_popup_open = false;

        let target = super::open_drawer_focused_on_selected_mod(&mut state);

        assert_eq!(target, None);
        assert!(!state.step2.update_selected_popup_open);
        assert_eq!(state.step2.versions_ui.focused_tp2, None);
    }

    #[test]
    fn source_switch_is_refused_while_a_check_runs() {
        use crate::app::step2_action::Step2Action;

        let mut state = WizardState::default();
        state
            .step2
            .selected_source_ids
            .insert("mod".to_string(), "old".to_string());
        state.step2.update_selected_check_running = true;
        let mut rx = None;

        super::handle_step2_download_source_action(
            &mut state,
            &mut rx,
            Step2Action::SetModDownloadSource {
                tp2: "mod".to_string(),
                source_id: "new".to_string(),
            },
        );

        assert_eq!(
            state
                .step2
                .selected_source_ids
                .get("mod")
                .map(String::as_str),
            Some("old")
        );
        assert_eq!(
            state.step2.scan_status,
            "Wait for the current check to finish"
        );
    }

    #[test]
    fn accept_latest_is_refused_while_a_check_runs() {
        use crate::app::step2_action::Step2Action;

        let mut state = WizardState::default();
        state
            .step2
            .update_selected_exact_version_retry_requests
            .push(crate::app::state::Step2UpdateRetryRequest {
                game_tab: "BGEE".to_string(),
                tp_file: "setup-buffbot.tp2".to_string(),
                label: "buffbot".to_string(),
                source_id: "primary".to_string(),
                repo: String::new(),
                source_url: String::new(),
                channel: None,
                tag: None,
                commit: None,
                branch: None,
                asset: None,
                pkg: None,
            });
        state.step2.update_selected_check_running = true;
        let mut rx = None;

        super::handle_step2_action(
            &mut state,
            &mut None,
            &mut None,
            &mut std::collections::VecDeque::new(),
            &mut rx,
            &mut None,
            Step2Action::AcceptLatestForExactVersionMisses,
        );

        assert_eq!(
            state
                .step2
                .update_selected_exact_version_retry_requests
                .len(),
            1
        );
        assert_eq!(
            state.step2.scan_status,
            "Wait for the current check to finish"
        );
    }

    #[test]
    fn use_known_source_is_refused_while_a_check_runs() {
        let mut state = WizardState::default();
        state.step2.update_selected_check_running = true;
        let mut rx = None;

        super::use_known_source(
            &mut state,
            &mut rx,
            "mod",
            "mod",
            "block",
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "My default",
        );

        assert_eq!(
            state.step2.scan_status,
            "Wait for the current check to finish"
        );
    }

    #[test]
    fn use_known_source_keeps_identity() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("use_known_source_keeps_identity");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let current = crate::app::mod_downloads::ModDownloadSource {
            tp2: "widget".to_string(),
            name: "Widget".to_string(),
            source_id: "primary".to_string(),
            source_label: "Primary".to_string(),
            github: Some("owner/widget".to_string()),
            tag: Some("v1.0".to_string()),
            source_default: true,
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("widget", "Widget");
        let block = crate::app::mod_downloads::complete_source_block(&current);
        std::fs::write(
            crate::app::mod_downloads::mod_downloads_user_path(),
            format!("{header}\n\n{block}\n"),
        )
        .unwrap();

        let mut other = current;
        other.github = Some("owner/widget-fork".to_string());
        other.tag = None;
        other.branch = Some("dev".to_string());
        let known_block = crate::app::mod_downloads::complete_source_block(&other);

        let mut state = WizardState::default();
        let mut rx = None;

        super::use_known_source(
            &mut state,
            &mut rx,
            "widget",
            "widget",
            &known_block,
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "My default",
        );

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "{}",
            state.step2.scan_status
        );
        let written =
            std::fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path()).unwrap();
        assert!(written.contains("id = \"primary\""));
        assert!(written.contains("label = \"Primary\""));
        assert!(written.contains("branch = \"dev\""));
        assert!(written.contains("owner/widget-fork"));
    }

    #[test]
    fn use_known_source_keeps_non_rule_fields() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("use_known_source_keeps_non_rule");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let current = crate::app::mod_downloads::ModDownloadSource {
            tp2: "widget".to_string(),
            name: "Widget".to_string(),
            source_id: "primary".to_string(),
            source_label: "Primary".to_string(),
            github: Some("owner/widget".to_string()),
            tag: Some("v1.0".to_string()),
            source_default: true,
            aliases: vec!["widget-alt".to_string()],
            subdir_require: Some("core".to_string()),
            config_files: vec!["config.ini".to_string()],
            exact_github: vec!["owner/exact".to_string()],
            tp2_rename: Some(crate::app::mod_downloads::ModDownloadTp2Rename {
                from: "old.tp2".to_string(),
                to: "new.tp2".to_string(),
            }),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("widget", "Widget");
        let block = crate::app::mod_downloads::complete_source_block(&current);
        std::fs::write(
            crate::app::mod_downloads::mod_downloads_user_path(),
            format!("{header}\n\n{block}\n"),
        )
        .unwrap();

        let fork = crate::app::mod_downloads::ModDownloadSource {
            tp2: "widget".to_string(),
            source_id: "someone".to_string(),
            source_label: "Someone".to_string(),
            github: Some("someone/widget-fork".to_string()),
            branch: Some("dev".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let known_block = crate::app::mod_downloads::complete_source_block(&fork);

        let mut state = WizardState::default();
        let mut rx = None;

        super::use_known_source(
            &mut state,
            &mut rx,
            "widget",
            "widget",
            &known_block,
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "My default",
        );

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "{}",
            state.step2.scan_status
        );
        let written =
            std::fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path()).unwrap();
        assert!(written.contains("someone/widget-fork"));
        assert!(written.contains("branch = \"dev\""));
        assert!(
            written.contains("widget-alt"),
            "aliases must survive: {written}"
        );
        assert!(
            written.contains("subdir_require = \"core\""),
            "subdir_require must survive: {written}"
        );
        assert!(
            written.contains("config.ini"),
            "config_files must survive: {written}"
        );
        assert!(
            written.contains("owner/exact"),
            "exact_github must survive: {written}"
        );
        assert!(
            written.contains("old.tp2") && written.contains("new.tp2"),
            "tp2_rename must survive: {written}"
        );
    }

    #[test]
    fn record_history_if_changed_skips_missing_user_file_and_records_on_change() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("record_replaced_skip_default");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let missing = super::read_replaced_user_source(
            "stratagems",
            "gibberlings3",
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
        );
        assert!(
            missing.is_none(),
            "a catalog-only mod with no user file must not resolve"
        );

        let old_source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("Gibberlings3/SwordCoastStratagems".to_string()),
            tag: Some("v35.10".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let mut new_source = old_source.clone();
        new_source.tag = None;
        new_source.branch = Some("main".to_string());

        let mut state = WizardState::default();
        super::record_history_if_changed(&mut state, "stratagems", None, &new_source, "My default");
        let store = crate::app::mod_source_history::load_store();
        assert!(
            store.history.is_empty(),
            "no history without an old source to compare against"
        );

        super::record_history_if_changed(
            &mut state,
            "stratagems",
            Some(&old_source),
            &new_source,
            "My default",
        );
        let store2 = crate::app::mod_source_history::load_store();
        assert_eq!(store2.history.len(), 1);
        assert_eq!(store2.history[0].saved_to, "My default");
        assert!(store2.history[0].block.contains("v35.10"));

        super::record_history_if_changed(
            &mut state,
            "stratagems",
            Some(&new_source),
            &new_source,
            "My default",
        );
        let store3 = crate::app::mod_source_history::load_store();
        assert_eq!(
            store3.history.len(),
            1,
            "a no-op resave must not add history"
        );
    }

    #[test]
    fn failed_save_records_no_history() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("failed_save_no_history");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let old_source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("Gibberlings3/SwordCoastStratagems".to_string()),
            tag: Some("v35.10".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("stratagems", "SCS");
        let old_block = crate::app::mod_downloads::complete_source_block(&old_source);
        std::fs::write(
            crate::app::mod_downloads::mod_downloads_user_path(),
            format!("{header}\n\n{old_block}\n"),
        )
        .unwrap();

        let text = "[[mods.sources]]\nid = \"different\"\nlabel = \"SCS\"\ntype = \"github\"\nurl = \"https://github.com/Gibberlings3/SwordCoastStratagems\"\nrepo = \"Gibberlings3/SwordCoastStratagems\"\nbranch = \"main\"\n";
        let request = super::SourceSaveRequest {
            tp2: "stratagems",
            name: "SCS",
            source_id: "gibberlings3",
            allow_id_change: false,
            text,
            target_path: None,
            new_source: None,
        };
        let destination = crate::app::step2_action::ModSourceEditDestination::GlobalDefault;

        let mut state = WizardState::default();
        let result = super::save_source_block_recording_history(
            &mut state,
            &request,
            destination,
            super::destination_label(destination),
            None,
        );

        assert!(result.is_err(), "{result:?}");
        let store = crate::app::mod_source_history::load_store();
        assert!(store.history.is_empty());
    }

    #[test]
    fn same_rule_resave_records_no_history() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("same_rule_resave_no_history");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("owner/scs".to_string()),
            tag: Some("v1.0".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let form = crate::app::source_form::from_source(
            &source,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: false,
                is_new_mod: false,
            },
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "stratagems",
        );

        let mut state = WizardState::default();
        state.step2.versions_ui.source_form = Some(form.clone());
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);
        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed")
        );
        let store_after_first = crate::app::mod_source_history::load_store();
        assert!(
            store_after_first.history.is_empty(),
            "first save has no prior block to diff against"
        );

        state.step2.versions_ui.source_form = Some(form);
        super::save_source_form(&mut state, &mut rx);
        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed")
        );
        let store_after_second = crate::app::mod_source_history::load_store();
        assert!(
            store_after_second.history.is_empty(),
            "an unchanged resave must not add history"
        );
    }

    #[test]
    fn history_records_the_replaced_user_block_only() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("history_records_replaced_only");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let old_source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("Gibberlings3/SwordCoastStratagems".to_string()),
            tag: Some("v35.10".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("stratagems", "SCS");
        let old_block = crate::app::mod_downloads::complete_source_block(&old_source);
        std::fs::write(
            crate::app::mod_downloads::mod_downloads_user_path(),
            format!("{header}\n\n{old_block}\n"),
        )
        .unwrap();

        let mut new_source = old_source;
        new_source.tag = None;
        new_source.branch = Some("main".to_string());
        let form = crate::app::source_form::from_source(
            &new_source,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: false,
                is_new_mod: false,
            },
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "stratagems",
        );
        let mut state = WizardState::default();
        state.step2.versions_ui.source_form = Some(form);
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "{}",
            state.step2.scan_status
        );

        let store = crate::app::mod_source_history::load_store();
        assert_eq!(store.history.len(), 1);
        assert!(store.history[0].block.contains("v35.10"));
        assert!(!store.history[0].block.contains("branch = \"main\""));
    }

    #[test]
    fn bio_default_changes_are_not_history() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("bio_default_not_history");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("owner/scs".to_string()),
            tag: Some("v1.0".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let form = crate::app::source_form::from_source(
            &source,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: false,
                is_new_mod: false,
            },
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "stratagems",
        );
        let mut state = WizardState::default();
        state.step2.versions_ui.source_form = Some(form);
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "{}",
            state.step2.scan_status
        );
        let store = crate::app::mod_source_history::load_store();
        assert!(store.history.is_empty());
    }

    #[test]
    fn form_note_saves_under_the_new_signature() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("form_note_saves");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let source = crate::app::mod_downloads::ModDownloadSource {
            tp2: "stratagems".to_string(),
            name: "SCS".to_string(),
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            github: Some("owner/scs".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let mut form = crate::app::source_form::from_source(
            &source,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: false,
                is_new_mod: false,
            },
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            "stratagems",
        );
        form.follow = crate::app::source_form::Follow::Tag;
        form.tag = "v35.17".to_string();
        form.note = "Custom reason".to_string();
        form.note_who = "My default".to_string();

        let written_source = crate::app::source_form::to_source(&form);
        let expected_signature = crate::app::mod_source_history::rule_signature(&written_source);
        let expected_key =
            crate::app::mod_source_history::note_key("stratagems", &expected_signature);

        let mut state = WizardState::default();
        state.step2.versions_ui.source_form = Some(form);
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);

        assert!(
            !state
                .step2
                .scan_status
                .starts_with("Save source entry failed"),
            "{}",
            state.step2.scan_status
        );

        let store = crate::app::mod_source_history::load_store();
        let note = store
            .notes
            .get(&expected_key)
            .expect("note must be saved under the new signature");
        assert_eq!(note.text, "Custom reason");
        assert_eq!(note.who, "My default");
    }

    #[test]
    fn use_known_source_records_history_with_the_list_name() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("use_known_source_list_name");
        let target_guard = TargetDirGuard::create("use_known_source_list_name_target");
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_guard.0.clone()));

        let current = crate::app::mod_downloads::ModDownloadSource {
            tp2: "widget".to_string(),
            name: "Widget".to_string(),
            source_id: "primary".to_string(),
            source_label: "Primary".to_string(),
            github: Some("owner/widget".to_string()),
            tag: Some("v1.0".to_string()),
            source_default: true,
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("widget", "Widget");
        let block = crate::app::mod_downloads::complete_source_block(&current);
        std::fs::write(
            target_guard.0.join("mod_downloads_user.toml"),
            format!("{header}\n\n{block}\n"),
        )
        .unwrap();

        let mut other = current;
        other.tag = None;
        other.branch = Some("dev".to_string());
        let known_block = crate::app::mod_downloads::complete_source_block(&other);

        let mut state = WizardState::default();
        let mut rx = None;
        super::handle_step2_download_source_action(
            &mut state,
            &mut rx,
            crate::app::step2_action::Step2Action::UseKnownSource {
                tp2: "widget".to_string(),
                card_key: "widget".to_string(),
                block: known_block,
                save_to: crate::app::step2_action::ModSourceEditDestination::ThisModlist,
                who: "Speedrun EET".to_string(),
            },
        );

        let store = crate::app::mod_source_history::load_store();
        assert_eq!(store.history.len(), 1, "{}", state.step2.scan_status);
        assert_eq!(store.history[0].saved_to, "Speedrun EET");
    }

    #[test]
    fn use_known_source_records_the_rule_in_effect_across_layers() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("use_known_source_across_layers");
        let target_guard = TargetDirGuard::create("use_known_source_across_layers_target");
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_guard.0.clone()));

        let current = crate::app::mod_downloads::ModDownloadSource {
            tp2: "widget".to_string(),
            name: "Widget".to_string(),
            source_id: "primary".to_string(),
            source_label: "Primary".to_string(),
            github: Some("owner/widget".to_string()),
            branch: Some("main".to_string()),
            source_default: true,
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let header = crate::app::mod_downloads::template_mod_header("widget", "Widget");
        let block = crate::app::mod_downloads::complete_source_block(&current);
        let global_path = crate::app::mod_downloads::mod_downloads_user_path();
        let global_text = format!("{header}\n\n{block}\n");
        std::fs::write(&global_path, &global_text).unwrap();

        let mut bookmark = current;
        bookmark.branch = None;
        bookmark.tag = Some("v2.0".to_string());
        let known_block = crate::app::mod_downloads::complete_source_block(&bookmark);

        let mut state = WizardState::default();
        let mut rx = None;
        super::handle_step2_download_source_action(
            &mut state,
            &mut rx,
            crate::app::step2_action::Step2Action::UseKnownSource {
                tp2: "widget".to_string(),
                card_key: "widget".to_string(),
                block: known_block,
                save_to: crate::app::step2_action::ModSourceEditDestination::ThisModlist,
                who: "Speedrun EET".to_string(),
            },
        );

        let store = crate::app::mod_source_history::load_store();
        assert_eq!(store.history.len(), 1, "{}", state.step2.scan_status);
        assert_eq!(store.history[0].saved_to, "Speedrun EET");
        assert!(
            store.history[0].block.contains("branch = \"main\""),
            "{}",
            store.history[0].block
        );
        let modlist_text =
            std::fs::read_to_string(target_guard.0.join("mod_downloads_user.toml")).unwrap();
        assert!(
            modlist_text.contains("tag = \"v2.0\""),
            "the pick must land in the modlist file; got:\n{modlist_text}"
        );
        assert!(
            !modlist_text.contains("branch = \"main\""),
            "{modlist_text}"
        );
        assert_eq!(std::fs::read_to_string(&global_path).unwrap(), global_text);
    }

    fn note_sheet_state() -> WizardState {
        let mut state = WizardState::default();
        state
            .step2
            .versions_ui
            .open_sheet(crate::app::state::VersionsSheet::Note, "widget".to_string());
        state
    }

    fn save_note_action() -> crate::app::step2_action::Step2Action {
        crate::app::step2_action::Step2Action::SaveSourceNote {
            tp2: "widget".to_string(),
            signature: "tag:v1.0".to_string(),
            text: "Pinned for the run".to_string(),
            who: "Speedrun EET".to_string(),
        }
    }

    #[test]
    fn save_source_note_closes_the_sheet_on_ok() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _config_guard = SourceFormConfigDirGuard::new("save_note_closes_sheet");

        let mut state = note_sheet_state();
        let mut rx = None;
        super::handle_step2_download_source_action(&mut state, &mut rx, save_note_action());

        assert_eq!(state.step2.versions_ui.sheet, None);
        assert_eq!(state.step2.versions_ui.sheet_tp2, None);
        assert_eq!(state.step2.versions_ui.sheet_error, None);
        assert_eq!(
            state.step2.versions_ui.pending_toast.as_deref(),
            Some("Note saved")
        );
    }

    #[test]
    fn save_source_note_keeps_the_sheet_and_sets_sheet_error_when_unreadable() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let config_guard = SourceFormConfigDirGuard::new("save_note_unreadable");
        std::fs::write(config_guard.0.join("mod_source_history.json"), "{ not json").unwrap();

        let mut state = note_sheet_state();
        let mut rx = None;
        super::handle_step2_download_source_action(&mut state, &mut rx, save_note_action());

        assert_eq!(
            state.step2.versions_ui.sheet,
            Some(crate::app::state::VersionsSheet::Note)
        );
        assert_eq!(
            state.step2.versions_ui.sheet_error.as_deref(),
            Some("Source history file is unreadable; not saved")
        );
        assert_eq!(state.step2.versions_ui.pending_toast, None);
    }

    fn new_mod_state(typed_tp2: &str) -> WizardState {
        let seed = crate::app::mod_downloads::ModDownloadSource {
            source_id: "primary".to_string(),
            source_label: "Primary".to_string(),
            github: Some("owner/widget".to_string()),
            ..crate::app::mod_downloads::ModDownloadSource::default()
        };
        let mut form = crate::app::source_form::from_source(
            &seed,
            crate::app::source_form::SourceFormIdentity {
                may_change_id: true,
                is_new_mod: true,
            },
            crate::app::step2_action::ModSourceEditDestination::GlobalDefault,
            super::NEW_MOD_CARD_KEY,
        );
        form.tp2 = typed_tp2.to_string();
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();
        state.step2.versions_ui.open_sheet(
            crate::app::state::VersionsSheet::EditSource,
            super::NEW_MOD_CARD_KEY.to_string(),
        );
        state.step2.versions_ui.source_form = Some(form);
        state
    }

    #[test]
    fn new_mod_save_pushes_a_pending_download() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("new_mod_pushes_pending");
        let target_guard = TargetDirGuard::create("new_mod_pushes_pending_target");
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_guard.0.clone()));

        let mut state = new_mod_state(" Setup-Widget.TP2 ");
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);

        assert!(
            state.step2.versions_ui.source_form.is_none(),
            "{}",
            state.step2.scan_status
        );
        assert_eq!(
            state.step2.log_pending_downloads,
            vec![crate::app::state::Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "Widget.tp2".to_string(),
                label: "Widget".to_string(),
                requested_version: None,
            }]
        );
        let written =
            std::fs::read_to_string(target_guard.0.join("mod_downloads_user.toml")).unwrap();
        assert!(written.contains("tp2 = \"Widget\""), "{written}");
        assert!(written.contains("owner/widget"), "{written}");
        assert_eq!(
            crate::app::added_mods::load_added_mods(),
            std::collections::BTreeSet::from(["widget".to_string()])
        );
        assert!(target_guard.0.join("added_mods.json").exists());

        let mut again = new_mod_state("widget");
        again.step2.log_pending_downloads = state.step2.log_pending_downloads.clone();
        super::save_source_form(&mut again, &mut rx);
        assert_eq!(again.step2.log_pending_downloads.len(), 1);
    }

    #[test]
    fn new_mod_save_skips_the_pending_entry_for_a_scanned_mod() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("new_mod_scanned");
        let target_guard = TargetDirGuard::create("new_mod_scanned_target");
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_guard.0.clone()));

        let mut state = new_mod_state("widget");
        state
            .step2
            .bgee_mods
            .push(make_mod_state("widget/setup-widget.tp2"));
        let mut rx = None;
        super::save_source_form(&mut state, &mut rx);

        assert!(
            state.step2.versions_ui.source_form.is_none(),
            "{}",
            state.step2.scan_status
        );
        assert!(state.step2.log_pending_downloads.is_empty());
    }

    #[test]
    fn new_mod_save_refuses_a_blank_tp2() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("new_mod_refuses_blank");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        for typed in ["", "   ", "setup-.tp2", "my mod", "mods/widget", "wid*get"] {
            let mut state = new_mod_state(typed);
            let mut rx = None;
            super::save_source_form(&mut state, &mut rx);

            let form = state
                .step2
                .versions_ui
                .source_form
                .as_ref()
                .expect("form stays open");
            assert_eq!(
                form.error.as_deref(),
                Some("Enter the mod's TP2 name first"),
                "{typed:?}"
            );
            assert!(state.step2.log_pending_downloads.is_empty(), "{typed:?}");
        }
        assert!(!crate::app::mod_downloads::mod_downloads_user_path().exists());
    }

    #[test]
    fn new_mod_save_forces_this_modlist() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient_guard = AmbientGuard::acquire();
        let _config_guard = SourceFormConfigDirGuard::new("new_mod_forces_modlist");
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let mut without_modlist = new_mod_state("widget");
        let mut rx = None;
        super::save_source_form(&mut without_modlist, &mut rx);
        assert_eq!(
            without_modlist
                .step2
                .versions_ui
                .source_form
                .as_ref()
                .and_then(|form| form.error.as_deref()),
            Some("Open a modlist first")
        );

        let target_guard = TargetDirGuard::create("new_mod_forces_modlist_target");
        crate::app::mod_downloads::set_active_modlist_dir(Some(target_guard.0.clone()));
        let mut state = new_mod_state("widget");
        assert_eq!(
            state
                .step2
                .versions_ui
                .source_form
                .as_ref()
                .map(|f| f.save_to),
            Some(crate::app::step2_action::ModSourceEditDestination::GlobalDefault)
        );
        super::save_source_form(&mut state, &mut rx);

        assert!(
            state.step2.versions_ui.source_form.is_none(),
            "{}",
            state.step2.scan_status
        );
        let written =
            std::fs::read_to_string(target_guard.0.join("mod_downloads_user.toml")).unwrap();
        assert!(written.contains("tp2 = \"widget\""), "{written}");
        let user_default =
            std::fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path())
                .unwrap_or_default();
        assert!(!user_default.contains("widget"), "{user_default}");
    }

    #[test]
    fn single_mod_check_of_a_pending_entry_sets_the_whole_folder_flag() {
        let mut state = WizardState::default();
        state
            .step2
            .log_pending_downloads
            .push(crate::app::state::Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: "widget.tp2".to_string(),
                label: "Widget".to_string(),
                requested_version: None,
            });
        state.step2.bgee_mods.push(make_mod_state("gadget.tp2"));
        let sources = crate::app::mod_downloads::ModDownloadsLoad::default();
        let mut rx = None;

        crate::app::app_step2_update_preview::preview_update_selected_mod(
            &mut state,
            &mut rx,
            &sources,
            ("BGEE".to_string(), "widget.tp2".to_string()),
        );
        assert!(state.step2.whole_folder_check_active);

        crate::app::app_step2_update_preview::preview_update_selected_mod(
            &mut state,
            &mut rx,
            &sources,
            ("BGEE".to_string(), "gadget.tp2".to_string()),
        );
        assert!(!state.step2.whole_folder_check_active);
    }

    fn eefixpack_asset(tag: &str) -> crate::app::state::Step2UpdateAsset {
        crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: "EEFixPack/setup-EEFixPack.tp2".to_string(),
            label: "EEFixPack".to_string(),
            source_id: "beamdog".to_string(),
            tag: tag.to_string(),
            asset_name: format!("EEFixPack-{tag}.zip"),
            asset_url: format!("https://example.com/EEFixPack-{tag}.zip"),
            installed_source_ref: None,
        }
    }

    fn eefixpack_key() -> String {
        crate::app::mod_downloads::normalize_mod_download_tp2("EEFixPack/setup-EEFixPack.tp2")
    }

    #[test]
    fn refetch_promotes_the_in_sync_asset_into_the_download_list() {
        let mut state = WizardState::<bool>::default();
        state
            .step2
            .update_selected_in_sync_assets
            .push(eefixpack_asset("v14.1"));

        let promoted = super::promote_in_sync_assets(&mut state.step2, &eefixpack_key());

        assert_eq!(promoted, vec![eefixpack_asset("v14.1")]);
        assert!(state.step2.update_selected_in_sync_assets.is_empty());
        let assets = &state.step2.update_selected_update_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].tag, "v14.1");
        assert_eq!(assets[0].asset_name, "EEFixPack-v14.1.zip");
        assert_eq!(
            state.step2.update_selected_update_sources,
            vec!["EEFixPack (v14.1)".to_string()]
        );
    }

    #[test]
    fn refetch_leaves_a_real_update_asset_alone() {
        let mut state = WizardState::<bool>::default();
        state
            .step2
            .update_selected_update_assets
            .push(eefixpack_asset("v15"));
        state
            .step2
            .update_selected_in_sync_assets
            .push(eefixpack_asset("v14.1"));

        assert!(super::promote_in_sync_assets(&mut state.step2, &eefixpack_key()).is_empty());

        let assets = &state.step2.update_selected_update_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].tag, "v15");
        let in_sync = &state.step2.update_selected_in_sync_assets;
        assert_eq!(in_sync.len(), 1);
        assert_eq!(in_sync[0].tag, "v14.1");
        assert!(state.step2.update_selected_update_sources.is_empty());
    }

    #[test]
    fn refetch_promotes_every_tab_entry() {
        let mut state = WizardState::<bool>::default();
        let bg2ee = crate::app::state::Step2UpdateAsset {
            game_tab: "BG2EE".to_string(),
            ..eefixpack_asset("v14.1")
        };
        state
            .step2
            .update_selected_in_sync_assets
            .extend([eefixpack_asset("v14.1"), bg2ee.clone()]);

        let promoted = super::promote_in_sync_assets(&mut state.step2, &eefixpack_key());

        assert_eq!(promoted, vec![eefixpack_asset("v14.1"), bg2ee.clone()]);
        assert!(state.step2.update_selected_in_sync_assets.is_empty());
        assert_eq!(
            state.step2.update_selected_update_assets,
            vec![eefixpack_asset("v14.1"), bg2ee]
        );
        assert_eq!(
            state.step2.update_selected_update_sources,
            vec![
                "EEFixPack (v14.1)".to_string(),
                "EEFixPack (v14.1)".to_string()
            ]
        );
    }

    #[test]
    fn refused_refetch_restores_the_in_sync_entry() {
        use crate::app::step2_action::Step2Action;

        let mut state = WizardState::<bool>::default();
        state.step1.download_archive = false;
        state
            .step2
            .update_selected_update_sources
            .push("Other (v2)".to_string());
        state
            .step2
            .update_selected_in_sync_assets
            .push(eefixpack_asset("v14.1"));
        let sources_before = state.step2.update_selected_update_sources.clone();

        super::handle_step2_action(
            &mut state,
            &mut None,
            &mut None,
            &mut std::collections::VecDeque::new(),
            &mut None,
            &mut None,
            Step2Action::DownloadUpdateFor {
                tp2: eefixpack_key(),
            },
        );

        assert_eq!(
            state.step2.scan_status,
            "Download Archive is disabled in Step 1"
        );
        assert!(state.step2.update_selected_update_assets.is_empty());
        assert_eq!(state.step2.update_selected_update_sources, sources_before);
        assert_eq!(
            state.step2.update_selected_in_sync_assets,
            vec![eefixpack_asset("v14.1")]
        );
    }

    #[test]
    fn invalidate_clears_the_in_sync_assets() {
        let mut state = WizardState::default();
        state
            .step2
            .update_selected_in_sync_assets
            .push(eefixpack_asset("v14.1"));

        super::invalidate_update_selected_results(&mut state);

        assert!(state.step2.update_selected_in_sync_assets.is_empty());
    }
}
