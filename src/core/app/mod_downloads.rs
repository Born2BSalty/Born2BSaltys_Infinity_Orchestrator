// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::BTreeSet;
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

pub(crate) fn set_active_modlist_dir(dir: Option<PathBuf>) {
    if let Ok(mut guard) = active_modlist_dir_mutex().lock() {
        *guard = dir;
    }
}

pub(crate) fn active_modlist_dir() -> Option<PathBuf> {
    active_modlist_dir_mutex()
        .lock()
        .ok()
        .and_then(|g| g.clone())
}

pub(crate) fn active_modlist_downloads_path() -> Option<PathBuf> {
    active_modlist_dir().map(|d| d.join("mod_downloads_user.toml"))
}

const MOD_DOWNLOADS_USER_FILE_NAME: &str = "mod_downloads_user.toml";

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadsFile {
    #[serde(default)]
    pub(crate) format: Option<u32>,
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
pub(crate) struct ModDownloadSourceOverlay {
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
    #[serde(default, rename = "type")]
    pub(crate) kind: Option<String>,
    pub(crate) channel: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) release: Option<String>,
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
    #[serde(default, rename = "type")]
    pub(crate) kind: Option<String>,
    pub(crate) channel: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) release: Option<String>,
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
    pub(crate) kind: Option<String>,
    #[serde(default)]
    pub(crate) channel: Option<String>,
    #[serde(default)]
    pub(crate) tag: Option<String>,
    #[serde(default)]
    pub(crate) commit: Option<String>,
    #[serde(default)]
    pub(crate) branch: Option<String>,
    #[serde(default)]
    pub(crate) release: Option<String>,
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
pub(crate) struct ModDownloadsOverlayLoad {
    pub(crate) sources: Vec<ModDownloadSourceOverlay>,
    pub(crate) error: Option<String>,
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

pub(crate) fn ensure_mod_downloads_files() -> io::Result<()> {
    let user_path = mod_downloads_user_path();

    if let Some(parent) = user_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !user_path.exists() {
        fs::write(&user_path, user_mod_downloads_content())?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SeedScope {
    #[default]
    Resolved,
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

pub(crate) fn load_two_tier_sources() -> ModDownloadsLoad {
    let user_path = mod_downloads_user_path();
    let default_load = load_source_overlays_from_str(default_mod_downloads_content(), "default");
    let user_load = load_source_overlays_from_path(&user_path);
    two_tier_from_overlays(default_load, user_load)
}

pub(crate) fn two_tier_from_overlays(
    default_load: ModDownloadsOverlayLoad,
    user_load: ModDownloadsOverlayLoad,
) -> ModDownloadsLoad {
    let mut by_source = BTreeMap::<String, ModDownloadSource>::new();

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
        let mut source = ModDownloadSource::default();
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

pub(crate) fn load_mod_download_sources() -> ModDownloadsLoad {
    let mut result = load_two_tier_sources();

    if let Some(per_modlist_path) = active_modlist_downloads_path().filter(|p| p.exists()) {
        let per_load = load_source_overlays_from_path(&per_modlist_path);
        apply_modlist_overlay(&mut result, per_load);
    }

    result
}

pub(crate) fn load_mod_download_sources_from_texts(
    default_text: &str,
    user_text: &str,
    modlist_text: &str,
) -> ModDownloadsLoad {
    let default_load = load_source_overlays_from_str(default_text, "default");
    let user_load = load_source_overlays_from_str(user_text, "user");
    let mut result = two_tier_from_overlays(default_load, user_load);

    if !modlist_text.trim().is_empty() {
        let per_load = load_source_overlays_from_str(modlist_text, "modlist");
        apply_modlist_overlay(&mut result, per_load);
    }

    result
}

fn apply_modlist_overlay(result: &mut ModDownloadsLoad, per_load: ModDownloadsOverlayLoad) {
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
        let mut source = ModDownloadSource::default();
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
    result.error = merge_load_errors(result.error.take(), per_load.error);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceTier {
    Default,
    User,
    Modlist,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceTiers {
    resolved: ModDownloadsLoad,
    user_keys: BTreeSet<String>,
    modlist_keys: BTreeSet<String>,
}

impl SourceTiers {
    pub(crate) fn resolve(&self, tp2: &str) -> Option<(ModDownloadSource, SourceTier)> {
        let source = self.resolved.resolve_source(tp2, None)?;
        let tier = self.tier_of(&source.tp2, &source.source_id);
        Some((source, tier))
    }

    pub(crate) fn tier_of(&self, tp2: &str, source_id: &str) -> SourceTier {
        let key = format!(
            "{}|{}",
            normalize_mod_download_tp2(tp2),
            normalize_source_id(source_id)
        );
        if self.modlist_keys.contains(&key) {
            SourceTier::Modlist
        } else if self.user_keys.contains(&key) {
            SourceTier::User
        } else {
            SourceTier::Default
        }
    }

    pub(crate) fn find_sources(&self, tp2: &str) -> Vec<ModDownloadSource> {
        self.resolved.find_sources(tp2)
    }
}

fn overlay_keys(overlays: &[ModDownloadSourceOverlay]) -> BTreeSet<String> {
    overlays
        .iter()
        .map(overlay_source_key)
        .filter(|key| !key.is_empty())
        .collect()
}

pub(crate) fn source_tiers_from_texts(
    default_text: &str,
    user_text: &str,
    modlist_text: &str,
) -> SourceTiers {
    let default_load = load_source_overlays_from_str(default_text, "default");
    let user_load = load_source_overlays_from_str(user_text, "user");
    let user_keys = overlay_keys(&user_load.sources);
    let mut resolved = two_tier_from_overlays(default_load, user_load);

    let modlist_keys = if modlist_text.trim().is_empty() {
        BTreeSet::new()
    } else {
        let modlist_load = load_source_overlays_from_str(modlist_text, "modlist");
        let modlist_keys = overlay_keys(&modlist_load.sources);
        apply_modlist_overlay(&mut resolved, modlist_load);
        modlist_keys
    };

    SourceTiers {
        resolved,
        user_keys,
        modlist_keys,
    }
}

pub(crate) fn load_source_tiers(modlist_text: &str) -> SourceTiers {
    let user_text = fs::read_to_string(mod_downloads_user_path()).unwrap_or_default();
    source_tiers_from_texts(default_mod_downloads_content(), &user_text, modlist_text)
}

pub(crate) fn source_open_url(source: &ModDownloadSource) -> Option<String> {
    let url = source.url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        return Some(url.to_string());
    }
    let github = source.github.as_deref()?.trim();
    if github.starts_with("http://") || github.starts_with("https://") {
        return Some(github.to_string());
    }
    let repo = github.trim_matches('/');
    let mut parts = repo.split('/');
    if matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty()
    ) {
        return Some(format!("https://github.com/{repo}"));
    }
    None
}

pub(crate) fn source_link_label(url: &str) -> String {
    let trimmed = url.trim();
    let trimmed = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    trimmed.strip_suffix('/').unwrap_or(trimmed).to_string()
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

    let preamble = if ranges.is_empty() {
        ""
    } else {
        &content[..ranges[0].0]
    };

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

    let preamble = if ranges.is_empty() {
        ""
    } else {
        &content[..ranges[0].0]
    };

    let mut out = preamble.trim_end().to_string();
    for (start, end) in &ranges {
        let block = &content[*start..*end];
        let maybe_text = if block_tp2_matches(block, &target) {
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
        |source| complete_source_block(&source),
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
    "release",
    "channel",
    "asset",
    "subdir_require",
    "aliases",
    "config_files",
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

pub(crate) fn template_mod_header(tp2: &str, label: &str) -> String {
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

fn complete_block_type(source: &ModDownloadSource) -> String {
    if source.github.is_some() {
        return "github".to_string();
    }
    if let Some(kind) = source.kind.as_deref().filter(|value| !value.is_empty()) {
        return kind.to_string();
    }
    if source_is_page_archive_url(&source.url) {
        return "page".to_string();
    }
    "url".to_string()
}

fn quoted_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_toml_string(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn complete_source_block(source: &ModDownloadSource) -> String {
    let mut lines = vec![
        "[[mods.sources]]".to_string(),
        format!("id = \"{}\"", escape_toml_string(&source.source_id)),
        format!("label = \"{}\"", escape_toml_string(&source.source_label)),
        format!(
            "type = \"{}\"",
            escape_toml_string(&complete_block_type(source))
        ),
        format!("url = \"{}\"", escape_toml_string(&source.url)),
        format!(
            "repo = \"{}\"",
            escape_toml_string(source.github.as_deref().unwrap_or_default())
        ),
        format!(
            "exact_github = [{}]",
            quoted_string_list(&source.exact_github)
        ),
        format!(
            "commit = \"{}\"",
            escape_toml_string(source.commit.as_deref().unwrap_or_default())
        ),
        format!(
            "tag = \"{}\"",
            escape_toml_string(source.tag.as_deref().unwrap_or_default())
        ),
        format!(
            "branch = \"{}\"",
            escape_toml_string(source.branch.as_deref().unwrap_or_default())
        ),
        format!(
            "release = \"{}\"",
            escape_toml_string(source.release.as_deref().unwrap_or_default())
        ),
        format!(
            "channel = \"{}\"",
            escape_toml_string(source.channel.as_deref().unwrap_or_default())
        ),
        format!(
            "asset = \"{}\"",
            escape_toml_string(source.asset.as_deref().unwrap_or_default())
        ),
        format!(
            "subdir_require = \"{}\"",
            escape_toml_string(source.subdir_require.as_deref().unwrap_or_default())
        ),
        format!("aliases = [{}]", quoted_string_list(&source.aliases)),
    ];
    if !source.config_files.is_empty() {
        lines.push(format!(
            "config_files = [{}]",
            quoted_string_list(&source.config_files)
        ));
    }
    if let Some(tp2_rename) = source.tp2_rename.as_ref() {
        lines.push(format!(
            "tp2_rename = {{ from = \"{}\", to = \"{}\" }}",
            escape_toml_string(&tp2_rename.from),
            escape_toml_string(&tp2_rename.to)
        ));
    }
    lines.push(format!(
        "pkg_windows = \"{}\"",
        escape_toml_string(source.pkg_windows.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "pkg_linux = \"{}\"",
        escape_toml_string(source.pkg_linux.as_deref().unwrap_or_default())
    ));
    lines.push(format!(
        "pkg_macos = \"{}\"",
        escape_toml_string(source.pkg_macos.as_deref().unwrap_or_default())
    ));
    lines.push(format!("default = {}", source.source_default));
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

#[cfg(test)]
pub(crate) static AMBIENT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) const fn default_mod_downloads_content() -> &'static str {
    include_str!("../config/default_mod_downloads.toml")
}

const fn user_mod_downloads_content() -> &'static str {
    include_str!("../config/user_mod_downloads.toml")
}

pub(crate) fn user_template_cheat_sheet() -> &'static str {
    let template = user_mod_downloads_content();
    template
        .find("# BIO download cheat sheet")
        .map_or("", |start| template[start..].trim_end())
}

pub(crate) fn user_template_migrated_header() -> &'static str {
    let template = user_mod_downloads_content();
    template
        .find("format = 2")
        .map_or(template, |start| &template[..start])
        .trim()
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
    load_source_overlays_from_str(&content, &path.display().to_string())
}

pub(crate) fn load_source_overlays_from_str(
    content: &str,
    origin: &str,
) -> ModDownloadsOverlayLoad {
    let parsed = match toml::from_str::<ModDownloadsFile>(content) {
        Ok(value) => value,
        Err(err) => {
            return ModDownloadsOverlayLoad {
                sources: Vec::new(),
                error: Some(format!("mod downloads parse failed for {origin}: {err}")),
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

pub(crate) fn overlay_tp2_key(source: &ModDownloadSourceOverlay) -> String {
    source
        .tp2
        .as_deref()
        .map(normalize_mod_download_tp2)
        .unwrap_or_default()
}

pub(crate) fn overlay_source_key(source: &ModDownloadSourceOverlay) -> String {
    let tp2 = overlay_tp2_key(source);
    if tp2.is_empty() {
        return String::new();
    }
    let source_id = normalize_source_id(source.source_id.as_deref().unwrap_or("primary"));
    format!("{tp2}|{source_id}")
}

pub(crate) fn apply_source_overlay(
    target: &mut ModDownloadSource,
    overlay: ModDownloadSourceOverlay,
) {
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
    if let Some(kind) = overlay.kind {
        target.kind = Some(kind);
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
    if let Some(release) = overlay.release {
        target.release = Some(release);
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

pub(crate) fn clear_other_source_defaults(
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
        kind,
        channel,
        tag,
        commit,
        branch,
        release,
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
        if trimmed.is_empty() {
            target.github = None;
        } else {
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
    if let Some(kind) = kind {
        target.kind = Some(kind);
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
    if let Some(release) = release {
        target.release = Some(release);
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

pub(crate) fn normalize_source(source: &mut ModDownloadSource) {
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
    source.kind =
        normalize_optional_string(source.kind.take()).map(|value| value.to_ascii_lowercase());
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
    source.release = normalize_optional_string(source.release.take());
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
    if source.release.is_some() {
        clear_release_source_conflicts(source);
        return true;
    }
    true
}

fn clear_commit_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.tag = None;
    source.branch = None;
    source.release = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_tag_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.branch = None;
    source.release = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_branch_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.release = None;
    source.asset = None;
    clear_source_packages(source);
}

fn clear_release_source_conflicts(source: &mut ModDownloadSource) {
    source.channel = None;
    source.tag = None;
    source.branch = None;
    source.commit = None;
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

pub(crate) fn source_is_valid(source: &ModDownloadSource) -> bool {
    !normalize_mod_download_tp2(&source.tp2).is_empty() && !source.url.is_empty()
}

pub(crate) fn normalize_source_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn sort_sources(sources: &mut [ModDownloadSource]) {
    sources.sort_by(|left, right| {
        normalize_mod_download_tp2(&left.tp2)
            .cmp(&normalize_mod_download_tp2(&right.tp2))
            .then_with(|| right.source_default.cmp(&left.source_default))
            .then_with(|| left.source_label.cmp(&right.source_label))
            .then_with(|| left.source_id.cmp(&right.source_id))
    });
}

pub(crate) fn merge_load_errors(left: Option<String>, right: Option<String>) -> Option<String> {
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
    fn complete_source_block_includes_every_field_and_indented_shape() {
        let block = complete_source_block(&argent77_source());

        assert_eq!(
            block,
            "  [[mods.sources]]\n  id = \"argent77\"\n  label = \"Argent77\"\n  type = \"github\"\n  url = \"https://github.com/Argent77/A7-ImprovedArcher\"\n  repo = \"Argent77/A7-ImprovedArcher\"\n  exact_github = []\n  commit = \"\"\n  tag = \"\"\n  branch = \"\"\n  release = \"\"\n  channel = \"\"\n  asset = \"\"\n  subdir_require = \"\"\n  aliases = []\n  pkg_windows = \"wzp,zip\"\n  pkg_linux = \"lin,zip\"\n  pkg_macos = \"mac,zip\"\n  default = false"
        );
    }

    #[test]
    fn complete_block_writes_exact_github_as_array() {
        let source = ModDownloadSource {
            name: "Fork Picker".to_string(),
            tp2: "forkpicker".to_string(),
            source_id: "main".to_string(),
            source_label: "Main".to_string(),
            url: "https://github.com/A/B".to_string(),
            github: Some("A/B".to_string()),
            exact_github: vec!["A/B".to_string(), "C/D".to_string()],
            ..Default::default()
        };
        let block = complete_source_block(&source);

        assert!(
            block.contains("exact_github = [\"A/B\", \"C/D\"]"),
            "exact_github must be a TOML array, not repeated string lines; got:\n{block}"
        );
        let mod_block = format!(
            "{}\n\n{block}\n",
            template_mod_header(&source.tp2, &source.name)
        );
        toml::from_str::<ModDownloadsFile>(&mod_block)
            .expect("exact_github as an array must parse");
    }

    #[test]
    fn complete_block_round_trips() {
        let sources = [
            ModDownloadSource {
                name: "GitHub Mod".to_string(),
                tp2: "githubmod".to_string(),
                source_id: "argent77".to_string(),
                source_label: "Argent77".to_string(),
                url: "https://github.com/Argent77/GitHubMod".to_string(),
                github: Some("Argent77/GitHubMod".to_string()),
                kind: Some("github".to_string()),
                exact_github: vec!["Fork/GitHubMod".to_string()],
                release: Some("v1.2.3".to_string()),
                asset: Some("GitHubMod-v1.2.3.zip".to_string()),
                aliases: vec!["oldname".to_string()],
                config_files: vec!["mod.ini".to_string()],
                tp2_rename: Some(ModDownloadTp2Rename {
                    from: "setup-old.tp2".to_string(),
                    to: "githubmod.tp2".to_string(),
                }),
                pkg_windows: Some("wzp,zip".to_string()),
                pkg_linux: Some("lin,zip".to_string()),
                pkg_macos: Some("mac,zip".to_string()),
                source_default: true,
                ..Default::default()
            },
            ModDownloadSource {
                name: "URL Mod".to_string(),
                tp2: "urlmod".to_string(),
                source_id: "direct".to_string(),
                source_label: "Direct".to_string(),
                url: "https://example.com/urlmod.zip".to_string(),
                kind: Some("url".to_string()),
                subdir_require: Some("v2".to_string()),
                source_default: true,
                ..Default::default()
            },
            ModDownloadSource {
                name: "Page Mod".to_string(),
                tp2: "pagemod".to_string(),
                source_id: "weaselmods".to_string(),
                source_label: "Weasel Mods".to_string(),
                url: "https://downloads.weaselmods.net/download/pagemod".to_string(),
                kind: Some("page".to_string()),
                release: Some("v1.1".to_string()),
                asset: Some("PageMod-v1.1.zip".to_string()),
                source_default: true,
                ..Default::default()
            },
        ];

        for source in sources {
            let block = complete_source_block(&source);
            let mod_block = format!(
                "{}\n\n{block}\n",
                template_mod_header(&source.tp2, &source.name)
            );
            let parsed = toml::from_str::<ModDownloadsFile>(&mod_block)
                .unwrap_or_else(|err| panic!("round-trip parse failed for {}: {err}", source.tp2));
            let overlay =
                flatten_mod_overlay_entries(parsed.mods.into_iter().next().expect("one mod block"))
                    .into_iter()
                    .next()
                    .expect("one source overlay");
            let mut round_tripped = ModDownloadSource::default();
            apply_source_overlay(&mut round_tripped, overlay);
            normalize_source(&mut round_tripped);

            assert_eq!(round_tripped.name, source.name);
            assert_eq!(round_tripped.tp2, source.tp2);
            assert_eq!(round_tripped.source_id, source.source_id);
            assert_eq!(round_tripped.source_label, source.source_label);
            assert_eq!(round_tripped.url, source.url);
            assert_eq!(round_tripped.github, source.github);
            assert_eq!(round_tripped.kind, source.kind);
            assert_eq!(round_tripped.exact_github, source.exact_github);
            assert_eq!(round_tripped.commit, source.commit);
            assert_eq!(round_tripped.tag, source.tag);
            assert_eq!(round_tripped.branch, source.branch);
            assert_eq!(round_tripped.release, source.release);
            assert_eq!(round_tripped.channel, source.channel);
            assert_eq!(round_tripped.asset, source.asset);
            assert_eq!(round_tripped.subdir_require, source.subdir_require);
            assert_eq!(round_tripped.aliases, source.aliases);
            assert_eq!(round_tripped.config_files, source.config_files);
            assert_eq!(
                round_tripped.tp2_rename.is_some(),
                source.tp2_rename.is_some()
            );
            assert_eq!(round_tripped.pkg_windows, source.pkg_windows);
            assert_eq!(round_tripped.pkg_linux, source.pkg_linux);
            assert_eq!(round_tripped.pkg_macos, source.pkg_macos);
            assert_eq!(round_tripped.source_default, source.source_default);
        }
    }

    #[test]
    fn template_example_block_is_complete() {
        let template = user_mod_downloads_content();
        let parsed = toml::from_str::<ModDownloadsFile>(template).expect("template parses");
        let mod_overlay = parsed.mods.into_iter().next().expect("example mod block");
        let overlay = flatten_mod_overlay_entries(mod_overlay)
            .into_iter()
            .next()
            .expect("example source overlay");
        let mut source = ModDownloadSource::default();
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);

        let expected = complete_source_block(&source);
        let example_start = template
            .find("  [[mods.sources]]")
            .expect("example source block");
        let example_end = template
            .find("# BIO download cheat sheet")
            .expect("example block ends before the cheat sheet");
        let example_block = template[example_start..example_end]
            .replace("\r\n", "\n")
            .trim_end()
            .to_string();

        assert_eq!(
            example_block, expected,
            "the template's example block must be a complete block"
        );
    }

    #[test]
    fn higher_layer_block_replaces_lower_whole() {
        let default_text = "[[mods]]\nname = \"Test\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/B\"\n  repo = \"A/B\"\n  aliases = [\"oldname\"]\n";
        let user_text = "[[mods]]\nname = \"Test\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/Fork\"\n  repo = \"A/Fork\"\n";

        let load = load_mod_download_sources_from_texts(default_text, user_text, "");
        let source = load.resolve_source("testmod", None).expect("resolves");

        assert!(
            source.aliases.is_empty(),
            "a higher block must replace the lower block whole, not merge its aliases in"
        );
        assert_eq!(source.url, "https://github.com/A/Fork");
    }

    #[test]
    fn missing_line_means_blank() {
        let default_text = "[[mods]]\nname = \"Test\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/B\"\n  repo = \"A/B\"\n  channel = \"preonly\"\n  subdir_require = \"v2\"\n  pkg_windows = \"win.zip\"\n";
        let modlist_text = "[[mods]]\nname = \"Test\"\ntp2 = \"testmod\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/B\"\n  repo = \"A/B\"\n  commit = \"deadbeef\"\n";

        let load = load_mod_download_sources_from_texts(default_text, "", modlist_text);
        let source = load.resolve_source("testmod", None).expect("resolves");

        assert_eq!(source.commit.as_deref(), Some("deadbeef"));
        assert!(
            source.pkg_windows.is_none(),
            "a missing pkg_windows line means blank"
        );
        assert!(
            source.channel.is_none(),
            "a missing channel line means blank"
        );
        assert!(
            source.subdir_require.is_none(),
            "a missing subdir_require line means blank"
        );
    }

    #[test]
    fn blank_repo_clears_github() {
        let overlay = ModDownloadSourceVariantOverlay {
            repo: Some(String::new()),
            ..Default::default()
        };
        let mut target = ModDownloadSourceOverlay {
            github: Some("Owner/Repo".to_string()),
            ..Default::default()
        };

        apply_source_variant_overlay(&mut target, overlay);

        assert!(
            target.github.is_none(),
            "a blank repo line must clear github, not leave it inherited"
        );
    }

    #[test]
    fn release_beats_channel_and_keeps_asset_and_packages() {
        let mut source = ModDownloadSource {
            channel: Some("preonly".to_string()),
            release: Some("  v35.17  ".to_string()),
            asset: Some("win-stratagems-v35.17.exe".to_string()),
            pkg_windows: Some("wzp,zip".to_string()),
            ..Default::default()
        };

        normalize_source(&mut source);

        assert!(source.channel.is_none(), "release must beat channel");
        assert_eq!(source.release.as_deref(), Some("v35.17"));
        assert_eq!(
            source.asset.as_deref(),
            Some("win-stratagems-v35.17.exe"),
            "release keeps asset"
        );
        assert_eq!(
            source.pkg_windows.as_deref(),
            Some("wzp,zip"),
            "release keeps the package lines"
        );
    }

    #[test]
    fn tag_clears_release() {
        let mut source = ModDownloadSource {
            tag: Some("v1.1".to_string()),
            release: Some("v1.2".to_string()),
            ..Default::default()
        };

        normalize_source(&mut source);

        assert_eq!(source.tag.as_deref(), Some("v1.1"));
        assert!(source.release.is_none(), "tag must clear release");
    }

    #[test]
    fn every_embedded_default_block_type_matches_its_url_shape() {
        let load = load_source_overlays_from_str(default_mod_downloads_content(), "default");
        for overlay in load.sources {
            let mut source = ModDownloadSource::default();
            apply_source_overlay(&mut source, overlay);
            normalize_source(&mut source);
            if !source_is_valid(&source) {
                continue;
            }
            let expected = if source.github.is_some() {
                "github"
            } else if source_is_page_archive_url(&source.url) {
                "page"
            } else {
                "url"
            };
            assert_eq!(
                source.kind.as_deref(),
                Some(expected),
                "{} ({}) declares a type that does not match its url shape",
                source.tp2,
                source.source_id
            );
        }
    }

    #[test]
    fn embedded_defaults_have_no_retired_channel_words() {
        let load = load_source_overlays_from_str(default_mod_downloads_content(), "default");
        for overlay in &load.sources {
            let Some(channel) = overlay.channel.as_deref() else {
                continue;
            };
            let normalized = channel.trim().to_ascii_lowercase();
            assert_ne!(
                normalized, "pre-release",
                "{:?} still uses pre-release",
                overlay.tp2
            );
            assert_ne!(
                normalized, "release",
                "{:?} still uses release",
                overlay.tp2
            );
            assert_ne!(
                normalized, "releases",
                "{:?} still uses releases",
                overlay.tp2
            );
        }
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

    struct ConfigDirGuard(PathBuf);

    impl ConfigDirGuard {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_mod_downloads_config_dir_test_{}_{}_{label}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
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
        let _config_guard = ConfigDirGuard::new("writer_isolates");
        set_active_modlist_dir(None);

        let tmp_global_dir = unique_tmp_dir("global");
        let tmp_per_dir = unique_tmp_dir("per");
        std::fs::create_dir_all(&tmp_global_dir).unwrap();
        std::fs::create_dir_all(&tmp_per_dir).unwrap();

        let global_path = tmp_global_dir.join("mod_downloads_user.toml");
        let per_path = tmp_per_dir.join("mod_downloads_user.toml");

        std::fs::write(&global_path, "# global sentinel\n").unwrap();

        let source_block = "  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/A/B\"\n  repo = \"A/B\"";
        let result = save_user_mod_download_source_block(
            "testmod",
            "TestMod",
            "main",
            false,
            source_block,
            Some(&per_path),
        );
        drop(result);

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

        let tmp_dir = unique_tmp_dir("seed");
        write_toml_source(&tmp_dir, "v16");
        set_active_modlist_dir(Some(tmp_dir.clone()));

        let two_tier = load_two_tier_sources();
        let three_tier = load_mod_download_sources();

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
        let mut source = ModDownloadSource::default();
        apply_source_overlay(&mut source, overlay_with_branch("master"));
        normalize_source(&mut source);

        assert_eq!(
            source.branch.as_deref(),
            Some("master"),
            "per-modlist branch=master must win"
        );
        assert!(
            source.tag.is_none(),
            "no tag survives a whole-block replace"
        );
        assert!(source.commit.is_none(), "commit must remain clear");
    }

    #[test]
    fn per_modlist_tag_replaces_global_commit() {
        let mut source = ModDownloadSource::default();
        apply_source_overlay(&mut source, overlay_with_tag("v1.0.0"));
        normalize_source(&mut source);

        assert_eq!(
            source.tag.as_deref(),
            Some("v1.0.0"),
            "per-modlist tag=v1.0.0 must win"
        );
        assert!(
            source.commit.is_none(),
            "no commit survives a whole-block replace"
        );
        assert!(source.branch.is_none(), "branch must remain clear");
    }

    #[test]
    fn per_modlist_tag_replaces_global_channel() {
        let mut source = ModDownloadSource::default();
        apply_source_overlay(&mut source, overlay_with_tag("v1.2.0"));
        normalize_source(&mut source);

        assert_eq!(
            source.tag.as_deref(),
            Some("v1.2.0"),
            "per-modlist tag=v1.2.0 must win"
        );
        assert!(
            source.channel.is_none(),
            "no channel survives a whole-block replace"
        );
        assert!(source.branch.is_none(), "branch must remain clear");
        assert!(source.commit.is_none(), "commit must remain clear");
    }

    #[test]
    fn per_modlist_overlay_without_selector_leaves_selector_blank() {
        let mut source = ModDownloadSource::default();
        apply_source_overlay(&mut source, overlay_with_no_selector());
        normalize_source(&mut source);

        assert!(
            source.tag.is_none(),
            "a whole-block replace never inherits a lower tier's tag"
        );
        assert_eq!(
            source.url, "https://github.com/Test/Fork",
            "the field the per-modlist overlay does list must still apply"
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
        let _config_guard = ConfigDirGuard::new("save_dedup_replace");

        let tmp_dir = unique_tmp_dir("save_dedup_replace");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");
        std::fs::write(&tmp_path, doubled_toml()).unwrap();

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

        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "cdtweaks"),
            1,
            "save must collapse duplicate blocks into one; got:\n{saved}"
        );
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
        let _config_guard = ConfigDirGuard::new("save_dedup_remove");

        let tmp_dir = unique_tmp_dir("save_dedup_remove");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");
        std::fs::write(&tmp_path, doubled_toml()).unwrap();

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
        let _config_guard = ConfigDirGuard::new("save_dedup_others");

        let tmp_dir = unique_tmp_dir("save_dedup_others");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");

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

        assert_eq!(
            count_mod_blocks_for_tp2(&saved, "cdtweaks"),
            1,
            "cdtweaks must have exactly one block after dedup; got:\n{saved}"
        );
        assert!(
            saved.contains("tag = \"v18\""),
            "new tag pin must be present; got:\n{saved}"
        );

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
        let _config_guard = ConfigDirGuard::new("save_dedup_clean");

        let tmp_dir = unique_tmp_dir("save_dedup_clean");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let tmp_path = tmp_dir.join("mod_downloads_user.toml");

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

    fn overlay_source_toml(tp2: &str, source_id: &str, url: &str) -> String {
        format!(
            "[[mods]]\nname = \"Test\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"{source_id}\"\n  label = \"Test\"\n  type = \"github\"\n  url = \"{url}\"\n"
        )
    }

    #[test]
    fn overlay_text_parses_like_a_file() {
        let text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let from_str = load_source_overlays_from_str(&text, "inline");

        let tmp_dir = unique_tmp_dir("overlay_text");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let path = tmp_dir.join("mod_downloads.toml");
        std::fs::write(&path, &text).unwrap();
        let from_path = load_source_overlays_from_path(&path);

        assert_eq!(from_str.sources.len(), from_path.sources.len());
        assert_eq!(
            from_str.sources[0].tp2.as_deref(),
            from_path.sources[0].tp2.as_deref()
        );

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn tiers_report_default_when_only_the_default_file_names_the_source() {
        let default_text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let tiers = source_tiers_from_texts(&default_text, "", "");
        let (source, tier) = tiers.resolve("testmod").expect("resolves");
        assert_eq!(tier, SourceTier::Default);
        assert_eq!(source.url, "https://github.com/A/B");
    }

    #[test]
    fn tiers_report_user_when_the_user_file_overrides_the_same_source() {
        let default_text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let user_text = overlay_source_toml("testmod", "main", "https://github.com/A/Fork");
        let tiers = source_tiers_from_texts(&default_text, &user_text, "");
        let (source, tier) = tiers.resolve("testmod").expect("resolves");
        assert_eq!(tier, SourceTier::User);
        assert_eq!(source.url, "https://github.com/A/Fork");
    }

    #[test]
    fn tiers_report_modlist_when_the_code_overrides_the_source() {
        let default_text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let modlist_text = overlay_source_toml("testmod", "main", "https://github.com/A/Pin");
        let tiers = source_tiers_from_texts(&default_text, "", &modlist_text);
        let (source, tier) = tiers.resolve("testmod").expect("resolves");
        assert_eq!(tier, SourceTier::Modlist);
        assert_eq!(source.url, "https://github.com/A/Pin");
    }

    #[test]
    fn tiers_resolve_none_for_an_unknown_tp2() {
        let default_text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let tiers = source_tiers_from_texts(&default_text, "", "");
        assert!(tiers.resolve("othermod").is_none());
    }

    #[test]
    fn empty_modlist_text_adds_no_keys() {
        let default_text = overlay_source_toml("testmod", "main", "https://github.com/A/B");
        let with_empty = source_tiers_from_texts(&default_text, "", "");
        let with_whitespace = source_tiers_from_texts(&default_text, "", "   \n");
        let (source_a, tier_a) = with_empty.resolve("testmod").expect("resolves");
        let (source_b, tier_b) = with_whitespace.resolve("testmod").expect("resolves");
        assert_eq!(tier_a, SourceTier::Default);
        assert_eq!(tier_b, SourceTier::Default);
        assert_eq!(source_a.url, source_b.url);
    }

    #[test]
    fn source_open_url_prefers_the_url_then_the_github_slug() {
        let with_url = ModDownloadSource {
            url: "https://example.com/mod".to_string(),
            github: Some("Owner/Repo".to_string()),
            ..Default::default()
        };
        assert_eq!(
            source_open_url(&with_url).as_deref(),
            Some("https://example.com/mod")
        );

        let slug_only = ModDownloadSource {
            github: Some("Owner/Repo".to_string()),
            ..Default::default()
        };
        assert_eq!(
            source_open_url(&slug_only).as_deref(),
            Some("https://github.com/Owner/Repo")
        );

        let neither = ModDownloadSource::default();
        assert!(source_open_url(&neither).is_none());
    }

    #[test]
    fn source_link_label_strips_the_scheme() {
        assert_eq!(
            source_link_label("https://github.com/Owner/Repo/"),
            "github.com/Owner/Repo"
        );
        assert_eq!(
            source_link_label("http://example.com/mod"),
            "example.com/mod"
        );
    }
}
