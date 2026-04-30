// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;
use std::path::Path;

use zip::read::ZipArchive;

pub(super) fn is_zip_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
}

pub(super) fn extract_zip_archive(archive_path: &Path, out_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|err| err.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|err| err.to_string())?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| err.to_string())?;
        let Some(relative) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };
        let destination = out_dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination).map_err(|err| err.to_string())?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let mut out = fs::File::create(&destination).map_err(|err| err.to_string())?;
        io::copy(&mut entry, &mut out).map_err(|err| err.to_string())?;
    }
    Ok(())
}
