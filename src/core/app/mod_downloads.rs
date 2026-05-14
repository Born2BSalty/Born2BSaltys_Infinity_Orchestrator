// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::platform_defaults::app_config_file;

const MOD_DOWNLOADS_USER_FILE_NAME: &str = "mod_downloads_user.toml";
const MOD_DOWNLOADS_DEFAULT_FILE_NAME: &str = "mod_downloads_default.toml";

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadsFile {
    #[serde(default)]
    mods: Vec<ModDownloadModOverlay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ModDownloadModOverlay {
    #[serde(flatten)]
    source: ModDownloadSourceOverlay,
    #[serde(default)]
    sources: Vec<ModDownloadSourceVariantOverlay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ModDownloadSourceOverlay {
    pub(crate) name: Option<String>,
    pub(crate) tp2: Option<String>,
    pub(crate) aliases: Option<Vec<String>>,
    pub(crate) config_files: Option<Vec<String>>,
    pub(crate) tp2_rename: Option<ModDownloadTp2Rename>,
    pub(crate) source_id: Option<String>,
    pub(crate) source_label: Option<String>,
    #[serde(default)]
    pub(crate) source_default: bool,
    #[serde(skip)]
    pub(crate) source_default_explicit: bool,
    pub(crate) url: Option<String>,
    pub(crate) github: Option<String>,
    pub(crate) exact_github: Option<Vec<String>>,
    pub(crate) channel: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) subdir_require: Option<String>,
    pub(crate) pkg_windows: Option<String>,
    pub(crate) pkg_linux: Option<String>,
    pub(crate) pkg_macos: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ModDownloadSourceVariantOverlay {
    pub(crate) id: Option<String>,
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) default: bool,
    pub(crate) aliases: Option<Vec<String>>,
    pub(crate) config_files: Option<Vec<String>>,
    pub(crate) tp2_rename: Option<ModDownloadTp2Rename>,
    pub(crate) url: Option<String>,
    pub(crate) repo: Option<String>,
    pub(crate) exact_github: Option<Vec<String>>,
    pub(crate) channel: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) subdir_require: Option<String>,
    pub(crate) pkg_windows: Option<String>,
    pub(crate) pkg_linux: Option<String>,
    pub(crate) pkg_macos: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadTp2Rename {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadSource {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) tp2: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(default)]
    pub(crate) config_files: Vec<String>,
    #[serde(default)]
    pub(crate) tp2_rename: Option<ModDownloadTp2Rename>,
    #[serde(default)]
    pub(crate) source_id: String,
    #[serde(default)]
    pub(crate) source_label: String,
    #[serde(default)]
    pub(crate) source_default: bool,
    #[serde(default)]
    pub(crate) source_default_explicit: bool,
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) github: Option<String>,
    #[serde(default)]
    pub(crate) exact_github: Vec<String>,
    #[serde(default)]
    pub(crate) channel: Option<String>,
    #[serde(default)]
    pub(crate) tag: Option<String>,
    #[serde(default)]
    pub(crate) commit: Option<String>,
    #[serde(default)]
    pub(crate) branch: Option<String>,
    #[serde(default)]
    pub(crate) asset: Option<String>,
    #[serde(default)]
    pub(crate) subdir_require: Option<String>,
    #[serde(default)]
    pub(crate) pkg_windows: Option<String>,
    #[serde(default)]
    pub(crate) pkg_linux: Option<String>,
    #[serde(default)]
    pub(crate) pkg_macos: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ModDownloadsLoad {
    pub(crate) sources: Vec<ModDownloadSource>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ModDownloadsOverlayLoad {
    sources: Vec<ModDownloadSourceOverlay>,
    error: Option<String>,
}

impl ModDownloadsLoad {
    pub(crate) fn default_source(&self, tp2: &str) -> Option<ModDownloadSource> {
        let sources = self.find_sources(tp2);
        sources
            .iter()
            .find(|source| source.source_default)
            .cloned()
            .or_else(|| sources.into_iter().next())
    }

    pub(crate) fn find_sources(&self, tp2: &str) -> Vec<ModDownloadSource> {
        let key = normalize_mod_download_tp2(tp2);
        let mut sources = self
            .sources
            .iter()
            .filter(|source| source_matches_tp2(source, &key))
            .cloned()
            .collect::<Vec<_>>();
        sort_sources(&mut sources);
        sources
    }

    pub(crate) fn resolve_source(
        &self,
        tp2: &str,
        selected_source_id: Option<&str>,
    ) -> Option<ModDownloadSource> {
        let sources = self.find_sources(tp2);
        if let Some(selected_source_id) = selected_source_id {
            let selected_key = normalize_source_id(selected_source_id);
            if let Some(source) = sources
                .iter()
                .find(|source| normalize_source_id(&source.source_id) == selected_key)
            {
                return Some(source.clone());
            }
        }
        sources.into_iter().next()
    }
}

pub(crate) fn mod_downloads_user_path() -> PathBuf {
    app_config_file(MOD_DOWNLOADS_USER_FILE_NAME, "config")
}

pub(crate) fn mod_downloads_default_path() -> PathBuf {
    app_config_file(MOD_DOWNLOADS_DEFAULT_FILE_NAME, "config")
}

pub(crate) fn ensure_mod_downloads_files() -> io::Result<()> {
    let default_path = mod_downloads_default_path();
    let user_path = mod_downloads_user_path();

    if let Some(parent) = default_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&default_path, default_mod_downloads_content())?;

    if !user_path.exists() {
        fs::write(&user_path, user_mod_downloads_content())?;
    }

    Ok(())
}

