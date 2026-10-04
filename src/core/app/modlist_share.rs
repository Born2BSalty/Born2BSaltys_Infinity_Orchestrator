// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

use crate::app::game_authority;
use crate::app::state::WizardState;
use crate::app::step5::diagnostics::build_weidu_export_lines;

const SHARE_CODE_PREFIX: &str = "BIO-MODLIST-V1:";

static PENDING_WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub(crate) fn push_pending_warnings(warnings: Vec<String>) {
    let mut last = LAST_WARNINGS.lock().unwrap_or_else(PoisonError::into_inner);
    if *last == warnings {
        return;
    }
    last.clone_from(&warnings);
    drop(last);
    if warnings.is_empty() {
        return;
    }
    PENDING_WARNINGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(warnings);
}

#[must_use]
pub(crate) fn take_pending_warnings() -> Vec<String> {
    std::mem::take(
        &mut *PENDING_WARNINGS
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ExportLogSource {
    Installed,
    #[default]
    Rebuilt,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ShareExportSources {
    pub(crate) mod_downloads_user: Option<String>,
    pub(crate) mod_installed_refs: Option<String>,
    pub(crate) unresolved_mods: Vec<String>,
    pub(crate) log_source: ExportLogSource,
    pub(crate) installed_source_ids: std::collections::BTreeMap<String, String>,
}

pub(crate) fn export_modlist_share_code(state: &WizardState) -> Result<String, String> {
    build_and_export_share_code(state, ExportLogSource::Rebuilt)
}

pub(crate) fn export_modlist_share_code_for_completed_install(
    state: &WizardState,
) -> Result<String, String> {
    build_and_export_share_code(state, ExportLogSource::Installed)
}

fn build_and_export_share_code(
    state: &WizardState,
    log_source: ExportLogSource,
) -> Result<String, String> {
    crate::app::mod_downloads::ensure_mod_downloads_files().map_err(|err| err.to_string())?;

    let include = checked_mod_tp2s(state);
    let lookup = crate::app::app_step2_update_source_refs::InstalledRefLookup::load(
        state.step1.mods_folder.trim(),
    );
    let installed = installed_refs_for_export(&lookup, &include);
    let resolved = build_resolved_source_overrides(state, &installed)?;

    let mod_installed_refs = if crate::app::mod_downloads::active_modlist_downloads_path().is_some()
    {
        build_per_modlist_installed_refs(state, &lookup, &include)
    } else {
        installed_refs_copy_without_archives(
            &crate::app::app_step2_update_source_refs::installed_source_refs_path(),
        )
    };

    export_modlist_share_code_with(
        state,
        &ShareExportSources {
            mod_downloads_user: resolved.toml,
            mod_installed_refs,
            unresolved_mods: resolved.unresolved,
            log_source,
            installed_source_ids:
                crate::app::app_step2_update_source_refs::installed_source_ids_from_refs_file(
                    &installed,
                ),
        },
    )
}

fn checked_mod_tp2s(state: &WizardState) -> std::collections::BTreeSet<String> {
    state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .filter(|mod_state| {
            mod_state
                .components
                .iter()
                .any(|component| component.checked)
        })
        .map(|mod_state| crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file))
        .collect()
}

pub(crate) fn export_modlist_share_code_with(
    state: &WizardState,
    sources: &ShareExportSources,
) -> Result<String, String> {
    let weidu_logs = export_weidu_logs(state, sources.log_source)?;
    let first_slot_export_text = weidu_logs.bgee.as_deref().or(weidu_logs.iwdee.as_deref());
    if relevant_weidu_text_is_empty(state, first_slot_export_text, weidu_logs.bg2ee.as_deref()) {
        return Err("No WeiDU entries available to export.".to_string());
    }

    let mod_downloads_user = sources.mod_downloads_user.clone();
    let mod_installed_refs = sources.mod_installed_refs.clone();

    let (mod_configs, config_warnings) =
        export_mod_config_files(state, &sources.installed_source_ids)?;
    for config_warning in &config_warnings {
        warn!("{config_warning}");
    }
    push_pending_warnings(config_warnings);
    let mut payload = json!({
        "format_version": 1,
        "bio_version": env!("CARGO_PKG_VERSION"),
        "game_install": state.step1.game_install.clone(),
        "install_mode": state.step1.install_mode.clone(),
        "weidu_logs": {
            "bgee": weidu_logs.bgee,
            "bg2ee": weidu_logs.bg2ee,
        },
        "source_overrides": {
            "mod_downloads_user_toml": mod_downloads_user,
        },
        "installed_refs": {
            "mod_installed_refs_toml": mod_installed_refs,
        },
        "mod_configs": {
            "files": mod_configs,
        },
    });
    move_first_slot_log_to_iwdee_key(&mut payload, weidu_logs.iwdee.as_deref());
    insert_unresolved_mods(&mut payload, &sources.unresolved_mods);
    insert_export_provenance(&mut payload, state);
    insert_game_version(&mut payload, state);
    let payload_text = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    encode_share_payload_text(&payload_text)
}

fn move_first_slot_log_to_iwdee_key(payload: &mut serde_json::Value, iwdee_text: Option<&str>) {
    let Some(text) = iwdee_text else {
        return;
    };
    let Some(logs) = payload
        .get_mut("weidu_logs")
        .and_then(|value| value.as_object_mut())
    else {
        return;
    };
    logs.remove("bgee");
    logs.insert("iwdee".to_string(), json!(text));
}

fn insert_game_version(payload: &mut serde_json::Value, state: &WizardState) {
    let Some(version) = crate::app::game_version::mint_tag(&state.step1, &state.step1.game_install)
    else {
        return;
    };
    let Some(obj) = payload.as_object_mut() else {
        return;
    };
    obj.insert("game_version".to_string(), json!(version.tag()));
}

fn insert_unresolved_mods(payload: &mut serde_json::Value, unresolved_mods: &[String]) {
    if unresolved_mods.is_empty() {
        return;
    }
    let Some(overrides) = payload
        .get_mut("source_overrides")
        .and_then(|value| value.as_object_mut())
    else {
        return;
    };
    overrides.insert("unresolved_mods".to_string(), json!(unresolved_mods));
}

pub(crate) fn encode_share_payload_text(payload_text: &str) -> Result<String, String> {
    let compressed = zlib_compress(payload_text.as_bytes())?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&compressed)
    ))
}

fn insert_export_provenance(payload: &mut serde_json::Value, state: &WizardState) {
    let Some(obj) = payload.as_object_mut() else {
        return;
    };
    if let Some(name) = state
        .modlist_share_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        obj.insert("name".to_string(), json!(name));
    }
    if let Some(author) = state
        .modlist_share_author
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        obj.insert("author".to_string(), json!(author));
    }
    if let Some(description) = state
        .modlist_share_description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        obj.insert("description".to_string(), json!(description));
    }
    if !state.modlist_share_forked_from.is_empty() {
        obj.insert(
            "forked_from".to_string(),
            json!(state.modlist_share_forked_from.clone()),
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ModlistSharePreview {
    pub(crate) bio_version: String,
    pub(crate) game_install: String,
    pub(crate) game_version: Option<String>,
    pub(crate) install_mode: String,
    pub(crate) bgee_entries: usize,
    pub(crate) bg2ee_entries: usize,
    pub(crate) has_source_overrides: bool,
    pub(crate) has_installed_refs: bool,
    pub(crate) bgee_log_text: String,
    pub(crate) bg2ee_log_text: String,
    pub(crate) source_overrides_text: String,
    pub(crate) installed_refs_text: String,
    pub(crate) mod_config_count: usize,
    pub(crate) mod_configs_text: String,
    pub(crate) allow_auto_install: bool,
    pub(crate) name: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) forked_from: Vec<ForkAncestor>,
    pub(crate) unresolved_mods: Vec<String>,
}

pub(crate) fn preview_modlist_share_code(code: &str) -> Result<ModlistSharePreview, String> {
    share_preview(&decode_share_payload(code)?)
}

pub(crate) fn import_modlist_share_code(
    state: &mut WizardState,
    code: &str,
) -> Result<ModlistSharePreview, String> {
    let payload = decode_share_payload(code)?;
    let preview = share_preview(&payload)?;
    let mut step1 = state.step1.clone();
    step1.game_install.clone_from(&payload.game_install);
    step1.install_mode =
        crate::app::state::Step1State::normalize_install_mode(&payload.install_mode).to_string();
    step1.sync_install_mode_flags();
    crate::app::modlist_config_files::save_pending_mod_configs(&payload.mod_configs.files)?;
    write_imported_weidu_logs(&step1, &payload)?;
    if let Some(text) = payload
        .source_overrides
        .mod_downloads_user_toml
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        if let Some(pin_path) = crate::app::mod_downloads::active_modlist_downloads_path() {
            if let Some(parent) = pin_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| format!("create per-modlist dir failed: {err}"))?;
            }
            let migrated_text = migrate_share_import_text(text)?;
            write_text_file(pin_path, &migrated_text)?;
        } else {
            warn!(
                target = "share",
                "import: no active modlist; skipping source overrides write \
                 (the global mod_downloads_user.toml is left untouched)"
            );
        }
    }
    if let Some(text) = payload
        .installed_refs
        .mod_installed_refs_toml
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        write_text_file(
            crate::app::app_step2_update_source_refs::installed_source_refs_path(),
            text,
        )?;
    }
    state.step1 = step1;
    state.reset_workflow_keep_step1();
    state.step2.selected_source_ids =
        crate::app::app_step2_update_source_refs::load_installed_source_ids();
    state.step5.last_status_text = "Imported modlist share code".to_string();
    warn_on_unresolved_mods(&preview);
    Ok(preview)
}

fn migrate_share_import_text(text: &str) -> Result<String, String> {
    let default_text = crate::app::mod_downloads::default_mod_downloads_content();
    let user_text = fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path())
        .unwrap_or_default();
    match crate::app::mod_downloads_migrate::migrate_source_text(
        text,
        default_text,
        &user_text,
        crate::app::mod_downloads_migrate::MigrateTier::Modlist,
    ) {
        crate::app::mod_downloads_migrate::MigrateResult::Rewritten(rewritten) => Ok(rewritten),
        crate::app::mod_downloads_migrate::MigrateResult::Unchanged => Ok(text.to_string()),
        crate::app::mod_downloads_migrate::MigrateResult::Unparseable(err) => Err(err),
    }
}

fn warn_on_unresolved_mods(preview: &ModlistSharePreview) {
    if preview.unresolved_mods.is_empty() {
        return;
    }
    warn!(
        target = "share",
        "import: {} mod(s) have no download source: {}",
        preview.unresolved_mods.len(),
        preview.unresolved_mods.join(", ")
    );
}

