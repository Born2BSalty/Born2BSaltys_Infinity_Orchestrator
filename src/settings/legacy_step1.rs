// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use serde_json::{Map, Value};

const DROPPED_STEP1_KEYS: [&str; 29] = [
    "game_install",
    "install_mode",
    "have_weidu_logs",
    "weidu_log_mode_enabled",
    "new_pre_eet_dir_enabled",
    "new_eet_dir_enabled",
    "generate_directory_enabled",
    "prepare_target_dirs_before_install",
    "backup_targets_before_eet_copy",
    "weidu_log_autolog",
    "weidu_log_logapp",
    "weidu_log_logextern",
    "weidu_log_log_component",
    "weidu_log_folder",
    "weidu_log_mode",
    "bgee_log_folder",
    "bgee_log_file",
    "bg2ee_log_folder",
    "bg2ee_log_file",
    "eet_bgee_log_folder",
    "eet_bg2ee_log_folder",
    "eet_pre_dir",
    "eet_new_dir",
    "game",
    "log_file",
    "generate_directory",
    "mods_folder",
    "eet_bgee_game_folder",
    "eet_bg2ee_game_folder",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyStep1Outcome {
    NoChange,
    Migrated {
        moved_mods_folder: Option<String>,
        stripped_keys: usize,
    },
}

pub fn migrate_legacy_step1_in_file(path: &Path) -> Result<LegacyStep1Outcome, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LegacyStep1Outcome::NoChange);
        }
        Err(err) => return Err(format!("failed reading {}: {err}", path.display())),
    };
    let Ok(mut root) = serde_json::from_str::<Value>(&raw) else {
        return Ok(LegacyStep1Outcome::NoChange);
    };
    let Some(step1) = root.get_mut("step1").and_then(Value::as_object_mut) else {
        return Ok(LegacyStep1Outcome::NoChange);
    };
    let moved_mods_folder = move_legacy_mods_folder(step1);
    let stripped_keys = strip_dropped_keys(step1);
    if moved_mods_folder.is_none() && stripped_keys == 0 {
        return Ok(LegacyStep1Outcome::NoChange);
    }
    write_atomically(path, &root)?;
    Ok(LegacyStep1Outcome::Migrated {
        moved_mods_folder,
        stripped_keys,
    })
}

