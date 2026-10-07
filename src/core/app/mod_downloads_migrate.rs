// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tracing::warn;

use crate::app::mod_downloads;
use crate::app::mod_downloads::{
    ModDownloadSource, ModDownloadSourceOverlay, ModDownloadsFile, ModDownloadsLoad,
    ModDownloadsOverlayLoad,
};

const MOD_DOWNLOADS_DEFAULT_FILE_NAME: &str = "mod_downloads_default.toml";
const COMPAT_RULES_DEFAULT_FILE_NAME: &str = "step2_compat_rules_default.toml";
const REFERENCE_FILE_HEADER: &str = "# Reference copy of BIO's built-in defaults, provided for reference only.\n# BIO does not read this file. Edits here change nothing and are overwritten when BIO starts.";

fn migrated_header() -> &'static str {
    mod_downloads::user_template_migrated_header()
}

#[derive(Clone, Copy)]
pub(crate) enum MigrateTier {
    User,
    Modlist,
}

pub(crate) enum MigrateResult {
    Unchanged,
    Rewritten(String),
    Unparseable(String),
}

fn old_two_tier_from_overlays(
    default_load: ModDownloadsOverlayLoad,
    user_load: ModDownloadsOverlayLoad,
) -> ModDownloadsLoad {
    let mut by_source = BTreeMap::<String, ModDownloadSource>::new();

    for overlay in default_load.sources {
        let key = mod_downloads::overlay_source_key(&overlay);
        if !key.is_empty() {
            let mut source = ModDownloadSource::default();
            mod_downloads::apply_source_overlay(&mut source, overlay);
            mod_downloads::normalize_source(&mut source);
            if !mod_downloads::source_is_valid(&source) {
                continue;
            }
            by_source.insert(key, source);
        }
    }
    for mut overlay in user_load.sources {
        let key = mod_downloads::overlay_source_key(&overlay);
        if key.is_empty() {
            continue;
        }
        let user_default_tp2 = overlay
            .source_default_explicit
            .then(|| mod_downloads::overlay_tp2_key(&overlay));
        if !overlay.source_default_explicit {
            overlay.source_default = false;
        }
        let mut source = by_source.remove(&key).unwrap_or_default();
        mod_downloads::apply_source_overlay(&mut source, overlay);
        mod_downloads::normalize_source(&mut source);
        if !mod_downloads::source_is_valid(&source) {
            continue;
        }
        if let Some(tp2_key) = user_default_tp2.as_deref() {
            mod_downloads::clear_other_source_defaults(&mut by_source, &key, tp2_key);
        }
        by_source.insert(key, source);
    }

    let mut sources = by_source.into_values().collect::<Vec<_>>();
    mod_downloads::sort_sources(&mut sources);
    let error = mod_downloads::merge_load_errors(default_load.error, user_load.error);
    ModDownloadsLoad { sources, error }
}

const fn old_overlay_has_version_selector(overlay: &ModDownloadSourceOverlay) -> bool {
    overlay.commit.is_some()
        || overlay.tag.is_some()
        || overlay.branch.is_some()
        || overlay.release.is_some()
        || overlay.channel.is_some()
        || overlay.asset.is_some()
}

fn old_clear_source_version_selectors(source: &mut ModDownloadSource) {
    source.commit = None;
    source.tag = None;
    source.branch = None;
    source.release = None;
    source.channel = None;
    source.asset = None;
}

