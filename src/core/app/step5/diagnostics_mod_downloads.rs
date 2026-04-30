// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::app::app_step2_update_download;
use crate::app::mod_downloads;
use crate::app::state::{Step2ModState, Step2UpdateAsset, Step2UpdateRetryRequest, WizardState};

pub(super) fn write_mod_download_diagnostics(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    let summary_dir = run_dir.join("summary");
    let config_dir = run_dir.join("config");
    let updates_dir = run_dir.join("updates");
    fs::create_dir_all(&summary_dir)?;
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(&updates_dir)?;
    written.extend(copy_raw_mod_download_configs(&config_dir)?);
    written.push(write_mod_download_sources_effective_json(
        &config_dir,
        state,
        timestamp_unix_secs,
    )?);
    written.push(write_update_resolution_json(
        &updates_dir,
        state,
        timestamp_unix_secs,
    )?);
    written.push(write_update_download_json(
        &updates_dir,
        state,
        timestamp_unix_secs,
    )?);
    written.push(write_update_extract_json(
        &updates_dir,
        state,
        timestamp_unix_secs,
    )?);
    written.push(write_tp2_resolution_json(
        &updates_dir,
        state,
        timestamp_unix_secs,
    )?);
    written.push(write_user_action_trace_json(
        &summary_dir,
        state,
        timestamp_unix_secs,
    )?);
    Ok(written)
}

fn copy_raw_mod_download_configs(run_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for (source, file_name) in [
        (
            mod_downloads::mod_downloads_default_path(),
            "mod_downloads_default.toml",
        ),
        (
            mod_downloads::mod_downloads_user_path(),
            "mod_downloads_user.toml",
        ),
    ] {
        let destination = run_dir.join(file_name);
        match fs::read_to_string(&source) {
            Ok(content) => {
                fs::write(&destination, content)?;
                written.push(destination);
            }
            Err(err) => {
                fs::write(
                    &destination,
                    format!("missing_or_unreadable={}\npath={}\n", err, source.display()),
                )?;
                written.push(destination);
            }
        }
    }
    Ok(written)
}

