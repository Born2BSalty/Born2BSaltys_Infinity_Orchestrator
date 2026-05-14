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
    export_modlist_share_code_with_auto_install(state, true)
}

pub(crate) fn export_modlist_share_code_with_auto_install(
    state: &WizardState,
    allow_auto_install: bool,
) -> Result<String, String> {
    crate::app::mod_downloads::ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let weidu_logs = export_weidu_logs(state)?;
    if relevant_weidu_text_is_empty(
        state,
        weidu_logs.bgee.as_deref(),
        weidu_logs.bg2ee.as_deref(),
    ) {
        return Err("No WeiDU entries available to export.".to_string());
    }
    let mod_downloads_user = read_optional_file_text(
        &crate::app::mod_downloads::mod_downloads_user_path(),
        omit_stock_mod_downloads_user,
    );
    let mod_installed_refs = read_optional_file_text(
        &crate::app::app_step2_update_source_refs::installed_source_refs_path(),
        |_| false,
    );
    let mod_configs = export_mod_config_files(state)?;
    let payload = json!({
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
        "allow_auto_install": allow_auto_install,
    });
    let payload_text = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    let compressed = zlib_compress(payload_text.as_bytes())?;
    Ok(format!(
        "{SHARE_CODE_PREFIX}{}",
        base64url_encode(&compressed)
    ))
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
}

pub(crate) fn preview_modlist_share_code(code: &str) -> Result<ModlistSharePreview, String> {
    let preview = share_preview(&decode_share_payload(code)?)?;
    let _allow_auto_install = preview.allow_auto_install;
    Ok(preview)
}

pub(crate) fn import_modlist_share_code(
    state: &mut WizardState,
    code: &str,
) -> Result<ModlistSharePreview, String> {
    let payload = decode_share_payload(code)?;
    let preview = share_preview(&payload)?;
    let mut step1 = state.step1.clone();
    step1.game_install = payload.game_install.clone();
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
        write_text_file(crate::app::mod_downloads::mod_downloads_user_path(), text)?;
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
    let bgee_entries = count_weidu_entries(payload.weidu_logs.bgee.as_deref());
    let bg2ee_entries = count_weidu_entries(payload.weidu_logs.bg2ee.as_deref());
    if match payload.game_install.as_str() {
        "EET" => bgee_entries == 0 && bg2ee_entries == 0,
        "BG2EE" => bg2ee_entries == 0,
        _ => bgee_entries == 0,
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
        bgee_entries,
        bg2ee_entries,
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
    })
}

fn default_true() -> bool {
    true
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
                import_log_target_path(step1, true)?,
            )?;
            let rewritten_bg2ee =
                rewrite_imported_eet_bg2ee_wlb_paths(step1, payload.weidu_logs.bg2ee.as_deref())?;
            write_imported_log(
                "BG2EE",
                rewritten_bg2ee.as_deref(),
                import_log_target_path(step1, false)?,
            )
        }
        "BG2EE" => write_imported_log(
            "BG2EE",
            payload.weidu_logs.bg2ee.as_deref(),
            import_log_target_path(step1, false)?,
        ),
        _ => write_imported_log(
            "BGEE",
            payload.weidu_logs.bgee.as_deref(),
            import_log_target_path(step1, true)?,
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

fn write_imported_log(label: &str, text: Option<&str>, path: PathBuf) -> Result<(), String> {
    let Some(text) = text.filter(|text| count_weidu_entries(Some(text)) > 0) else {
        return Err(format!("Imported {label} WeiDU log has no entries."));
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&path, text).map_err(|err| format!("Write {label} WeiDU log failed: {err}"))
}

fn write_text_file(path: PathBuf, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, text).map_err(|err| err.to_string())
}

fn count_weidu_entries(text: Option<&str>) -> usize {
    text.map(|text| {
        text.lines()
            .filter(|line| {
                let line = line.trim();
                !line.is_empty() && !line.starts_with("//")
            })
            .count()
    })
    .unwrap_or(0)
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
    bgee_text: Option<&str>,
    bg2ee_text: Option<&str>,
) -> bool {
    match state.step1.game_install.as_str() {
        "EET" => weidu_text_has_no_entries(bgee_text) && weidu_text_has_no_entries(bg2ee_text),
        "BG2EE" => weidu_text_has_no_entries(bg2ee_text),
        _ => weidu_text_has_no_entries(bgee_text),
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
        let pad = chunk.iter().filter(|value| **value == 64).count();
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
