// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::install::weidu_scan;

pub(super) fn pick_language_id(
    weidu: &Path,
    tp2: &Path,
    game_dir: &Path,
    mods_root: &Path,
) -> Option<String> {
    let langs = match weidu_scan::list_languages_for_game(weidu, tp2, game_dir, mods_root) {
        Ok(v) => v,
        Err(_) => return Some("0".to_string()),
    };
    langs
        .into_iter()
        .find(|entry| entry.label.to_ascii_lowercase().contains("english"))
        .map(|entry| entry.id)
        .or_else(|| Some("0".to_string()))
}
