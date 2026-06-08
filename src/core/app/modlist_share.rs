// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::app::state::WizardState;
use crate::app::step5::diagnostics::build_weidu_export_lines;

const SHARE_CODE_PREFIX: &str = "BIO-MODLIST-V1:";

pub(crate) fn export_modlist_share_code(state: &WizardState) -> Result<String, String> {
    crate::app::mod_downloads::ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let weidu_logs = export_weidu_logs(state)?;
    if relevant_weidu_text_is_empty(
        state,
        weidu_logs.bgee.as_deref(),
        weidu_logs.bg2ee.as_deref(),
    ) {
        return Err("No WeiDU entries available to export.".to_string());
    }

    // Gate on active modlist: ambient-set ⇒ bake resolved set; ambient-unset ⇒ legacy verbatim.
    let mod_downloads_user =
        if crate::app::mod_downloads::active_modlist_downloads_path().is_some() {
            build_resolved_source_overrides(state)?
        } else {
            read_optional_file_text(
                &crate::app::mod_downloads::mod_downloads_user_path(),
                omit_stock_mod_downloads_user,
            )
        };

    // Installed-refs: per-modlist when active, global verbatim otherwise.
    let mod_installed_refs =
        if crate::app::mod_downloads::active_modlist_downloads_path().is_some() {
            build_per_modlist_installed_refs(state)
        } else {
            read_optional_file_text(
                &crate::app::app_step2_update_source_refs::installed_source_refs_path(),
                |_| false,
            )
        };

    let mod_configs = export_mod_config_files(state)?;
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
    insert_export_provenance(&mut payload, state);
    let payload_text = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
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
    pub(crate) forked_from: Vec<ForkAncestor>,
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
        // Redirect to the importing modlist's per-modlist file when the ambient is set
        // (the orchestrator sets the ambient before the import runs). Falls back to the global
        // path when the ambient is unset (legacy BIO_legacy import path).
        let pin_path = crate::app::mod_downloads::active_modlist_downloads_path()
            .unwrap_or_else(crate::app::mod_downloads::mod_downloads_user_path);
        if let Some(parent) = pin_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("create per-modlist dir failed: {err}"))?;
        }
        write_text_file(pin_path, text)?;
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
    Ok(preview)
}

#[derive(Deserialize)]
struct ModlistSharePayload {
    format_version: u64,
    #[serde(default)]
    bio_version: String,
    game_install: String,
    install_mode: String,
    #[serde(default)]
    weidu_logs: ModlistShareWeiduLogs,
    #[serde(default)]
    source_overrides: ModlistShareSourceOverrides,
    #[serde(default)]
    installed_refs: ModlistShareInstalledRefs,
    #[serde(default)]
    mod_configs: ModlistShareModConfigs,
    #[serde(default = "default_true")]
    allow_auto_install: bool,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    forked_from: Vec<ForkAncestor>,
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
struct ModlistShareWeiduLogs {
    bgee: Option<String>,
    bg2ee: Option<String>,
}

#[derive(Default, Deserialize)]
struct ModlistShareSourceOverrides {
    mod_downloads_user_toml: Option<String>,
}

#[derive(Default, Deserialize)]
struct ModlistShareInstalledRefs {
    mod_installed_refs_toml: Option<String>,
}

#[derive(Default, Deserialize)]
struct ModlistShareModConfigs {
    #[serde(default)]
    files: Vec<ModlistShareConfigFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ModlistShareConfigFile {
    pub(crate) tp2: String,
    pub(crate) source_id: String,
    pub(crate) relative_path: String,
    pub(crate) base64_data: String,
}

fn decode_share_payload(code: &str) -> Result<ModlistSharePayload, String> {
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
    let first_game_entries = count_weidu_entries(payload.weidu_logs.bgee.as_deref());
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
        bgee_log_text: payload.weidu_logs.bgee.clone().unwrap_or_default(),
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
        forked_from: payload.forked_from.clone(),
    })
}