pub(crate) fn load_user_mod_download_source_block(
    tp2: &str,
    label: &str,
    source_id: &str,
    allow_source_id_change: bool,
) -> Result<String, String> {
    ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    if !allow_source_id_change {
        return Ok(load_mod_download_sources()
            .resolve_source(tp2, Some(source_id))
            .map(|source| source_to_editor_block(&source))
            .unwrap_or_else(|| template_source_block(label, source_id)));
    }
    let path = mod_downloads_user_path();
    let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    if let Some(block) = find_mod_block(&content, tp2)
        && let Some(source_block) = find_source_block(&block, source_id)
    {
        return Ok(source_block);
    }
    Ok(template_source_block(label, source_id))
}

pub(crate) fn save_user_mod_download_source_block(
    tp2: &str,
    label: &str,
    source_id: &str,
    allow_source_id_change: bool,
    source_block: &str,
) -> Result<(), String> {
    ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let path = mod_downloads_user_path();
    let content = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    if source_block.trim().is_empty() {
        let updated = remove_source_block(&content, tp2, source_id);
        toml::from_str::<ModDownloadsFile>(&updated).map_err(|err| err.to_string())?;
        fs::write(&path, updated).map_err(|err| err.to_string())?;
        return Ok(());
    }
    if !allow_source_id_change
        && let Some(edited_source_id) = source_block.lines().find_map(source_id_from_line)
        && normalize_source_id(&edited_source_id) != normalize_source_id(source_id)
    {
        return Err(format!(
            "Source id cannot be changed from \"{}\" to \"{}\" in Edit Source",
            source_id.trim(),
            edited_source_id.trim()
        ));
    }
    let source_input = normalize_source_save_input(tp2, label, source_block)?;
    let target_mod_exists = find_mod_block(&content, &source_input.tp2).is_some()
        || !load_mod_download_sources()
            .find_sources(&source_input.tp2)
            .is_empty();
    if !target_mod_exists {
        if !source_input.has_mod_parent {
            return Err(new_source_parent_error());
        }
        let updated = append_mod_block(&content, source_block);
        toml::from_str::<ModDownloadsFile>(&updated).map_err(|err| err.to_string())?;
        fs::write(&path, updated).map_err(|err| err.to_string())?;
        return Ok(());
    }
    let updated = replace_or_append_source_block(
        &content,
        &source_input.tp2,
        &source_input.label,
        source_id,
        &source_input.source_block,
    );
    toml::from_str::<ModDownloadsFile>(&updated).map_err(|err| err.to_string())?;
    fs::write(&path, updated).map_err(|err| err.to_string())
}

pub(crate) fn load_mod_download_sources() -> ModDownloadsLoad {
    let default_path = mod_downloads_default_path();
    let user_path = mod_downloads_user_path();
    let mut by_source = BTreeMap::<String, ModDownloadSource>::new();
    let default_load = load_source_overlays_from_path(&default_path);
    let user_load = load_source_overlays_from_path(&user_path);

    for overlay in default_load.sources {
        let key = overlay_source_key(&overlay);
        if !key.is_empty() {
            let mut source = ModDownloadSource::default();
            apply_source_overlay(&mut source, overlay);
            normalize_source(&mut source);
            if !source_is_valid(&source) {
                continue;
            }
            by_source.insert(key, source);
        }
    }
    for mut overlay in user_load.sources {
        let key = overlay_source_key(&overlay);
        if key.is_empty() {
            continue;
        }
        let user_default_tp2 = overlay
            .source_default_explicit
            .then(|| overlay_tp2_key(&overlay));
        if !overlay.source_default_explicit {
            overlay.source_default = false;
        }
        let mut source = by_source.remove(&key).unwrap_or_default();
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);
        if !source_is_valid(&source) {
            continue;
        }
        if let Some(tp2_key) = user_default_tp2.as_deref() {
            clear_other_source_defaults(&mut by_source, &key, tp2_key);
        }
        by_source.insert(key, source);
    }

    let mut sources = by_source.into_values().collect::<Vec<_>>();
    sort_sources(&mut sources);
    ModDownloadsLoad {
        sources,
        error: merge_load_errors(default_load.error, user_load.error),
    }
}

