// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;

use serde::Deserialize;

use crate::app::mod_display_name::{invalidate_names, rename_key};
use crate::app::mod_downloads::{
    ModDownloadsFile, ensure_mod_downloads_files, mod_downloads_user_path,
    normalize_mod_download_tp2,
};

const NAMES_KEY: &str = "names";

#[derive(Deserialize)]
struct RenamesFile {
    #[serde(default)]
    names: BTreeMap<String, String>,
}

#[must_use]
pub(crate) fn user_mod_renames() -> BTreeMap<String, String> {
    fs::read_to_string(mod_downloads_user_path())
        .ok()
        .and_then(|text| toml::from_str::<RenamesFile>(&text).ok())
        .map_or_else(BTreeMap::new, |file| renames_from_entries(file.names))
}

fn renames_from_entries(entries: BTreeMap<String, String>) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .filter_map(|(key, name)| {
            let key = normalize_mod_download_tp2(&key);
            let name = name.trim();
            (!key.is_empty() && !name.is_empty()).then(|| (key, name.to_string()))
        })
        .collect()
}

pub(crate) fn set_user_mod_name(tp2: &str, name: &str) -> Result<(), String> {
    let key = rename_key(tp2);
    if key.is_empty() {
        return Err("This mod has no tp2 name to rename".to_string());
    }
    ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let path = mod_downloads_user_path();
    let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let updated = rename_in_text(&content, &key, name.trim())?;
    toml::from_str::<ModDownloadsFile>(&updated).map_err(|err| err.to_string())?;
    fs::write(&path, updated).map_err(|err| err.to_string())?;
    invalidate_names();
    Ok(())
}

fn rename_in_text(content: &str, key: &str, name: &str) -> Result<String, String> {
    let mut document = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|err| err.to_string())?;
    if !document.contains_key(NAMES_KEY) && name.is_empty() {
        return Ok(content.to_string());
    }
    let item = document
        .entry(NAMES_KEY)
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
    if let Some(inline) = item.as_inline_table() {
        *item = toml_edit::Item::Table(inline.clone().into_table());
    }
    let names = item
        .as_table_mut()
        .ok_or_else(|| "names in the user file is not a table".to_string())?;
    names.set_implicit(true);
    names.set_dotted(true);
    names.retain(|existing, _| existing == key || normalize_mod_download_tp2(existing) != key);
    if name.is_empty() {
        names.remove(key);
    } else {
        names[key] = toml_edit::value(name);
    }
    if names.is_empty() {
        document.remove(NAMES_KEY);
    }
    Ok(document.to_string())
}

#[cfg(test)]
#[path = "mod_rename_tests.rs"]
mod tests;