fn old_apply_modlist_overlay(result: &mut ModDownloadsLoad, per_load: ModDownloadsOverlayLoad) {
    let mut by_source: BTreeMap<String, ModDownloadSource> = result
        .sources
        .drain(..)
        .map(|s| {
            let key = format!(
                "{}|{}",
                mod_downloads::normalize_mod_download_tp2(&s.tp2),
                mod_downloads::normalize_source_id(&s.source_id)
            );
            (key, s)
        })
        .collect();
    for mut overlay in per_load.sources {
        let key = mod_downloads::overlay_source_key(&overlay);
        if key.is_empty() {
            continue;
        }
        let per_default_tp2 = overlay
            .source_default_explicit
            .then(|| mod_downloads::overlay_tp2_key(&overlay));
        if !overlay.source_default_explicit {
            overlay.source_default = false;
        }
        let mut source = by_source.remove(&key).unwrap_or_default();
        if old_overlay_has_version_selector(&overlay) {
            old_clear_source_version_selectors(&mut source);
        }
        mod_downloads::apply_source_overlay(&mut source, overlay);
        mod_downloads::normalize_source(&mut source);
        if !mod_downloads::source_is_valid(&source) {
            continue;
        }
        if let Some(tp2_key) = per_default_tp2.as_deref() {
            mod_downloads::clear_other_source_defaults(&mut by_source, &key, tp2_key);
        }
        by_source.insert(key, source);
    }
    result.sources = by_source.into_values().collect();
    mod_downloads::sort_sources(&mut result.sources);
    result.error = mod_downloads::merge_load_errors(result.error.take(), per_load.error);
}

#[cfg(test)]
fn resolve_old_rules(default_text: &str, user_text: &str, modlist_text: &str) -> ModDownloadsLoad {
    let default_load = mod_downloads::load_source_overlays_from_str(default_text, "default");
    let user_load = mod_downloads::load_source_overlays_from_str(user_text, "user");
    let mut result = old_two_tier_from_overlays(default_load, user_load);

    if !modlist_text.trim().is_empty() {
        let per_load = mod_downloads::load_source_overlays_from_str(modlist_text, "modlist");
        old_apply_modlist_overlay(&mut result, per_load);
    }

    result
}

fn source_key(source: &ModDownloadSource) -> String {
    format!(
        "{}|{}",
        mod_downloads::normalize_mod_download_tp2(&source.tp2),
        mod_downloads::normalize_source_id(&source.source_id)
    )
}

fn map_channel_word(channel: Option<String>) -> Option<String> {
    let lower = channel
        .as_deref()
        .map(|value| value.trim().to_ascii_lowercase());
    match lower.as_deref() {
        Some("pre-release") => Some("preonly".to_string()),
        _ => channel,
    }
}

fn map_channel_words_in_overlays(mut load: ModDownloadsOverlayLoad) -> ModDownloadsOverlayLoad {
    for overlay in &mut load.sources {
        overlay.channel = map_channel_word(overlay.channel.take());
    }
    load
}

fn text_format_at_least_2(text: &str) -> bool {
    toml::from_str::<ModDownloadsFile>(text)
        .ok()
        .and_then(|file| file.format)
        .is_some_and(|format| format >= 2)
}

