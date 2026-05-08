// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::app::app_step2_update_source_refs::{self, InstalledSourceRefs};
use crate::app::mod_downloads::{self, ModDownloadSource};
use crate::app::state::{Step1State, WizardState};

#[derive(Debug, Clone, Default)]
pub struct ModlistPackExportReport {
    pub output_dir: PathBuf,
    pub files_written: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
struct CopiedModlistFiles {
    weidu_bgee: Option<String>,
    weidu_bg2ee: Option<String>,
    mod_downloads_user: Option<String>,
    mod_installed_refs: Option<String>,
}

#[derive(Debug, Serialize)]
struct BioModlistToml {
    format_version: u32,
    name: String,
    bio_version: String,
    install_mode: String,
    game_install: String,
    files: BioModlistFilesToml,
}

#[derive(Debug, Default, Serialize)]
struct BioModlistFilesToml {
    lock: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    weidu_bgee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weidu_bg2ee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mod_downloads_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mod_installed_refs: Option<String>,
}

#[derive(Debug, Serialize)]
struct BioModlistLockToml {
    format_version: u32,
    mods: Vec<ModlistLockEntry>,
}

#[derive(Debug, Serialize)]
struct ModlistLockEntry {
    tp2: String,
    name: String,
    source_id: String,
    source_type: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    source_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
}

pub fn export_modlist_pack(
    state: &WizardState,
    output_dir: &Path,
) -> Result<ModlistPackExportReport, String> {
    fs::create_dir_all(output_dir).map_err(|err| err.to_string())?;

    let mut report = ModlistPackExportReport {
        output_dir: output_dir.to_path_buf(),
        files_written: Vec::new(),
        warnings: Vec::new(),
    };
    let mut copied_files = CopiedModlistFiles::default();

    copy_weidu_logs_for_pack(&state.step1, output_dir, &mut copied_files, &mut report);
    copy_pack_support_files(output_dir, &mut copied_files, &mut report);
    write_bio_modlist_lock_toml(state, output_dir, &mut report)?;
    write_bio_modlist_toml(state, output_dir, &copied_files, &mut report)?;

    Ok(report)
}

fn write_bio_modlist_toml(
    state: &WizardState,
    output_dir: &Path,
    copied_files: &CopiedModlistFiles,
    report: &mut ModlistPackExportReport,
) -> Result<(), String> {
    let file = BioModlistToml {
        format_version: 1,
        name: "BIO Modlist".to_string(),
        bio_version: env!("CARGO_PKG_VERSION").to_string(),
        install_mode: state.step1.install_mode.clone(),
        game_install: state.step1.game_install.clone(),
        files: BioModlistFilesToml {
            lock: "bio_modlist.lock.toml".to_string(),
            weidu_bgee: copied_files.weidu_bgee.clone(),
            weidu_bg2ee: copied_files.weidu_bg2ee.clone(),
            mod_downloads_user: copied_files.mod_downloads_user.clone(),
            mod_installed_refs: copied_files.mod_installed_refs.clone(),
        },
    };
    write_toml_file(output_dir, "bio_modlist.toml", &file, report)
}

fn write_bio_modlist_lock_toml(
    state: &WizardState,
    output_dir: &Path,
    report: &mut ModlistPackExportReport,
) -> Result<(), String> {
    let source_load = mod_downloads::load_mod_download_sources();
    if let Some(err) = source_load.error.as_ref() {
        report
            .warnings
            .push(format!("Mod download sources load warning: {err}"));
    }
    let installed_refs = app_step2_update_source_refs::load_installed_source_refs_file();
    let mods = build_lock_entries(state, &source_load, &installed_refs, report);
    if mods.is_empty() {
        report
            .warnings
            .push("No mod lock entries available from current BIO state.".to_string());
    }
    write_toml_file(
        output_dir,
        "bio_modlist.lock.toml",
        &BioModlistLockToml {
            format_version: 1,
            mods,
        },
        report,
    )
}

fn build_lock_entries(
    state: &WizardState,
    source_load: &mod_downloads::ModDownloadsLoad,
    installed_refs: &InstalledSourceRefs,
    report: &mut ModlistPackExportReport,
) -> Vec<ModlistLockEntry> {
    let mut keys = BTreeSet::<String>::new();
    keys.extend(installed_refs.refs.keys().cloned());
    keys.extend(installed_refs.sources.keys().cloned());

    let mut entries = Vec::new();
    for tp2 in keys {
        let source_id = installed_refs
            .sources
            .get(&tp2)
            .cloned()
            .unwrap_or_default();
        let source_ref = installed_refs.refs.get(&tp2).cloned();
        let source = source_load.resolve_source(
            &tp2,
            (!source_id.trim().is_empty()).then_some(source_id.as_str()),
        );
        entries.push(lock_entry_from_source(
            &tp2,
            &source_id,
            source_ref,
            source.as_ref(),
        ));
    }
    entries
}

fn lock_entry_from_source(
    tp2: &str,
    source_id: &str,
    source_ref: Option<String>,
    source: Option<&ModDownloadSource>,
) -> ModlistLockEntry {
    let name = source
        .map(|source| source.name.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(tp2)
        .to_string();
    let repo = source.and_then(|source| source.github.clone());
    let url = source
        .map(|source| source.url.trim().to_string())
        .filter(|value| !value.is_empty());
    let download_url = source_download_url(source, source_ref.as_deref());
    let status = if source_ref.is_some() {
        "pinned"
    } else {
        "unpinned"
    };
    ModlistLockEntry {
        tp2: tp2.to_string(),
        name,
        source_id: if source_id.trim().is_empty() {
            source
                .map(|source| source.source_id.clone())
                .unwrap_or_default()
        } else {
            source_id.to_string()
        },
        source_type: source.map(source_type).unwrap_or("unknown").to_string(),
        status: status.to_string(),
        reason: (status == "unpinned").then(|| "missing ref/checksum".to_string()),
        repo,
        url,
        source_ref,
        download_url,
    }
}

fn source_type(source: &ModDownloadSource) -> &'static str {
    if source.github.is_some() {
        "github"
    } else if mod_downloads::source_is_page_archive_url(&source.url) {
        "page"
    } else if !source.url.trim().is_empty() {
        "url"
    } else {
        "unknown"
    }
}

fn source_download_url(
    source: Option<&ModDownloadSource>,
    source_ref: Option<&str>,
) -> Option<String> {
    let source = source?;
    if let Some(repo) = source.github.as_deref() {
        if let Some(commit) = source_ref.and_then(|value| value.strip_prefix("commit@")) {
            return Some(format!(
                "https://github.com/{}/archive/{}.zip",
                repo.trim(),
                commit.trim()
            ));
        }
        if let Some(tag) = source_ref.and_then(|value| value.strip_prefix("tag@")) {
            return Some(format!(
                "https://github.com/{}/archive/refs/tags/{}.zip",
                repo.trim(),
                tag.trim()
            ));
        }
        if let Some(tag) = source_ref.filter(|value| !value.contains('@')) {
            return Some(format!(
                "https://github.com/{}/archive/refs/tags/{}.zip",
                repo.trim(),
                tag.trim()
            ));
        }
        if let Some(branch) = source.branch.as_deref() {
            return Some(format!(
                "https://github.com/{}/archive/refs/heads/{}.zip",
                repo.trim(),
                branch.trim()
            ));
        }
    }
    mod_downloads::is_direct_archive_url(&source.url).then(|| source.url.clone())
}

fn copy_weidu_logs_for_pack(
    step1: &Step1State,
    output_dir: &Path,
    copied_files: &mut CopiedModlistFiles,
    report: &mut ModlistPackExportReport,
) {
    for info in crate::app::step5::log_files::source_log_infos(step1) {
        let file_name = match info.tag {
            "bgee" => "weidu-bgee.log",
            "bg2ee" => "weidu-bg2ee.log",
            other => {
                report
                    .warnings
                    .push(format!("Unknown WeiDU log tag skipped: {other}"));
                continue;
            }
        };
        if !info.exists {
            report
                .warnings
                .push(format!("Missing WeiDU log: {}", info.path.display()));
            continue;
        }
        let destination = output_dir.join(file_name);
        match fs::copy(&info.path, &destination) {
            Ok(_) => {
                match info.tag {
                    "bgee" => copied_files.weidu_bgee = Some(file_name.to_string()),
                    "bg2ee" => copied_files.weidu_bg2ee = Some(file_name.to_string()),
                    _ => {}
                }
                report.files_written.push(destination);
            }
            Err(err) => report.warnings.push(format!(
                "Could not copy WeiDU log {}: {err}",
                info.path.display()
            )),
        }
    }
}

fn copy_pack_support_files(
    output_dir: &Path,
    copied_files: &mut CopiedModlistFiles,
    report: &mut ModlistPackExportReport,
) {
    copy_optional_support_file(
        &mod_downloads::mod_downloads_user_path(),
        output_dir,
        "mod_downloads_user.toml",
        &mut copied_files.mod_downloads_user,
        report,
    );
    copy_optional_support_file(
        &app_step2_update_source_refs::installed_source_refs_path(),
        output_dir,
        "mod_installed_refs.toml",
        &mut copied_files.mod_installed_refs,
        report,
    );
}

fn copy_optional_support_file(
    source: &Path,
    output_dir: &Path,
    file_name: &str,
    copied_slot: &mut Option<String>,
    report: &mut ModlistPackExportReport,
) {
    if !source.is_file() {
        report
            .warnings
            .push(format!("Missing support file: {}", source.display()));
        return;
    }
    let destination = output_dir.join(file_name);
    match fs::copy(source, &destination) {
        Ok(_) => {
            *copied_slot = Some(file_name.to_string());
            report.files_written.push(destination);
        }
        Err(err) => report.warnings.push(format!(
            "Could not copy support file {}: {err}",
            source.display()
        )),
    }
}

fn write_toml_file<T: Serialize>(
    output_dir: &Path,
    file_name: &str,
    value: &T,
    report: &mut ModlistPackExportReport,
) -> Result<(), String> {
    let path = output_dir.join(file_name);
    let text = toml::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&path, text).map_err(|err| err.to_string())?;
    report.files_written.push(path);
    Ok(())
}
