// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

pub(super) fn is_seven_zip_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("7z"))
        .unwrap_or(false)
}

pub(super) fn extract_seven_zip_archive(archive_path: &Path, out_dir: &Path) -> Result<(), String> {
    sevenz_rust2::decompress_file(archive_path, out_dir).map_err(|err| err.to_string())
}
