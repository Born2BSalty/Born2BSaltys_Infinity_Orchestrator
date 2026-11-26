// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::app::mod_downloads::{self, ModDownloadsLoad, normalize_mod_download_tp2};

const NAMES_RECHECK_INTERVAL: Duration = Duration::from_secs(1);

#[must_use]
pub(crate) fn mod_tp2_key(tp_file: &str) -> String {
    normalize_mod_download_tp2(tp_file)
}

#[must_use]
pub(crate) fn format_mod_display_name(
    show_names: bool,
    human_name: Option<&str>,
    tp_file: &str,
) -> String {
    let tp2 = mod_tp2_key(tp_file);
    human_name
        .map(str::trim)
        .filter(|name| show_names && !name.is_empty())
        .map_or(tp2, str::to_string)
}

#[must_use]
pub(crate) fn human_names_from_load(load: &ModDownloadsLoad) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for source in &load.sources {
        let name = source.name.trim();
        if name.is_empty() {
            continue;
        }
        for tp2 in source.declared_tp2_names() {
            names
                .entry(normalize_mod_download_tp2(&tp2))
                .or_insert_with(|| name.to_string());
        }
    }
    for (key, name) in load.layer_names.iter_highest_first() {
        names.entry(key.clone()).or_insert_with(|| name.clone());
    }
    names
}

fn canonical_tp2s_from_load(load: &ModDownloadsLoad) -> HashMap<String, String> {
    let mut canonical = HashMap::new();
    for source in &load.sources {
        let tp2 = normalize_mod_download_tp2(&source.tp2);
        if !tp2.is_empty() {
            canonical.entry(tp2.clone()).or_insert(tp2);
        }
    }
    for source in &load.sources {
        let tp2 = normalize_mod_download_tp2(&source.tp2);
        if tp2.is_empty() {
            continue;
        }
        for alias in source.declared_tp2_names() {
            let alias = normalize_mod_download_tp2(&alias);
            if !alias.is_empty() {
                canonical.entry(alias).or_insert_with(|| tp2.clone());
            }
        }
    }
    canonical
}

#[derive(Default)]
struct NamesCache {
    names: HashMap<String, String>,
    canonical: HashMap<String, String>,
    stamp: Option<CatalogStamp>,
    checked_at: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogStamp {
    user: Option<SystemTime>,
    modlist_path: Option<PathBuf>,
    modlist: Option<SystemTime>,
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

fn current_stamp() -> CatalogStamp {
    let modlist_path = mod_downloads::active_modlist_downloads_path();
    CatalogStamp {
        user: modified(&mod_downloads::mod_downloads_user_path()),
        modlist: modlist_path.as_deref().and_then(modified),
        modlist_path,
    }
}

impl NamesCache {
    fn refresh_if_due(&mut self) {
        if self
            .checked_at
            .is_some_and(|at| at.elapsed() < NAMES_RECHECK_INTERVAL)
        {
            return;
        }
        self.checked_at = Some(Instant::now());
        let stamp = current_stamp();
        if self.stamp.as_ref() == Some(&stamp) {
            return;
        }
        let load = mod_downloads::load_mod_download_sources();
        self.names = human_names_from_load(&load);
        self.canonical = canonical_tp2s_from_load(&load);
        self.stamp = Some(stamp);
    }

    fn rename_key(&self, tp_file: &str) -> String {
        let key = mod_tp2_key(tp_file);
        self.canonical.get(&key).cloned().unwrap_or(key)
    }

    fn tp2_names_of(&self, key: &str) -> BTreeSet<String> {
        self.canonical
            .iter()
            .filter(|(_, canonical)| canonical.as_str() == key)
            .map(|(name, _)| name.clone())
            .chain(std::iter::once(key.to_string()))
            .collect()
    }

    fn name(&self, tp_file: &str) -> Option<String> {
        self.names.get(&mod_tp2_key(tp_file)).cloned()
    }
}

thread_local! {
    static SHOW_NAMES: Cell<bool> = const { Cell::new(false) };
    static NAMES: RefCell<NamesCache> = RefCell::new(NamesCache::default());
}

fn with_names<T>(read: impl FnOnce(&NamesCache) -> T) -> T {
    NAMES.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.refresh_if_due();
        read(&cache)
    })
}

pub(crate) fn set_display_mod_names(show_names: bool) {
    SHOW_NAMES.with(|slot| slot.set(show_names));
}