#[derive(Deserialize)]
pub(crate) struct ModlistSharePayload {
    pub(crate) format_version: u64,
    #[serde(default)]
    pub(crate) bio_version: String,
    pub(crate) game_install: String,
    #[serde(default)]
    pub(crate) game_version: Option<String>,
    pub(crate) install_mode: String,
    #[serde(default)]
    pub(crate) weidu_logs: ModlistShareWeiduLogs,
    #[serde(default)]
    pub(crate) source_overrides: ModlistShareSourceOverrides,
    #[serde(default)]
    pub(crate) installed_refs: ModlistShareInstalledRefs,
    #[serde(default)]
    pub(crate) mod_configs: ModlistShareModConfigs,
    #[serde(default = "default_true")]
    pub(crate) allow_auto_install: bool,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) author: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) forked_from: Vec<ForkAncestor>,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ForkAncestor {
    pub(crate) name: String,
    pub(crate) author: String,
}

#[derive(Default, Deserialize)]
pub(crate) struct ModlistShareWeiduLogs {
    pub(crate) bgee: Option<String>,
    pub(crate) bg2ee: Option<String>,
    #[serde(default)]
    pub(crate) iwdee: Option<String>,
}

pub(crate) fn first_slot_weidu_text(logs: &ModlistShareWeiduLogs) -> Option<&str> {
    logs.iwdee
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .or(logs.bgee.as_deref())
}

#[derive(Default, Deserialize)]
pub(crate) struct ModlistShareSourceOverrides {
    pub(crate) mod_downloads_user_toml: Option<String>,
    #[serde(default)]
    pub(crate) unresolved_mods: Vec<String>,
}

#[derive(Default, Deserialize)]
pub(crate) struct ModlistShareInstalledRefs {
    pub(crate) mod_installed_refs_toml: Option<String>,
}

#[derive(Default, Deserialize)]
pub(crate) struct ModlistShareModConfigs {
    #[serde(default)]
    pub(crate) files: Vec<ModlistShareConfigFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ModlistShareConfigFile {
    pub(crate) tp2: String,
    pub(crate) source_id: String,
    pub(crate) relative_path: String,
    pub(crate) base64_data: String,
}

pub(crate) fn decode_share_payload(code: &str) -> Result<ModlistSharePayload, String> {
    let trimmed = code.trim();
    let encoded = trimmed
        .strip_prefix(SHARE_CODE_PREFIX)
        .ok_or_else(|| "Share code must start with BIO-MODLIST-V1:".to_string())?;
    let bytes = base64url_decode(encoded)?;
    let bytes = zlib_decompress(&bytes)?;
    let payload: ModlistSharePayload =
        serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
    if payload.format_version != 1 {
        return Err(format!(
            "Unsupported modlist share format version: {}",
            payload.format_version
        ));
    }
    Ok(payload)
}

fn share_preview(payload: &ModlistSharePayload) -> Result<ModlistSharePreview, String> {
    let install_mode =
        crate::app::state::Step1State::normalize_install_mode(&payload.install_mode).to_string();
    let first_game_entries = count_weidu_entries(first_slot_weidu_text(&payload.weidu_logs));
    let second_game_entries = count_weidu_entries(payload.weidu_logs.bg2ee.as_deref());
    if match payload.game_install.as_str() {
        "EET" => first_game_entries == 0 && second_game_entries == 0,
        "BG2EE" => second_game_entries == 0,
        _ => first_game_entries == 0,
    } {
        return Err("No WeiDU entries available to import.".to_string());
    }
    let mod_configs_text = payload
        .mod_configs
        .files
        .iter()
        .map(|file| {
            format!(
                "{} | {} | {}",
                file.tp2.trim(),
                file.source_id.trim(),
                file.relative_path.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(ModlistSharePreview {
        bio_version: payload.bio_version.clone(),
        game_install: payload.game_install.clone(),
        game_version: payload
            .game_version
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string),
        install_mode,
        bgee_entries: first_game_entries,
        bg2ee_entries: second_game_entries,
        has_source_overrides: payload
            .source_overrides
            .mod_downloads_user_toml
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty()),
        has_installed_refs: payload
            .installed_refs
            .mod_installed_refs_toml
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty()),
        bgee_log_text: first_slot_weidu_text(&payload.weidu_logs)
            .unwrap_or_default()
            .to_string(),
        bg2ee_log_text: payload.weidu_logs.bg2ee.clone().unwrap_or_default(),
        source_overrides_text: payload
            .source_overrides
            .mod_downloads_user_toml
            .clone()
            .unwrap_or_default(),
        installed_refs_text: payload
            .installed_refs
            .mod_installed_refs_toml
            .clone()
            .unwrap_or_default(),
        mod_config_count: payload.mod_configs.files.len(),
        mod_configs_text,
        allow_auto_install: payload.allow_auto_install,
        name: payload.name.clone(),
        author: payload.author.clone(),
        description: payload.description.clone(),
        forked_from: payload.forked_from.clone(),
        unresolved_mods: payload.source_overrides.unresolved_mods.clone(),
    })
}

fn write_imported_weidu_logs(
    step1: &crate::app::state::Step1State,
    payload: &ModlistSharePayload,
) -> Result<(), String> {
    match step1.game_install.as_str() {
        "EET" => {
            let rewritten_bg2ee =
                rewrite_imported_eet_bg2ee_wlb_paths(step1, payload.weidu_logs.bg2ee.as_deref())?;
            if count_weidu_entries(payload.weidu_logs.bgee.as_deref()) == 0
                && count_weidu_entries(rewritten_bg2ee.as_deref()) == 0
            {
                return Err("Imported WeiDU logs have no entries.".to_string());
            }
            write_imported_log_allow_empty(
                "BGEE",
                payload.weidu_logs.bgee.as_deref(),
                &import_log_target_path(step1, true)?,
            )?;
            write_imported_log_allow_empty(
                "BG2EE",
                rewritten_bg2ee.as_deref(),
                &import_log_target_path(step1, false)?,
            )
        }
        "BG2EE" => write_imported_log(
            "BG2EE",
            payload.weidu_logs.bg2ee.as_deref(),
            &import_log_target_path(step1, false)?,
        ),
        _ => write_imported_log(
            game_authority::first_slot_tab(&step1.game_install),
            first_slot_weidu_text(&payload.weidu_logs),
            &import_log_target_path(step1, true)?,
        ),
    }
}

fn rewrite_imported_eet_bg2ee_wlb_paths(
    step1: &crate::app::state::Step1State,
    text: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(text) = text else {
        return Ok(None);
    };
    let marker = "@wlb-inputs:";
    let mut changed = false;
    let mut out = Vec::<String>::new();
    let local_bg1_path = local_eet_bg1_source_path(step1);
    for line in text.lines() {
        let Some(marker_pos) = line.to_ascii_lowercase().find(marker) else {
            out.push(line.to_string());
            continue;
        };
        let spec_start = marker_pos + marker.len();
        let (head, spec) = line.split_at(spec_start);
        let mut tokens = Vec::<String>::new();
        for token in spec.trim().split(',') {
            if wlb_token_is_path_like(token) {
                if local_bg1_path.is_empty() {
                    return Err(
                        "Imported EET WLB input requires local BGEE/BG1 path. Set BGEE game path before importing."
                            .to_string(),
                    );
                }
                changed = true;
                tokens.push(requote_like(token, local_bg1_path));
            } else {
                tokens.push(token.trim().to_string());
            }
        }
        out.push(format!("{head} {}", tokens.join(",")));
    }
    if changed {
        Ok(Some(out.join("\n")))
    } else {
        Ok(Some(text.to_string()))
    }
}

fn local_eet_bg1_source_path(step1: &crate::app::state::Step1State) -> &str {
    if step1.new_pre_eet_dir_enabled {
        step1.eet_pre_dir.trim()
    } else {
        step1.eet_bgee_game_folder.trim()
    }
}

fn wlb_token_is_path_like(token: &str) -> bool {
    let token = token.trim().trim_matches('"').trim_matches('\'');
    if token.starts_with('/') {
        return true;
    }
    let mut chars = token.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic()
    )
}

fn requote_like(original: &str, value: &str) -> String {
    let trimmed = original.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        format!("\"{value}\"")
    } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        format!("'{value}'")
    } else {
        value.to_string()
    }
}

fn import_log_target_path(
    step1: &crate::app::state::Step1State,
    bgee: bool,
) -> Result<PathBuf, String> {
    if step1.installs_exactly_from_weidu_logs() {
        let value = if bgee {
            &step1.bgee_log_file
        } else {
            &step1.bg2ee_log_file
        };
        if value.trim().is_empty() {
            return Err(format!(
                "Set {} WeiDU Log File before importing.",
                if bgee {
                    game_authority::first_slot_tab(&step1.game_install)
                } else {
                    "BG2EE"
                }
            ));
        }
        return Ok(PathBuf::from(value.trim()));
    }
    let value = match (step1.game_install.as_str(), bgee) {
        ("EET", true) => &step1.eet_bgee_log_folder,
        ("EET", false) => &step1.eet_bg2ee_log_folder,
        (_, true) => &step1.bgee_log_folder,
        (_, false) => &step1.bg2ee_log_folder,
    };
    if value.trim().is_empty() {
        return Err(format!(
            "Set {} WeiDU Log Folder before importing.",
            if bgee {
                game_authority::first_slot_tab(&step1.game_install)
            } else {
                "BG2EE"
            }
        ));
    }
    Ok(PathBuf::from(value.trim()).join("weidu.log"))
}

fn write_imported_log(label: &str, text: Option<&str>, path: &Path) -> Result<(), String> {
    let Some(text) = text.filter(|text| count_weidu_entries(Some(text)) > 0) else {
        return Err(format!("Imported {label} WeiDU log has no entries."));
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, text).map_err(|err| format!("Write {label} WeiDU log failed: {err}"))
}

fn write_imported_log_allow_empty(
    label: &str,
    text: Option<&str>,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, text.unwrap_or_default())
        .map_err(|err| format!("Write {label} WeiDU log failed: {err}"))
}

fn write_text_file(path: PathBuf, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, text).map_err(|err| err.to_string())
}

fn count_weidu_entries(text: Option<&str>) -> usize {
    text.map_or(0, |text| {
        text.lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with("//")
            })
            .count()
    })
}

struct ExportWeiduLogs {
    bgee: Option<String>,
    bg2ee: Option<String>,
    iwdee: Option<String>,
}

fn first_slot_export_fields(
    game_install: &str,
    first_slot_text: Option<String>,
) -> (Option<String>, Option<String>) {
    if game_authority::share_log_key(game_authority::first_slot_tab(game_install)) == "iwdee" {
        (None, first_slot_text)
    } else {
        (first_slot_text, None)
    }
}

fn export_weidu_logs(
    state: &WizardState,
    log_source: ExportLogSource,
) -> Result<ExportWeiduLogs, String> {
    if state.step1.installs_exactly_from_weidu_logs() {
        let first_slot_text = read_exact_source_weidu_log(
            state,
            crate::app::app_step2_log::resolve_bgee_weidu_log_path,
        )?;
        let (bgee, iwdee) = first_slot_export_fields(&state.step1.game_install, first_slot_text);
        return Ok(ExportWeiduLogs {
            bgee,
            bg2ee: read_exact_source_weidu_log(
                state,
                crate::app::app_step2_log::resolve_bg2_weidu_log_path,
            )?,
            iwdee,
        });
    }
    match log_source {
        ExportLogSource::Rebuilt => {
            let (bgee, iwdee) = first_slot_export_fields(
                &state.step1.game_install,
                Some(rebuilt_weidu_log_text(&state.step3.bgee_items)),
            );
            Ok(ExportWeiduLogs {
                bgee,
                bg2ee: Some(rebuilt_weidu_log_text(&state.step3.bg2ee_items)),
                iwdee,
            })
        }
        ExportLogSource::Installed => Ok(installed_weidu_logs(state)),
    }
}