fn write_mod_download_sources_effective_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("mod_download_sources_effective.json");
    let load = mod_downloads::load_mod_download_sources();
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "load_error": load.error,
        "selected_source_ids": state.step2.selected_source_ids,
        "sources": load.sources.iter().map(|source| json!({
            "name": source.name,
            "tp2": source.tp2,
            "normalized_tp2": mod_downloads::normalize_mod_download_tp2(&source.tp2),
            "aliases": source.aliases,
            "normalized_aliases": source.aliases.iter().map(|alias| {
                mod_downloads::normalize_mod_download_tp2(alias)
            }).collect::<Vec<_>>(),
            "tp2_rename": source.tp2_rename.as_ref().map(|rename| json!({
                "from": rename.from,
                "to": rename.to,
            })),
            "source_id": source.source_id,
            "source_label": source.source_label,
            "source_default": source.source_default,
            "url": source.url,
            "repo": source.github,
            "exact_github": source.exact_github,
            "channel": source.channel,
            "tag": source.tag,
            "branch": source.branch,
            "asset": source.asset,
            "subdir_require": source.subdir_require,
            "pkg_windows": source.pkg_windows,
            "pkg_linux": source.pkg_linux,
            "pkg_macos": source.pkg_macos,
            "chosen_pkg_for_current_os": mod_downloads::preferred_pkg_for_current_platform(source),
        })).collect::<Vec<_>>()
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn write_update_resolution_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("update_resolution.json");
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "has_run": state.step2.update_selected_has_run,
        "check_running": state.step2.update_selected_check_running,
        "check_done_count": state.step2.update_selected_check_done_count,
        "check_total_count": state.step2.update_selected_check_total_count,
        "last_was_full_selection": state.step2.update_selected_last_was_full_selection,
        "last_selection_signature": state.step2.update_selected_last_selection_signature,
        "target_game_tab": state.step2.update_selected_target_game_tab,
        "target_tp_file": state.step2.update_selected_target_tp_file,
        "selected_source_ids": state.step2.selected_source_ids,
        "check_requests": state.step2.update_selected_check_requests.iter().map(retry_request_json).collect::<Vec<_>>(),
        "exact_version_retry_requests": state.step2.update_selected_exact_version_retry_requests.iter().map(retry_request_json).collect::<Vec<_>>(),
        "resolved_assets": state.step2.update_selected_update_assets.iter().map(update_asset_json).collect::<Vec<_>>(),
        "locked_assets": state.step2.update_selected_locked_update_assets.iter().map(update_asset_json).collect::<Vec<_>>(),
        "update_sources": state.step2.update_selected_update_sources,
        "locked_update_sources": state.step2.update_selected_locked_update_sources,
        "missing_sources": state.step2.update_selected_missing_sources,
        "known_sources": state.step2.update_selected_known_sources,
        "manual_sources": state.step2.update_selected_manual_sources,
        "unknown_sources": state.step2.update_selected_unknown_sources,
        "failed_sources": state.step2.update_selected_failed_sources,
        "exact_version_failed_sources": state.step2.update_selected_exact_version_failed_sources,
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn write_update_download_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("update_download.json");
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "download_archive_enabled": state.step1.download_archive,
        "download_running": state.step2.update_selected_download_running,
        "archive_dir": state.step1.mods_archive_folder,
        "assets": state.step2.update_selected_update_assets.iter().map(|asset| {
            let archive_name = app_step2_update_download::archive_file_name(asset);
            let archive_path = archive_dir.join(&archive_name);
            json!({
                "asset": update_asset_json(asset),
                "archive_file_name": archive_name,
                "archive_path": archive_path,
                "archive_exists": archive_path.exists(),
            })
        }).collect::<Vec<_>>(),
        "downloaded_sources": state.step2.update_selected_downloaded_sources,
        "download_failed_sources": state.step2.update_selected_download_failed_sources,
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn write_update_extract_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("update_extract.json");
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    let source_load = mod_downloads::load_mod_download_sources();
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "extract_running": state.step2.update_selected_extract_running,
        "mods_root": state.step1.mods_folder,
        "backup_root": state.step1.mods_backup_folder,
        "source_load_error": source_load.error,
        "jobs": state.step2.update_selected_update_assets.iter().map(|asset| {
            let archive_name = app_step2_update_download::archive_file_name(asset);
            let archive_path = archive_dir.join(&archive_name);
            let source = resolve_selected_source(state, &source_load, &asset.tp_file);
            json!({
                "asset": update_asset_json(asset),
                "archive_file_name": archive_name,
                "archive_path": archive_path,
                "archive_exists": archive_path.exists(),
                "selected_source_id": selected_source_id(state, &asset.tp_file),
                "aliases": source.as_ref().map(|source| source.aliases.clone()).unwrap_or_default(),
                "tp2_rename": source.as_ref().and_then(|source| source.tp2_rename.as_ref()).map(|rename| json!({
                    "from": rename.from,
                    "to": rename.to,
                })),
                "subdir_require": source.as_ref().and_then(|source| source.subdir_require.clone()),
                "current_mod_root": current_mod_root(state, &asset.game_tab, &asset.tp_file),
            })
        }).collect::<Vec<_>>(),
        "extracted_sources": state.step2.update_selected_extracted_sources,
        "extract_failed_sources": state.step2.update_selected_extract_failed_sources,
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn write_tp2_resolution_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("tp2_resolution.json");
    let source_load = mod_downloads::load_mod_download_sources();
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "mods_root": state.step1.mods_folder,
        "source_load_error": source_load.error,
        "selected_source_ids": state.step2.selected_source_ids,
        "tabs": {
            "BGEE": tp2_resolution_for_mods(state, &source_load, "BGEE", &state.step2.bgee_mods),
            "BG2EE": tp2_resolution_for_mods(state, &source_load, "BG2EE", &state.step2.bg2ee_mods),
        }
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn write_user_action_trace_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("user_action_trace.json");
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "note": "BIO does not yet keep a timestamped action log; this file records the final action-related state.",
        "current_step": state.current_step,
        "step2_status": state.step2.scan_status,
        "step2_is_scanning": state.step2.is_scanning,
        "update_popup_open": state.step2.update_selected_popup_open,
        "update_has_run": state.step2.update_selected_has_run,
        "update_check_running": state.step2.update_selected_check_running,
        "update_download_running": state.step2.update_selected_download_running,
        "update_extract_running": state.step2.update_selected_extract_running,
        "source_editor_open": state.step2.mod_download_source_editor_open,
        "source_editor_tp2": state.step2.mod_download_source_editor_tp2,
        "source_editor_source_id": state.step2.mod_download_source_editor_source_id,
        "source_editor_error": state.step2.mod_download_source_editor_error,
        "selected_source_ids": state.step2.selected_source_ids,
        "pending_saved_log_apply": state.step2.pending_saved_log_apply,
        "pending_saved_log_update_preview": state.step2.pending_saved_log_update_preview,
        "pending_saved_log_download": state.step2.pending_saved_log_download,
        "install_running": state.step5.install_running,
        "last_status": state.step5.last_status_text,
        "last_exit_code": state.step5.last_exit_code,
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn retry_request_json(request: &Step2UpdateRetryRequest) -> serde_json::Value {
    json!({
        "game_tab": request.game_tab,
        "tp_file": request.tp_file,
        "label": request.label,
        "source_id": request.source_id,
        "repo": request.repo,
        "source_url": request.source_url,
        "channel": request.channel,
        "tag": request.tag,
        "branch": request.branch,
        "asset": request.asset,
        "chosen_pkg_for_current_os": request.pkg,
    })
}

