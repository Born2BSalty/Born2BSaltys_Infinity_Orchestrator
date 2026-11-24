// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::app::mod_downloads::{self, ModDownloadsLoad, normalize_mod_download_tp2};
use crate::settings::redesign_fields::ModDisplayFormat;

const NAMES_RECHECK_INTERVAL: Duration = Duration::from_secs(1);

#[must_use]
pub(crate) fn mod_tp2_key(tp_file: &str) -> String {
    normalize_mod_download_tp2(tp_file)
}

#[must_use]
pub(crate) fn format_mod_display_name(
    format: ModDisplayFormat,
    human_name: Option<&str>,
    tp_file: &str,
) -> String {
    let tp2 = mod_tp2_key(tp_file);
    let human = human_name
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case(&tp2));
    match (format, human) {
        (ModDisplayFormat::NameAndTp2, Some(name)) if tp2.is_empty() => name.to_string(),
        (ModDisplayFormat::NameAndTp2, Some(name)) => format!("{name} ({tp2})"),
        (ModDisplayFormat::Name, Some(name)) => name.to_string(),
        (ModDisplayFormat::Tp2 | ModDisplayFormat::NameAndTp2 | ModDisplayFormat::Name, _) => tp2,
    }
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
    names
}

#[derive(Default)]
struct NamesCache {
    names: HashMap<String, String>,
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
        self.names = human_names_from_load(&mod_downloads::load_mod_download_sources());
        self.stamp = Some(stamp);
    }
}

thread_local! {
    static FORMAT: Cell<ModDisplayFormat> = const { Cell::new(ModDisplayFormat::NameAndTp2) };
    static NAMES: RefCell<NamesCache> = RefCell::new(NamesCache::default());
}

pub(crate) fn set_mod_display_format(format: ModDisplayFormat) {
    FORMAT.with(|slot| slot.set(format));
}

#[must_use]
pub(crate) fn mod_display_format() -> ModDisplayFormat {
    FORMAT.with(Cell::get)
}

#[must_use]
pub(crate) fn catalog_human_name(tp_file: &str) -> Option<String> {
    let key = mod_tp2_key(tp_file);
    NAMES.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.refresh_if_due();
        cache.names.get(&key).cloned()
    })
}

#[must_use]
pub(crate) fn mod_display_label(tp_file: &str) -> String {
    format_mod_display_name(
        mod_display_format(),
        catalog_human_name(tp_file).as_deref(),
        tp_file,
    )
}

#[must_use]
pub(crate) fn mod_display_label_with_name(human_name: Option<&str>, tp_file: &str) -> String {
    format_mod_display_name(mod_display_format(), human_name, tp_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_tp2_is_the_default_format() {
        assert_eq!(ModDisplayFormat::default(), ModDisplayFormat::NameAndTp2);
    }

    #[test]
    fn each_format_renders_its_shape() {
        let cases = [
            (ModDisplayFormat::NameAndTp2, "Tweaks Anthology (cdtweaks)"),
            (ModDisplayFormat::Name, "Tweaks Anthology"),
            (ModDisplayFormat::Tp2, "cdtweaks"),
        ];
        for (format, expected) in cases {
            assert_eq!(
                format_mod_display_name(format, Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
                expected
            );
        }
    }

    #[test]
    fn every_format_falls_back_to_the_tp2_without_a_human_name() {
        for format in ModDisplayFormat::all() {
            assert_eq!(
                format_mod_display_name(*format, None, "CDTWEAKS/SETUP-CDTWEAKS.TP2"),
                "cdtweaks"
            );
            assert_eq!(
                format_mod_display_name(*format, Some("  "), "setup-cdtweaks.tp2"),
                "cdtweaks"
            );
        }
    }

    #[test]
    fn a_human_name_equal_to_the_tp2_is_not_repeated() {
        assert_eq!(
            format_mod_display_name(ModDisplayFormat::NameAndTp2, Some("CDTweaks"), "cdtweaks"),
            "cdtweaks"
        );
    }

    #[test]
    fn the_label_follows_the_chosen_format() {
        set_mod_display_format(ModDisplayFormat::Tp2);
        assert_eq!(
            mod_display_label_with_name(Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
            "cdtweaks"
        );
        set_mod_display_format(ModDisplayFormat::Name);
        assert_eq!(
            mod_display_label_with_name(Some("Tweaks Anthology"), "setup-cdtweaks.tp2"),
            "Tweaks Anthology"
        );
        set_mod_display_format(ModDisplayFormat::default());
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
    }
}
