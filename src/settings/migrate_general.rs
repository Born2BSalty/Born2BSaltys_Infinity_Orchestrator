// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use tracing::warn;

use crate::settings::model::AppSettings;
use crate::settings::redesign_fields::RedesignSettings;
use crate::settings::store::SettingsStore;

pub const LEGACY_GENERAL_FILE_NAME: &str = "bio_redesign_settings.json";

#[derive(Debug)]
pub enum LegacyGeneralOutcome {
    NoLegacyFile,
    Merged,
    LegacyUnreadable(Option<PathBuf>),
    MergedFileUnreadable(PathBuf),
    SaveFailed(anyhow::Error),
}

#[must_use]
pub fn fold_legacy_general_settings(store: &SettingsStore) -> LegacyGeneralOutcome {
    let legacy = store.path().with_file_name(LEGACY_GENERAL_FILE_NAME);
    if !legacy.is_file() {
        return LegacyGeneralOutcome::NoLegacyFile;
    }

    let Ok(legacy_general) = load_legacy(&legacy) else {
        let backup = backup_legacy_file(&legacy);
        return LegacyGeneralOutcome::LegacyUnreadable(backup);
    };

    let mut main_backup_path = None;
    let mut settings = match store.load() {
        Ok(settings) => settings,
        Err(_) if store.path().is_file() => {
            match store.backup_corrupt_file() {
                Ok(backup) => main_backup_path = Some(backup),
                Err(err) => warn!(
                    target = "orchestrator",
                    "failed backing up unreadable settings file {} before the merge: {err}",
                    store.path().display()
                ),
            }
            AppSettings::default()
        }
        Err(_) => AppSettings::default(),
    };
    settings.general = legacy_general;

    match store.save(&settings) {
        Ok(()) => {
            if let Err(err) = std::fs::remove_file(&legacy) {
                warn!(
                    target = "orchestrator",
                    "failed removing legacy general settings file {}: {err}",
                    legacy.display()
                );
            }
            main_backup_path.map_or(LegacyGeneralOutcome::Merged, |backup| {
                LegacyGeneralOutcome::MergedFileUnreadable(backup)
            })
        }
        Err(err) => LegacyGeneralOutcome::SaveFailed(err),
    }
}

fn load_legacy(legacy: &std::path::Path) -> Result<RedesignSettings, ()> {
    let raw = std::fs::read_to_string(legacy).map_err(|_| ())?;
    serde_json::from_str::<RedesignSettings>(&raw).map_err(|_| ())
}