fn find_mod_block(content: &str, tp2: &str) -> Option<String> {
    let target = normalize_mod_download_tp2(tp2);
    for (start, end) in mod_block_ranges(content) {
        let block = &content[start..end];
        if block_tp2_matches(block, &target) {
            return Some(block.trim().to_string());
        }
    }
    None
}

fn find_source_block(mod_block: &str, source_id: &str) -> Option<String> {
    let target = normalize_source_id(source_id);
    for (start, end) in source_block_ranges(mod_block) {
        let block = &mod_block[start..end];
        if source_block_id_matches(block, &target) {
            return Some(normalize_source_block_for_editor(block));
        }
    }
    None
}

fn replace_or_append_source_block(
    content: &str,
    tp2: &str,
    label: &str,
    source_id: &str,
    source_block: &str,
) -> String {
    let target = normalize_mod_download_tp2(tp2);
    let source_block = source_block.trim();
    for (start, end) in mod_block_ranges(content) {
        let block = &content[start..end];
        if block_tp2_matches(block, &target) {
            let updated_block =
                replace_or_append_source_in_mod_block(block, source_id, source_block);
            let mut updated = String::new();
            updated.push_str(content[..start].trim_end());
            updated.push_str("\n\n");
            updated.push_str(updated_block.trim());
            updated.push_str("\n\n");
            updated.push_str(content[end..].trim_start());
            return updated.trim_end().to_string() + "\n";
        }
    }
    let mut updated = content.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&template_mod_header(tp2, label));
    updated.push_str("\n\n");
    updated.push_str(&normalize_source_block_indent(source_block));
    updated.push('\n');
    updated
}

fn append_mod_block(content: &str, mod_block: &str) -> String {
    let mut updated = content.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(mod_block.trim());
    updated.push('\n');
    updated
}

struct SourceSaveInput {
    tp2: String,
    label: String,
    source_block: String,
    has_mod_parent: bool,
}

fn normalize_source_save_input(
    tp2: &str,
    label: &str,
    source_block: &str,
) -> Result<SourceSaveInput, String> {
    let Ok(parsed) = toml::from_str::<ModDownloadsFile>(source_block) else {
        return Ok(SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    };
    let Some(mod_overlay) = parsed.mods.first() else {
        return Ok(SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    };
    let Some(parsed_tp2) = mod_overlay.source.tp2.as_deref() else {
        return Ok(SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    };
    let Some(parsed_label) = mod_overlay.source.name.as_deref() else {
        return Ok(SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    };
    let Some((source_start, source_end)) = source_block_ranges(source_block).first().copied()
    else {
        return Ok(SourceSaveInput {
            tp2: parsed_tp2.trim().to_string(),
            label: parsed_label.trim().to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    };
    if parsed_tp2.trim().is_empty() || parsed_label.trim().is_empty() {
        return Ok(SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    }
    if mod_overlay.sources.is_empty() {
        return Ok(SourceSaveInput {
            tp2: parsed_tp2.trim().to_string(),
            label: parsed_label.trim().to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        });
    }
    Ok(SourceSaveInput {
        tp2: parsed_tp2.trim().to_string(),
        label: parsed_label.trim().to_string(),
        source_block: source_block[source_start..source_end].trim().to_string(),
        has_mod_parent: true,
    })
}

fn new_source_parent_error() -> String {
    "New source entry must include a [[mods]] block with name and tp2.\n\nExample:\n\n[[mods]]\nname = \"My Mod\"\ntp2 = \"mymod\"\n\n  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/OWNER/REPO\"\n  repo = \"OWNER/REPO\""
        .to_string()
}

fn remove_source_block(content: &str, tp2: &str, source_id: &str) -> String {
    let target = normalize_mod_download_tp2(tp2);
    for (start, end) in mod_block_ranges(content) {
        let block = &content[start..end];
        if block_tp2_matches(block, &target) {
            let mut updated = String::new();
            updated.push_str(content[..start].trim_end());
            if let Some(updated_block) = remove_source_from_mod_block(block, source_id) {
                if !updated.is_empty() {
                    updated.push_str("\n\n");
                }
                updated.push_str(updated_block.trim());
            }
            let tail = content[end..].trim_start();
            if !tail.is_empty() {
                if !updated.is_empty() {
                    updated.push_str("\n\n");
                }
                updated.push_str(tail);
            }
            return updated.trim_end().to_string() + "\n";
        }
    }
    content.trim_end().to_string() + "\n"
}

fn remove_source_from_mod_block(mod_block: &str, source_id: &str) -> Option<String> {
    let target = normalize_source_id(source_id);
    let mut updated = String::new();
    let mut cursor = 0usize;
    let mut kept_source = false;
    for (start, end) in source_block_ranges(mod_block) {
        if source_block_id_matches(&mod_block[start..end], &target) {
            updated.push_str(mod_block[cursor..start].trim_end());
        } else {
            updated.push_str(&mod_block[cursor..end]);
            kept_source = true;
        }
        cursor = end;
    }
    updated.push_str(&mod_block[cursor..]);
    kept_source.then(|| updated.trim_end().to_string())
}

fn mod_block_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut starts = Vec::new();
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        if line.trim() == "[[mods]]" {
            starts.push(offset);
        }
        offset += line.len();
    }
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(content.len());
            (*start, end)
        })
        .collect()
}

fn source_block_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut starts = Vec::new();
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        if line.trim() == "[[mods.sources]]" {
            starts.push(offset);
        }
        offset += line.len();
    }
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(content.len());
            (*start, end)
        })
        .collect()
}