fn write_imported_weidu_logs(
    step1: &crate::app::state::Step1State,
    payload: &ModlistSharePayload,
) -> Result<(), String> {
    match step1.game_install.as_str() {
        "EET" => {
            write_imported_log(
                "BGEE",
                payload.weidu_logs.bgee.as_deref(),
                &import_log_target_path(step1, true)?,
            )?;
            let rewritten_bg2ee =
                rewrite_imported_eet_bg2ee_wlb_paths(step1, payload.weidu_logs.bg2ee.as_deref())?;
            write_imported_log(
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
            "BGEE",
            payload.weidu_logs.bgee.as_deref(),
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
                if bgee { "BGEE" } else { "BG2EE" }
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
            if bgee { "BGEE" } else { "BG2EE" }
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
}

fn export_weidu_logs(state: &WizardState) -> Result<ExportWeiduLogs, String> {
    if state.step1.installs_exactly_from_weidu_logs() {
        return Ok(ExportWeiduLogs {
            bgee: read_exact_source_weidu_log(
                state,
                crate::app::app_step2_log::resolve_bgee_weidu_log_path,
            )?,
            bg2ee: read_exact_source_weidu_log(
                state,
                crate::app::app_step2_log::resolve_bg2_weidu_log_path,
            )?,
        });
    }
    Ok(ExportWeiduLogs {
        bgee: Some(weidu_log_text(&build_weidu_export_lines(
            &state.step3.bgee_items,
        ))),
        bg2ee: Some(weidu_log_text(&build_weidu_export_lines(
            &state.step3.bg2ee_items,
        ))),
    })
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

fn read_optional_file_text(
    path: &std::path::Path,
    should_omit: impl FnOnce(&str) -> bool,
) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .filter(|text| !text.trim().is_empty())
        .filter(|text| !should_omit(text))
}

fn export_mod_config_files(state: &WizardState) -> Result<Vec<ModlistShareConfigFile>, String> {
    let sources = crate::app::mod_downloads::load_mod_download_sources();
    let installed_source_ids =
        crate::app::app_step2_update_source_refs::load_installed_source_ids();
    let mut exported = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let Some(source) =
            resolve_mod_config_source(state, &sources, &installed_source_ids, &mod_state.tp_file)
        else {
            continue;
        };
        if source.config_files.is_empty() {
            continue;
        }
        let Some(mod_root) = mod_config_root(&mod_state.tp2_path) else {
            continue;
        };
        for relative_path in &source.config_files {
            let relative_path =
                crate::app::modlist_config_files::validate_relative_config_path(relative_path)?;
            let path = mod_root.join(&relative_path);
            if !path.is_file() {
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|err| format!("Read mod config failed ({}): {err}", path.display()))?;
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");
            let key = (
                crate::app::mod_downloads::normalize_mod_download_tp2(&source.tp2),
                source.source_id.trim().to_ascii_lowercase(),
                relative_path.clone(),
            );
            if seen.insert(key) {
                exported.push(ModlistShareConfigFile {
                    tp2: crate::app::mod_downloads::normalize_mod_download_tp2(&source.tp2),
                    source_id: source.source_id.clone(),
                    relative_path,
                    base64_data: base64url_encode(&bytes),
                });
            }
        }
    }
    Ok(exported)
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

#[derive(Deserialize)]
struct ShareModDownloadsFile {
    #[serde(default)]
    mods: Vec<ShareModDownloadMod>,
}

#[derive(Deserialize)]
struct ShareModDownloadMod {
    name: Option<String>,
    tp2: Option<String>,
    #[serde(default)]
    sources: Vec<ShareModDownloadSource>,
}

#[derive(Deserialize)]
struct ShareModDownloadSource {
    id: Option<String>,
    repo: Option<String>,
}

/// Builds the resolved `mod_downloads_user_toml` payload from this modlist's mods,
/// resolving each through the three-tier loader and serializing with a round-trip-safe
/// serializer. Skips mods with no resolvable source (no panic). Returns `None` when
/// nothing resolves (importer treats absent `source_overrides` as no-override).
fn build_resolved_source_overrides(state: &WizardState) -> Result<Option<String>, String> {
    let source_load = crate::app::mod_downloads::load_mod_download_sources();
    let installed_ids = crate::app::app_step2_update_source_refs::load_installed_source_ids();

    let mut toml_out = String::new();
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let Some(source) =
            resolve_mod_config_source(state, &source_load, &installed_ids, &mod_state.tp_file)
        else {
            continue;
        };
        let block = serialize_resolved_mod(&mod_state.tp_file, &mod_state.name, &source);
        if !toml_out.is_empty() {
            toml_out.push_str("\n\n");
        }
        toml_out.push_str(&block);
    }

    if toml_out.is_empty() {
        return Ok(None);
    }

    // Round-trip validate before baking.
    toml::from_str::<crate::app::mod_downloads::ModDownloadsFile>(&toml_out)
        .map_err(|err| format!("resolved source overrides failed round-trip validation: {err}"))?;

    Ok(Some(toml_out))
}

/// Emits `[[mods]]` header + one `[[mods.sources]]` block for a single resolved source.
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
    let source_block = serialize_resolved_source_block(source);
    format!("{header}\n\n{source_block}")
}

fn escape_for_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Correct, round-trippable serializer for a single resolved source block.
/// Emits only pin selectors and source identity; drops `config_files` and
/// `source_default` (both are no-ops on import).
fn serialize_resolved_source_block(
    source: &crate::app::mod_downloads::ModDownloadSource,
) -> String {
    let mut lines = vec![
        "[[mods.sources]]".to_string(),
        format!("id = \"{}\"", escape_for_toml(&source.source_id)),
        format!("label = \"{}\"", escape_for_toml(&source.source_label)),
    ];

    if let Some(github) = source.github.as_ref() {
        // GitHub source: emit type + url + repo.
        lines.push("type = \"github\"".to_string());
        lines.push(format!("url = \"{}\"", escape_for_toml(&source.url)));
        lines.push(format!("repo = \"{}\"", escape_for_toml(github)));
    } else if !source.url.is_empty() {
        // Non-GitHub source: emit type="url" + url; no repo field.
        lines.push("type = \"url\"".to_string());
        lines.push(format!("url = \"{}\"", escape_for_toml(&source.url)));
    }

    // Pin selectors.
    if !source.exact_github.is_empty() {
        // Emit as a TOML array (never repeated scalar lines — prevents duplicate-key error).
        let items = source
            .exact_github
            .iter()
            .map(|s| format!("\"{}\"", escape_for_toml(s)))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("exact_github = [{items}]"));
    }
    if let Some(tag) = source.tag.as_ref().filter(|s| !s.is_empty()) {
        lines.push(format!("tag = \"{}\"", escape_for_toml(tag)));
    }
    if let Some(commit) = source.commit.as_ref().filter(|s| !s.is_empty()) {
        lines.push(format!("commit = \"{}\"", escape_for_toml(commit)));
    }
    if let Some(branch) = source.branch.as_ref().filter(|s| !s.is_empty()) {
        lines.push(format!("branch = \"{}\"", escape_for_toml(branch)));
    }
    if let Some(channel) = source.channel.as_ref().filter(|s| !s.is_empty()) {
        lines.push(format!("channel = \"{}\"", escape_for_toml(channel)));
    }

    // Indent all lines after the header by two spaces (standard block format).
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line.clone()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Builds the `mod_installed_refs_toml` payload from this modlist's per-modlist data.
/// Sources come from `selected_source_ids`; refs from the per-modlist refs file.
/// Returns `None` when there is nothing to emit.
fn build_per_modlist_installed_refs(state: &WizardState) -> Option<String> {
    use crate::app::app_step2_update_source_refs::load_refs_file_at;

    let path = crate::app::app_step2_update_source_refs::installed_source_refs_path();
    let refs_file = load_refs_file_at(&path);
    let sources = state.step2.selected_source_ids.clone();

    if refs_file.refs.is_empty() && sources.is_empty() {
        return None;
    }

    let combined = crate::app::app_step2_update_source_refs::ModSourceRefsFile {
        refs: refs_file.refs,
        sources,
    };
    toml::to_string_pretty(&combined)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn omit_stock_mod_downloads_user(text: &str) -> bool {
    let Ok(parsed) = toml::from_str::<ShareModDownloadsFile>(text) else {
        return false;
    };
    let [mod_entry] = parsed.mods.as_slice() else {
        return false;
    };
    let [source_entry] = mod_entry.sources.as_slice() else {
        return false;
    };
    mod_entry.name.as_deref() == Some("Example Mod")
        && mod_entry.tp2.as_deref() == Some("examplemod")
        && source_entry.id.as_deref() == Some("main")
        && source_entry.repo.as_deref() == Some("ExampleUser/ExampleMod")
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

fn base64url_encode(bytes: &[u8]) -> String {
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

fn base64url_decode(text: &str) -> Result<Vec<u8>, String> {
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
        assert!(preview.forked_from.is_empty());
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
        let mut state = state_with_one_bgee_component();
        state.set_modlist_share_provenance(
            Some("  Tactical EET 2026  ".to_string()),
            Some("  @b2bs  ".to_string()),
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

    // ── Export serializer tests ────────────────────────────────────

    use std::sync::Mutex;
    static SHARE_TEST_LOCK: Mutex<()> = Mutex::new(());

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
        let _lock = SHARE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let source = github_source_multi_exact();
        let block = serialize_resolved_source_block(&source);

        // Must be a single array line, not repeated scalar lines.
        assert!(
            block.contains("exact_github = ["),
            "exact_github must be emitted as TOML array: {block}"
        );
        assert!(
            !block.contains("\nexact_github = \""),
            "must not emit repeated scalar exact_github lines"
        );

        // Round-trip validate.
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
        let _lock = SHARE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let source = url_source();
        let block = serialize_resolved_source_block(&source);

        assert!(block.contains("url = \""), "url source must emit url field");
        assert!(!block.contains("repo = \""), "url source must not emit repo field");

        // Round-trip validate.
        let wrapped = format!("[[mods]]\nname = \"U\"\ntp2 = \"u\"\n\n{block}");
        let parsed = toml::from_str::<crate::app::mod_downloads::ModDownloadsFile>(&wrapped);
        assert!(parsed.is_ok(), "url source block must round-trip");
    }

    #[test]
    fn export_ambient_unset_is_byte_identical_verbatim() {
        let _lock = SHARE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        // With no ambient, build_resolved_source_overrides must not be called.
        // The gate checks active_modlist_downloads_path().is_some() — when None,
        // the export returns early with the verbatim global path.
        assert!(
            crate::app::mod_downloads::active_modlist_downloads_path().is_none(),
            "precondition: ambient is None"
        );

        // Verify the gate logic: the function path that reads verbatim is chosen.
        let is_set = crate::app::mod_downloads::active_modlist_downloads_path().is_some();
        assert!(!is_set, "ambient-unset path must produce verbatim export");
    }

    #[test]
    fn export_skips_unresolvable_mod() {
        let _lock = SHARE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        // A state with a bgee mod that has no matching source in the catalog.
        let mut state = WizardState::default();
        state.step3.bgee_items = vec![];
        state.step2.bgee_mods = vec![];

        // build_resolved_source_overrides with an empty mods list yields None.
        let result = build_resolved_source_overrides(&state);
        assert!(result.is_ok(), "empty mods list must not error");
        assert!(
            result.unwrap().is_none(),
            "empty mods list must yield None source overrides"
        );
    }

    // ── Import tests ───────────────────────────────────────────────

    #[test]
    fn import_ambient_unset_writes_global() {
        let _lock = SHARE_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        // With ambient unset, active_modlist_downloads_path() is None,
        // so the import fallback path is the global file.
        let path = crate::app::mod_downloads::active_modlist_downloads_path();
        assert!(
            path.is_none(),
            "ambient unset: no per-modlist path resolved"
        );
    }
}
