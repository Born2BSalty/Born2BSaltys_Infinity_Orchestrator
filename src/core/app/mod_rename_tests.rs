// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::sync::MutexGuard;
use std::sync::atomic::{AtomicU64, Ordering};

use super::set_mod_name;
use crate::app::mod_display_name::{
    catalog_human_name, invalidate_names, mod_display_label, mod_display_label_with_name,
    set_display_mod_names,
};
use crate::app::mod_downloads::{
    self, ModDownloadSource, complete_source_block, default_mod_downloads_content,
    load_mod_download_sources, load_mod_download_sources_from_texts, mod_downloads_user_path,
    mod_header_with_categories, save_global_mod_download_source_block,
    save_user_mod_download_source_block,
};

const CUSTOM_SOURCE: &str = "# my own comment\nformat = 2\n\n[[mods]]\nname = \"Custom Tweaks Source\"\ntp2 = \"cdtweaks\"\n\n  [[mods.sources]]\n  id = \"custom\"\n  type = \"url\"\n  url = \"https://example.test/custom.zip\"\n";

struct Fixture {
    root: PathBuf,
    previous_modlist: Option<PathBuf>,
    _lock: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new(user_text: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let lock = mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = std::env::temp_dir().join(format!(
            "bio_mod_rename_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let fixture = Self {
            root,
            previous_modlist: mod_downloads::active_modlist_dir(),
            _lock: lock,
        };
        mod_downloads::set_active_modlist_dir(None);
        crate::platform_defaults::set_config_dir_override(Some(fixture.root.clone()));
        set_display_mod_names(true);
        let path = mod_downloads_user_path();
        assert!(path.starts_with(&fixture.root));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, user_text).unwrap();
        invalidate_names();
        fixture
    }
}

fn open_modlist(list_text: Option<&str>) -> PathBuf {
    let dir = crate::platform_defaults::app_config_dir()
        .expect("the fixture's config dir")
        .join("modlists")
        .join("rename-list");
    mod_downloads::set_active_modlist_dir(Some(dir));
    let path = mod_downloads::active_modlist_downloads_path().unwrap();
    if let Some(text) = list_text {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    }
    invalidate_names();
    path
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_display_mod_names(false);
        mod_downloads::set_active_modlist_dir(self.previous_modlist.take());
        crate::platform_defaults::clear_config_dir_override_if(&self.root);
        invalidate_names();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn user_text() -> String {
    std::fs::read_to_string(mod_downloads_user_path()).unwrap()
}

fn entries_for(path: &Path, tp2: &str) -> Vec<toml::Table> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let parsed = toml::from_str::<toml::Table>(&text).unwrap();
    parsed
        .get("mods")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .filter(|entry| {
            entry
                .get("tp2")
                .and_then(toml::Value::as_str)
                .is_some_and(|value| {
                    mod_downloads::normalize_mod_download_tp2(value)
                        == mod_downloads::normalize_mod_download_tp2(tp2)
                })
        })
        .cloned()
        .collect()
}

fn entry_name(entry: &toml::Table) -> Option<&str> {
    entry.get("name").and_then(toml::Value::as_str)
}

fn catalog_name(tp2: &str) -> String {
    load_mod_download_sources_from_texts(default_mod_downloads_content(), "", "")
        .mod_name(tp2)
        .unwrap_or_else(|| mod_downloads::normalize_mod_download_tp2(tp2))
}

fn first_catalog_tp2() -> String {
    load_mod_download_sources_from_texts(default_mod_downloads_content(), "", "")
        .sources
        .into_iter()
        .find(|source| !source.name.trim().is_empty())
        .map(|source| mod_downloads::normalize_mod_download_tp2(&source.tp2))
        .expect("the catalog names at least one mod")
}

fn url_source(tp2: &str, id: &str, url: &str) -> ModDownloadSource {
    ModDownloadSource {
        tp2: tp2.to_string(),
        source_id: id.to_string(),
        source_label: id.to_string(),
        url: url.to_string(),
        kind: Some("url".to_string()),
        ..ModDownloadSource::default()
    }
}