fn installed_weidu_logs(state: &WizardState) -> ExportWeiduLogs {
    match state.step1.game_install.as_str() {
        "EET" => ExportWeiduLogs {
            bgee: Some(installed_or_rebuilt_log(
                &state.step1.eet_pre_dir,
                &state.step3.bgee_items,
            )),
            bg2ee: Some(installed_or_rebuilt_log(
                &state.step1.eet_new_dir,
                &state.step3.bg2ee_items,
            )),
            iwdee: None,
        },
        "BG2EE" => ExportWeiduLogs {
            bgee: Some(rebuilt_weidu_log_text(&state.step3.bgee_items)),
            bg2ee: Some(installed_or_rebuilt_log(
                &state.step1.generate_directory,
                &state.step3.bg2ee_items,
            )),
            iwdee: None,
        },
        _ => {
            let (bgee, iwdee) = first_slot_export_fields(
                &state.step1.game_install,
                Some(installed_or_rebuilt_log(
                    &state.step1.generate_directory,
                    &state.step3.bgee_items,
                )),
            );
            ExportWeiduLogs {
                bgee,
                bg2ee: Some(rebuilt_weidu_log_text(&state.step3.bg2ee_items)),
                iwdee,
            }
        }
    }
}

fn installed_weidu_log_path(dir: &str) -> Option<PathBuf> {
    let dir = dir.trim();
    if dir.is_empty() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    entries.flatten().map(|entry| entry.path()).find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("weidu.log"))
    })
}

fn rebuilt_weidu_log_text(items: &[crate::app::state::Step3ItemState]) -> String {
    weidu_log_text(&build_weidu_export_lines(items))
}

fn installed_or_rebuilt_log(dir: &str, items: &[crate::app::state::Step3ItemState]) -> String {
    let Some(path) = installed_weidu_log_path(dir) else {
        warn!(
            target = "share",
            "no installed WeiDU log found in {}; falling back to rebuilt lines",
            dir.trim()
        );
        return rebuilt_weidu_log_text(items);
    };
    match fs::read_to_string(&path) {
        Ok(text) if !text.trim().is_empty() => reappend_wlb_inputs(&text, items),
        Ok(_) => {
            warn!(
                target = "share",
                "installed WeiDU log at {} is empty; falling back to rebuilt lines",
                path.display()
            );
            rebuilt_weidu_log_text(items)
        }
        Err(err) => {
            warn!(
                target = "share",
                "installed WeiDU log missing at {} ({err}); falling back to rebuilt lines",
                path.display()
            );
            rebuilt_weidu_log_text(items)
        }
    }
}

fn reappend_wlb_inputs(log_text: &str, items: &[crate::app::state::Step3ItemState]) -> String {
    let markers = collect_wlb_markers(items);
    if markers.is_empty() {
        return log_text.to_string();
    }
    log_text
        .lines()
        .map(|line| reappend_wlb_marker_on_line(line, &markers))
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_wlb_markers(
    items: &[crate::app::state::Step3ItemState],
) -> Vec<(String, String, String)> {
    let marker = "@wlb-inputs:";
    items
        .iter()
        .filter_map(|item| {
            let lower = item.raw_line.to_ascii_lowercase();
            let start = lower.find(marker)?;
            let value = item.raw_line[start + marker.len()..].trim();
            if value.is_empty() {
                return None;
            }
            Some((
                crate::app::mod_downloads::normalize_mod_download_tp2(&item.tp_file),
                item.component_id.trim().to_string(),
                value.to_string(),
            ))
        })
        .collect()
}

fn reappend_wlb_marker_on_line(line: &str, markers: &[(String, String, String)]) -> String {
    if line.to_ascii_lowercase().contains("@wlb-inputs:") {
        return line.to_string();
    }
    let Ok(component) = crate::mods::component::Component::parse_weidu_line(line) else {
        return line.to_string();
    };
    let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&component.tp_file);
    let component_id = component.component.trim().to_string();
    let Some((_, _, value)) = markers.iter().find(|(marker_tp2, marker_component, _)| {
        *marker_tp2 == tp2 && *marker_component == component_id
    }) else {
        return line.to_string();
    };
    format!("{line} // @wlb-inputs: {value}")
}

fn read_exact_source_weidu_log(
    state: &WizardState,
    resolve_path: fn(&crate::app::state::Step1State) -> Option<std::path::PathBuf>,
) -> Result<Option<String>, String> {
    let Some(path) = resolve_path(&state.step1) else {
        return Ok(None);
    };
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Read source WeiDU log failed ({}): {err}", path.display()))?;
    Ok(Some(text))
}

fn export_mod_config_files(
    state: &WizardState,
    installed_source_ids: &std::collections::BTreeMap<String, String>,
) -> Result<(Vec<ModlistShareConfigFile>, Vec<String>), String> {
    let sources = crate::app::mod_downloads::load_mod_download_sources();
    let mut exported = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut walked_tp2_paths = std::collections::BTreeSet::new();

    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        if mod_state
            .components
            .iter()
            .all(|component| !component.checked)
        {
            continue;
        }
        if !walked_tp2_paths.insert(normalized_tp2_path(&mod_state.tp2_path)) {
            continue;
        }
        let Some(source) =
            resolve_mod_config_source(state, &sources, installed_source_ids, &mod_state.tp_file)
        else {
            continue;
        };
        let Some(mod_root) = mod_config_root(&mod_state.tp2_path) else {
            continue;
        };
        let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&source.tp2);
        let mut missing = Vec::new();
        let mut invalid = Vec::new();
        for name in &source.config_files {
            let Ok(relative_path) =
                crate::app::modlist_config_files::validate_relative_config_path(name)
            else {
                invalid.push(name.clone());
                continue;
            };
            if crate::app::modlist_config_files::is_os_artifact_file(&relative_path) {
                continue;
            }
            let path = mod_root.join(&relative_path);
            if !path.is_file() {
                missing.push(name.clone());
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|err| format!("Read mod config failed ({}): {err}", path.display()))?;
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");
            let key = (
                tp2.clone(),
                source.source_id.trim().to_ascii_lowercase(),
                relative_path.clone(),
            );
            if seen.insert(key) {
                exported.push(ModlistShareConfigFile {
                    tp2: tp2.clone(),
                    source_id: source.source_id.clone(),
                    relative_path,
                    base64_data: base64url_encode(&bytes),
                });
            }
        }
        if !missing.is_empty() {
            warnings.push(format!(
                "{}: {} config file(s) named by its source are missing on disk: {}",
                mod_state.name,
                missing.len(),
                missing.join(", ")
            ));
        }
        if !invalid.is_empty() {
            warnings.push(format!(
                "{}: {} config file name(s) are invalid and were skipped: {}",
                mod_state.name,
                invalid.len(),
                invalid.join(", ")
            ));
        }
    }
    Ok((exported, warnings))
}

fn normalized_tp2_path(tp2_path: &str) -> String {
    tp2_path.trim().replace('\\', "/").to_ascii_lowercase()
}

fn resolve_mod_config_source(
    state: &WizardState,
    sources: &crate::app::mod_downloads::ModDownloadsLoad,
    installed_source_ids: &std::collections::BTreeMap<String, String>,
    tp_file: &str,
) -> Option<crate::app::mod_downloads::ModDownloadSource> {
    let selected_source_id = installed_source_id_for_mod(sources, installed_source_ids, tp_file)
        .or_else(|| {
            let key = crate::app::mod_downloads::normalize_mod_download_tp2(tp_file);
            state.step2.selected_source_ids.get(&key).cloned()
        });
    sources.resolve_source(tp_file, selected_source_id.as_deref())
}

fn installed_source_id_for_mod(
    sources: &crate::app::mod_downloads::ModDownloadsLoad,
    installed_source_ids: &std::collections::BTreeMap<String, String>,
    tp_file: &str,
) -> Option<String> {
    let key = crate::app::mod_downloads::normalize_mod_download_tp2(tp_file);
    if let Some(source_id) = installed_source_ids.get(&key) {
        return Some(source_id.clone());
    }
    for source in sources.find_sources(tp_file) {
        let key = crate::app::mod_downloads::normalize_mod_download_tp2(&source.tp2);
        if let Some(source_id) = installed_source_ids.get(&key) {
            return Some(source_id.clone());
        }
        for alias in &source.aliases {
            let key = crate::app::mod_downloads::normalize_mod_download_tp2(alias);
            if let Some(source_id) = installed_source_ids.get(&key) {
                return Some(source_id.clone());
            }
        }
    }
    None
}

fn mod_config_root(tp2_path: &str) -> Option<PathBuf> {
    Some(Path::new(tp2_path.trim()).parent()?.to_path_buf())
}

fn relevant_weidu_text_is_empty(
    state: &WizardState,
    first_game_text: Option<&str>,
    second_game_text: Option<&str>,
) -> bool {
    match state.step1.game_install.as_str() {
        "EET" => {
            weidu_text_has_no_entries(first_game_text)
                && weidu_text_has_no_entries(second_game_text)
        }
        "BG2EE" => weidu_text_has_no_entries(second_game_text),
        _ => weidu_text_has_no_entries(first_game_text),
    }
}

fn weidu_text_has_no_entries(text: Option<&str>) -> bool {
    text.is_none_or(|text| {
        text.lines().all(|line| {
            let line = line.trim();
            line.is_empty() || line.starts_with("//")
        })
    })
}

fn weidu_log_text(lines: &[String]) -> String {
    let header = [
        "// Log of Currently Installed WeiDU Mods",
        "// The top of the file is the 'oldest' mod",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
    ];
    let mut out = header
        .iter()
        .map(|line| (*line).to_string())
        .collect::<Vec<_>>();
    out.extend(lines.iter().cloned());
    out.join("\n")
}

pub(crate) fn commit_sha_from_installed_ref(installed_ref: &str) -> Option<String> {
    let trimmed = installed_ref.trim();
    let (_, sha) = trimmed.rsplit_once('@')?;
    if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(sha.to_string())
}

pub(crate) fn pin_source_to_installed_ref(
    source: &mut crate::app::mod_downloads::ModDownloadSource,
    installed_source_id: Option<&str>,
    installed_ref: Option<&str>,
) {
    let Some(installed_ref) = installed_ref else {
        return;
    };
    if source.github.is_some() {
        let id = installed_source_id.unwrap_or(&source.source_id);
        if let Some((pinned, _)) =
            crate::app::mod_source_history::bookmark_block(&*source, Some(id), Some(installed_ref))
        {
            *source = pinned;
        }
        return;
    }
    let Some(sha) = commit_sha_from_installed_ref(installed_ref) else {
        return;
    };
    source.commit = Some(sha);
    source.branch = None;
}

pub(crate) struct ResolvedSources {
    pub(crate) toml: Option<String>,
    pub(crate) unresolved: Vec<String>,
}