fn update_asset_json(asset: &Step2UpdateAsset) -> serde_json::Value {
    json!({
        "game_tab": asset.game_tab,
        "tp_file": asset.tp_file,
        "label": asset.label,
        "source_id": asset.source_id,
        "tag": asset.tag,
        "asset_name": asset.asset_name,
        "asset_url": asset.asset_url,
        "installed_source_ref": asset.installed_source_ref,
    })
}

fn tp2_resolution_for_mods(
    state: &WizardState,
    source_load: &mod_downloads::ModDownloadsLoad,
    tab: &str,
    mods: &[Step2ModState],
) -> Vec<serde_json::Value> {
    mods.iter()
        .map(|mod_state| {
            let normalized_tp2 = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
            let source = resolve_selected_source(state, source_load, &mod_state.tp_file);
            json!({
                "tab": tab,
                "name": mod_state.name,
                "tp_file": mod_state.tp_file,
                "normalized_tp2": normalized_tp2,
                "tp2_path": mod_state.tp2_path,
                "selected_source_id": selected_source_id(state, &mod_state.tp_file),
                "matched_source": source.as_ref().map(|source| json!({
                    "source_id": source.source_id,
                    "source_label": source.source_label,
                    "tp2": source.tp2,
                    "aliases": source.aliases,
                    "subdir_require": source.subdir_require,
                    "tp2_rename": source.tp2_rename.as_ref().map(|rename| json!({
                        "from": rename.from,
                        "to": rename.to,
                    })),
                })),
            })
        })
        .collect()
}

fn resolve_selected_source(
    state: &WizardState,
    sources: &mod_downloads::ModDownloadsLoad,
    tp_file: &str,
) -> Option<mod_downloads::ModDownloadSource> {
    sources.resolve_source(tp_file, selected_source_id(state, tp_file).as_deref())
}

fn selected_source_id(state: &WizardState, tp_file: &str) -> Option<String> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
    state.step2.selected_source_ids.get(&tp2_key).cloned()
}

fn current_mod_root(state: &WizardState, game_tab: &str, tp_file: &str) -> Option<PathBuf> {
    let mods = if game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mod_state = mods.iter().find(|mod_state| mod_state.tp_file == tp_file)?;
    Path::new(mod_state.tp2_path.trim())
        .parent()
        .map(Path::to_path_buf)
}