#[test]
fn set_mod_name_adds_updates_and_clears() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    let user = mod_downloads_user_path();

    set_mod_name(&user, "setup-bionameonly.tp2", "  Only Name  ").unwrap();
    let added = entries_for(&user, "bionameonly");
    assert_eq!(added.len(), 1);
    assert_eq!(entry_name(&added[0]), Some("Only Name"));
    assert!(!added[0].contains_key("sources"));
    assert_eq!(mod_display_label("bionameonly"), "Only Name");
    assert!(!user_text().contains("names."));

    set_mod_name(&user, "bionameonly", "Better Name").unwrap();
    let updated = entries_for(&user, "bionameonly");
    assert_eq!(updated.len(), 1);
    assert_eq!(entry_name(&updated[0]), Some("Better Name"));

    set_mod_name(&user, "bionameonly", "   ").unwrap();
    assert_eq!(entries_for(&user, "bionameonly").len(), 0);
    assert_eq!(mod_display_label("bionameonly"), "bionameonly");

    set_mod_name(&user, "cdtweaks", "My Tweaks").unwrap();
    let renamed = entries_for(&user, "cdtweaks");
    assert_eq!(renamed.len(), 1);
    assert_eq!(entry_name(&renamed[0]), Some("My Tweaks"));
    assert!(user_text().contains("https://example.test/custom.zip"));
    assert!(user_text().contains("# my own comment"));

    set_mod_name(&user, "cdtweaks", "").unwrap();
    let cleared = entries_for(&user, "cdtweaks");
    assert_eq!(cleared.len(), 1);
    assert_eq!(
        entry_name(&cleared[0]),
        Some(catalog_name("cdtweaks").as_str())
    );
    assert!(user_text().contains("https://example.test/custom.zip"));
}

#[test]
fn clearing_a_rename_keeps_a_flat_entry_with_source_fields() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    let user = mod_downloads_user_path();
    let pinned = format!(
        "{}\n[[mods]]\nname = \"Pinned Name\"\ntp2 = \"biopinned\"\ntag = \"v2\"\n",
        user_text()
    );
    std::fs::write(&user, pinned).unwrap();

    set_mod_name(&user, "biopinned", "").unwrap();

    let kept = entries_for(&user, "biopinned");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].get("tag").and_then(toml::Value::as_str), Some("v2"));
}

#[test]
fn a_rename_shows_in_every_label_and_never_changes_sources() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    let ids_before = load_mod_download_sources()
        .find_sources("cdtweaks")
        .into_iter()
        .map(|source| (source.source_id, source.url))
        .collect::<Vec<_>>();

    set_mod_name(
        &mod_downloads_user_path(),
        "setup-cdtweaks.tp2",
        "My Tweaks",
    )
    .unwrap();

    assert_eq!(
        catalog_human_name("CDTWEAKS/SETUP-CDTWEAKS.TP2").as_deref(),
        Some("My Tweaks")
    );
    assert_eq!(mod_display_label_with_name(None, "cdtweaks"), "My Tweaks");
    assert_eq!(
        mod_display_label_with_name(Some("  "), "cdtweaks"),
        "My Tweaks"
    );
    assert_eq!(
        mod_display_label_with_name(Some("Other"), "cdtweaks"),
        "Other"
    );
    let after = load_mod_download_sources().find_sources("cdtweaks");
    assert!(after.iter().all(|source| source.name == "My Tweaks"));
    assert_eq!(
        after
            .into_iter()
            .map(|source| (source.source_id, source.url))
            .collect::<Vec<_>>(),
        ids_before
    );

    set_display_mod_names(false);
    assert_eq!(mod_display_label("setup-cdtweaks.tp2"), "cdtweaks");
}

#[test]
fn a_rename_survives_saving_and_deleting_a_source() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    let user = mod_downloads_user_path();
    set_mod_name(&user, "cdtweaks", "My Tweaks").unwrap();

    save_user_mod_download_source_block(
        "cdtweaks",
        "Custom Tweaks Source",
        "custom",
        false,
        "  [[mods.sources]]\n  id = \"custom\"\n  type = \"url\"\n  url = \"https://example.test/edited.zip\"",
        None,
    )
    .unwrap();
    assert!(user_text().contains("https://example.test/edited.zip"));
    let saved = entries_for(&user, "cdtweaks");
    assert_eq!(saved.len(), 1);
    assert_eq!(entry_name(&saved[0]), Some("My Tweaks"));
    invalidate_names();
    assert_eq!(mod_display_label("cdtweaks"), "My Tweaks");

    save_user_mod_download_source_block(
        "cdtweaks",
        "Custom Tweaks Source",
        "custom",
        false,
        "",
        None,
    )
    .unwrap();
    assert!(!user_text().contains("example.test/edited.zip"));
    let removed = entries_for(&user, "cdtweaks");
    assert_eq!(removed.len(), 1);
    assert_eq!(entry_name(&removed[0]), Some("My Tweaks"));
    assert!(!removed[0].contains_key("sources"));
    invalidate_names();
    assert_eq!(mod_display_label("cdtweaks"), "My Tweaks");
}

