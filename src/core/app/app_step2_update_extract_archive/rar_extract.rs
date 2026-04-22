// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use unrar::Archive;

pub(super) fn is_rar_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("rar"))
        .unwrap_or(false)
}

pub(super) fn extract_rar_archive(archive_path: &Path, out_dir: &Path) -> Result<(), String> {
    let mut archive = Archive::new(archive_path)
        .as_first_part()
        .open_for_processing()
        .map_err(|err| err.to_string())?;
    loop {
        let Some(entry) = archive.read_header().map_err(|err| err.to_string())? else {
            break;
        };
        archive = entry.extract_with_base(out_dir).map_err(|err| err.to_string())?;
    }
    Ok(())
}
