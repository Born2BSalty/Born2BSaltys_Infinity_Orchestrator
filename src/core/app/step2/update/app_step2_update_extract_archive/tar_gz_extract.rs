// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive as TarArchive;

pub(super) fn is_tar_gz_archive(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            lower.ends_with(".tar.gz") || lower.ends_with(".tgz")
        })
        .unwrap_or(false)
}

pub(super) fn extract_tar_gz_archive(archive_path: &Path, out_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|err| err.to_string())?;
    let decoder = GzDecoder::new(file);
    let mut archive = TarArchive::new(decoder);
    let entries = archive.entries().map_err(|err| err.to_string())?;
    for entry in entries {
        let mut entry = entry.map_err(|err| err.to_string())?;
        let unpacked = entry.unpack_in(out_dir).map_err(|err| err.to_string())?;
        if !unpacked {
            return Err("tar entry escaped output directory".to_string());
        }
    }
    Ok(())
}