fn backup_legacy_file(legacy: &std::path::Path) -> Option<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let new_path = legacy.with_extension(format!("json.corrupt-{ts}"));
    match std::fs::rename(legacy, &new_path) {
        Ok(()) => Some(new_path),
        Err(err) => {
            warn!(
                target = "orchestrator",
                "failed backing up unreadable legacy general settings file {}: {err}",
                legacy.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static C: AtomicU64 = AtomicU64::new(0);

    fn temp_paths(label: &str) -> (PathBuf, PathBuf) {
        let n = C.fetch_add(1, Ordering::Relaxed);
        let stem = format!("bio_migrate_test_{}_{}_{}", std::process::id(), n, label);
        let sub_dir = std::env::temp_dir().join(stem);
        let _ = std::fs::remove_dir_all(&sub_dir);
        std::fs::create_dir_all(&sub_dir).expect("create test dir");
        let main = sub_dir.join("bio_settings.json");
        let legacy = sub_dir.join(LEGACY_GENERAL_FILE_NAME);
        (main, legacy)
    }

    fn cleanup(main: &std::path::Path, legacy: &std::path::Path) {
        let _ = std::fs::remove_file(main);
        let _ = std::fs::remove_file(legacy);
        if let Some(parent) = main.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn no_legacy_file_touches_nothing() {
        let (main, legacy) = temp_paths("no_legacy");
        let store = SettingsStore::new_with_path(&main);
        let outcome = fold_legacy_general_settings(&store);
        assert!(matches!(outcome, LegacyGeneralOutcome::NoLegacyFile));
        assert!(!main.exists());
        cleanup(&main, &legacy);
    }

    #[test]
    fn legacy_values_land_in_general_and_the_legacy_file_is_removed() {
        let (main, legacy) = temp_paths("fold");
        let store = SettingsStore::new_with_path(&main);
        let settings = AppSettings {
            step1: crate::settings::model::Step1Settings {
                mods_folder: "M".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        store.save(&settings).expect("save main");
        let legacy_settings = RedesignSettings {
            user_name: "@old".to_string(),
            theme_palette: crate::settings::redesign_fields::ThemeChoice::Light,
            ..RedesignSettings::default()
        };
        std::fs::write(
            &legacy,
            serde_json::to_string(&legacy_settings).expect("serialize legacy"),
        )
        .expect("write legacy");

        let outcome = fold_legacy_general_settings(&store);
        assert!(matches!(outcome, LegacyGeneralOutcome::Merged));

        let reloaded = store.load().expect("reload main");
        assert_eq!(reloaded.general.user_name, "@old");
        assert_eq!(
            reloaded.general.theme_palette,
            crate::settings::redesign_fields::ThemeChoice::Light
        );
        assert_eq!(reloaded.step1.mods_folder, "M");
        assert!(!legacy.exists());
        cleanup(&main, &legacy);
    }

    #[test]
    fn legacy_wins_over_an_existing_general_block() {
        let (main, legacy) = temp_paths("wins");
        let store = SettingsStore::new_with_path(&main);
        let settings = AppSettings {
            general: RedesignSettings {
                user_name: "@new".to_string(),
                ..RedesignSettings::default()
            },
            ..Default::default()
        };
        store.save(&settings).expect("save main");
        let legacy_settings = RedesignSettings {
            user_name: "@old".to_string(),
            ..RedesignSettings::default()
        };
        std::fs::write(
            &legacy,
            serde_json::to_string(&legacy_settings).expect("serialize legacy"),
        )
        .expect("write legacy");

        let outcome = fold_legacy_general_settings(&store);
        assert!(matches!(outcome, LegacyGeneralOutcome::Merged));
        let reloaded = store.load().expect("reload main");
        assert_eq!(reloaded.general.user_name, "@old");
        cleanup(&main, &legacy);
    }

    #[test]
    fn missing_main_file_is_created_from_defaults_plus_legacy() {
        let (main, legacy) = temp_paths("missing_main");
        let store = SettingsStore::new_with_path(&main);
        let legacy_settings = RedesignSettings {
            user_name: "@old".to_string(),
            ..RedesignSettings::default()
        };
        std::fs::write(
            &legacy,
            serde_json::to_string(&legacy_settings).expect("serialize legacy"),
        )
        .expect("write legacy");

        let outcome = fold_legacy_general_settings(&store);
        assert!(matches!(outcome, LegacyGeneralOutcome::Merged));
        let reloaded = store.load().expect("reload main");
        assert_eq!(reloaded.general.user_name, "@old");
        assert_eq!(
            reloaded.step1,
            crate::settings::model::Step1Settings::default()
        );
        cleanup(&main, &legacy);
    }

    #[test]
    fn unreadable_legacy_is_backed_up_and_main_is_untouched() {
        let (main, legacy) = temp_paths("unreadable_legacy");
        let store = SettingsStore::new_with_path(&main);
        let settings = AppSettings::default();
        store.save(&settings).expect("save main");
        let main_bytes_before = std::fs::read(&main).expect("read main before");
        std::fs::write(&legacy, b"{not json").expect("write garbage legacy");

        let outcome = fold_legacy_general_settings(&store);
        let backup = match outcome {
            LegacyGeneralOutcome::LegacyUnreadable(Some(p)) => p,
            other => panic!("expected LegacyUnreadable with a backup path, got {other:?}"),
        };
        let name = backup
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(name.contains("corrupt-"));
        let main_bytes_after = std::fs::read(&main).expect("read main after");
        assert_eq!(main_bytes_before, main_bytes_after);
        cleanup(&main, &legacy);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn unreadable_main_is_backed_up_before_the_merge() {
        let (main, legacy) = temp_paths("unreadable_main");
        std::fs::write(&main, b"{not json").expect("write garbage main");
        let store = SettingsStore::new_with_path(&main);
        let legacy_settings = RedesignSettings {
            user_name: "@old".to_string(),
            ..RedesignSettings::default()
        };
        std::fs::write(
            &legacy,
            serde_json::to_string(&legacy_settings).expect("serialize legacy"),
        )
        .expect("write legacy");

        let outcome = fold_legacy_general_settings(&store);
        let backup = match outcome {
            LegacyGeneralOutcome::MergedFileUnreadable(p) => p,
            other => panic!("expected MergedFileUnreadable, got {other:?}"),
        };
        assert!(backup.exists());
        let reloaded = store.load().expect("reload main");
        assert_eq!(reloaded.general.user_name, "@old");
        assert_eq!(
            reloaded.step1,
            crate::settings::model::Step1Settings::default()
        );
        cleanup(&main, &legacy);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn save_failure_leaves_the_legacy_file_in_place() {
        let (main, legacy) = temp_paths("save_failure");
        std::fs::create_dir_all(&main).expect("make main a directory");
        let store = SettingsStore::new_with_path(&main);
        let legacy_settings = RedesignSettings::default();
        std::fs::write(
            &legacy,
            serde_json::to_string(&legacy_settings).expect("serialize legacy"),
        )
        .expect("write legacy");

        let outcome = fold_legacy_general_settings(&store);
        assert!(matches!(outcome, LegacyGeneralOutcome::SaveFailed(_)));
        assert!(legacy.exists());
        let _ = std::fs::remove_dir_all(&main);
        cleanup(&main, &legacy);
    }
}