fn block_tp2_matches(block: &str, target: &str) -> bool {
    block
        .lines()
        .find_map(tp2_value_from_line)
        .is_some_and(|value| normalize_mod_download_tp2(&value) == target)
}

fn replace_or_append_source_in_mod_block(
    mod_block: &str,
    source_id: &str,
    source_block: &str,
) -> String {
    let target = normalize_source_id(source_id);
    let source_block = normalize_source_block_indent(source_block);
    let source_sets_default = source_block_has_default(&source_block);
    for (start, end) in source_block_ranges(mod_block) {
        if source_block_id_matches(&mod_block[start..end], &target) {
            let mut updated = String::new();
            updated.push_str(mod_block[..start].trim_end());
            updated.push_str("\n\n");
            updated.push_str(&source_block);
            updated.push_str("\n\n");
            updated.push_str(mod_block[end..].trim_start());
            let updated = updated.trim_end().to_string();
            return if source_sets_default {
                enforce_single_default_source(&updated, &target)
            } else {
                updated
            };
        }
    }
    let mut updated = mod_block.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(&source_block);
    if source_sets_default {
        enforce_single_default_source(&updated, &target)
    } else {
        updated
    }
}

fn source_block_id_matches(block: &str, target: &str) -> bool {
    block
        .lines()
        .find_map(source_id_from_line)
        .is_some_and(|value| normalize_source_id(&value) == target)
}

fn source_block_has_default(block: &str) -> bool {
    block
        .lines()
        .any(|line| bool_value_from_assignment(line.trim(), "default").unwrap_or(false))
}

fn enforce_single_default_source(mod_block: &str, selected_source_id: &str) -> String {
    let mut updated = String::new();
    let mut cursor = 0usize;
    let mut wrote_source = false;
    for (start, end) in source_block_ranges(mod_block) {
        updated.push_str(&mod_block[cursor..start]);
        let source_block = &mod_block[start..end];
        if wrote_source && !updated.ends_with("\n\n") {
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push('\n');
        }
        if source_block_id_matches(source_block, selected_source_id) {
            updated.push_str(&normalize_source_block_indent(source_block));
        } else {
            updated.push_str(&normalize_source_block_indent(&remove_default_true_lines(
                source_block,
            )));
        }
        wrote_source = true;
        cursor = end;
    }
    updated.push_str(&mod_block[cursor..]);
    updated.trim_end().to_string()
}

fn remove_default_true_lines(block: &str) -> String {
    let mut cleaned = block
        .lines()
        .filter(|line| !bool_value_from_assignment(line.trim(), "default").unwrap_or(false))
        .collect::<Vec<_>>()
        .join("\n");
    if block.ends_with('\n') {
        cleaned.push('\n');
    }
    cleaned
}