pub(crate) fn migrate_source_text(
    text: &str,
    default_text: &str,
    user_text: &str,
    tier: MigrateTier,
) -> MigrateResult {
    let parsed = match toml::from_str::<ModDownloadsFile>(text) {
        Ok(value) => value,
        Err(err) => return MigrateResult::Unparseable(err.to_string()),
    };
    if parsed.format.unwrap_or(0) >= 2 {
        return MigrateResult::Unchanged;
    }

    let own_overlays = mod_downloads::load_source_overlays_from_str(text, "migrate");
    if let Some(err) = own_overlays.error {
        return MigrateResult::Unparseable(err);
    }
    let mapped_own_overlays = map_channel_words_in_overlays(own_overlays.clone());

    let resolved = match tier {
        MigrateTier::User => {
            let default_load =
                mod_downloads::load_source_overlays_from_str(default_text, "default");
            old_two_tier_from_overlays(default_load, mapped_own_overlays)
        }
        MigrateTier::Modlist => {
            let default_load =
                mod_downloads::load_source_overlays_from_str(default_text, "default");
            let user_load = mod_downloads::load_source_overlays_from_str(user_text, "user");
            let mut base = if text_format_at_least_2(user_text) {
                mod_downloads::two_tier_from_overlays(default_load, user_load)
            } else {
                old_two_tier_from_overlays(default_load, map_channel_words_in_overlays(user_load))
            };
            old_apply_modlist_overlay(&mut base, mapped_own_overlays);
            base
        }
    };
    let mut by_key = BTreeMap::<String, ModDownloadSource>::new();
    for source in resolved.sources {
        by_key.insert(source_key(&source), source);
    }

    let mut ordered_keys = Vec::<String>::new();
    let mut seen_keys = BTreeSet::<String>::new();
    for overlay in &own_overlays.sources {
        let key = mod_downloads::overlay_source_key(overlay);
        if key.is_empty() || !seen_keys.insert(key.clone()) {
            continue;
        }
        ordered_keys.push(key);
    }

    let mut tp2_order = Vec::<String>::new();
    let mut grouped = BTreeMap::<String, Vec<ModDownloadSource>>::new();
    for key in ordered_keys {
        let Some(source) = by_key.get(&key).cloned() else {
            continue;
        };
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&source.tp2);
        if !grouped.contains_key(&tp2_key) {
            tp2_order.push(tp2_key.clone());
        }
        grouped.entry(tp2_key).or_default().push(source);
    }

    let mut out = String::new();
    out.push_str(migrated_header());
    out.push_str("\n\nformat = 2");
    for tp2_key in tp2_order {
        let Some(sources) = grouped.get(&tp2_key) else {
            continue;
        };
        let Some(first) = sources.first() else {
            continue;
        };
        out.push_str("\n\n");
        out.push_str(&mod_downloads::template_mod_header(&first.tp2, &first.name));
        for source in sources {
            out.push_str("\n\n");
            out.push_str(&mod_downloads::complete_source_block(source));
        }
    }
    out.push_str("\n\n");
    out.push_str(mod_downloads::user_template_cheat_sheet());
    out.push('\n');

    MigrateResult::Rewritten(out)
}

fn backup_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    name.push_str(".v1.bak");
    path.with_file_name(name)
}

fn backup_once(path: &Path) -> std::io::Result<()> {
    let backup_path = backup_path_for(path);
    if backup_path.exists() {
        return Ok(());
    }
    fs::copy(path, &backup_path).map(|_| ())
}

fn migrate_one_file(path: &Path, default_text: &str, user_text: &str, tier: MigrateTier) {
    let Ok(existing) = fs::read_to_string(path) else {
        return;
    };
    if existing.trim().is_empty() {
        return;
    }
    match migrate_source_text(&existing, default_text, user_text, tier) {
        MigrateResult::Unchanged => {}
        MigrateResult::Unparseable(err) => {
            warn!(
                target = "mod_downloads_migrate",
                "skip {}: {err}",
                path.display()
            );
        }
        MigrateResult::Rewritten(new_text) => {
            if let Err(err) = backup_once(path) {
                warn!(
                    target = "mod_downloads_migrate",
                    "backup failed for {}: {err}",
                    path.display()
                );
                return;
            }
            if let Err(err) = fs::write(path, new_text) {
                warn!(
                    target = "mod_downloads_migrate",
                    "write failed for {}: {err}",
                    path.display()
                );
            }
        }
    }
}

fn modlists_root_dir() -> Option<PathBuf> {
    crate::registry::store_workspace::modlist_data_dir("_")
        .parent()
        .map(Path::to_path_buf)
}

fn reference_file_text(embedded: &str) -> String {
    format!("{REFERENCE_FILE_HEADER}\n\n{embedded}")
}

fn write_reference_file(dir: &Path, name: &str, embedded: &str) {
    let path = dir.join(name);
    if fs::symlink_metadata(&path).is_ok_and(|meta| !meta.file_type().is_file()) {
        warn!(
            target = "mod_downloads_migrate",
            "{} is not a regular file; leaving it as is",
            path.display()
        );
        return;
    }
    let content = reference_file_text(embedded);
    if fs::read_to_string(&path).is_ok_and(|existing| existing == content) {
        return;
    }
    if let Err(err) = fs::create_dir_all(dir).and_then(|()| fs::write(&path, content)) {
        warn!(
            target = "mod_downloads_migrate",
            "write {} failed: {err}",
            path.display()
        );
    }
}