#[test]
fn a_source_save_never_copies_a_list_name_into_the_user_file() {
    let _fixture = Fixture::new("format = 2\n");
    let tp2 = first_catalog_tp2();
    let list = open_modlist(Some(&format!(
        "format = 2\n\n[[mods]]\nname = \"List Name\"\ntp2 = \"{tp2}\"\n"
    )));
    assert_eq!(mod_display_label(&tp2), "List Name");
    let source = url_source(&tp2, "bio-rename-test", "https://example.test/global.zip");
    let text = format!(
        "{}\n\n{}\n",
        mod_header_with_categories(&tp2, "List Name", &[]),
        complete_source_block(&source)
    );

    save_global_mod_download_source_block(&tp2, "List Name", "bio-rename-test", true, &text, None)
        .unwrap();

    let user = mod_downloads_user_path();
    let saved = entries_for(&user, &tp2);
    assert_eq!(saved.len(), 1);
    assert_eq!(entry_name(&saved[0]), Some(catalog_name(&tp2).as_str()));
    assert!(!user_text().contains("List Name"));
    assert!(user_text().contains("https://example.test/global.zip"));
    assert_eq!(entry_name(&entries_for(&list, &tp2)[0]), Some("List Name"));
    invalidate_names();
    assert_eq!(mod_display_label(&tp2), "List Name");
}

fn catalog_tp2_with_alias() -> (String, String) {
    load_mod_download_sources_from_texts(default_mod_downloads_content(), "", "")
        .sources
        .into_iter()
        .find_map(|source| {
            let tp2 = mod_downloads::normalize_mod_download_tp2(&source.tp2);
            source
                .declared_tp2_names()
                .into_iter()
                .map(|name| mod_downloads::normalize_mod_download_tp2(&name))
                .find(|alias| !alias.is_empty() && *alias != tp2)
                .map(|alias| (tp2, alias))
        })
        .expect("the catalog declares at least one tp2 alias")
}

fn mod_entry_count(path: &Path) -> usize {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    toml::from_str::<toml::Table>(&text)
        .unwrap()
        .get("mods")
        .and_then(toml::Value::as_array)
        .map_or(0, Vec::len)
}

#[test]
fn a_rename_through_the_file_tp2_updates_an_alias_entry() {
    let (tp2, alias) = catalog_tp2_with_alias();
    let _fixture = Fixture::new(&format!(
        "format = 2\n\n[[mods]]\nname = \"Alias Name\"\ntp2 = \"{alias}\"\n"
    ));
    let user = mod_downloads_user_path();

    set_mod_name(&user, &tp2, "Renamed Through File").unwrap();

    let aliased = entries_for(&user, &alias);
    assert_eq!(aliased.len(), 1);
    assert_eq!(entry_name(&aliased[0]), Some("Renamed Through File"));
    assert_eq!(entries_for(&user, &tp2).len(), 0);
    assert_eq!(mod_entry_count(&user), 1);
}

#[test]
fn a_list_source_save_takes_the_user_name_for_a_new_header() {
    let _fixture = Fixture::new("format = 2\n");
    let tp2 = first_catalog_tp2();
    set_mod_name(&mod_downloads_user_path(), &tp2, "User Name").unwrap();
    let list = open_modlist(None);
    let source = url_source(&tp2, "bio-rename-test", "https://example.test/list.zip");
    let text = format!(
        "{}\n\n{}\n",
        mod_header_with_categories(&tp2, "Form Name", &[]),
        complete_source_block(&source)
    );

    save_user_mod_download_source_block(
        &tp2,
        "Form Name",
        "bio-rename-test",
        true,
        &text,
        Some(&list),
    )
    .unwrap();

    let saved = entries_for(&list, &tp2);
    assert_eq!(saved.len(), 1);
    assert_eq!(entry_name(&saved[0]), Some("User Name"));
}
