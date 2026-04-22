// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;
use walkdir::WalkDir;

use crate::app::mod_downloads;

use super::Step2UpdateExtractResult;
use super::plan::Step2UpdateExtractJob;

#[path = "app_step2_update_extract_archive/rar_extract.rs"]
mod rar_extract;
#[path = "app_step2_update_extract_archive/seven_zip_extract.rs"]
mod seven_zip_extract;
#[path = "app_step2_update_extract_archive/tar_gz_extract.rs"]
mod tar_gz_extract;
#[path = "app_step2_update_extract_archive/zip_extract.rs"]
mod zip_extract;

pub(super) fn extract_update_archives(jobs: &[Step2UpdateExtractJob]) -> Step2UpdateExtractResult {
    let mut result = Step2UpdateExtractResult {
        extracted: Vec::new(),
        failed: Vec::new(),
    };
    for job in jobs {
        match extract_one_archive(job) {
            Ok(target_root) => {
                result
                    .extracted
                    .push(format!("{} -> {}", job.label, target_root.display()))
            }
            Err(err) => result.failed.push(format!("{}: {err}", job.label)),
        }
    }
    result
}

fn extract_one_archive(job: &Step2UpdateExtractJob) -> Result<PathBuf, String> {
    let temp_root = temp_extract_root(&job.tp_file);
    if temp_root.exists() {
        let _ = fs::remove_dir_all(&temp_root);
    }
    fs::create_dir_all(&temp_root).map_err(|err| err.to_string())?;

    let result = (|| {
        extract_archive(&job.archive_path, &temp_root)?;
        let extracted_root = find_extracted_mod_root(&temp_root, &job.tp_file)?;
        let target_root = resolve_target_root(job, &extracted_root)?;
        replace_mod_root(
            job.target_root.as_deref(),
            &target_root,
            &extracted_root,
            &job.backup_root,
            &job.backup_version_tag,
        )?;
        if let Some(source_ref) = &job.installed_source_ref {
            super::super::app_step2_update_source_refs::save_installed_source_ref(
                &job.tp_file,
                source_ref,
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(target_root)
    })();

    let _ = fs::remove_dir_all(&temp_root);
    result
}

fn resolve_target_root(
    job: &Step2UpdateExtractJob,
    extracted_root: &Path,
) -> Result<PathBuf, String> {
    let folder_name = extracted_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "extracted mod folder name is missing".to_string())?;
    Ok(job.mods_root.join(folder_name))
}

fn extract_archive(archive_path: &Path, out_dir: &Path) -> Result<(), String> {
    if zip_extract::is_zip_archive(archive_path) {
        return zip_extract::extract_zip_archive(archive_path, out_dir);
    }
    if tar_gz_extract::is_tar_gz_archive(archive_path) {
        return tar_gz_extract::extract_tar_gz_archive(archive_path, out_dir);
    }
    if seven_zip_extract::is_seven_zip_archive(archive_path) {
        return seven_zip_extract::extract_seven_zip_archive(archive_path, out_dir);
    }
    if rar_extract::is_rar_archive(archive_path) {
        return rar_extract::extract_rar_archive(archive_path, out_dir);
    }
    Err("unsupported archive format (only .zip, .tar.gz, .7z, and .rar supported)".to_string())
}

fn temp_extract_root(tp_file: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "bio_step2_update_{}_{}_{}",
        std::process::id(),
        ts,
        mod_downloads::normalize_mod_download_tp2(tp_file)
    ))
}

fn find_extracted_mod_root(root: &Path, tp_file: &str) -> Result<PathBuf, String> {
    let expected = mod_downloads::normalize_mod_download_tp2(tp_file);
    let mut matched_tp2 = None::<PathBuf>;
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".tp2") {
            continue;
        }
        if mod_downloads::normalize_mod_download_tp2(name) == expected {
            matched_tp2 = Some(path.to_path_buf());
            break;
        }
    }
    let tp2_path =
        matched_tp2.ok_or_else(|| "matching .tp2 not found in extracted archive".to_string())?;
    let tp2_parent = tp2_path
        .parent()
        .ok_or_else(|| "matching .tp2 parent folder not found".to_string())?;
    if tp2_parent
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| mod_downloads::normalize_mod_download_tp2(value) == expected)
    {
        return Ok(tp2_parent.to_path_buf());
    }
    let mod_dir = find_matching_child_mod_dir(tp2_parent, &expected)
        .ok_or_else(|| "matching mod folder not found for root-level .tp2".to_string())?;
    let file_name = tp2_path
        .file_name()
        .ok_or_else(|| "matching .tp2 file name is missing".to_string())?;
    let destination = mod_dir.join(file_name);
    if !destination.exists() {
        fs::rename(&tp2_path, &destination).map_err(|err| err.to_string())?;
    }
    Ok(mod_dir)
}

fn find_matching_child_mod_dir(parent: &Path, expected: &str) -> Option<PathBuf> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if mod_downloads::normalize_mod_download_tp2(&name.to_string_lossy()) == expected {
            matches.push(entry.path());
        }
    }
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

fn replace_mod_root(
    existing_root: Option<&Path>,
    target_root: &Path,
    extracted_root: &Path,
    backup_root: &Path,
    backup_version_tag: &str,
) -> Result<(), String> {
    if let Some(existing_root) = existing_root
        && existing_root != target_root
    {
        backup_existing_mod_root(existing_root, backup_root, backup_version_tag)?;
    }
    if target_root.exists() {
        backup_existing_mod_root(target_root, backup_root, backup_version_tag)?;
    }
    move_dir_or_copy(extracted_root, target_root).map_err(|err| err.to_string())
}

fn backup_existing_mod_root(
    root: &Path,
    backup_root: &Path,
    backup_version_tag: &str,
) -> Result<(), String> {
    let parent = root
        .parent()
        .ok_or_else(|| "current mod folder has no parent".to_string())?;
    if !same_windows_drive(parent, backup_root) {
        return Err("Backup must be on the same drive as Mods Folder".to_string());
    }
    let name = sanitize_backup_name_part(
        root.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("mod"),
    );
    let version_or_ref = sanitize_backup_name_part(backup_version_tag);
    let ts = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let backup = backup_root.join(format!("{name}__{version_or_ref}__{ts}"));
    fs::create_dir_all(backup_root).map_err(|err| err.to_string())?;
    fs::rename(root, &backup).map_err(|err| err.to_string())
}

fn sanitize_backup_name_part(value: &str) -> String {
    let cleaned = value
        .trim()
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ if ch.is_control() => '-',
            _ => ch,
        })
        .collect::<String>();
    let cleaned = cleaned.trim_matches([' ', '.']).trim();
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned.to_string()
    }
}

fn same_windows_drive(left: &Path, right: &Path) -> bool {
    let left = left.to_string_lossy();
    let right = right.to_string_lossy();
    left.get(..2)
        .zip(right.get(..2))
        .is_none_or(|(left, right)| !left.ends_with(':') || left.eq_ignore_ascii_case(right))
}

fn move_dir_or_copy(src: &Path, dst: &Path) -> io::Result<()> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_all(src, dst)?;
            fs::remove_dir_all(src)
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
