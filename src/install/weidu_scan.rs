// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod command;
mod components;
mod language;

use std::path::Path;

use anyhow::Result;

use crate::mods::component::Component;

#[derive(Debug, Clone)]
pub struct LanguageEntry {
    pub id: String,
    pub label: String,
}

pub fn list_languages(weidu_binary: &Path, tp2_path: &Path) -> Result<Vec<LanguageEntry>> {
    language::list_languages(weidu_binary, tp2_path)
}

pub fn list_languages_for_game(
    weidu_binary: &Path,
    tp2_path: &Path,
    game_directory: &Path,
    working_directory: &Path,
) -> Result<Vec<LanguageEntry>> {
    language::list_languages_for_game(weidu_binary, tp2_path, game_directory, working_directory)
}

pub fn list_components(
    weidu_binary: &Path,
    game_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<Component>> {
    components::list_components(weidu_binary, game_directory, tp2_path, language)
}

pub fn list_components_lines(
    weidu_binary: &Path,
    game_directory: &Path,
    working_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<String>> {
    components::list_components_lines(
        weidu_binary,
        game_directory,
        working_directory,
        tp2_path,
        language,
    )
}
