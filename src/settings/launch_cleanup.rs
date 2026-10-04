// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::Result;

use crate::settings::store::SettingsStore;

pub fn clear_unreachable_eet_sources_in_file(store: &SettingsStore) -> Result<bool> {
    if !store.path().is_file() {
        return Ok(false);
    }
    let mut settings = store.load()?;
    if settings.step1.clear_unreachable_eet_sources().is_none() {
        return Ok(false);
    }
    store.save(&settings)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::AppSettings;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = C.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "bio_launch_cleanup_test_{}_{}_{}",
            std::process::id(),
            n,
            label
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn settings_with_stale_sources() -> AppSettings {
        let mut settings = AppSettings::default();
        settings.step1.new_pre_eet_dir_enabled = true;
        settings.step1.new_eet_dir_enabled = true;
        settings.step1.bgee_game_folder = "C:\\Games\\BGEE".to_string();
        settings.step1.bg2ee_game_folder = "C:\\Games\\BG2EE".to_string();
        settings.step1.eet_bgee_game_folder = "C:\\Games\\OldEetBgee".to_string();
        settings.step1.eet_bg2ee_game_folder = "C:\\Games\\OldEetBg2ee".to_string();
        settings.step1.mods_folder = "C:\\Games\\mods".to_string();
        settings
    }

    #[test]
    fn missing_file_is_left_alone() {
        let dir = temp_dir("missing");
        let store = SettingsStore::new_with_path(dir.join("bio_settings.json"));
        assert!(!clear_unreachable_eet_sources_in_file(&store).expect("ok"));
        assert!(!store.path().exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_sources_are_cleared_and_everything_else_is_kept() {
        let dir = temp_dir("stale");
        let store = SettingsStore::new_with_path(dir.join("bio_settings.json"));
        store
            .save(&settings_with_stale_sources())
            .expect("seed file");
        assert!(clear_unreachable_eet_sources_in_file(&store).expect("ok"));
        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded.step1.eet_bgee_game_folder, "");
        assert_eq!(reloaded.step1.eet_bg2ee_game_folder, "");
        assert_eq!(reloaded.step1.bgee_game_folder, "C:\\Games\\BGEE");
        assert_eq!(reloaded.step1.bg2ee_game_folder, "C:\\Games\\BG2EE");
        assert_eq!(reloaded.step1.mods_folder, "C:\\Games\\mods");
        assert!(reloaded.step1.new_pre_eet_dir_enabled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_clean_file_is_not_rewritten() {
        let dir = temp_dir("clean");
        let store = SettingsStore::new_with_path(dir.join("bio_settings.json"));
        let mut settings = settings_with_stale_sources();
        settings.step1.eet_bgee_game_folder.clear();
        settings.step1.eet_bg2ee_game_folder.clear();
        store.save(&settings).expect("seed file");
        let before = std::fs::read(store.path()).expect("read before");
        assert!(!clear_unreachable_eet_sources_in_file(&store).expect("ok"));
        let after = std::fs::read(store.path()).expect("read after");
        assert_eq!(before, after);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_reachable_source_is_kept() {
        let dir = temp_dir("reachable");
        let store = SettingsStore::new_with_path(dir.join("bio_settings.json"));
        let mut settings = settings_with_stale_sources();
        settings.step1.new_eet_dir_enabled = false;
        store.save(&settings).expect("seed file");
        assert!(clear_unreachable_eet_sources_in_file(&store).expect("ok"));
        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded.step1.eet_bgee_game_folder, "");
        assert_eq!(
            reloaded.step1.eet_bg2ee_game_folder,
            "C:\\Games\\OldEetBg2ee"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