fn build_resolved_source_overrides(
    state: &WizardState,
    installed: &crate::app::app_step2_update_source_refs::ModSourceRefsFile,
) -> Result<ResolvedSources, String> {
    let default_text = crate::app::mod_downloads::default_mod_downloads_content();
    let user_text = fs::read_to_string(crate::app::mod_downloads::mod_downloads_user_path())
        .unwrap_or_default();
    let modlist_text = crate::app::mod_downloads::active_modlist_downloads_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default();
    build_resolved_source_overrides_from_texts(
        state,
        default_text,
        &user_text,
        &modlist_text,
        installed,
    )
}

fn installed_refs_for_export(
    lookup: &crate::app::app_step2_update_source_refs::InstalledRefLookup,
    include: &std::collections::BTreeSet<String>,
) -> crate::app::app_step2_update_source_refs::ModSourceRefsFile {
    use crate::app::app_step2_update_source_refs::{installed_source_refs_path, load_refs_file_at};

    installed_refs_folder_first(
        lookup,
        load_refs_file_at(&installed_source_refs_path()),
        include,
    )
}

fn installed_refs_folder_first(
    lookup: &crate::app::app_step2_update_source_refs::InstalledRefLookup,
    mut list: crate::app::app_step2_update_source_refs::ModSourceRefsFile,
    include: &std::collections::BTreeSet<String>,
) -> crate::app::app_step2_update_source_refs::ModSourceRefsFile {
    for tp2 in include {
        let Some(source_id) = lookup.source_id(tp2) else {
            continue;
        };
        list.sources.insert(tp2.clone(), source_id);
        match lookup.source_id_and_ref(tp2) {
            Some((_, source_ref)) => list.refs.insert(tp2.clone(), source_ref),
            None => list.refs.remove(tp2),
        };
    }
    list
}

pub(crate) fn build_resolved_source_overrides_from_texts(
    state: &WizardState,
    default_text: &str,
    user_text: &str,
    modlist_text: &str,
    refs_file: &crate::app::app_step2_update_source_refs::ModSourceRefsFile,
) -> Result<ResolvedSources, String> {
    use crate::app::app_step2_update_source_refs::installed_source_ids_from_refs_file;

    let source_load = crate::app::mod_downloads::load_mod_download_sources_from_texts(
        default_text,
        user_text,
        modlist_text,
    );
    let installed_ids = installed_source_ids_from_refs_file(refs_file);

    let mut toml_out = String::new();
    let mut unresolved = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        if !mod_state
            .components
            .iter()
            .any(|component| component.checked)
        {
            continue;
        }
        let normalized_tp2 =
            crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
        if !seen.insert(normalized_tp2.clone()) {
            continue;
        }
        let Some(mut source) =
            resolve_mod_config_source(state, &source_load, &installed_ids, &mod_state.tp_file)
        else {
            unresolved.push(display_mod_name(mod_state));
            continue;
        };
        let installed_ref = refs_file.refs.get(&normalized_tp2);
        pin_source_to_installed_ref(
            &mut source,
            installed_ids.get(&normalized_tp2).map(String::as_str),
            installed_ref.map(String::as_str),
        );
        let block = serialize_resolved_mod(&mod_state.tp_file, &mod_state.name, &source);
        if !toml_out.is_empty() {
            toml_out.push_str("\n\n");
        }
        toml_out.push_str(&block);
    }

    let toml = if toml_out.is_empty() {
        None
    } else {
        let full_text = format!("format = 2\n\n{toml_out}");
        toml::from_str::<crate::app::mod_downloads::ModDownloadsFile>(&full_text).map_err(
            |err| format!("resolved source overrides failed round-trip validation: {err}"),
        )?;
        Some(full_text)
    };

    Ok(ResolvedSources { toml, unresolved })
}

fn display_mod_name(mod_state: &crate::app::state::Step2ModState) -> String {
    let name = mod_state.name.trim();
    if name.is_empty() {
        mod_state.tp_file.trim().to_string()
    } else {
        name.to_string()
    }
}

fn serialize_resolved_mod(
    tp2: &str,
    label: &str,
    source: &crate::app::mod_downloads::ModDownloadSource,
) -> String {
    let name = if label.trim().is_empty() {
        tp2.trim()
    } else {
        label.trim()
    };
    let header = format!(
        "[[mods]]\nname = \"{}\"\ntp2 = \"{}\"",
        escape_for_toml(name),
        escape_for_toml(tp2.trim()),
    );
    let source_block = crate::app::mod_downloads::complete_source_block(source);
    format!("{header}\n\n{source_block}")
}

fn escape_for_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_per_modlist_installed_refs(
    state: &WizardState,
    lookup: &crate::app::app_step2_update_source_refs::InstalledRefLookup,
    include: &std::collections::BTreeSet<String>,
) -> Option<String> {
    refs_copy_text(
        lookup.refs_for_export(include),
        state.step2.selected_source_ids.clone(),
    )
}

fn installed_refs_copy_without_archives(path: &std::path::Path) -> Option<String> {
    let refs_file = crate::app::app_step2_update_source_refs::load_refs_file_at(path);
    refs_copy_text(refs_file.refs, refs_file.sources)
}

fn refs_copy_text(
    refs: std::collections::BTreeMap<String, String>,
    sources: std::collections::BTreeMap<String, String>,
) -> Option<String> {
    if refs.is_empty() && sources.is_empty() {
        return None;
    }
    let combined = crate::app::app_step2_update_source_refs::ModSourceRefsFile {
        folder: None,
        refs,
        sources,
        archives: std::collections::BTreeMap::new(),
    };
    toml::to_string_pretty(&combined)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn zlib_compress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(|err| err.to_string())?;
    encoder.finish().map_err(|err| err.to_string())
}

fn zlib_decompress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(bytes);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|err| {
        format!("Share code is not a supported compressed BIO modlist payload: {err}")
    })?;
    Ok(out)
}

pub(crate) fn base64url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        }
    }
    out
}