fn non_blank_string(step1: &Map<String, Value>, key: &str) -> Option<String> {
    step1
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn move_legacy_mods_folder(step1: &mut Map<String, Value>) -> Option<String> {
    if non_blank_string(step1, "global_mods_folder").is_some() {
        return None;
    }
    let legacy = non_blank_string(step1, "mods_folder")?;
    step1.insert(
        "global_mods_folder".to_string(),
        Value::String(legacy.clone()),
    );
    Some(legacy)
}

fn strip_dropped_keys(step1: &mut Map<String, Value>) -> usize {
    DROPPED_STEP1_KEYS
        .iter()
        .filter(|key| step1.remove(**key).is_some())
        .count()
}

fn write_atomically(path: &Path, root: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(root)
        .map_err(|err| format!("failed serializing {}: {err}", path.display()))?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, text.as_bytes())
        .map_err(|err| format!("failed writing {}: {err}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "failed renaming {} to {}: {err}",
            tmp_path.display(),
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let root = Self(std::env::temp_dir().join(format!(
                "bio_legacy_step1_{}_{}_{label}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            )));
            std::fs::create_dir_all(&root.0).expect("create temp root");
            root
        }

        fn settings_file(&self) -> PathBuf {
            self.0.join("bio_settings.json")
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_json(path: &Path, value: &Value) {
        std::fs::write(
            path,
            serde_json::to_string(value).expect("serialize fixture"),
        )
        .expect("write fixture");
    }

    fn read_step1(path: &Path) -> Map<String, Value> {
        let raw = std::fs::read_to_string(path).expect("read back");
        let root: Value = serde_json::from_str(&raw).expect("parse back");
        root.get("step1")
            .and_then(Value::as_object)
            .cloned()
            .expect("step1 object")
    }

    #[test]
    fn moves_the_mods_folder_when_global_is_blank() {
        for global in [None, Some("   ")] {
            let root = TempRoot::new("move");
            let path = root.settings_file();
            let mut step1 = serde_json::json!({
                "mods_folder": "D:\\Old\\Mods",
                "bgee_game_folder": "D:\\Games\\BGEE"
            });
            if let Some(global) = global {
                step1["global_mods_folder"] = Value::String(global.to_string());
            }
            write_json(
                &path,
                &serde_json::json!({ "exe_fingerprint": "x", "step1": step1 }),
            );

            let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

            assert_eq!(
                outcome,
                LegacyStep1Outcome::Migrated {
                    moved_mods_folder: Some("D:\\Old\\Mods".to_string()),
                    stripped_keys: 1,
                }
            );
            let step1 = read_step1(&path);
            assert_eq!(step1["global_mods_folder"], "D:\\Old\\Mods");
            assert_eq!(step1["bgee_game_folder"], "D:\\Games\\BGEE");
            assert!(!step1.contains_key("mods_folder"));
        }
    }

    #[test]
    fn leaves_a_set_global_folder_alone() {
        let root = TempRoot::new("keep_global");
        let path = root.settings_file();
        write_json(
            &path,
            &serde_json::json!({
                "step1": {
                    "mods_folder": "D:\\Old\\Mods",
                    "global_mods_folder": "E:\\Global"
                }
            }),
        );

        let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

        assert_eq!(
            outcome,
            LegacyStep1Outcome::Migrated {
                moved_mods_folder: None,
                stripped_keys: 1,
            }
        );
        let step1 = read_step1(&path);
        assert_eq!(step1["global_mods_folder"], "E:\\Global");
        assert!(!step1.contains_key("mods_folder"));
    }

    #[test]
    fn strips_every_per_list_key_and_keeps_the_globals() {
        let root = TempRoot::new("strip");
        let path = root.settings_file();
        let mut step1 = Map::new();
        for key in DROPPED_STEP1_KEYS {
            step1.insert(key.to_string(), Value::String("stale".to_string()));
        }
        step1.insert(
            "global_mods_folder".to_string(),
            Value::String("E:\\Global".to_string()),
        );
        step1.insert(
            "weidu_binary".to_string(),
            Value::String("C:\\weidu.exe".to_string()),
        );
        step1.insert("depth".to_string(), Value::from(7));
        write_json(
            &path,
            &serde_json::json!({ "exe_fingerprint": "x", "step1": step1, "general": {} }),
        );

        let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

        assert_eq!(
            outcome,
            LegacyStep1Outcome::Migrated {
                moved_mods_folder: None,
                stripped_keys: DROPPED_STEP1_KEYS.len(),
            }
        );
        let step1 = read_step1(&path);
        for key in DROPPED_STEP1_KEYS {
            assert!(!step1.contains_key(key), "{key} must be stripped");
        }
        assert!(!step1.contains_key("eet_bgee_game_folder"));
        assert!(!step1.contains_key("eet_bg2ee_game_folder"));
        assert_eq!(step1["global_mods_folder"], "E:\\Global");
        assert_eq!(step1["weidu_binary"], "C:\\weidu.exe");
        assert_eq!(step1["depth"], 7);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn a_clean_file_is_not_rewritten() {
        let root = TempRoot::new("clean");
        let path = root.settings_file();
        write_json(
            &path,
            &serde_json::json!({
                "exe_fingerprint": "x",
                "step1": { "global_mods_folder": "", "bgee_game_folder": "D:\\Games\\BGEE" }
            }),
        );
        let before = std::fs::read(&path).expect("read before");

        let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

        assert_eq!(outcome, LegacyStep1Outcome::NoChange);
        assert_eq!(std::fs::read(&path).expect("read after"), before);
    }

    #[test]
    fn non_object_or_unparseable_json_is_left_untouched() {
        for text in [
            "[1, 2, 3]",
            "\"just a string\"",
            "{\"step1\": [\"mods_folder\"]}",
            "{ not json",
        ] {
            let root = TempRoot::new("untouched");
            let path = root.settings_file();
            std::fs::write(&path, text).expect("write fixture");

            let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

            assert_eq!(outcome, LegacyStep1Outcome::NoChange, "{text}");
            assert_eq!(
                std::fs::read_to_string(&path).expect("read after"),
                text,
                "{text}"
            );
        }
    }

    #[test]
    fn a_missing_file_is_not_created() {
        let root = TempRoot::new("missing");
        let path = root.settings_file();

        let outcome = migrate_legacy_step1_in_file(&path).expect("migrate");

        assert_eq!(outcome, LegacyStep1Outcome::NoChange);
        assert!(!path.exists());
    }
}