#[must_use]
pub(crate) fn display_mod_names() -> bool {
    SHOW_NAMES.with(Cell::get)
}

pub(crate) fn invalidate_names() {
    NAMES.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.stamp = None;
        cache.checked_at = None;
    });
}

#[must_use]
pub(crate) fn rename_key(tp_file: &str) -> String {
    with_names(|cache| cache.rename_key(tp_file))
}

#[must_use]
pub(crate) fn tp2_names_of(key: &str) -> BTreeSet<String> {
    with_names(|cache| cache.tp2_names_of(key))
}

#[must_use]
pub(crate) fn catalog_human_name(tp_file: &str) -> Option<String> {
    with_names(|cache| cache.name(tp_file))
}

#[must_use]
pub(crate) fn mod_display_label(tp_file: &str) -> String {
    if !display_mod_names() {
        return mod_tp2_key(tp_file);
    }
    format_mod_display_name(
        display_mod_names(),
        catalog_human_name(tp_file).as_deref(),
        tp_file,
    )
}

#[must_use]
pub(crate) fn mod_display_label_with_name(human_name: Option<&str>, tp_file: &str) -> String {
    if !display_mod_names() {
        return format_mod_display_name(false, human_name, tp_file);
    }
    let given = human_name.map(str::trim).filter(|name| !name.is_empty());
    if given.is_some() {
        return format_mod_display_name(true, given, tp_file);
    }
    format_mod_display_name(true, catalog_human_name(tp_file).as_deref(), tp_file)
}

#[cfg(test)]
pub(crate) struct NamesShownForTest(PathBuf);

#[cfg(test)]
impl NamesShownForTest {
    pub(crate) fn new(show_names: bool) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "bio_mod_names_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::platform_defaults::set_config_dir_override(Some(root.clone()));
        set_display_mod_names(show_names);
        invalidate_names();
        Self(root)
    }
}

#[cfg(test)]
impl Drop for NamesShownForTest {
    fn drop(&mut self) {
        set_display_mod_names(false);
        crate::platform_defaults::clear_config_dir_override_if(&self.0);
        invalidate_names();
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tp2_names_are_shown_by_default() {
        assert!(!display_mod_names());
        assert_eq!(
            mod_display_label_with_name(Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
            "cdtweaks"
        );
    }

    #[test]
    fn each_setting_renders_its_shape() {
        assert_eq!(
            format_mod_display_name(true, Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
            "Tweaks Anthology"
        );
        assert_eq!(
            format_mod_display_name(false, Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
            "cdtweaks"
        );
    }

    #[test]
    fn mod_names_fall_back_to_the_tp2_without_a_human_name() {
        for show_names in [false, true] {
            assert_eq!(
                format_mod_display_name(show_names, None, "CDTWEAKS/SETUP-CDTWEAKS.TP2"),
                "cdtweaks"
            );
            assert_eq!(
                format_mod_display_name(show_names, Some("  "), "setup-cdtweaks.tp2"),
                "cdtweaks"
            );
        }
    }

    #[test]
    fn the_label_follows_the_setting() {
        let _names = NamesShownForTest::new(true);
        assert_eq!(
            mod_display_label_with_name(Some("Tweaks Anthology"), "setup-biolabeltweaks.tp2"),
            "Tweaks Anthology"
        );
        set_display_mod_names(false);
        assert_eq!(
            mod_display_label_with_name(Some("Tweaks Anthology"), "setup-biolabeltweaks.tp2"),
            "biolabeltweaks"
        );
    }

    #[test]
    fn human_names_are_keyed_by_tp2_and_aliases() {
        let load = mod_downloads::load_mod_download_sources_from_texts(
            "[[mods]]\nname = \"Folder Named\"\ntp2 = \"filename\"\naliases = [\"foldername\"]\n\n  [[mods.sources]]\n  id = \"primary\"\n  type = \"url\"\n  url = \"https://example.test/foldername.zip\"\n",
            "",
            "",
        );
        let names = human_names_from_load(&load);
        assert_eq!(
            names.get("filename").map(String::as_str),
            Some("Folder Named")
        );
        assert_eq!(
            names.get("foldername").map(String::as_str),
            Some("Folder Named")
        );
        let canonical = canonical_tp2s_from_load(&load);
        assert_eq!(
            canonical.get("foldername").map(String::as_str),
            Some("filename")
        );
        assert_eq!(
            canonical.get("filename").map(String::as_str),
            Some("filename")
        );
    }
}
