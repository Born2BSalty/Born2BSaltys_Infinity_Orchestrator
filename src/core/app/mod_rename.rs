// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::app::mod_display_name::{invalidate_names, rename_key, tp2_names_of};
use crate::app::mod_downloads::{
    ModDownloadsFile, ensure_mod_downloads_files, mod_downloads_user_path, mod_name_below,
    normalize_mod_download_tp2,
};

const NEW_FILE_HEADER: &str = "format = 2\n";
const ENTRY_METADATA_KEYS: [&str; 3] = ["name", "tp2", "categories"];

pub(crate) fn set_mod_name(target: &Path, tp2: &str, name: &str) -> Result<(), String> {
    let key = rename_key(tp2);
    if key.is_empty() {
        return Err("This mod has no tp2 name to rename".to_string());
    }
    if target == mod_downloads_user_path() {
        ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    }
    let content = match fs::read_to_string(target) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.to_string()),
    };
    let below = mod_name_below(target, &key);
    let updated = name_in_text(
        &content,
        &ModNameEdit {
            key: &key,
            tp2_names: &tp2_names_of(&key),
            name: name.trim(),
            unrenamed_name: below.as_deref().unwrap_or(&key),
        },
    )?;
    if updated == content {
        return Ok(());
    }
    toml::from_str::<ModDownloadsFile>(&updated).map_err(|err| err.to_string())?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(target, updated).map_err(|err| err.to_string())?;
    invalidate_names();
    Ok(())
}

struct ModNameEdit<'a> {
    key: &'a str,
    tp2_names: &'a BTreeSet<String>,
    name: &'a str,
    unrenamed_name: &'a str,
}

impl ModNameEdit<'_> {
    fn matches(&self, entry: &toml_edit::Table) -> bool {
        entry
            .get("tp2")
            .and_then(toml_edit::Item::as_str)
            .is_some_and(|value| self.tp2_names.contains(&normalize_mod_download_tp2(value)))
    }
}

fn entry_has_sources(entry: &toml_edit::Table) -> bool {
    entry
        .iter()
        .any(|(key, _)| !ENTRY_METADATA_KEYS.contains(&key))
}

fn name_in_text(content: &str, edit: &ModNameEdit<'_>) -> Result<String, String> {
    let base = if content.trim().is_empty() && !edit.name.is_empty() {
        NEW_FILE_HEADER
    } else {
        content
    };
    let mut document = base
        .parse::<toml_edit::DocumentMut>()
        .map_err(|err| err.to_string())?;
    if !document.contains_key("mods") {
        if edit.name.is_empty() {
            return Ok(content.to_string());
        }
        document.insert(
            "mods",
            toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
        );
    }
    let entries = document
        .get_mut("mods")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
        .ok_or_else(|| "mods in this file is not a list of [[mods]] entries".to_string())?;
    if edit.name.is_empty() {
        clear_name(entries, edit);
    } else {
        write_name(entries, edit);
    }
    let updated = document.to_string();
    Ok(if updated == base {
        content.to_string()
    } else {
        updated
    })
}

fn write_name(entries: &mut toml_edit::ArrayOfTables, edit: &ModNameEdit<'_>) {
    let mut found = false;
    for entry in entries.iter_mut().filter(|entry| edit.matches(entry)) {
        entry["name"] = toml_edit::value(edit.name);
        found = true;
    }
    if !found {
        let mut entry = toml_edit::Table::new();
        entry["name"] = toml_edit::value(edit.name);
        entry["tp2"] = toml_edit::value(edit.key);
        entries.push(entry);
    }
}

fn clear_name(entries: &mut toml_edit::ArrayOfTables, edit: &ModNameEdit<'_>) {
    entries.retain(|entry| {
        !edit.matches(entry) || entry_has_sources(entry) || entry.contains_key("categories")
    });
    for entry in entries.iter_mut().filter(|entry| edit.matches(entry)) {
        if entry_has_sources(entry) {
            entry["name"] = toml_edit::value(edit.unrenamed_name);
        } else {
            entry.remove("name");
        }
    }
}

#[cfg(test)]
#[path = "mod_rename_tests.rs"]
mod tests;