pub(crate) fn base64url_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut values = Vec::new();
    for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
        match ch {
            'A'..='Z' => values.push(ch as u8 - b'A'),
            'a'..='z' => values.push(ch as u8 - b'a' + 26),
            '0'..='9' => values.push(ch as u8 - b'0' + 52),
            '-' => values.push(62),
            '_' => values.push(63),
            _ => return Err("Share code contains invalid base64url characters.".to_string()),
        }
    }
    let remainder = values.len() % 4;
    if remainder == 1 {
        return Err("Share code base64url length is invalid.".to_string());
    }
    if remainder != 0 {
        values.extend(std::iter::repeat_n(64, 4 - remainder));
    }
    let mut out = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        let pad = usize::from(chunk[0] == 64)
            + usize::from(chunk[1] == 64)
            + usize::from(chunk[2] == 64)
            + usize::from(chunk[3] == 64);
        if pad > 2 || chunk[..4 - pad].contains(&64) {
            return Err("Share code base64 padding is invalid.".to_string());
        }
        let c0 = chunk[0];
        let c1 = chunk[1];
        let c2 = if chunk[2] == 64 { 0 } else { chunk[2] };
        let c3 = if chunk[3] == 64 { 0 } else { chunk[3] };
        out.push((c0 << 2) | (c1 >> 4));
        if pad < 2 {
            out.push(((c1 & 0b0000_1111) << 4) | (c2 >> 2));
        }
        if pad == 0 {
            out.push(((c2 & 0b0000_0011) << 6) | c3);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_one_bgee_component() -> WizardState {
        let mut state = WizardState::default();
        state.step3.bgee_items = vec![crate::app::state::Step3ItemState {
            tp_file: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "EEFixPack".to_string(),
            component_label: "Core Fixes".to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }];
        state
    }

    const FIELDLESS_PAYLOAD_JSON: &str = r#"{
        "format_version": 1,
        "bio_version": "0.1.0-test",
        "game_install": "BGEE",
        "install_mode": "start_from_scratch",
        "weidu_logs": { "bgee": "~MOD\\MOD.TP2~ #0 #0 // A component: 1.0" }
    }"#;

    #[test]
    fn absent_provenance_keys_parse_to_today_defaults() {
        let payload: ModlistSharePayload =
            serde_json::from_str(FIELDLESS_PAYLOAD_JSON).expect("fieldless payload must parse");
        assert!(
            payload.allow_auto_install,
            "absent allow_auto_install must default true"
        );
        assert_eq!(payload.name, None, "absent name must default None");
        assert_eq!(payload.author, None, "absent author must default None");
        assert_eq!(
            payload.description, None,
            "absent description must default None"
        );
        assert!(
            payload.forked_from.is_empty(),
            "absent forked_from must default empty"
        );
    }

    #[test]
    fn share_preview_projects_defaults_for_fieldless_code() {
        let payload: ModlistSharePayload =
            serde_json::from_str(FIELDLESS_PAYLOAD_JSON).expect("fieldless payload must parse");
        let preview = share_preview(&payload).expect("preview must build");
        assert!(preview.allow_auto_install);
        assert_eq!(preview.name, None);
        assert_eq!(preview.author, None);
        assert_eq!(preview.forked_from.len(), 0);
        assert_eq!(preview.game_install, "BGEE");
        assert_eq!(preview.bgee_entries, 1);
    }

    #[test]
    fn present_provenance_keys_are_surfaced_through_preview() {
        let json = r#"{
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD\\MOD.TP2~ #0 #0 // A component: 1.0" },
            "allow_auto_install": false,
            "name": "Born2BSalty's EET tactical playthrough",
            "author": "@b2bs",
            "forked_from": [
                { "name": "EET Basics", "author": "@olim" },
                { "name": "EET Tactical", "author": "@b2bs" }
            ]
        }"#;
        let payload: ModlistSharePayload =
            serde_json::from_str(json).expect("payload with provenance must parse");
        let preview = share_preview(&payload).expect("preview must build");
        assert!(
            !preview.allow_auto_install,
            "explicit false must be carried (draft-code gate)"
        );
        assert_eq!(
            preview.name.as_deref(),
            Some("Born2BSalty's EET tactical playthrough")
        );
        assert_eq!(preview.author.as_deref(), Some("@b2bs"));
        assert_eq!(
            preview.forked_from,
            vec![
                ForkAncestor {
                    name: "EET Basics".to_string(),
                    author: "@olim".to_string(),
                },
                ForkAncestor {
                    name: "EET Tactical".to_string(),
                    author: "@b2bs".to_string(),
                },
            ]
        );
    }

    #[test]
    fn fork_ancestor_serde_round_trips() {
        let original = ForkAncestor {
            name: "EET Basics".to_string(),
            author: "@olim".to_string(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let back: ForkAncestor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, back);
    }

    #[test]
    fn export_share_code_bakes_current_modlist_provenance() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = ConfigDirGuard::new("provenance");
        let mut state = state_with_one_bgee_component();
        state.set_modlist_share_provenance(
            Some("  Tactical EET 2026  ".to_string()),
            Some("  @b2bs  ".to_string()),
            None,
            vec![ForkAncestor {
                name: "Root build".to_string(),
                author: "@root".to_string(),
            }],
        );

        let code = export_modlist_share_code(&state).expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");

        assert_eq!(preview.name.as_deref(), Some("Tactical EET 2026"));
        assert_eq!(preview.author.as_deref(), Some("@b2bs"));
        assert_eq!(
            preview.forked_from,
            vec![ForkAncestor {
                name: "Root build".to_string(),
                author: "@root".to_string(),
            }]
        );
    }

    #[test]
    fn export_bakes_the_description_when_set() {
        let mut state = state_with_one_bgee_component();
        state.set_modlist_share_provenance(
            None,
            None,
            Some("  BG2EE with the fixpack  ".to_string()),
            Vec::new(),
        );

        let code =
            export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");

        assert_eq!(
            preview.description.as_deref(),
            Some("BG2EE with the fixpack")
        );
    }

    #[test]
    fn blank_description_is_omitted() {
        let mut state = state_with_one_bgee_component();
        state.set_modlist_share_provenance(None, None, Some("   ".to_string()), Vec::new());

        let code =
            export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");

        assert_eq!(preview.description, None);
    }

    #[test]
    fn export_with_empty_sources_omits_overrides_and_refs() {
        let state = state_with_one_bgee_component();
        let code =
            export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");
        assert!(!preview.has_source_overrides);
        assert!(!preview.has_installed_refs);
    }

    #[test]
    fn export_with_supplied_sources_embeds_them() {
        let state = state_with_one_bgee_component();
        let sources = ShareExportSources {
            mod_downloads_user: Some("[[mods]]\nname = \"X\"".to_string()),
            mod_installed_refs: Some("[sources]\nx = \"y\"".to_string()),
            ..ShareExportSources::default()
        };
        let code = export_modlist_share_code_with(&state, &sources).expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");
        assert!(preview.has_source_overrides);
        assert!(preview.has_installed_refs);
        assert!(
            preview
                .source_overrides_text
                .contains("[[mods]]\nname = \"X\"")
        );
        assert!(preview.installed_refs_text.contains("[sources]\nx = \"y\""));
    }

    struct GameVersionFixture {
        path: std::path::PathBuf,
    }

    impl GameVersionFixture {
        fn new(label: &str, patch: &[u8]) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "bio_modlist_share_game_version_{label}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("chitin.key"), patch).unwrap();
            Self { path: root }
        }
    }

    impl Drop for GameVersionFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn export_writes_the_game_version_when_the_install_copy_reads_2_6() {
        let fixture = GameVersionFixture::new("writes_2_6", b"data/PATCH26.BIF");
        let mut state = state_with_one_bgee_component();
        state.step3.bg2ee_items = state.step3.bgee_items.clone();
        state.step1.game_install = "BG2EE".to_string();
        state.step1.generate_directory = fixture.path.to_string_lossy().to_string();

        let code =
            export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");
        let payload = decode_share_payload(&code).expect("decode");

        assert_eq!(payload.game_version.as_deref(), Some("2.6"));
    }

    #[test]
    fn export_omits_the_game_version_without_a_readable_folder() {
        let mut state = state_with_one_bgee_component();
        state.step3.bg2ee_items = state.step3.bgee_items.clone();
        state.step1.game_install = "BG2EE".to_string();
        state.step1.generate_directory = String::new();

        let code =
            export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");
        let payload = decode_share_payload(&code).expect("decode");

        assert_eq!(payload.game_version, None);
    }

    #[test]
    fn preview_surfaces_the_tag_and_blank_is_none() {
        let payload: ModlistSharePayload = serde_json::from_str(
            r#"{
                "format_version": 1,
                "game_install": "BGEE",
                "game_version": "2.6",
                "install_mode": "start_from_scratch",
                "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
            }"#,
        )
        .expect("payload parses");
        let preview = share_preview(&payload).expect("preview");
        assert_eq!(preview.game_version.as_deref(), Some("2.6"));

        let blank_payload: ModlistSharePayload = serde_json::from_str(
            r#"{
                "format_version": 1,
                "game_install": "BGEE",
                "game_version": "   ",
                "install_mode": "start_from_scratch",
                "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" }
            }"#,
        )
        .expect("blank payload parses");
        let blank_preview = share_preview(&blank_payload).expect("preview");
        assert_eq!(blank_preview.game_version, None);
    }

    #[test]
    fn untagged_code_previews_none() {
        let payload: ModlistSharePayload =
            serde_json::from_str(FIELDLESS_PAYLOAD_JSON).expect("fieldless payload must parse");
        let preview = share_preview(&payload).expect("preview");
        assert_eq!(preview.game_version, None);
    }

    struct AmbientGuard(Option<std::path::PathBuf>);
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

    struct ConfigDirGuard(std::path::PathBuf);
    impl ConfigDirGuard {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_modlist_share_config_dir_test_{}_{}_{label}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }
    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn github_source_multi_exact() -> crate::app::mod_downloads::ModDownloadSource {
        crate::app::mod_downloads::ModDownloadSource {
            name: "TestMod".to_string(),
            tp2: "testmod".to_string(),
            source_id: "main".to_string(),
            source_label: "Main".to_string(),
            url: "https://github.com/A/B".to_string(),
            github: Some("A/B".to_string()),
            exact_github: vec!["A/B@v1".to_string(), "A/B@v2".to_string()],
            tag: Some("v16".to_string()),
            ..Default::default()
        }
    }

    fn url_source() -> crate::app::mod_downloads::ModDownloadSource {
        crate::app::mod_downloads::ModDownloadSource {
            name: "UrlMod".to_string(),
            tp2: "urlmod".to_string(),
            source_id: "weasel".to_string(),
            source_label: "Weasel".to_string(),
            url: "https://example.com/mod.zip".to_string(),
            github: None,
            tag: Some("v3".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn export_serializer_emits_exact_github_as_array() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let source = github_source_multi_exact();
        let block = crate::app::mod_downloads::complete_source_block(&source);

        assert!(
            block.contains("exact_github = ["),
            "exact_github must be emitted as TOML array: {block}"
        );
        assert!(
            !block.contains("\nexact_github = \""),
            "must not emit repeated scalar exact_github lines"
        );

        let wrapped = format!("[[mods]]\nname = \"T\"\ntp2 = \"t\"\n\n{block}");
        let parsed = toml::from_str::<crate::app::mod_downloads::ModDownloadsFile>(&wrapped);
        assert!(
            parsed.is_ok(),
            "serialized block must round-trip: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn export_serializer_non_github_source_url_no_repo() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let source = url_source();
        let block = crate::app::mod_downloads::complete_source_block(&source);

        assert!(block.contains("url = \""), "url source must emit url field");
        assert!(
            block.contains("repo = \"\""),
            "a complete block still lists repo, blank when there is none"
        );

        let wrapped = format!("[[mods]]\nname = \"U\"\ntp2 = \"u\"\n\n{block}");
        let parsed = toml::from_str::<crate::app::mod_downloads::ModDownloadsFile>(&wrapped);
        assert!(parsed.is_ok(), "url source block must round-trip");
    }

    fn default_tier_toml(tp2: &str, url: &str) -> String {
        format!("[[mods]]\nname = \"{tp2}\"\ntp2 = \"{tp2}\"\nurl = \"{url}\"\n")
    }

    #[test]
    fn default_tier_mod_is_written_into_the_code() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        assert!(
            crate::app::mod_downloads::active_modlist_downloads_path().is_none(),
            "precondition: ambient is None"
        );

        let default_text = default_tier_toml("testmod", "https://github.com/T/M");
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![make_step2_mod("testmod", "TestMod")];

        let resolved =
            build_resolved_source_overrides_from_texts(&state, &default_text, "", "", &no_refs())
                .expect("resolve");
        assert!(
            resolved
                .toml
                .as_deref()
                .is_some_and(|toml| toml.contains("\"testmod\"")),
            "a mod known only to the default tier must get a source block: {:?}",
            resolved.toml
        );
        assert_eq!(resolved.unresolved.len(), 0);
    }

    #[test]
    fn unresolvable_mod_is_listed_by_name_and_omitted_from_sources() {
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![make_step2_mod("nosource", "NoSourceMod")];

        let resolved = build_resolved_source_overrides_from_texts(&state, "", "", "", &no_refs())
            .expect("resolve must not error");
        assert!(
            resolved.toml.is_none(),
            "a mod no tier knows must not produce a source block"
        );
        assert_eq!(resolved.unresolved, vec!["NoSourceMod".to_string()]);
    }

    #[test]
    fn unchecked_mods_are_neither_exported_nor_unresolved() {
        let default_text = default_tier_toml("checkedmod", "https://github.com/T/M");
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            make_step2_mod("checkedmod", "CheckedMod"),
            make_unchecked_step2_mod("uncheckedmod", "UncheckedMod"),
        ];

        let resolved =
            build_resolved_source_overrides_from_texts(&state, &default_text, "", "", &no_refs())
                .expect("resolve");
        let toml_out = resolved.toml.expect("checked mod produced a source block");
        assert!(
            toml_out.contains("\"checkedmod\""),
            "checked mod must be exported: {toml_out}"
        );
        assert!(
            !toml_out.contains("\"uncheckedmod\""),
            "unchecked mod must not be exported: {toml_out}"
        );
        assert!(
            resolved.unresolved.is_empty(),
            "unchecked mod must not be counted as unresolved: {:?}",
            resolved.unresolved
        );
    }

    #[test]
    fn unresolved_mods_round_trip_through_the_payload_and_preview() {
        let sources = ShareExportSources {
            unresolved_mods: vec!["NoSourceMod".to_string()],
            ..ShareExportSources::default()
        };
        let code = export_modlist_share_code_with(&state_with_one_bgee_component(), &sources)
            .expect("export");
        let preview = preview_modlist_share_code(&code).expect("preview");
        assert_eq!(preview.unresolved_mods, vec!["NoSourceMod".to_string()]);
    }

    #[test]
    fn empty_unresolved_mods_key_is_omitted_from_the_payload() {
        let code = export_modlist_share_code_with(
            &state_with_one_bgee_component(),
            &ShareExportSources::default(),
        )
        .expect("export");
        let payload = decode_share_payload(&code).expect("decode");
        assert_eq!(payload.source_overrides.unresolved_mods.len(), 0);
    }

    #[test]
    fn empty_mods_list_yields_no_overrides_and_no_unresolved() {
        let mut state = WizardState::default();
        state.step3.bgee_items = vec![];
        state.step2.bgee_mods = vec![];

        let result = build_resolved_source_overrides_from_texts(&state, "", "", "", &no_refs());
        assert!(result.is_ok(), "empty mods list must not error");
        let resolved = result.unwrap();
        assert!(
            resolved.toml.is_none(),
            "empty mods list must yield None source overrides"
        );
        assert_eq!(resolved.unresolved.len(), 0);
    }

    fn count_mods_blocks_for_tp2(toml_out: &str, tp2: &str) -> usize {
        let target = crate::app::mod_downloads::normalize_mod_download_tp2(tp2);
        let mut count = 0usize;
        let mut in_block = false;
        let mut block_matched = false;
        for line in toml_out.lines() {
            let trimmed = line.trim();
            if trimmed == "[[mods]]" {
                if in_block && block_matched {
                    count += 1;
                }
                in_block = true;
                block_matched = false;
            } else if in_block
                && trimmed.starts_with("tp2")
                && let Some(val) = trimmed
                    .strip_prefix("tp2")
                    .and_then(|r| r.trim_start().strip_prefix('='))
            {
                let val = val.trim().trim_matches('"');
                if crate::app::mod_downloads::normalize_mod_download_tp2(val) == target {
                    block_matched = true;
                }
            }
        }
        if in_block && block_matched {
            count += 1;
        }
        count
    }

    fn checked_component(id: &str) -> crate::app::state::Step2ComponentState {
        crate::app::state::Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: true,
            selected_order: Some(1),
        }
    }

    fn make_step2_mod(tp_file: &str, name: &str) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: name.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("{tp_file}/{tp_file}.tp2"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: true,
            hidden_components: Vec::new(),
            components: vec![checked_component("0")],
        }
    }

    fn make_unchecked_step2_mod(tp_file: &str, name: &str) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            checked: false,
            components: Vec::new(),
            ..make_step2_mod(tp_file, name)
        }
    }

    fn per_modlist_source_toml(tp2: &str) -> String {
        format!(
            "[[mods]]\nname = \"{tp2}\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  url = \"https://github.com/T/M\"\n  repo = \"T/M\"\n  tag = \"v1\"\n  default = true\n"
        )
    }

    #[test]
    fn export_dedup_eet_both_tabs_emit_each_mod_once() {
        let modlist_text = per_modlist_source_toml("cdtweaks");

        let mut state = WizardState::default();
        let mod_entry = make_step2_mod("cdtweaks", "cdtweaks");
        state.step2.bgee_mods = vec![mod_entry.clone()];
        state.step2.bg2ee_mods = vec![mod_entry];

        let result =
            build_resolved_source_overrides_from_texts(&state, "", "", &modlist_text, &no_refs());
        assert!(result.is_ok(), "build must not error: {:?}", result.err());

        let toml_out = result
            .unwrap()
            .toml
            .expect("resolved overrides must produce Some output");

        let block_count = count_mods_blocks_for_tp2(&toml_out, "cdtweaks");
        assert_eq!(
            block_count, 1,
            "EET dual-tab mod must appear exactly once in export; got {block_count}:\n{toml_out}"
        );
    }

    #[test]
    fn export_dedup_single_game_tab_is_noop() {
        let modlist_text = per_modlist_source_toml("testmod");

        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![make_step2_mod("testmod", "TestMod")];
        state.step2.bg2ee_mods = vec![];

        let result =
            build_resolved_source_overrides_from_texts(&state, "", "", &modlist_text, &no_refs());
        assert!(result.is_ok(), "build must not error: {:?}", result.err());

        let toml_out = result
            .unwrap()
            .toml
            .expect("single-game must produce Some output");

        let block_count = count_mods_blocks_for_tp2(&toml_out, "testmod");
        assert_eq!(
            block_count, 1,
            "single-game mod must appear exactly once; got {block_count}:\n{toml_out}"
        );
    }

    #[test]
    fn installed_branch_ref_becomes_a_commit_pin() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            branch: Some("master".to_string()),
            ..Default::default()
        };
        pin_source_to_installed_ref(
            &mut source,
            None,
            Some("master@7649ced6cd25865874d787ec1a9abbc67b068729"),
        );
        assert_eq!(
            source.commit.as_deref(),
            Some("7649ced6cd25865874d787ec1a9abbc67b068729")
        );
        assert_eq!(source.branch, None);
    }

    #[test]
    fn installed_commit_ref_becomes_a_commit_pin() {
        let mut source = crate::app::mod_downloads::ModDownloadSource::default();
        pin_source_to_installed_ref(
            &mut source,
            None,
            Some("commit@bfd167f7a52dfa6c9e694955a074a85991b0c358"),
        );
        assert_eq!(
            source.commit.as_deref(),
            Some("bfd167f7a52dfa6c9e694955a074a85991b0c358")
        );
        assert_eq!(source.branch, None);
    }

    #[test]
    fn tag_source_keeps_its_tag() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            tag: Some("v1.2.0".to_string()),
            ..Default::default()
        };
        pin_source_to_installed_ref(&mut source, None, Some("v1.2.0"));
        assert_eq!(source.tag.as_deref(), Some("v1.2.0"));
        assert_eq!(source.commit, None);
        assert_eq!(source.branch, None);
    }

    #[test]
    fn tag_with_at_sign_is_not_a_commit_pin() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            tag: Some("mymod@1.0".to_string()),
            ..Default::default()
        };
        pin_source_to_installed_ref(&mut source, None, Some("mymod@1.0"));
        assert_eq!(source.tag.as_deref(), Some("mymod@1.0"));
        assert_eq!(source.commit, None);
        assert_eq!(source.branch, None);
    }

    #[test]
    fn missing_ref_leaves_the_source_untouched() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            branch: Some("master".to_string()),
            ..Default::default()
        };
        pin_source_to_installed_ref(&mut source, None, None);
        assert_eq!(source.branch.as_deref(), Some("master"));
        assert_eq!(source.commit, None);
    }

    fn no_refs() -> crate::app::app_step2_update_source_refs::ModSourceRefsFile {
        crate::app::app_step2_update_source_refs::ModSourceRefsFile::default()
    }

    struct FolderRefsRoot(std::path::PathBuf);

    impl FolderRefsRoot {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let root = Self(std::env::temp_dir().join(format!(
                "bio_folderrefs_share_test_{}_{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            )));
            fs::create_dir_all(&root.0).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(root.0.clone()));
            root
        }

        fn dir(&self, name: &str) -> std::path::PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            dir
        }
    }

    impl Drop for FolderRefsRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn share_pins_fall_back_to_the_folder_record() {
        use crate::app::app_step2_update_source_refs::{
            InstalledRefLookup, installed_source_refs_path, mods_folder_refs_path,
        };
        const SHA: &str = "7649ced6cd25865874d787ec1a9abbc67b068729";

        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        let root = FolderRefsRoot::new();
        let list_dir = root.dir("list");
        crate::app::mod_downloads::set_active_modlist_dir(Some(list_dir.clone()));
        fs::write(
            list_dir.join("mod_downloads_user.toml"),
            "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/ModA\"\n  repo = \"Owner/ModA\"\n  branch = \"master\"\n  default = true\n",
        )
        .unwrap();
        assert!(installed_source_refs_path().starts_with(&list_dir));
        assert!(!installed_source_refs_path().exists());

        let mods = root.dir("mods").to_string_lossy().into_owned();
        let folder_path = mods_folder_refs_path(&mods).unwrap();
        assert!(folder_path.starts_with(&root.0));
        fs::create_dir_all(folder_path.parent().unwrap()).unwrap();
        fs::write(
            &folder_path,
            format!("[refs]\nmoda = \"master@{SHA}\"\n\n[sources]\nmoda = \"main\"\n"),
        )
        .unwrap();

        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![make_step2_mod("moda", "ModA")];

        let include = checked_mod_tp2s(&state);
        let unpinned = build_resolved_source_overrides(
            &state,
            &installed_refs_for_export(
                &InstalledRefLookup::load(state.step1.mods_folder.trim()),
                &include,
            ),
        )
        .expect("resolve")
        .toml
        .expect("a resolved mod produces text");
        assert!(!unpinned.contains(SHA), "{unpinned}");

        state.step1.mods_folder = mods;
        let lookup = InstalledRefLookup::load(state.step1.mods_folder.trim());
        let pinned =
            build_resolved_source_overrides(&state, &installed_refs_for_export(&lookup, &include))
                .expect("resolve")
                .toml
                .expect("a resolved mod produces text");
        assert!(pinned.contains(&format!("commit = \"{SHA}\"")), "{pinned}");
        let refs_copy =
            build_per_modlist_installed_refs(&state, &lookup, &include).expect("refs copy");
        assert!(
            refs_copy.contains(&format!("moda = \"master@{SHA}\"")),
            "{refs_copy}"
        );
    }

    #[test]
    fn share_export_takes_folder_records_only_for_checked_mods() {
        use crate::app::app_step2_update_source_refs::{InstalledRefLookup, parse_refs_file_text};

        let list_text = "[refs]\nmodc = \"v3\"\n\n[sources]\nmodc = \"main\"\n";
        let folder = parse_refs_file_text(
            "[refs]\nmoda = \"master@aaaaaaa\"\nmodb = \"master@bbbbbbb\"\n\n[sources]\nmoda = \"main\"\nmodb = \"main\"\n",
        );
        let lookup = InstalledRefLookup::from_files(Some(folder), parse_refs_file_text(list_text));

        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            make_step2_mod("moda", "ModA"),
            make_unchecked_step2_mod("modb", "ModB"),
        ];
        let include = checked_mod_tp2s(&state);

        let refs_copy =
            build_per_modlist_installed_refs(&state, &lookup, &include).expect("refs copy");
        assert!(
            refs_copy.contains("moda = \"master@aaaaaaa\""),
            "{refs_copy}"
        );
        assert!(!refs_copy.contains("modb"), "{refs_copy}");
        assert!(refs_copy.contains("modc = \"v3\""), "{refs_copy}");

        let installed =
            installed_refs_folder_first(&lookup, parse_refs_file_text(list_text), &include);
        assert_eq!(
            installed.sources.get("moda").map(String::as_str),
            Some("main")
        );
        assert!(!installed.sources.contains_key("modb"));
        assert!(!installed.refs.contains_key("modb"));
        assert_eq!(installed.refs.get("modc").map(String::as_str), Some("v3"));
    }

    struct RefsCopyRoot(std::path::PathBuf);

    impl Drop for RefsCopyRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn global_refs_copy_never_carries_archive_records() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = RefsCopyRoot(std::env::temp_dir().join(format!(
            "bio_refs_copy_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir_all(&root.0).unwrap();
        let path = root.0.join("mod_installed_refs.toml");
        fs::write(
            &path,
            "[refs]\ncdtweaks = \"v19\"\n\n[sources]\ncdtweaks = \"gibberlings3\"\n\n\
             [archives.cdtweaks]\nname = \"cdtweaks__gibberlings3__v19.zip\"\nsize = 12\n\
             hash = \"abc\"\n",
        )
        .unwrap();

        let copy = installed_refs_copy_without_archives(&path).expect("refs copy");
        assert!(copy.contains("cdtweaks = \"v19\""));
        assert!(copy.contains("cdtweaks = \"gibberlings3\""));
        assert!(!copy.contains("archives"), "got: {copy}");
        assert_eq!(
            installed_refs_copy_without_archives(&root.0.join("none.toml")),
            None
        );
    }

    #[test]
    fn serialized_block_carries_the_pinned_commit_and_no_branch() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            source_id: "gibberlings3".to_string(),
            source_label: "Gibberlings3".to_string(),
            url: "https://github.com/Gibberlings3/Tweaks-Anthology".to_string(),
            github: Some("Gibberlings3/Tweaks-Anthology".to_string()),
            branch: Some("master".to_string()),
            ..Default::default()
        };
        pin_source_to_installed_ref(
            &mut source,
            None,
            Some("master@7649ced6cd25865874d787ec1a9abbc67b068729"),
        );
        let block = crate::app::mod_downloads::complete_source_block(&source);
        assert!(block.contains("commit = \"7649ced6cd25865874d787ec1a9abbc67b068729\""));
        assert!(
            block.contains("branch = \"\""),
            "a pinned commit clears branch to blank"
        );
    }

    fn github_pin_source() -> crate::app::mod_downloads::ModDownloadSource {
        crate::app::mod_downloads::ModDownloadSource {
            github: Some("owner/repo".into()),
            source_id: "primary".into(),
            ..Default::default()
        }
    }

    #[test]
    fn channel_source_pins_the_installed_release() {
        let mut source = github_pin_source();
        pin_source_to_installed_ref(&mut source, None, Some("v19"));
        assert_eq!(source.release.as_deref(), Some("v19"));
        assert_eq!(source.channel, None);
        assert_eq!(source.tag, None);
        assert_eq!(source.branch, None);
        assert_eq!(source.commit, None);
    }

    #[test]
    fn pre_release_channel_pins_the_installed_release_keeping_asset() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            channel: Some("pre-release".into()),
            asset: Some("mod-win.zip".into()),
            ..github_pin_source()
        };
        pin_source_to_installed_ref(&mut source, None, Some("v3.1"));
        assert_eq!(source.release.as_deref(), Some("v3.1"));
        assert_eq!(source.asset.as_deref(), Some("mod-win.zip"));
        assert_eq!(source.channel, None);
    }

    #[test]
    fn release_source_pins_the_installed_release() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            release: Some("v18".into()),
            ..github_pin_source()
        };
        pin_source_to_installed_ref(&mut source, None, Some("v19"));
        assert_eq!(source.release.as_deref(), Some("v19"));
    }

    #[test]
    fn github_branch_ref_becomes_a_commit_pin_and_drops_the_asset() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            branch: Some("master".into()),
            asset: Some("x.zip".into()),
            ..github_pin_source()
        };
        pin_source_to_installed_ref(
            &mut source,
            None,
            Some("master@7649ced6cd25865874d787ec1a9abbc67b068729"),
        );
        assert_eq!(
            source.commit.as_deref(),
            Some("7649ced6cd25865874d787ec1a9abbc67b068729")
        );
        assert_eq!(source.branch, None);
        assert_eq!(source.asset, None);
    }

    #[test]
    fn github_tag_source_keeps_a_tag() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            tag: Some("v1.2.0".into()),
            ..github_pin_source()
        };
        pin_source_to_installed_ref(&mut source, None, Some("v1.2.0"));
        assert_eq!(source.tag.as_deref(), Some("v1.2.0"));
        assert_eq!(source.release, None);
    }

    #[test]
    fn page_archive_source_with_a_bare_tag_stays_unpinned() {
        let original = crate::app::mod_downloads::ModDownloadSource {
            source_id: "weasel".into(),
            url: "https://example.test/mod.zip".into(),
            ..Default::default()
        };
        let mut source = original.clone();
        pin_source_to_installed_ref(&mut source, None, Some("1.2"));
        assert_eq!(source, original);
    }

    #[test]
    fn mismatched_source_id_leaves_the_source_untouched() {
        let original = github_pin_source();
        let mut source = original.clone();
        pin_source_to_installed_ref(&mut source, Some("other"), Some("v19"));
        assert_eq!(source, original);
    }

    #[test]
    fn missing_source_id_assumes_the_current_source() {
        let mut source = github_pin_source();
        pin_source_to_installed_ref(&mut source, None, Some("v19"));
        assert_eq!(source.release.as_deref(), Some("v19"));
    }

    #[test]
    fn serialized_block_carries_the_pinned_release_and_no_channel() {
        let mut source = crate::app::mod_downloads::ModDownloadSource {
            url: "https://github.com/owner/repo".into(),
            ..github_pin_source()
        };
        pin_source_to_installed_ref(&mut source, None, Some("v19"));
        let block = crate::app::mod_downloads::complete_source_block(&source);
        assert!(block.contains("release = \"v19\""), "{block}");
        assert!(
            block
                .lines()
                .filter(|line| line.trim_start().starts_with("channel"))
                .all(|line| line.trim_end().ends_with("\"\"")),
            "{block}"
        );
    }

    #[test]
    fn import_without_an_active_list_leaves_the_global_sources_untouched() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        assert!(
            crate::app::mod_downloads::active_modlist_downloads_path().is_none(),
            "no active list ⇒ the import write target is None, so the source \
             overrides write is skipped and the global mod_downloads_user.toml \
             is never touched"
        );
    }

    #[test]
    fn installed_logs_are_preferred_when_present() {
        let mut state = WizardState::default();
        state.step1.game_install = "EET".to_string();

        let eet_pre_dir =
            std::env::temp_dir().join(format!("bio_share_installed_pre_{}", std::process::id()));
        let eet_new_dir =
            std::env::temp_dir().join(format!("bio_share_installed_new_{}", std::process::id()));
        let first_game_order_log_dir =
            std::env::temp_dir().join(format!("bio_share_order_bgee_{}", std::process::id()));
        let second_game_order_log_dir =
            std::env::temp_dir().join(format!("bio_share_order_bg2ee_{}", std::process::id()));
        std::fs::create_dir_all(&eet_pre_dir).unwrap();
        std::fs::create_dir_all(&eet_new_dir).unwrap();
        std::fs::create_dir_all(&first_game_order_log_dir).unwrap();
        std::fs::create_dir_all(&second_game_order_log_dir).unwrap();
        std::fs::write(
            eet_pre_dir.join("WeiDU.log"),
            "~EET/EET.TP2~ #0 #0 // EET core: v14.0\n",
        )
        .unwrap();
        std::fs::write(
            eet_new_dir.join("WeiDU.log"),
            "~A/A.TP2~ #0 #0 // Some component: v1.0\n",
        )
        .unwrap();
        std::fs::write(
            first_game_order_log_dir.join("weidu.log"),
            "~EET/EET.TP2~ #0 #0 // EET core\n",
        )
        .unwrap();
        std::fs::write(
            second_game_order_log_dir.join("weidu.log"),
            "~A/A.TP2~ #0 #0 // Some component\n",
        )
        .unwrap();
        state.step1.eet_pre_dir = eet_pre_dir.to_string_lossy().to_string();
        state.step1.eet_new_dir = eet_new_dir.to_string_lossy().to_string();
        state.step1.eet_bgee_log_folder = first_game_order_log_dir.to_string_lossy().to_string();
        state.step1.eet_bg2ee_log_folder = second_game_order_log_dir.to_string_lossy().to_string();
        state.step3.bg2ee_items = vec![crate::app::state::Step3ItemState {
            tp_file: "A.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "A".to_string(),
            component_label: "Some component".to_string(),
            raw_line: "~A/A.TP2~ #0 #0 // Some component: v1.0 // @wlb-inputs: y".to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }];

        let logs = export_weidu_logs(&state, ExportLogSource::Installed).expect("export logs");
        let bg2ee = logs.bg2ee.expect("bg2ee log present");
        assert!(bg2ee.contains(": v1.0"), "version tail survives: {bg2ee}");
        assert!(
            bg2ee.contains("@wlb-inputs: y"),
            "wlb-inputs marker re-appended: {bg2ee}"
        );

        let _ = std::fs::remove_dir_all(&eet_pre_dir);
        let _ = std::fs::remove_dir_all(&eet_new_dir);
        let _ = std::fs::remove_dir_all(&first_game_order_log_dir);
        let _ = std::fs::remove_dir_all(&second_game_order_log_dir);
    }

    #[test]
    fn single_game_installed_log_goes_to_that_games_key() {
        let mut state = WizardState::default();
        state.step1.game_install = "BG2EE".to_string();

        let generate_dir =
            std::env::temp_dir().join(format!("bio_share_installed_gen_{}", std::process::id()));
        std::fs::create_dir_all(&generate_dir).unwrap();
        std::fs::write(
            generate_dir.join("WeiDU.log"),
            "~B/B.TP2~ #0 #0 // Some component: v2.0\n",
        )
        .unwrap();
        state.step1.generate_directory = generate_dir.to_string_lossy().to_string();
        state.step3.bg2ee_items = vec![crate::app::state::Step3ItemState {
            tp_file: "B.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "B".to_string(),
            component_label: "Some component".to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }];

        let logs = export_weidu_logs(&state, ExportLogSource::Installed).expect("export logs");
        let second_game_text = logs.bg2ee.expect("bg2ee log present");
        assert!(
            second_game_text.contains(": v2.0"),
            "bg2ee carries the tail: {second_game_text}"
        );
        let first_game_text = logs.bgee.expect("bgee log present");
        assert!(
            !first_game_text.contains(": v2.0"),
            "bgee must not read the other game's folder: {first_game_text}"
        );

        let _ = std::fs::remove_dir_all(&generate_dir);
    }

    #[test]
    fn missing_installed_log_falls_back_to_rebuilt_lines() {
        let state = state_with_one_bgee_component();
        let logs = export_weidu_logs(&state, ExportLogSource::Installed).expect("export logs");
        let bgee = logs.bgee.expect("rebuilt fallback present");
        assert!(
            bgee.contains("EEFixPack"),
            "fallback rebuilt from step3: {bgee}"
        );
    }

    #[test]
    fn rebuilt_is_the_default() {
        assert_eq!(ExportLogSource::default(), ExportLogSource::Rebuilt);
        assert_eq!(
            ShareExportSources::default().log_source,
            ExportLogSource::Rebuilt
        );
    }

    fn decode_payload_json(code: &str) -> serde_json::Value {
        let encoded = code
            .trim()
            .strip_prefix(SHARE_CODE_PREFIX)
            .expect("share code prefix");
        let bytes = base64url_decode(encoded).expect("base64 decode");
        let bytes = zlib_decompress(&bytes).expect("zlib decode");
        serde_json::from_slice(&bytes).expect("json parse")
    }

    #[test]
    fn iwdee_export_writes_the_iwdee_key() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = ConfigDirGuard::new("iwdee");
        let mut state = state_with_one_bgee_component();
        state.step1.game_install = "IWDEE".to_string();

        let code = export_modlist_share_code(&state).expect("export");
        let json = decode_payload_json(&code);
        let logs = json.get("weidu_logs").expect("weidu_logs object");

        assert!(
            logs.get("iwdee")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|text| text.contains("EEFIXPACK")),
            "iwdee key must carry the first-slot log text: {logs:?}"
        );
        assert!(
            logs.get("bgee").is_none(),
            "bgee key must be absent for an IWDEE export: {logs:?}"
        );
    }

    #[test]
    fn old_iwdee_code_with_bgee_key_still_imports() {
        let json = r#"{
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "IWDEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD\\MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#;
        let code = encode_share_payload_text(json).expect("encode");
        let preview = preview_modlist_share_code(&code).expect("preview");

        assert_eq!(preview.game_install, "IWDEE");
        assert_eq!(preview.bgee_entries, 1);
        assert!(preview.bgee_log_text.contains("A component"));
    }

    #[test]
    fn new_iwdee_code_with_iwdee_key_imports() {
        let json = r#"{
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "IWDEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "iwdee": "~MOD\\MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#;
        let code = encode_share_payload_text(json).expect("encode");
        let preview = preview_modlist_share_code(&code).expect("preview");

        assert_eq!(preview.game_install, "IWDEE");
        assert_eq!(preview.bgee_entries, 1);
        assert!(preview.bgee_log_text.contains("A component"));
    }

    #[test]
    fn a_missing_side_stays_null_unless_the_list_is_iwdee() {
        let mut untouched = json!({ "weidu_logs": { "bgee": "first", "bg2ee": null } });
        move_first_slot_log_to_iwdee_key(&mut untouched, None);
        assert_eq!(
            untouched,
            json!({ "weidu_logs": { "bgee": "first", "bg2ee": null } })
        );

        let mut moved = json!({ "weidu_logs": { "bgee": null, "bg2ee": null } });
        move_first_slot_log_to_iwdee_key(&mut moved, Some("order"));
        assert_eq!(
            moved,
            json!({ "weidu_logs": { "bg2ee": null, "iwdee": "order" } })
        );
    }

    #[test]
    fn bgee_bg2ee_eet_weidu_logs_json_is_unchanged() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = ConfigDirGuard::new("bgee_bg2ee_eet");
        for game in ["BGEE", "BG2EE", "EET"] {
            let mut state = state_with_one_bgee_component();
            state.step1.game_install = game.to_string();
            if game != "BGEE" {
                state.step3.bg2ee_items.clone_from(&state.step3.bgee_items);
            }

            let code = export_modlist_share_code(&state).expect("export");
            let json = decode_payload_json(&code);
            let logs = json
                .get("weidu_logs")
                .and_then(serde_json::Value::as_object)
                .expect("weidu_logs object");

            let mut keys: Vec<&String> = logs.keys().collect();
            keys.sort();
            assert_eq!(
                keys,
                vec!["bg2ee", "bgee"],
                "no iwdee key for {game}: {logs:?}"
            );

            let expected_first_slot = rebuilt_weidu_log_text(&state.step3.bgee_items);
            let expected_second_slot = rebuilt_weidu_log_text(&state.step3.bg2ee_items);
            assert_eq!(
                logs["bgee"].as_str(),
                Some(expected_first_slot.as_str()),
                "bgee text for {game}"
            );
            assert_eq!(
                logs["bg2ee"].as_str(),
                Some(expected_second_slot.as_str()),
                "bg2ee text for {game}"
            );
        }
    }

    #[test]
    fn share_export_writes_marker_and_complete_blocks() {
        let mut state = state_with_one_bgee_component();
        state.step2.bgee_mods = vec![make_step2_mod("moda", "ModA")];

        let default_text = "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModA\"\n  repo = \"Owner/ModA\"\n";

        let resolved =
            build_resolved_source_overrides_from_texts(&state, default_text, "", "", &no_refs())
                .expect("resolves");
        let toml = resolved.toml.expect("a resolved mod produces text");

        assert!(
            toml.starts_with("format = 2"),
            "the exported text must start with the format marker; got:\n{toml}"
        );
        assert!(
            toml.contains("[[mods.sources]]") && toml.contains("default = true"),
            "the exported text must carry a complete block; got:\n{toml}"
        );
    }

    struct ShareImportTestRoot {
        root: std::path::PathBuf,
        config_dir: std::path::PathBuf,
        modlist_dir: std::path::PathBuf,
    }

    impl ShareImportTestRoot {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "bio_modlist_share_import_migrate_{}_{}_{label}",
                std::process::id(),
                id
            ));
            let config_dir = root.join("config");
            let modlist_dir = root.join("modlist");
            std::fs::create_dir_all(&config_dir).unwrap();
            std::fs::create_dir_all(&modlist_dir).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(config_dir.clone()));
            crate::app::mod_downloads::set_active_modlist_dir(Some(modlist_dir.clone()));
            Self {
                root,
                config_dir,
                modlist_dir,
            }
        }
    }

    impl Drop for ShareImportTestRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.config_dir);
            crate::app::mod_downloads::set_active_modlist_dir(None);
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn share_import_migrates_codes_without_marker() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = ShareImportTestRoot::new("marker");
        std::fs::write(
            root.config_dir.join("mod_downloads_user.toml"),
            "format = 2\n\n",
        )
        .unwrap();

        let mut state = state_with_one_bgee_component();
        state.step2.bgee_mods = vec![make_step2_mod("moda", "ModA")];
        state.step1.bgee_log_folder = root.modlist_dir.to_string_lossy().to_string();

        let payload_json = serde_json::json!({
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD/MOD.TP2~ #0 #0 // A component: 1.0" },
            "source_overrides": {
                "mod_downloads_user_toml": "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModA\"\n  repo = \"Owner/ModA\"\n  channel = \"pre-release\"\n"
            }
        });
        let code =
            encode_share_payload_text(&payload_json.to_string()).expect("encode fixture code");

        import_modlist_share_code(&mut state, &code).expect("import");

        let written =
            std::fs::read_to_string(root.modlist_dir.join("mod_downloads_user.toml")).unwrap();
        assert!(
            written.starts_with("# BIO mod download user file"),
            "an imported code without the marker must be migrated on the way in; got:\n{written}"
        );
        assert!(
            written.contains("channel = \"preonly\""),
            "pre-release must map to preonly on import; got:\n{written}"
        );
    }

    fn per_modlist_source_toml_with_config(tp2: &str, config_files: &[&str]) -> String {
        let names = config_files
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[[mods]]\nname = \"{tp2}\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  url = \"https://github.com/T/M\"\n  repo = \"T/M\"\n  tag = \"v1\"\n  default = true\n  config_files = [{names}]\n"
        )
    }

    fn config_fixture_mod(
        root: &std::path::Path,
        tp2: &str,
        checked: bool,
    ) -> crate::app::state::Step2ModState {
        let mod_dir = root.join("mods").join(tp2);
        std::fs::create_dir_all(&mod_dir).expect("create mod dir");
        std::fs::write(mod_dir.join(format!("{tp2}.tp2")), "BACKUP ~x~\n").expect("write tp2");
        std::fs::write(mod_dir.join(format!("{tp2}.ini")), "value=edited\n").expect("write ini");
        crate::app::state::Step2ModState {
            tp2_path: mod_dir
                .join(format!("{tp2}.tp2"))
                .to_string_lossy()
                .to_string(),
            components: vec![crate::app::state::Step2ComponentState {
                checked,
                ..checked_component("0")
            }],
            ..make_step2_mod(tp2, tp2)
        }
    }

    #[test]
    fn config_files_export_skips_unchecked_mods() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = ShareImportTestRoot::new("config_export_unchecked");
        std::fs::write(
            root.modlist_dir.join("mod_downloads_user.toml"),
            format!(
                "{}\n{}",
                per_modlist_source_toml_with_config("biocfgchecked", &["biocfgchecked.ini"]),
                per_modlist_source_toml_with_config("biocfgunchecked", &["biocfgunchecked.ini"])
            ),
        )
        .expect("write modlist sources");
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![
            config_fixture_mod(&root.root, "biocfgchecked", true),
            config_fixture_mod(&root.root, "biocfgunchecked", false),
        ];

        let (exported, _warnings) =
            export_mod_config_files(&state, &std::collections::BTreeMap::new())
                .expect("export runs");

        let exported = exported
            .iter()
            .map(|file| (file.tp2.as_str(), file.relative_path.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(exported, vec![("biocfgchecked", "biocfgchecked.ini")]);
    }

    #[test]
    fn config_files_export_warns_about_missing_and_invalid_names() {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = ShareImportTestRoot::new("config_export_warnings");
        std::fs::write(
            root.modlist_dir.join("mod_downloads_user.toml"),
            per_modlist_source_toml_with_config(
                "biocfgwarn",
                &["present.ini", "absent.ini", "../escape.ini"],
            ),
        )
        .expect("write modlist sources");
        let fixture = config_fixture_mod(&root.root, "biocfgwarn", true);
        let mod_dir = root.root.join("mods").join("biocfgwarn");
        std::fs::write(mod_dir.join("present.ini"), "value=1\n").expect("write present ini");
        let mut state = WizardState::default();
        state.step2.bgee_mods = vec![fixture];

        let (exported, warnings) =
            export_mod_config_files(&state, &std::collections::BTreeMap::new())
                .expect("export runs");

        let exported = exported
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(exported, vec!["present.ini"]);
        assert_eq!(
            warnings,
            vec![
                "biocfgwarn: 1 config file(s) named by its source are missing on disk: absent.ini"
                    .to_string(),
                "biocfgwarn: 1 config file name(s) are invalid and were skipped: ../escape.ini"
                    .to_string(),
            ]
        );
    }

    struct ImportLogRoot(std::path::PathBuf);

    impl ImportLogRoot {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = Self(std::env::temp_dir().join(format!(
                "bio_modlist_share_import_logs_{}_{id}_{label}",
                std::process::id()
            )));
            std::fs::create_dir_all(&root.0).expect("create temp root");
            root
        }

        fn eet_step1(&self) -> crate::app::state::Step1State {
            crate::app::state::Step1State {
                game_install: "EET".to_string(),
                eet_bgee_log_folder: self.0.join("bgee").to_string_lossy().to_string(),
                eet_bg2ee_log_folder: self.0.join("bg2ee").to_string_lossy().to_string(),
                ..crate::app::state::Step1State::default()
            }
        }
    }

    impl Drop for ImportLogRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const EET_LOG_HEADER: &str =
        "// Log of Currently Installed WeiDU Mods\n// The top of the file is the 'oldest' mod\n";

    fn eet_payload(first_side: &str, second_side: &str) -> ModlistSharePayload {
        serde_json::from_value(serde_json::json!({
            "format_version": 1,
            "game_install": "EET",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": first_side, "bg2ee": second_side }
        }))
        .expect("payload parses")
    }

    #[test]
    fn eet_import_accepts_one_empty_side() {
        let root = ImportLogRoot::new("eet_one_side");
        let step1 = root.eet_step1();
        let entries_log = format!("{EET_LOG_HEADER}~MOD/MOD.TP2~ #0 #0 // A component: 1.0\n");
        let payload = eet_payload(&entries_log, EET_LOG_HEADER);

        let result = write_imported_weidu_logs(&step1, &payload);

        assert_eq!(result, Ok(()));
        let first_written =
            std::fs::read_to_string(root.0.join("bgee").join("weidu.log")).expect("bgee log");
        let second_written =
            std::fs::read_to_string(root.0.join("bg2ee").join("weidu.log")).expect("bg2ee log");
        assert_eq!(first_written, entries_log);
        assert_eq!(second_written, EET_LOG_HEADER);
    }

    #[test]
    fn eet_import_refuses_both_sides_empty() {
        let root = ImportLogRoot::new("eet_both_empty");
        let step1 = root.eet_step1();
        let payload = eet_payload(EET_LOG_HEADER, EET_LOG_HEADER);

        let result = write_imported_weidu_logs(&step1, &payload);

        assert_eq!(
            result,
            Err("Imported WeiDU logs have no entries.".to_string())
        );
    }
}
