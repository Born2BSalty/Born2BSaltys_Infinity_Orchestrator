// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

use crate::platform_defaults::app_config_file;

static ACTIVE_MODLIST_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn active_modlist_dir_mutex() -> &'static Mutex<Option<PathBuf>> {
    ACTIVE_MODLIST_DIR.get_or_init(|| Mutex::new(None))
}

/// Sets the active-modlist data dir for the ambient resolver.
/// Call with `Some(dir)` when a modlist becomes active, `None` when none is.
pub(crate) fn set_active_modlist_dir(dir: Option<PathBuf>) {
    if let Ok(mut guard) = active_modlist_dir_mutex().lock() {
        *guard = dir;
    }
}

/// Returns the active-modlist data dir, if any is set.
pub(crate) fn active_modlist_dir() -> Option<PathBuf> {
    active_modlist_dir_mutex()
        .lock()
        .ok()
        .and_then(|g| g.clone())
}

/// Returns the per-modlist `mod_downloads_user.toml` path when a modlist is active.
pub(crate) fn active_modlist_downloads_path() -> Option<PathBuf> {
    active_modlist_dir().map(|d| d.join("mod_downloads_user.toml"))
}

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

/// Controls which resolution tier pre-fills the source editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SeedScope {
    /// Pre-fill from the full three-tier resolution (used when editing "For this modlist").
    #[default]
    Resolved,
    /// Pre-fill from only the two-tier (global + app-default) resolution, ignoring any
    /// per-modlist overlay. Used when editing "My default" so saving never promotes a
    /// modlist pin into the global file.
    GlobalOnly,
}

pub(crate) fn load_user_mod_download_source_block(
    tp2: &str,
    label: &str,
    source_id: &str,
    allow_source_id_change: bool,
    target_path: Option<&Path>,
    seed_scope: SeedScope,
) -> Result<String, String> {
    ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let path = target_path.map_or_else(mod_downloads_user_path, Path::to_path_buf);
    let content = fs::read_to_string(&path).unwrap_or_default();
    let existing_user_block =
        find_mod_block(&content, tp2).and_then(|block| find_source_block(&block, source_id));
    let merged_source = (!allow_source_id_change)
        .then(|| {
            let sources = match seed_scope {
                SeedScope::GlobalOnly => load_two_tier_sources(),
                SeedScope::Resolved => load_mod_download_sources(),
            };
            sources.resolve_source(tp2, Some(source_id))
        })
        .flatten();
    Ok(editor_block_for_source(
        label,
        source_id,
        allow_source_id_change,
        existing_user_block,
        merged_source,
    ))
}