fn normalize_source_block_indent(block: &str) -> String {
    let mut ordered = SOURCE_BLOCK_FIELD_ORDER
        .iter()
        .map(|key| (*key, Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut unknown = Vec::<String>::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "[[mods.sources]]" {
            continue;
        }
        if let Some(key) = assignment_key(trimmed)
            && let Some(lines) = ordered.get_mut(key)
        {
            lines.push(trimmed.to_string());
            continue;
        }
        unknown.push(trimmed.to_string());
    }

    let mut lines = vec!["  [[mods.sources]]".to_string()];
    for key in SOURCE_BLOCK_FIELD_ORDER {
        if let Some(values) = ordered.get(key) {
            lines.extend(values.iter().map(|line| format!("  {line}")));
        }
    }
    lines.extend(unknown.iter().map(|line| format!("  {line}")));

    let mut cleaned = lines.join("\n");
    if block.ends_with('\n') {
        cleaned.push('\n');
    }
    cleaned
}

fn normalize_source_block_for_editor(block: &str) -> String {
    normalize_source_block_indent(block)
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

const SOURCE_BLOCK_FIELD_ORDER: &[&str] = &[
    "id",
    "label",
    "type",
    "url",
    "repo",
    "exact_github",
    "channel",
    "tag",
    "commit",
    "branch",
    "asset",
    "subdir_require",
    "config_files",
    "aliases",
    "tp2_rename",
    "pkg_windows",
    "pkg_linux",
    "pkg_macos",
    "default",
];

fn assignment_key(line: &str) -> Option<&str> {
    line.split_once('=')
        .map(|(key, _)| key.trim())
        .filter(|key| !key.is_empty())
}

fn source_id_from_line(line: &str) -> Option<String> {
    quoted_value_from_assignment(line.trim(), "id")
}

fn tp2_value_from_line(line: &str) -> Option<String> {
    quoted_value_from_assignment(line.trim(), "tp2")
}

fn quoted_value_from_assignment(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    Some(value.trim_matches('"').to_string())
}

fn bool_value_from_assignment(line: &str, key: &str) -> Option<bool> {
    let rest = line.strip_prefix(key)?.trim_start();
    let value = rest.strip_prefix('=')?.trim();
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn template_mod_header(tp2: &str, label: &str) -> String {
    let name = if label.trim().is_empty() {
        tp2.trim()
    } else {
        label.trim()
    };
    format!(
        "[[mods]]\nname = \"{}\"\ntp2 = \"{}\"",
        escape_toml_string(name),
        escape_toml_string(tp2.trim())
    )
}

fn source_to_editor_block(source: &ModDownloadSource) -> String {
    let mut lines = vec![
        "[[mods.sources]]".to_string(),
        format!("id = \"{}\"", escape_toml_string(&source.source_id)),
        format!("label = \"{}\"", escape_toml_string(&source.source_label)),
        "type = \"github\"".to_string(),
        format!("url = \"{}\"", escape_toml_string(&source.url)),
    ];
    if let Some(github) = source.github.as_ref() {
        lines.push(format!("repo = \"{}\"", escape_toml_string(github)));
    }
    for exact_github in &source.exact_github {
        lines.push(format!(
            "exact_github = \"{}\"",
            escape_toml_string(exact_github)
        ));
    }
    if let Some(channel) = source.channel.as_ref() {
        lines.push(format!("channel = \"{}\"", escape_toml_string(channel)));
    }
    if let Some(tag) = source.tag.as_ref() {
        lines.push(format!("tag = \"{}\"", escape_toml_string(tag)));
    }
    if let Some(commit) = source.commit.as_ref() {
        lines.push(format!("commit = \"{}\"", escape_toml_string(commit)));
    }
    if let Some(branch) = source.branch.as_ref() {
        lines.push(format!("branch = \"{}\"", escape_toml_string(branch)));
    }
    if let Some(asset) = source.asset.as_ref() {
        lines.push(format!("asset = \"{}\"", escape_toml_string(asset)));
    }
    if let Some(subdir_require) = source.subdir_require.as_ref() {
        lines.push(format!(
            "subdir_require = \"{}\"",
            escape_toml_string(subdir_require)
        ));
    }
    if !source.config_files.is_empty() {
        lines.push(format!(
            "config_files = [{}]",
            source
                .config_files
                .iter()
                .map(|config_file| format!("\"{}\"", escape_toml_string(config_file)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !source.aliases.is_empty() {
        lines.push(format!(
            "aliases = [{}]",
            source
                .aliases
                .iter()
                .map(|alias| format!("\"{}\"", escape_toml_string(alias)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(tp2_rename) = source.tp2_rename.as_ref() {
        lines.push(format!(
            "tp2_rename = {{ from = \"{}\", to = \"{}\" }}",
            escape_toml_string(&tp2_rename.from),
            escape_toml_string(&tp2_rename.to)
        ));
    }
    if let Some(pkg_windows) = source.pkg_windows.as_ref() {
        lines.push(format!(
            "pkg_windows = \"{}\"",
            escape_toml_string(pkg_windows)
        ));
    }
    if let Some(pkg_linux) = source.pkg_linux.as_ref() {
        lines.push(format!("pkg_linux = \"{}\"", escape_toml_string(pkg_linux)));
    }
    if let Some(pkg_macos) = source.pkg_macos.as_ref() {
        lines.push(format!("pkg_macos = \"{}\"", escape_toml_string(pkg_macos)));
    }
    if source.source_default && source.source_default_explicit {
        lines.push("default = true".to_string());
    }
    lines.join("\n")
}

fn template_source_block(_label: &str, source_id: &str) -> String {
    format!(
        "[[mods.sources]]\nid = \"{}\"\nlabel = \"GitHub\"\ntype = \"github\"\nurl = \"https://github.com/OWNER/REPO\"\nrepo = \"OWNER/REPO\"\ndefault = true",
        escape_toml_string(source_id.trim())
    )
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(crate) fn normalize_mod_download_tp2(value: &str) -> String {
    let replaced = value.replace('\\', "/").to_ascii_lowercase();
    let file = replaced.rsplit('/').next().unwrap_or(&replaced).trim();
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

pub(crate) fn source_matches_tp2(source: &ModDownloadSource, normalized_tp2: &str) -> bool {
    normalize_mod_download_tp2(&source.tp2) == normalized_tp2
        || source
            .aliases
            .iter()
            .any(|alias| normalize_mod_download_tp2(alias) == normalized_tp2)
}

pub(crate) fn source_is_auto_resolvable(source: &ModDownloadSource) -> bool {
    source.github.is_some()
        || is_direct_archive_url(&source.url)
        || source_is_sentrizeal_download_url(&source.url)
        || source_is_page_archive_url(&source.url)
}

pub(crate) fn preferred_pkg_for_current_platform(source: &ModDownloadSource) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        source.pkg_windows.clone()
    }
    #[cfg(target_os = "linux")]
    {
        source.pkg_linux.clone()
    }
    #[cfg(target_os = "macos")]
    {
        source.pkg_macos.clone()
    }
}

pub(crate) fn is_direct_archive_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    [
        ".zip", ".7z", ".rar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

pub(crate) fn source_is_sentrizeal_download_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://www.sentrizeal.com/downloaditm")
        || lower.starts_with("http://www.sentrizeal.com/downloaditm")
        || lower.starts_with("https://sentrizeal.com/downloaditm")
        || lower.starts_with("http://sentrizeal.com/downloaditm")
}

fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == content => Ok(()),
        _ => fs::write(path, content),
    }
}

fn default_mod_downloads_content() -> &'static str {
    include_str!("../config/default_mod_downloads.toml")
}

fn user_mod_downloads_content() -> &'static str {
    include_str!("../config/user_mod_downloads.toml")
}

pub(crate) fn source_is_weaselmods_page_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://downloads.weaselmods.net/download/")
        || lower.starts_with("http://downloads.weaselmods.net/download/")
}

pub(crate) fn source_is_morpheus_mart_page_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://www.morpheus-mart.com/")
        || lower.starts_with("http://www.morpheus-mart.com/")
        || lower.starts_with("https://morpheus-mart.com/")
        || lower.starts_with("http://morpheus-mart.com/")
}

pub(crate) fn source_is_page_archive_url(url: &str) -> bool {
    source_is_weaselmods_page_url(url) || source_is_morpheus_mart_page_url(url)
}

fn load_source_overlays_from_path(path: &Path) -> ModDownloadsOverlayLoad {
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return ModDownloadsOverlayLoad {
                sources: Vec::new(),
                error: Some(format!(
                    "mod downloads load failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    let parsed = match toml::from_str::<ModDownloadsFile>(&content) {
        Ok(value) => value,
        Err(err) => {
            return ModDownloadsOverlayLoad {
                sources: Vec::new(),
                error: Some(format!(
                    "mod downloads parse failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    ModDownloadsOverlayLoad {
        sources: parsed
            .mods
            .into_iter()
            .flat_map(flatten_mod_overlay_entries)
            .collect(),
        error: None,
    }
}

fn overlay_tp2_key(source: &ModDownloadSourceOverlay) -> String {
    source
        .tp2
        .as_deref()
        .map(normalize_mod_download_tp2)
        .unwrap_or_default()
}

fn overlay_source_key(source: &ModDownloadSourceOverlay) -> String {
    let tp2 = overlay_tp2_key(source);
    if tp2.is_empty() {
        return String::new();
    }
    let source_id = normalize_source_id(source.source_id.as_deref().unwrap_or("primary"));
    format!("{tp2}|{source_id}")
}

fn apply_source_overlay(target: &mut ModDownloadSource, overlay: ModDownloadSourceOverlay) {
    if let Some(name) = overlay.name {
        target.name = name;
    }
    if let Some(tp2) = overlay.tp2 {
        target.tp2 = tp2;
    }
    if let Some(aliases) = overlay.aliases {
        target.aliases = aliases;
    }
    if let Some(config_files) = overlay.config_files {
        target.config_files = config_files;
    }
    if let Some(tp2_rename) = overlay.tp2_rename {
        target.tp2_rename = Some(tp2_rename);
    }
    if let Some(source_id) = overlay.source_id {
        target.source_id = source_id;
    }
    if let Some(source_label) = overlay.source_label {
        target.source_label = source_label;
    }
    if overlay.source_default_explicit {
        target.source_default_explicit = true;
    }
    if overlay.source_default {
        target.source_default = true;
    }
    if let Some(url) = overlay.url {
        target.url = url;
    }
    if let Some(github) = overlay.github {
        target.github = Some(github);
    }
    if let Some(exact_github) = overlay.exact_github {
        target.exact_github = exact_github;
    }
    if let Some(channel) = overlay.channel {
        target.channel = Some(channel);
    }
    if let Some(tag) = overlay.tag {
        target.tag = Some(tag);
    }
    if let Some(commit) = overlay.commit {
        target.commit = Some(commit);
    }
    if let Some(branch) = overlay.branch {
        target.branch = Some(branch);
    }
    if let Some(asset) = overlay.asset {
        target.asset = Some(asset);
    }
    if let Some(subdir_require) = overlay.subdir_require {
        target.subdir_require = Some(subdir_require);
    }
    if let Some(pkg_windows) = overlay.pkg_windows {
        target.pkg_windows = Some(pkg_windows);
    }
    if let Some(pkg_linux) = overlay.pkg_linux {
        target.pkg_linux = Some(pkg_linux);
    }
    if let Some(pkg_macos) = overlay.pkg_macos {
        target.pkg_macos = Some(pkg_macos);
    }
}

fn clear_other_source_defaults(
    sources: &mut BTreeMap<String, ModDownloadSource>,
    selected_key: &str,
    normalized_tp2: &str,
) {
    for (key, source) in sources {
        if key.as_str() != selected_key && source_matches_tp2(source, normalized_tp2) {
            source.source_default = false;
        }
    }
}

fn flatten_mod_overlay_entries(
    mod_overlay: ModDownloadModOverlay,
) -> Vec<ModDownloadSourceOverlay> {
    if mod_overlay.sources.is_empty() {
        let mut overlay = mod_overlay.source;
        overlay.source_default_explicit = overlay.source_default;
        if overlay.source_id.is_none() {
            overlay.source_id = Some("primary".to_string());
        }
        if overlay.source_label.is_none() {
            overlay.source_label = Some("Primary".to_string());
        }
        overlay.source_default = true;
        return vec![overlay];
    }

    let has_default = mod_overlay.sources.iter().any(|source| source.default);
    mod_overlay
        .sources
        .into_iter()
        .enumerate()
        .map(|(index, source_overlay)| {
            let source_default_explicit =
                mod_overlay.source.source_default || source_overlay.default;
            let mut overlay = mod_overlay.source.clone();
            apply_source_variant_overlay(&mut overlay, source_overlay);
            overlay.source_default_explicit = source_default_explicit;
            if overlay.source_id.is_none() {
                overlay.source_id = Some(if index == 0 {
                    "primary".to_string()
                } else {
                    format!("source-{}", index + 1)
                });
            }
            if overlay.source_label.is_none() {
                overlay.source_label = overlay.source_id.clone();
            }
            if !has_default && index == 0 {
                overlay.source_default = true;
            }
            overlay
        })
        .collect()
}

fn apply_source_variant_overlay(
    target: &mut ModDownloadSourceOverlay,
    overlay: ModDownloadSourceVariantOverlay,
) {
    let ModDownloadSourceVariantOverlay {
        id,
        label,
        default,
        aliases,
        config_files,
        tp2_rename,
        url,
        repo,
        exact_github,
        channel,
        tag,
        commit,
        branch,
        asset,
        subdir_require,
        pkg_windows,
        pkg_linux,
        pkg_macos,
    } = overlay;

    if let Some(id) = id {
        target.source_id = Some(id);
    }
    if let Some(label) = label {
        target.source_label = Some(label);
    }
    if default {
        target.source_default = true;
    }
    if let Some(aliases) = aliases {
        target.aliases = Some(aliases);
    }
    if let Some(config_files) = config_files {
        target.config_files = Some(config_files);
    }
    if let Some(tp2_rename) = tp2_rename {
        target.tp2_rename = Some(tp2_rename);
    }
    if let Some(url) = url {
        target.url = Some(url);
    }
    if let Some(repo) = repo {
        let trimmed = repo.trim().to_string();
        if !trimmed.is_empty() {
            target.github = Some(trimmed.clone());
            let current_url = target.url.as_deref().map(str::trim).unwrap_or_default();
            if current_url.is_empty() {
                target.url = Some(format!("https://github.com/{trimmed}"));
            }
        }
    }
    if let Some(exact_github) = exact_github {
        target.exact_github = Some(exact_github);
    }
    if let Some(channel) = channel {
        target.channel = Some(channel);
    }
    if let Some(tag) = tag {
        target.tag = Some(tag);
    }
    if let Some(commit) = commit {
        target.commit = Some(commit);
    }
    if let Some(branch) = branch {
        target.branch = Some(branch);
    }
    if let Some(asset) = asset {
        target.asset = Some(asset);
    }
    if let Some(subdir_require) = subdir_require {
        target.subdir_require = Some(subdir_require);
    }
    if let Some(pkg_windows) = pkg_windows {
        target.pkg_windows = Some(pkg_windows);
    }
    if let Some(pkg_linux) = pkg_linux {
        target.pkg_linux = Some(pkg_linux);
    }
    if let Some(pkg_macos) = pkg_macos {
        target.pkg_macos = Some(pkg_macos);
    }
}

fn normalize_source(source: &mut ModDownloadSource) {
    source.name = source.name.trim().to_string();
    source.tp2 = source.tp2.trim().to_string();
    source.aliases = source
        .aliases
        .iter()
        .map(|alias| alias.trim().to_string())
        .filter(|alias| !alias.is_empty())
        .collect();
    source.aliases.dedup();
    source.config_files = source
        .config_files
        .iter()
        .map(|config_file| config_file.trim().to_string())
        .filter(|config_file| !config_file.is_empty())
        .collect();
    source.config_files.sort();
    source.config_files.dedup();
    source.tp2_rename = source.tp2_rename.take().and_then(|rename| {
        let from = rename.from.trim().to_string();
        let to = rename.to.trim().to_string();
        (!from.is_empty() && !to.is_empty()).then_some(ModDownloadTp2Rename { from, to })
    });
    source.source_id = source.source_id.trim().to_string();
    if source.source_id.is_empty() {
        source.source_id = "primary".to_string();
    }
    source.source_label = source.source_label.trim().to_string();
    if source.source_label.is_empty() {
        source.source_label = source.source_id.clone();
    }
    source.url = source.url.trim().to_string();
    source.github = source
        .github
        .take()
        .map(|github| github.trim().to_string())
        .filter(|github| !github.is_empty());
    source.exact_github = source
        .exact_github
        .iter()
        .map(|github| github.trim().to_string())
        .filter(|github| !github.is_empty())
        .collect();
    if let Some(primary) = source.github.as_deref() {
        source
            .exact_github
            .retain(|github| !github.eq_ignore_ascii_case(primary));
    }
    source.channel = source
        .channel
        .take()
        .map(|channel| channel.trim().to_string())
        .map(|channel| {
            if channel == "releases" {
                "release".to_string()
            } else {
                channel
            }
        })
        .filter(|channel| {
            matches!(
                channel.as_str(),
                "release" | "pre-release" | "preonly" | "master" | "ifeellucky"
            )
        });
    source.tag = source
        .tag
        .take()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty());
    source.commit = source
        .commit
        .take()
        .map(|commit| commit.trim().to_string())
        .filter(|commit| !commit.is_empty());
    source.branch = source
        .branch
        .take()
        .map(|branch| branch.trim().to_string())
        .filter(|branch| !branch.is_empty());
    source.asset = source
        .asset
        .take()
        .map(|asset| asset.trim().to_string())
        .filter(|asset| !asset.is_empty());
    source.subdir_require = source
        .subdir_require
        .take()
        .map(|subdir_require| subdir_require.trim().to_string())
        .filter(|subdir_require| !subdir_require.is_empty());
    if source.commit.is_some() {
        source.channel = None;
        source.tag = None;
        source.branch = None;
        source.asset = None;
        source.pkg_windows = None;
        source.pkg_linux = None;
        source.pkg_macos = None;
        return;
    }
    if source.tag.is_some() {
        source.channel = None;
        source.branch = None;
        source.asset = None;
        source.pkg_windows = None;
        source.pkg_linux = None;
        source.pkg_macos = None;
        return;
    }
    if source.branch.is_some() {
        source.channel = None;
        source.asset = None;
        source.pkg_windows = None;
        source.pkg_linux = None;
        source.pkg_macos = None;
        return;
    }
    source.pkg_windows = source
        .pkg_windows
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
    source.pkg_linux = source
        .pkg_linux
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
    source.pkg_macos = source
        .pkg_macos
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
}

fn source_is_valid(source: &ModDownloadSource) -> bool {
    !normalize_mod_download_tp2(&source.tp2).is_empty() && !source.url.is_empty()
}

fn normalize_source_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn sort_sources(sources: &mut [ModDownloadSource]) {
    sources.sort_by(|left, right| {
        normalize_mod_download_tp2(&left.tp2)
            .cmp(&normalize_mod_download_tp2(&right.tp2))
            .then_with(|| right.source_default.cmp(&left.source_default))
            .then_with(|| left.source_label.cmp(&right.source_label))
            .then_with(|| left.source_id.cmp(&right.source_id))
    });
}

fn merge_load_errors(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("{left} | {right}")),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
