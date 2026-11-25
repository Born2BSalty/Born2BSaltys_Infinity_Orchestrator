// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::MutexGuard;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{set_user_mod_name, user_mod_renames};
use crate::app::mod_display_name::{
    catalog_human_name, invalidate_names, mod_display_label, mod_display_label_with_name,
    set_display_mod_names,
};
use crate::app::mod_downloads::{
    self, load_mod_download_sources, mod_downloads_user_path, save_user_mod_download_source_block,
};

const FIXTURE_CATALOG: &str = "format = 2\n\n[[mods]]\nname = \"Fixture Catalog Name\"\ntp2 = \"biorenamefixture\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  type = \"url\"\n  url = \"https://example.test/primary.zip\"\n\n  [[mods.sources]]\n  id = \"mirror\"\n  type = \"url\"\n  url = \"https://example.test/mirror.zip\"\n";

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

fn renames(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, name)| ((*key).to_string(), (*name).to_string()))
        .collect()
}

fn source_names(tp2: &str) -> Vec<(String, String)> {
    load_mod_download_sources()
        .find_sources(tp2)
        .into_iter()
        .map(|source| (source.source_id, source.name))
        .collect()
}

fn assert_line_added_before_first_table(after: &str, original: &str, line: &str) {
    let line_start = after
        .find(&format!("{line}\n"))
        .expect("rename line exists");
    let first_table = after.find("[[mods]]").expect("a [[mods]] table exists");
    assert!(line_start < first_table, "{after}");
    assert_eq!(after.replacen(&format!("{line}\n"), "", 1), original);
}

#[test]
fn a_rename_shows_in_every_label_and_never_in_sources() {
    let _fixture = Fixture::new(FIXTURE_CATALOG);
    let sources_before = source_names("biorenamefixture");
    assert_eq!(sources_before.len(), 2);
    assert_eq!(
        catalog_human_name("setup-biorenamefixture.tp2").as_deref(),
        Some("Fixture Catalog Name")
    );

    set_user_mod_name("setup-biorenamefixture.tp2", "My Fixture").unwrap();

    assert_eq!(
        catalog_human_name("BIORENAMEFIXTURE/SETUP-BIORENAMEFIXTURE.TP2").as_deref(),
        Some("My Fixture")
    );
    assert_eq!(
        mod_display_label("setup-biorenamefixture.tp2"),
        "My Fixture"
    );
    assert_eq!(
        mod_display_label_with_name(Some("Fixture Catalog Name"), "biorenamefixture"),
        "My Fixture"
    );
    assert_eq!(source_names("biorenamefixture"), sources_before);

    set_display_mod_names(false);
    assert_eq!(
        mod_display_label("setup-biorenamefixture.tp2"),
        "biorenamefixture"
    );
}

#[test]
fn set_user_mod_name_writes_a_root_key_then_updates_then_clears() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    let default_folder = crate::platform_defaults::app_config_dir()
        .unwrap()
        .join(crate::platform_defaults::CONFIG_DEFAULT_FOLDER);

    set_user_mod_name("setup-cdtweaks.tp2", "  My Tweaks  ").unwrap();
    assert_line_added_before_first_table(
        &user_text(),
        CUSTOM_SOURCE,
        "names.cdtweaks = \"My Tweaks\"",
    );
    assert_eq!(user_mod_renames(), renames(&[("cdtweaks", "My Tweaks")]));

    set_user_mod_name("cdtweaks", "Better Tweaks").unwrap();
    assert_line_added_before_first_table(
        &user_text(),
        CUSTOM_SOURCE,
        "names.cdtweaks = \"Better Tweaks\"",
    );
    assert_eq!(
        user_mod_renames(),
        renames(&[("cdtweaks", "Better Tweaks")])
    );

    set_user_mod_name("cdtweaks", "   ").unwrap();
    assert_eq!(user_text(), CUSTOM_SOURCE);
    assert!(user_mod_renames().is_empty());
    assert!(!default_folder.exists());
}

#[test]
fn a_rename_survives_saving_and_deleting_a_source() {
    let _fixture = Fixture::new(CUSTOM_SOURCE);
    set_user_mod_name("cdtweaks", "My Tweaks").unwrap();
    let expected = renames(&[("cdtweaks", "My Tweaks")]);
    let line = "names.cdtweaks = \"My Tweaks\"\n";

    save_user_mod_download_source_block(
        "cdtweaks",
        "Custom Tweaks Source",
        "custom",
        false,
        "  [[mods.sources]]\n  id = \"custom\"\n  type = \"url\"\n  url = \"https://example.test/edited.zip\"",
        None,
    )
    .unwrap();
    let saved = user_text();
    assert!(saved.contains("https://example.test/edited.zip"), "{saved}");
    assert_eq!(saved.matches(line).count(), 1, "{saved}");
    assert_eq!(user_mod_renames(), expected);

    save_user_mod_download_source_block(
        "cdtweaks",
        "Custom Tweaks Source",
        "custom",
        false,
        "",
        None,
    )
    .unwrap();
    let removed = user_text();
    assert!(!removed.contains("example.test/edited.zip"), "{removed}");
    assert_eq!(removed.matches(line).count(), 1, "{removed}");
    assert_eq!(user_mod_renames(), expected);
}

#[test]
fn an_alias_renames_the_catalog_entry() {
    let _fixture = Fixture::new(
        "format = 2\n\n[[mods]]\nname = \"Folder Named\"\ntp2 = \"biofilename\"\naliases = [\"biofoldername\"]\n\n  [[mods.sources]]\n  id = \"primary\"\n  type = \"url\"\n  url = \"https://example.test/biofoldername.zip\"\n",
    );

    set_user_mod_name("BioFolderName/setup-biofoldername.tp2", "Renamed Mod").unwrap();

    assert_eq!(
        user_mod_renames(),
        renames(&[("biofilename", "Renamed Mod")])
    );
    assert_eq!(mod_display_label("setup-biofilename.tp2"), "Renamed Mod");
    assert_eq!(mod_display_label("biofoldername"), "Renamed Mod");
}

#[test]
fn a_hand_written_names_table_moves_above_the_mods_on_rename() {
    let original = format!(
        "{CUSTOM_SOURCE}
[names]
other = \"Other Mod\"
"
    );
    let _fixture = Fixture::new(&original);
    set_user_mod_name("cdtweaks", "My Tweaks").unwrap();
    let text = user_text();
    let first_mods = text.find("[[mods]]").unwrap();
    assert!(!text.contains("[names]"));
    assert!(text.find("names.other").unwrap() < first_mods);
    assert!(text.find("names.cdtweaks").unwrap() < first_mods);
    assert_eq!(
        user_mod_renames(),
        renames(&[("cdtweaks", "My Tweaks"), ("other", "Other Mod")])
    );
}

#[test]
fn an_inline_names_table_is_accepted() {
    let _fixture = Fixture::new(
        "format = 2
names = { other = \"Other Mod\" }
",
    );
    set_user_mod_name("cdtweaks", "My Tweaks").unwrap();
    assert_eq!(
        user_mod_renames(),
        renames(&[("cdtweaks", "My Tweaks"), ("other", "Other Mod")])
    );
}