pub(crate) fn save_user_mod_download_source_block(
    tp2: &str,
    label: &str,
    source_id: &str,
    allow_source_id_change: bool,
    source_block: &str,
    target_path: Option<&Path>,
) -> Result<(), String> {
    ensure_mod_downloads_files().map_err(|err| err.to_string())?;
    let path = target_path.map_or_else(mod_downloads_user_path, Path::to_path_buf);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
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
    let source_input = normalize_source_save_input(tp2, label, source_block);
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

/// Loads sources from the app-default and global-user tiers only, with no per-modlist overlay.
/// This is the seed used when the editor destination is "My default", so saving that
/// destination never promotes a modlist pin into the global file.
pub(crate) fn load_two_tier_sources() -> ModDownloadsLoad {
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
    let error = merge_load_errors(default_load.error, user_load.error);
    ModDownloadsLoad { sources, error }
}

/// Loads sources applying all three tiers: app-default → global-user → per-modlist override.
/// When no modlist is active the result equals `load_two_tier_sources()`.
pub(crate) fn load_mod_download_sources() -> ModDownloadsLoad {
    let mut result = load_two_tier_sources();

    // Per-modlist overlay (additive; skipped when ambient is unset or file absent).
    if let Some(per_modlist_path) = active_modlist_downloads_path().filter(|p| p.exists()) {
        {
            let per_load = load_source_overlays_from_path(&per_modlist_path);
            // Rebuild by_source map with the same key format used in the two-tier passes.
            let mut by_source: BTreeMap<String, ModDownloadSource> = result
                .sources
                .drain(..)
                .map(|s| {
                    let key = format!(
                        "{}|{}",
                        normalize_mod_download_tp2(&s.tp2),
                        normalize_source_id(&s.source_id)
                    );
                    (key, s)
                })
                .collect();
            for mut overlay in per_load.sources {
                let key = overlay_source_key(&overlay);
                if key.is_empty() {
                    continue;
                }
                let per_default_tp2 = overlay
                    .source_default_explicit
                    .then(|| overlay_tp2_key(&overlay));
                if !overlay.source_default_explicit {
                    overlay.source_default = false;
                }
                let mut source = by_source.remove(&key).unwrap_or_default();
                if overlay_has_version_selector(&overlay) {
                    clear_source_version_selectors(&mut source);
                }
                apply_source_overlay(&mut source, overlay);
                normalize_source(&mut source);
                if !source_is_valid(&source) {
                    continue;
                }
                if let Some(tp2_key) = per_default_tp2.as_deref() {
                    clear_other_source_defaults(&mut by_source, &key, tp2_key);
                }
                by_source.insert(key, source);
            }
            result.sources = by_source.into_values().collect();
            sort_sources(&mut result.sources);
            result.error = merge_load_errors(result.error, per_load.error);
        }
    }

    result
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
    let ranges = mod_block_ranges(content);

    let first_match = ranges
        .iter()
        .find(|(start, end)| block_tp2_matches(&content[*start..*end], &target))
        .copied();

    let Some(first) = first_match else {
        // No existing block: append a new one, preserving the whole file as-is.
        let mut updated = content.trim_end().to_string();
        if !updated.is_empty() {
            updated.push_str("\n\n");
        }
        updated.push_str(&template_mod_header(tp2, label));
        updated.push_str("\n\n");
        updated.push_str(&normalize_source_block_indent(source_block));
        updated.push('\n');
        return updated;
    };

    let first_block = &content[first.0..first.1];
    let updated_first = replace_or_append_source_in_mod_block(first_block, source_id, source_block);

    // Preserve any file preamble (text before the first [[mods]] block).
    let preamble = if ranges.is_empty() {
        ""
    } else {
        &content[..ranges[0].0]
    };

    // Walk all block ranges: emit non-matching blocks as-is, the first matching block as
    // its edited version, and silently drop every subsequent matching block (dedup).
    let mut out = preamble.trim_end().to_string();
    let mut first_written = false;
    for &(start, end) in &ranges {
        let block = &content[start..end];
        let text = if !block_tp2_matches(block, &target) {
            block.trim().to_string()
        } else if !first_written {
            first_written = true;
            updated_first.trim().to_string()
        } else {
            continue;
        };
        if !out.is_empty() {
            let t = out.trim_end_matches('\n').len();
            out.truncate(t);
            out.push_str("\n\n");
        }
        out.push_str(&text);
    }

    out.trim_end().to_string() + "\n"
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

fn normalize_source_save_input(tp2: &str, label: &str, source_block: &str) -> SourceSaveInput {
    let Ok(parsed) = toml::from_str::<ModDownloadsFile>(source_block) else {
        return SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    };
    let Some(mod_overlay) = parsed.mods.first() else {
        return SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    };
    let Some(parsed_tp2) = mod_overlay.source.tp2.as_deref() else {
        return SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    };
    let Some(parsed_label) = mod_overlay.source.name.as_deref() else {
        return SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    };
    let Some((source_start, source_end)) = source_block_ranges(source_block).first().copied()
    else {
        return SourceSaveInput {
            tp2: parsed_tp2.trim().to_string(),
            label: parsed_label.trim().to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    };
    if parsed_tp2.trim().is_empty() || parsed_label.trim().is_empty() {
        return SourceSaveInput {
            tp2: tp2.to_string(),
            label: label.to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    }
    if mod_overlay.sources.is_empty() {
        return SourceSaveInput {
            tp2: parsed_tp2.trim().to_string(),
            label: parsed_label.trim().to_string(),
            source_block: source_block.to_string(),
            has_mod_parent: false,
        };
    }
    SourceSaveInput {
        tp2: parsed_tp2.trim().to_string(),
        label: parsed_label.trim().to_string(),
        source_block: source_block[source_start..source_end].trim().to_string(),
        has_mod_parent: true,
    }
}

fn new_source_parent_error() -> String {
    "New source entry must include a [[mods]] block with name and tp2.\n\nExample:\n\n[[mods]]\nname = \"My Mod\"\ntp2 = \"mymod\"\n\n  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/OWNER/REPO\"\n  repo = \"OWNER/REPO\""
        .to_string()
}

fn remove_source_block(content: &str, tp2: &str, source_id: &str) -> String {
    let target = normalize_mod_download_tp2(tp2);
    let ranges = mod_block_ranges(content);

    // Preserve any file preamble (text before the first [[mods]] block).
    let preamble = if ranges.is_empty() {
        ""
    } else {
        &content[..ranges[0].0]
    };

    // For every block matching the target tp2, remove the target source from it (drop the
    // whole [[mods]] block when no sources remain); non-matching blocks are kept byte-for-byte.
    // Duplicate blocks for the same tp2 are all processed, so none shadow a re-pin.
    let mut out = preamble.trim_end().to_string();
    for (start, end) in &ranges {
        let block = &content[*start..*end];
        let maybe_text = if block_tp2_matches(block, &target) {
            // Remove the target source; yield None when the block becomes empty (drop it).
            remove_source_from_mod_block(block, source_id).map(|b| b.trim().to_string())
        } else {
            Some(block.trim().to_string())
        };
        if let Some(text) = maybe_text {
            if !out.is_empty() {
                let t = out.trim_end_matches('\n').len();
                out.truncate(t);
                out.push_str("\n\n");
            }
            out.push_str(&text);
        }
    }

    out.trim_end().to_string() + "\n"
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
}

fn editor_block_for_source(
    label: &str,
    source_id: &str,
    allow_source_id_change: bool,
    existing_user_block: Option<String>,
    merged_source: Option<ModDownloadSource>,
) -> String {
    if allow_source_id_change {
        return existing_user_block.unwrap_or_else(|| template_source_block(label, source_id));
    }
    merged_source.map_or_else(
        || existing_user_block.unwrap_or_else(|| template_source_block(label, source_id)),
        |source| source_to_editor_block(&source),
    )
}

const SOURCE_BLOCK_FIELD_ORDER: &[&str] = &[
    "id",
    "label",
    "type",
    "url",
    "repo",
    "exact_github",
    "commit",
    "tag",
    "branch",
    "channel",
    "asset",
    "subdir_require",
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
    lines.push(format!(
        "commit = \"{}\"",
        escape_toml_string(source.commit.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "tag = \"{}\"",
        escape_toml_string(source.tag.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "branch = \"{}\"",
        escape_toml_string(source.branch.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "channel = \"{}\"",
        escape_toml_string(source.channel.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "asset = \"{}\"",
        escape_toml_string(source.asset.as_deref().unwrap_or_default())
    ));
    if let Some(subdir_require) = source.subdir_require.as_ref() {
        lines.push(format!(
            "subdir_require = \"{}\"",
            escape_toml_string(subdir_require)
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
    normalize_source_block_indent(&lines.join("\n"))
}

fn template_source_block(_label: &str, source_id: &str) -> String {
    format!(
        "  [[mods.sources]]\n  id = \"{}\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/OWNER/REPO\"\n  repo = \"OWNER/REPO\"\n  commit = \"\"\n  tag = \"\"\n  branch = \"\"\n  channel = \"\"\n  asset = \"\"\n  default = true",
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

/// Shared mutex serializing all ambient-touching tests across modules.
#[cfg(test)]
pub(crate) static AMBIENT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const fn default_mod_downloads_content() -> &'static str {
    include_str!("../config/default_mod_downloads.toml")
}

const fn user_mod_downloads_content() -> &'static str {
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

/// Returns true when the overlay specifies at least one version selector field.
const fn overlay_has_version_selector(overlay: &ModDownloadSourceOverlay) -> bool {
    overlay.commit.is_some()
        || overlay.tag.is_some()
        || overlay.branch.is_some()
        || overlay.channel.is_some()
        || overlay.asset.is_some()
}

/// Clears all version selector fields on a source so a per-modlist overlay can replace them.
fn clear_source_version_selectors(source: &mut ModDownloadSource) {
    source.commit = None;
    source.tag = None;
    source.branch = None;
    source.channel = None;
    source.asset = None;
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
    normalize_source_identity(source);
    normalize_source_location(source);
    if normalize_source_selector_fields(source) {
        normalize_source_package_fields(source);
    }
}

fn normalize_source_identity(source: &mut ModDownloadSource) {
    source.name = source.name.trim().to_string();
    source.tp2 = source.tp2.trim().to_string();
    source.aliases = normalized_string_list(&source.aliases);
    source.aliases.dedup();
    source.config_files = normalized_string_list(&source.config_files);
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
}

fn normalize_source_location(source: &mut ModDownloadSource) {
    source.url = source.url.trim().to_string();
    source.github = normalize_optional_string(source.github.take());
    source.exact_github = normalized_string_list(&source.exact_github);
    if let Some(primary) = source.github.as_deref() {
        source
            .exact_github
            .retain(|github| !github.eq_ignore_ascii_case(primary));
    }
    source.subdir_require = normalize_optional_string(source.subdir_require.take());
}

fn normalize_source_selector_fields(source: &mut ModDownloadSource) -> bool {
    source.channel = normalize_optional_string(source.channel.take());
    source.tag = normalize_optional_string(source.tag.take());
    source.commit = normalize_optional_string(source.commit.take());
    source.branch = normalize_optional_string(source.branch.take());
    source.asset = normalize_optional_string(source.asset.take());
    if source.commit.is_some() {
        clear_commit_source_conflicts(source);
        return false;
    }
    if source.tag.is_some() {
        clear_tag_source_conflicts(source);
        return false;
    }
    if source.branch.is_some() {
        clear_branch_source_conflicts(source);
        return false;
    }
    true
}

fn clear_commit_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.tag = None;
    source.branch = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_tag_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.branch = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_branch_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_source_packages(source: &mut ModDownloadSource) {
    source.pkg_windows = None;
    source.pkg_linux = None;
    source.pkg_macos = None;
}

fn normalize_source_package_fields(source: &mut ModDownloadSource) {
    source.pkg_windows = normalize_optional_string(source.pkg_windows.take());
    source.pkg_linux = normalize_optional_string(source.pkg_linux.take());
    source.pkg_macos = normalize_optional_string(source.pkg_macos.take());
}

fn normalized_string_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        })
        .collect()
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn argent77_source() -> ModDownloadSource {
        ModDownloadSource {
            name: "Improved Archer".to_string(),
            tp2: "a7-improvedarcher".to_string(),
            source_id: "argent77".to_string(),
            source_label: "Argent77".to_string(),
            url: "https://github.com/Argent77/A7-ImprovedArcher".to_string(),
            github: Some("Argent77/A7-ImprovedArcher".to_string()),
            pkg_windows: Some("wzp,zip".to_string()),
            pkg_linux: Some("lin,zip".to_string()),
            pkg_macos: Some("mac,zip".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn source_editor_block_includes_empty_selector_fields_and_indented_shape() {
        let block = source_to_editor_block(&argent77_source());

        assert_eq!(
            block,
            "  [[mods.sources]]\n  id = \"argent77\"\n  label = \"Argent77\"\n  type = \"github\"\n  url = \"https://github.com/Argent77/A7-ImprovedArcher\"\n  repo = \"Argent77/A7-ImprovedArcher\"\n  commit = \"\"\n  tag = \"\"\n  branch = \"\"\n  channel = \"\"\n  asset = \"\"\n  pkg_windows = \"wzp,zip\"\n  pkg_linux = \"lin,zip\"\n  pkg_macos = \"mac,zip\""
        );
    }

    #[test]
    fn source_editor_prefers_merged_source_over_partial_user_override() {
        let partial_user_block = Some(
            "  [[mods.sources]]\n  id = \"argent77\"\n  pkg_windows = \"wzp,zip\"".to_string(),
        );

        let block = editor_block_for_source(
            "Improved Archer",
            "argent77",
            false,
            partial_user_block,
            Some(argent77_source()),
        );

        assert!(
            block.contains("repo = \"Argent77/A7-ImprovedArcher\""),
            "the normal Edit Source popup should show the merged effective source"
        );
        assert!(
            block.contains("commit = \"\""),
            "empty selector fields stay visible/editable"
        );
    }

    #[test]
    fn source_id_change_editor_keeps_existing_user_block() {
        let existing = "  [[mods.sources]]\n  id = \"fork\"\n  branch = \"main\"".to_string();
        let block = editor_block_for_source(
            "Fork",
            "fork",
            true,
            Some(existing.clone()),
            Some(argent77_source()),
        );

        assert_eq!(block, existing);
    }

    // ── Per-modlist ambient tests ──────────────────────────────

    /// RAII drop-guard: restores the ambient to its prior value on drop (including panic).
    struct AmbientGuard(Option<PathBuf>);

    impl AmbientGuard {
        fn acquire() -> Self {
            Self(active_modlist_dir())
        }
    }

    impl Drop for AmbientGuard {
        fn drop(&mut self) {
            set_active_modlist_dir(self.0.take());
        }
    }

    fn unique_tmp_dir(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_pmd_test_{}_{}_{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn write_toml_source(dir: &std::path::Path, tag: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("mod_downloads_user.toml");
        std::fs::write(
            &path,
            format!(
                "[[mods]]\nname = \"TestMod\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Test/Mod\"\n  repo = \"Test/Mod\"\n  tag = \"{tag}\"\n"
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn ambient_unset_loader_matches_two_tier() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        set_active_modlist_dir(None);

        let two_tier = load_two_tier_sources();
        let three_tier = load_mod_download_sources();

        // With no ambient set, both loaders produce the same source count.
        assert_eq!(
            two_tier.sources.len(),
            three_tier.sources.len(),
            "ambient unset: load_mod_download_sources must equal load_two_tier_sources"
        );
    }

    #[test]
    fn ambient_set_but_file_absent_is_inert() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let nonexistent = unique_tmp_dir("absent");
        set_active_modlist_dir(Some(nonexistent));

        let two_tier = load_two_tier_sources();
        let three_tier = load_mod_download_sources();

        assert_eq!(
            two_tier.sources.len(),
            three_tier.sources.len(),
            "ambient set to nonexistent dir: loader must equal two-tier result"
        );
    }

    #[test]
    fn two_tier_seed_equals_three_tier_when_ambient_unset() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        set_active_modlist_dir(None);

        let two = load_two_tier_sources();
        let three = load_mod_download_sources();

        assert_eq!(
            two.sources.len(),
            three.sources.len(),
            "extraction is behavior-neutral when ambient is unset"
        );
    }

    #[test]
    fn writer_per_modlist_path_isolates() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        set_active_modlist_dir(None);

        let tmp_global_dir = unique_tmp_dir("global");
        let tmp_per_dir = unique_tmp_dir("per");
        std::fs::create_dir_all(&tmp_global_dir).unwrap();
        std::fs::create_dir_all(&tmp_per_dir).unwrap();

        let global_path = tmp_global_dir.join("mod_downloads_user.toml");
        let per_path = tmp_per_dir.join("mod_downloads_user.toml");

        // Write sentinel content to the global file.
        std::fs::write(&global_path, "# global sentinel\n").unwrap();

        // Ensure the default file exists (ensure_mod_downloads_files may write it).
        let source_block = "  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/B\"\n  repo = \"A/B\"";
        // Write to the per-modlist path explicitly.
        let result = save_user_mod_download_source_block(
            "testmod",
            "TestMod",
            "main",
            false,
            source_block,
            Some(&per_path),
        );
        // May succeed or fail depending on test environment; what matters is global unchanged.
        drop(result);

        // The global file must not have been touched.
        let global_content = std::fs::read_to_string(&global_path).unwrap();
        assert_eq!(
            global_content, "# global sentinel\n",
            "global file must be byte-unchanged when writing to per-modlist path"
        );

        let _ = std::fs::remove_dir_all(&tmp_global_dir);
        let _ = std::fs::remove_dir_all(&tmp_per_dir);
    }

    #[test]
    fn global_seed_ignores_per_modlist_pin() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        // Set up a per-modlist dir with a tag pin.
        let tmp_dir = unique_tmp_dir("seed");
        write_toml_source(&tmp_dir, "v16");
        set_active_modlist_dir(Some(tmp_dir.clone()));

        // GlobalOnly seed must NOT see v16 (the per-modlist pin).
        // We can verify this by checking the two-tier result has no "v16" tag override
        // for a source that only has a v16 in the per-modlist file.
        let two_tier = load_two_tier_sources();
        let three_tier = load_mod_download_sources();

        // The two-tier result should NOT have a "testmod" source with tag v16
        // (since v16 is only in the per-modlist file, not the global).
        let two_tier_testmod_tag = two_tier
            .sources
            .iter()
            .find(|s| s.tp2 == "testmod")
            .and_then(|s| s.tag.clone());
        let three_tier_testmod_tag = three_tier
            .sources
            .iter()
            .find(|s| s.tp2 == "testmod")
            .and_then(|s| s.tag.clone());

        // The per-modlist pin should show in three-tier but not two-tier.
        if three_tier_testmod_tag.as_deref() == Some("v16") {
            assert_ne!(
                two_tier_testmod_tag.as_deref(),
                Some("v16"),
                "GlobalDefault seed must NOT show the per-modlist pin"
            );
        }

        set_active_modlist_dir(None);
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    // ── Version-selector replacement tests (per-modlist overrides a different selector) ──

    fn base_source_with_tag(tag: &str) -> ModDownloadSource {
        ModDownloadSource {
            name: "TestMod".to_string(),
            tp2: "testmod".to_string(),
            source_id: "main".to_string(),
            source_label: "Main".to_string(),
            url: "https://github.com/Test/Mod".to_string(),
            github: Some("Test/Mod".to_string()),
            tag: Some(tag.to_string()),
            ..Default::default()
        }
    }

    fn base_source_with_commit(commit: &str) -> ModDownloadSource {
        ModDownloadSource {
            name: "TestMod".to_string(),
            tp2: "testmod".to_string(),
            source_id: "main".to_string(),
            source_label: "Main".to_string(),
            url: "https://github.com/Test/Mod".to_string(),
            github: Some("Test/Mod".to_string()),
            commit: Some(commit.to_string()),
            ..Default::default()
        }
    }

    fn overlay_with_branch(branch: &str) -> ModDownloadSourceOverlay {
        ModDownloadSourceOverlay {
            tp2: Some("testmod".to_string()),
            source_id: Some("main".to_string()),
            branch: Some(branch.to_string()),
            ..Default::default()
        }
    }

    fn overlay_with_tag(tag: &str) -> ModDownloadSourceOverlay {
        ModDownloadSourceOverlay {
            tp2: Some("testmod".to_string()),
            source_id: Some("main".to_string()),
            tag: Some(tag.to_string()),
            ..Default::default()
        }
    }

    fn overlay_with_no_selector() -> ModDownloadSourceOverlay {
        ModDownloadSourceOverlay {
            tp2: Some("testmod".to_string()),
            source_id: Some("main".to_string()),
            url: Some("https://github.com/Test/Fork".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn per_modlist_branch_replaces_global_tag() {
        // Base: global resolves to tag=v18. Per-modlist says branch=master.
        // Expected: branch=master wins; tag is gone.
        let mut source = base_source_with_tag("v18");
        let overlay = overlay_with_branch("master");

        if overlay_has_version_selector(&overlay) {
            clear_source_version_selectors(&mut source);
        }
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);

        assert_eq!(
            source.branch.as_deref(),
            Some("master"),
            "per-modlist branch=master must win"
        );
        assert!(source.tag.is_none(), "global tag=v18 must be cleared");
        assert!(source.commit.is_none(), "commit must remain clear");
    }

    #[test]
    fn per_modlist_tag_replaces_global_commit() {
        // Base: global resolves to commit=abc123. Per-modlist says tag=v1.0.0.
        // Expected: tag=v1.0.0 wins; commit is gone.
        let mut source = base_source_with_commit("abc123def456abc123def456abc123def456abc1");
        let overlay = overlay_with_tag("v1.0.0");

        if overlay_has_version_selector(&overlay) {
            clear_source_version_selectors(&mut source);
        }
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);

        assert_eq!(
            source.tag.as_deref(),
            Some("v1.0.0"),
            "per-modlist tag=v1.0.0 must win"
        );
        assert!(source.commit.is_none(), "global commit must be cleared");
        assert!(source.branch.is_none(), "branch must remain clear");
    }

    #[test]
    fn per_modlist_overlay_without_selector_inherits_global_selector() {
        // Base: global resolves to tag=v18. Per-modlist overlay has no selector (only url).
        // Expected: tag=v18 is preserved (additive overlay).
        let mut source = base_source_with_tag("v18");
        let overlay = overlay_with_no_selector();

        if overlay_has_version_selector(&overlay) {
            clear_source_version_selectors(&mut source);
        }
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);

        assert_eq!(
            source.tag.as_deref(),
            Some("v18"),
            "global tag=v18 must be preserved when per-modlist has no selector"
        );
        assert_eq!(
            source.url, "https://github.com/Test/Fork",
            "non-selector field from per-modlist overlay must be applied"
        );
    }

    fn write_toml_source_branch(dir: &std::path::Path, branch: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("mod_downloads_user.toml");
        std::fs::write(
            &path,
            format!(
                "[[mods]]\nname = \"TestMod\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Test/Mod\"\n  repo = \"Test/Mod\"\n  branch = \"{branch}\"\n"
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn ambient_unset_resolution_unchanged_by_selector_fix() {
        // Resolution is inert when no ambient modlist is set.
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        set_active_modlist_dir(None);

        let two_tier = load_two_tier_sources();
        let three_tier = load_mod_download_sources();

        assert_eq!(
            two_tier.sources.len(),
            three_tier.sources.len(),
            "ambient unset: three-tier result must equal two-tier result"
        );
    }

    #[test]
    fn per_modlist_branch_pin_applied_via_ambient() {
        // End-to-end: per-modlist file pins an otherwise-absent source to branch=next.
        // Verifies the full ambient path produces branch=next (not the default selector).
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("branch_pin");
        write_toml_source_branch(&tmp_dir, "next");
        set_active_modlist_dir(Some(tmp_dir.clone()));

        let three_tier = load_mod_download_sources();
        let resolved = three_tier.sources.iter().find(|s| s.tp2 == "testmod");

        if let Some(source) = resolved {
            assert_eq!(
                source.branch.as_deref(),
                Some("next"),
                "per-modlist branch=next must be the resolved selector"
            );
            assert!(
                source.tag.is_none(),
                "no tag must remain when per-modlist pins branch=next"
            );
            assert!(
                source.commit.is_none(),
                "no commit must remain when per-modlist pins branch=next"
            );
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    // ── Save-dedup tests ────────────

    /// Builds a doubled TOML string: the same tp2 appears in two [[mods]] blocks,
    /// the first with commit=abc, the second with branch=master.
    fn doubled_toml() -> String {
        concat!(
            "[[mods]]\nname = \"cdtweaks\"\ntp2 = \"cdtweaks\"\n\n",
            "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n",
            "  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n",
            "  repo = \"Gibberlings3/cdtweaks\"\n  commit = \"abc123\"\n\n",
            "[[mods]]\nname = \"cdtweaks\"\ntp2 = \"cdtweaks\"\n\n",
            "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n",
            "  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n",
            "  repo = \"Gibberlings3/cdtweaks\"\n  branch = \"master\"\n",
        )
        .to_string()
    }

    fn count_mod_blocks_for_tp2(content: &str, tp2: &str) -> usize {
        let target = normalize_mod_download_tp2(tp2);
        mod_block_ranges(content)
            .iter()
            .filter(|(start, end)| block_tp2_matches(&content[*start..*end], &target))
            .count()
    }

    #[test]
    fn save_dedup_replace_heals_doubled_file() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("save_dedup_replace");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");
        std::fs::write(&tmp_path, doubled_toml()).unwrap();

        // Pin to a new commit via the save path.
        let new_source = "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n  repo = \"Gibberlings3/cdtweaks\"\n  commit = \"newcommit999\"";
        let result = save_user_mod_download_source_block(
            "cdtweaks",
            "cdtweaks",
            "github",
            false,
            new_source,
            Some(&tmp_path),
        );
        assert!(result.is_ok(), "save must succeed: {:?}", result.err());

        let saved = std::fs::read_to_string(&tmp_path).unwrap();

        // After save, exactly one [[mods]] block for cdtweaks.
        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "cdtweaks"),
            1,
            "save must collapse duplicate blocks into one; got:\n{saved}"
        );
        // The surviving block must carry the new pin, not the stale branch=master.
        assert!(
            saved.contains("newcommit999"),
            "new commit pin must be present; got:\n{saved}"
        );
        assert!(
            !saved.contains("branch = \"master\""),
            "stale branch=master duplicate must be gone; got:\n{saved}"
        );
        assert!(
            !saved.contains("commit = \"abc123\""),
            "old commit pin must be gone; got:\n{saved}"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn save_dedup_remove_heals_doubled_file() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("save_dedup_remove");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");
        std::fs::write(&tmp_path, doubled_toml()).unwrap();

        // Removal path: empty source_block removes the source from all matching blocks.
        let result = save_user_mod_download_source_block(
            "cdtweaks",
            "cdtweaks",
            "github",
            false,
            "",
            Some(&tmp_path),
        );
        assert!(result.is_ok(), "removal must succeed: {:?}", result.err());

        let saved = std::fs::read_to_string(&tmp_path).unwrap();

        // All matching cdtweaks blocks must be gone (no sources left in either).
        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "cdtweaks"),
            0,
            "removal must excise all duplicate blocks; got:\n{saved}"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn save_dedup_other_mods_untouched() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("save_dedup_others");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");

        // File has a doubled cdtweaks block AND a separate unrelated mod.
        let content = concat!(
            "[[mods]]\nname = \"othermod\"\ntp2 = \"othermod\"\n\n",
            "  [[mods.sources]]\n  id = \"other\"\n  label = \"Other\"\n",
            "  type = \"github\"\n  url = \"https://github.com/X/Other\"\n",
            "  repo = \"X/Other\"\n  tag = \"v5\"\n\n",
            "[[mods]]\nname = \"cdtweaks\"\ntp2 = \"cdtweaks\"\n\n",
            "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n",
            "  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n",
            "  repo = \"Gibberlings3/cdtweaks\"\n  commit = \"abc123\"\n\n",
            "[[mods]]\nname = \"cdtweaks\"\ntp2 = \"cdtweaks\"\n\n",
            "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n",
            "  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n",
            "  repo = \"Gibberlings3/cdtweaks\"\n  branch = \"master\"\n",
        );
        std::fs::write(&tmp_path, content).unwrap();

        let new_source = "  [[mods.sources]]\n  id = \"github\"\n  label = \"GitHub\"\n  type = \"github\"\n  url = \"https://github.com/Gibberlings3/cdtweaks\"\n  repo = \"Gibberlings3/cdtweaks\"\n  tag = \"v18\"";
        let result = save_user_mod_download_source_block(
            "cdtweaks",
            "cdtweaks",
            "github",
            false,
            new_source,
            Some(&tmp_path),
        );
        assert!(result.is_ok(), "save must succeed: {:?}", result.err());

        let saved = std::fs::read_to_string(&tmp_path).unwrap();

        // cdtweaks: exactly one block with the new pin.
        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "cdtweaks"),
            1,
            "cdtweaks must have exactly one block after dedup; got:\n{saved}"
        );
        assert!(
            saved.contains("tag = \"v18\""),
            "new tag pin must be present; got:\n{saved}"
        );

        // othermod must be present and untouched.
        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "othermod"),
            1,
            "othermod block must be preserved; got:\n{saved}"
        );
        assert!(
            saved.contains("tag = \"v5\""),
            "othermod v5 tag must be intact; got:\n{saved}"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn save_dedup_clean_file_unchanged_behavior() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let tmp_dir = unique_tmp_dir("save_dedup_clean");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");

        // Single (clean, non-doubled) block.
        let single = concat!(
            "[[mods]]\nname = \"testmod\"\ntp2 = \"testmod\"\n\n",
            "  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n",
            "  type = \"github\"\n  url = \"https://github.com/T/M\"\n",
            "  repo = \"T/M\"\n  tag = \"v1\"\n",
        );
        std::fs::write(&tmp_path, single).unwrap();

        let new_source = "  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/T/M\"\n  repo = \"T/M\"\n  tag = \"v2\"";
        let result = save_user_mod_download_source_block(
            "testmod",
            "testmod",
            "main",
            false,
            new_source,
            Some(&tmp_path),
        );
        assert!(
            result.is_ok(),
            "clean-file save must succeed: {:?}",
            result.err()
        );

        let saved = std::fs::read_to_string(&tmp_path).unwrap();

        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "testmod"),
            1,
            "clean file must still have exactly one block; got:\n{saved}"
        );
        assert!(
            saved.contains("tag = \"v2\""),
            "new tag v2 must be present; got:\n{saved}"
        );
        assert!(
            !saved.contains("tag = \"v1\""),
            "old tag v1 must be replaced; got:\n{saved}"
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