fn write_reference_files() {
    let Some(dir) = crate::platform_defaults::app_config_dir() else {
        return;
    };
    write_reference_file(
        &dir,
        MOD_DOWNLOADS_DEFAULT_FILE_NAME,
        mod_downloads::default_mod_downloads_content(),
    );
    write_reference_file(
        &dir,
        COMPAT_RULES_DEFAULT_FILE_NAME,
        crate::app::compat_rules::default_step2_rules_content(),
    );
}

pub fn migrate_source_files_at_launch() {
    let default_text = mod_downloads::default_mod_downloads_content();

    let user_path = mod_downloads::mod_downloads_user_path();
    migrate_one_file(&user_path, default_text, "", MigrateTier::User);
    let user_text_after = fs::read_to_string(&user_path).unwrap_or_default();

    if let Some(modlists_root) = modlists_root_dir()
        && let Ok(entries) = fs::read_dir(&modlists_root)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file = path.join("mod_downloads_user.toml");
            if file.is_file() {
                migrate_one_file(&file, default_text, &user_text_after, MigrateTier::Modlist);
            }
        }
    }

    write_reference_files();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MigrateTestRoot {
        path: PathBuf,
    }

    impl MigrateTestRoot {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_mod_downloads_migrate_test_{}_{}_{label}",
                std::process::id(),
                id
            ));
            fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self { path }
        }
    }

    impl Drop for MigrateTestRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.path);
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn assert_same_meaning(key: &str, old: &ModDownloadSource, new: &ModDownloadSource) {
        assert_eq!(new.url, old.url, "{key}: url must be preserved");
        assert_eq!(new.github, old.github, "{key}: github must be preserved");
        assert_eq!(new.commit, old.commit, "{key}: commit must be preserved");
        assert_eq!(new.tag, old.tag, "{key}: tag must be preserved");
        assert_eq!(new.branch, old.branch, "{key}: branch must be preserved");
        assert_eq!(new.release, old.release, "{key}: release must be preserved");
        assert_eq!(
            new.channel,
            map_channel_word(old.channel.clone()),
            "{key}: channel must map old words to new ones"
        );
        assert_eq!(new.asset, old.asset, "{key}: asset must be preserved");
        assert_eq!(
            new.subdir_require, old.subdir_require,
            "{key}: subdir_require must be preserved"
        );
        assert_eq!(new.aliases, old.aliases, "{key}: aliases must be preserved");
        assert_eq!(
            new.exact_github, old.exact_github,
            "{key}: exact_github must be preserved"
        );
        assert_eq!(
            new.config_files, old.config_files,
            "{key}: config_files must be preserved"
        );
        assert_eq!(
            new.source_default, old.source_default,
            "{key}: source_default must be preserved"
        );
        assert_eq!(
            new.pkg_windows, old.pkg_windows,
            "{key}: pkg_windows must be preserved"
        );
        assert_eq!(
            new.pkg_linux, old.pkg_linux,
            "{key}: pkg_linux must be preserved"
        );
        assert_eq!(
            new.pkg_macos, old.pkg_macos,
            "{key}: pkg_macos must be preserved"
        );
    }

    const FIXTURE_DEFAULT_TEXT: &str = "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModA\"\n  repo = \"Owner/ModA\"\n  channel = \"pre-release\"\n  pkg_windows = \"wzp,zip\"\n\n[[mods]]\nname = \"ModB\"\ntp2 = \"modb\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModB\"\n  repo = \"Owner/ModB\"\n  tag = \"v1.0\"\n  pkg_windows = \"wzp,zip\"\n";

    const FIXTURE_USER_TEXT: &str = "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  aliases = [\"oldmoda\"]\n";

    const FIXTURE_MODLIST_TEXT: &str = "[[mods]]\nname = \"ModB\"\ntp2 = \"modb\"\n\n  [[mods.sources]]\n  id = \"main\"\n  subdir_require = \"v2\"\n";

    const RICH_DEFAULT_TEXT: &str = "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModA\"\n  repo = \"Owner/ModA\"\n  pkg_windows = \"wzp,zip\"\n\n  [[mods.sources]]\n  id = \"mirror\"\n  label = \"Mirror\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModA-Mirror\"\n  repo = \"Owner/ModA-Mirror\"\n  default = true\n\n[[mods]]\nname = \"ModB\"\ntp2 = \"modb\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModB\"\n  repo = \"Owner/ModB\"\n  tag = \"v1.0\"\n  pkg_windows = \"wzp,zip\"\n\n[[mods]]\nname = \"ModC\"\ntp2 = \"modc\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModC\"\n  repo = \"Owner/ModC\"\n  channel = \"releases\"\n\n[[mods]]\nname = \"ModD\"\ntp2 = \"modd\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModD\"\n  repo = \"Owner/ModD\"\n\n[[mods]]\nname = \"ModE\"\ntp2 = \"mode\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModE\"\n  repo = \"Owner/ModE\"\n\n[[mods]]\nname = \"ModF\"\ntp2 = \"modf\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModF\"\n  repo = \"Owner/ModF\"\n  exact_github = [\"Fork/ModF\"]\n\n[[mods]]\nname = \"ModG\"\ntp2 = \"modg\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModG\"\n  repo = \"Owner/ModG\"\n  aliases = [\"oldmodg\"]\n\n[[mods]]\nname = \"ModH\"\ntp2 = \"modh\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModH\"\n  repo = \"Owner/ModH\"\n  subdir_require = \"v2\"\n\n[[mods]]\nname = \"ModI\"\ntp2 = \"modi\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/ModI\"\n  repo = \"Owner/ModI\"\n";

    const RICH_USER_TEXT: &str = "[[mods]]\nname = \"ModA\"\ntp2 = \"moda\"\n\n  [[mods.sources]]\n  id = \"main\"\n  aliases = [\"oldmoda\"]\n\n[[mods]]\nname = \"ModD\"\ntp2 = \"modd\"\n\n  [[mods.sources]]\n  id = \"main\"\n  channel = \"pre-release\"\n\n[[mods]]\nname = \"ModI\"\ntp2 = \"modi\"\n\n  [[mods.sources]]\n  id = \"main\"\n  commit = \"abcdef1234567\"\n  pkg_windows = \"wzp,zip\"\n";

    const RICH_MODLIST_TEXT: &str = "[[mods]]\nname = \"ModB\"\ntp2 = \"modb\"\n\n  [[mods.sources]]\n  id = \"main\"\n  subdir_require = \"v2\"\n\n[[mods]]\nname = \"ModE\"\ntp2 = \"mode\"\n\n  [[mods.sources]]\n  id = \"main\"\n  channel = \"pre-release\"\n";

    #[test]
    fn migration_preserves_resolution() {
        let old_before = resolve_old_rules(RICH_DEFAULT_TEXT, RICH_USER_TEXT, RICH_MODLIST_TEXT);

        let MigrateResult::Rewritten(user_migrated) =
            migrate_source_text(RICH_USER_TEXT, RICH_DEFAULT_TEXT, "", MigrateTier::User)
        else {
            panic!("expected the user tier to be rewritten")
        };
        let MigrateResult::Rewritten(modlist_migrated) = migrate_source_text(
            RICH_MODLIST_TEXT,
            RICH_DEFAULT_TEXT,
            &user_migrated,
            MigrateTier::Modlist,
        ) else {
            panic!("expected the modlist tier to be rewritten")
        };

        let new_after = mod_downloads::load_mod_download_sources_from_texts(
            RICH_DEFAULT_TEXT,
            &user_migrated,
            &modlist_migrated,
        );

        let old_by_key = old_before
            .sources
            .iter()
            .map(|source| (source_key(source), source.clone()))
            .collect::<BTreeMap<_, _>>();
        let new_by_key = new_after
            .sources
            .iter()
            .map(|source| (source_key(source), source.clone()))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(
            old_by_key.keys().collect::<Vec<_>>(),
            new_by_key.keys().collect::<Vec<_>>(),
            "migration must preserve exactly the same set of resolved keys"
        );

        for (key, old_source) in &old_by_key {
            let new_source = new_by_key
                .get(key)
                .unwrap_or_else(|| panic!("missing key after migration: {key}"));
            assert_same_meaning(key, old_source, new_source);
        }
    }

    #[test]
    fn modlist_import_does_not_remap_a_new_format_user_pre_release() {
        let default_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n";
        let user_text = "format = 2\n\n[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  channel = \"pre-release\"\n  default = true\n";
        let code_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n";

        let MigrateResult::Rewritten(migrated) =
            migrate_source_text(code_text, default_text, user_text, MigrateTier::Modlist)
        else {
            panic!("expected a rewrite")
        };

        assert!(
            migrated.contains("channel = \"pre-release\""),
            "a pre-release inherited from an already-migrated My default must not be remapped; got:\n{migrated}"
        );
    }

    #[test]
    fn modlist_migration_maps_pre_release_from_an_old_format_user_file() {
        let default_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n";
        let user_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  channel = \"pre-release\"\n";
        let code_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n";

        let MigrateResult::Rewritten(migrated) =
            migrate_source_text(code_text, default_text, user_text, MigrateTier::Modlist)
        else {
            panic!("expected a rewrite")
        };

        assert!(
            migrated.contains("channel = \"preonly\""),
            "a pre-release inherited from an old-format user file must be mapped; got:\n{migrated}"
        );
    }

    #[test]
    fn modlist_import_code_pin_beats_a_new_format_user_release() {
        let default_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n";
        let user_text = "format = 2\n\n[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  release = \"v35.17\"\n  default = true\n";

        let channel_code_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  channel = \"pre-release\"\n";
        let MigrateResult::Rewritten(channel_migrated) = migrate_source_text(
            channel_code_text,
            default_text,
            user_text,
            MigrateTier::Modlist,
        ) else {
            panic!("expected a rewrite")
        };
        assert!(
            channel_migrated.contains("channel = \"preonly\""),
            "the code's own channel selector must win; got:\n{channel_migrated}"
        );
        assert!(
            channel_migrated.contains("release = \"\""),
            "a channel pin must clear an inherited release; got:\n{channel_migrated}"
        );

        let asset_code_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  asset = \"x.zip\"\n";
        let MigrateResult::Rewritten(asset_migrated) = migrate_source_text(
            asset_code_text,
            default_text,
            user_text,
            MigrateTier::Modlist,
        ) else {
            panic!("expected a rewrite")
        };
        assert!(
            asset_migrated.contains("asset = \"x.zip\""),
            "the code's own asset selector must win; got:\n{asset_migrated}"
        );
        assert!(
            asset_migrated.contains("release = \"\""),
            "an asset pin must clear an inherited release; got:\n{asset_migrated}"
        );
    }

    #[test]
    fn modlist_import_does_not_refill_new_format_user_blanks_from_the_catalog() {
        let default_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  pkg_windows = \"wzp,zip\"\n";
        let user_text = "format = 2\n\n[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  type = \"github\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  default = true\n";
        let code_text = "[[mods]]\nname = \"SCS\"\ntp2 = \"scs\"\n\n  [[mods.sources]]\n  id = \"main\"\n  label = \"Main\"\n  url = \"https://github.com/Owner/SCS\"\n  repo = \"Owner/SCS\"\n  subdir_require = \"v2\"\n";

        let MigrateResult::Rewritten(migrated) =
            migrate_source_text(code_text, default_text, user_text, MigrateTier::Modlist)
        else {
            panic!("expected a rewrite")
        };

        assert!(
            migrated.contains("pkg_windows = \"\""),
            "a new-format user block's blank pkg_windows must not be refilled from the catalog; got:\n{migrated}"
        );
    }

    #[test]
    fn migration_is_idempotent() {
        let MigrateResult::Rewritten(migrated) = migrate_source_text(
            FIXTURE_USER_TEXT,
            FIXTURE_DEFAULT_TEXT,
            "",
            MigrateTier::User,
        ) else {
            panic!("expected a rewrite")
        };

        match migrate_source_text(&migrated, FIXTURE_DEFAULT_TEXT, "", MigrateTier::User) {
            MigrateResult::Unchanged => {}
            _ => panic!("migrating an already-migrated file must be a no-op"),
        }
    }

    #[test]
    fn migration_skips_unparseable_text() {
        let bad_text = "[[mods]\nnot valid toml";

        match migrate_source_text(bad_text, FIXTURE_DEFAULT_TEXT, "", MigrateTier::User) {
            MigrateResult::Unparseable(_) => {}
            _ => panic!("unparseable text must not be silently rewritten"),
        }
    }

    #[test]
    fn migration_launch_writes_backup_once_and_writes_reference_files() {
        let root = MigrateTestRoot::new("launch");
        let user_path = root.path.join("mod_downloads_user.toml");
        fs::write(&user_path, FIXTURE_USER_TEXT).unwrap();
        fs::write(root.path.join("mod_downloads_default.toml"), "# stale\n").unwrap();
        fs::write(
            root.path.join("step2_compat_rules_default.toml"),
            "# stale\n",
        )
        .unwrap();
        let modlist_dir = root.path.join("modlists").join("list1");
        fs::create_dir_all(&modlist_dir).unwrap();
        let modlist_path = modlist_dir.join("mod_downloads_user.toml");
        fs::write(&modlist_path, FIXTURE_MODLIST_TEXT).unwrap();

        migrate_source_files_at_launch();

        assert!(
            root.path.join("mod_downloads_user.toml.v1.bak").is_file(),
            "the user tier must be backed up once"
        );
        assert!(
            modlist_path
                .with_file_name("mod_downloads_user.toml.v1.bak")
                .is_file(),
            "the modlist tier must be backed up once"
        );
        assert_reference_files_current(&root.path);

        let user_after_first = fs::read_to_string(&user_path).unwrap();
        let modlist_after_first = fs::read_to_string(&modlist_path).unwrap();

        migrate_source_files_at_launch();

        assert_eq!(
            fs::read_to_string(&user_path).unwrap(),
            user_after_first,
            "a second launch must not rewrite an already-migrated file"
        );
        assert_eq!(
            fs::read_to_string(&modlist_path).unwrap(),
            modlist_after_first,
            "a second launch must not rewrite an already-migrated file"
        );
        assert_reference_files_current(&root.path);
    }

    fn assert_reference_files_current(dir: &Path) {
        let expected = [
            (
                "mod_downloads_default.toml",
                mod_downloads::default_mod_downloads_content(),
            ),
            (
                "step2_compat_rules_default.toml",
                crate::app::compat_rules::default_step2_rules_content(),
            ),
        ];
        for (name, embedded) in expected {
            let text = fs::read_to_string(dir.join(name))
                .unwrap_or_else(|err| panic!("{name} must be written at launch: {err}"));
            assert!(
                text.starts_with(
                    "# Reference copy of BIO's built-in defaults, provided for reference only.\n# BIO does not read this file. Edits here change nothing and are overwritten when BIO starts.\n\n"
                ),
                "{name} must start with the reference header and a blank line"
            );
            assert_eq!(
                text,
                reference_file_text(embedded),
                "{name} must hold the header followed by the embedded text unchanged"
            );
        }
    }

    #[test]
    fn reference_files_are_rewritten_when_edited() {
        let root = MigrateTestRoot::new("reference");

        migrate_source_files_at_launch();
        assert_reference_files_current(&root.path);

        fs::write(root.path.join("mod_downloads_default.toml"), "# edited\n").unwrap();
        fs::write(
            root.path.join("step2_compat_rules_default.toml"),
            "# edited\n",
        )
        .unwrap();

        migrate_source_files_at_launch();
        assert_reference_files_current(&root.path);
    }
}
