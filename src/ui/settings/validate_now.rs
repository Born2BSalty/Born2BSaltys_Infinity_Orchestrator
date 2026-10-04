// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::source_check::{self, SourceGame};
use crate::app::state::Step1State;
use crate::app::state_validation;
use crate::ui::settings::state_settings::{PathStatus, ValidationReport};

pub const FIELD_BGEE_GAME_FOLDER: &str = "bgee_game_folder";
pub const FIELD_BG2EE_GAME_FOLDER: &str = "bg2ee_game_folder";
pub const FIELD_IWDEE_GAME_FOLDER: &str = "iwdee_game_folder";
pub const FIELD_EET_BGEE_GAME_FOLDER: &str = "eet_bgee_game_folder";
pub const FIELD_EET_BG2EE_GAME_FOLDER: &str = "eet_bg2ee_game_folder";
pub const FIELD_GLOBAL_MODS_FOLDER: &str = "global_mods_folder";
pub const FIELD_MODS_ARCHIVE_FOLDER: &str = "mods_archive_folder";
pub const FIELD_MODS_BACKUP_FOLDER: &str = "mods_backup_folder";
pub const FIELD_WEIDU_LOG_FOLDER: &str = "weidu_log_folder";
pub const FIELD_WEIDU_BINARY: &str = "weidu_binary";
pub const FIELD_MOD_INSTALLER_BINARY: &str = "mod_installer_binary";

pub const GAME_FOLDER_FIELDS: [&str; 5] = [
    FIELD_BGEE_GAME_FOLDER,
    FIELD_EET_BGEE_GAME_FOLDER,
    FIELD_BG2EE_GAME_FOLDER,
    FIELD_EET_BG2EE_GAME_FOLDER,
    FIELD_IWDEE_GAME_FOLDER,
];

#[must_use]
pub fn is_game_folder_field(field: &str) -> bool {
    GAME_FOLDER_FIELDS.contains(&field)
}

#[derive(Debug, Clone, Copy)]
enum FieldRole {
    Game,
    Working,
    Binary,
}

fn field_role(field: &str) -> FieldRole {
    match field {
        FIELD_BGEE_GAME_FOLDER
        | FIELD_BG2EE_GAME_FOLDER
        | FIELD_IWDEE_GAME_FOLDER
        | FIELD_EET_BGEE_GAME_FOLDER
        | FIELD_EET_BG2EE_GAME_FOLDER => FieldRole::Game,
        FIELD_WEIDU_BINARY | FIELD_MOD_INSTALLER_BINARY => FieldRole::Binary,
        _ => FieldRole::Working,
    }
}

#[must_use]
pub fn run_now(step1: &Step1State) -> ValidationReport {
    let mut report = ValidationReport::default();

    let folder_fields: [(&'static str, &str); 6] = [
        (FIELD_BGEE_GAME_FOLDER, &step1.bgee_game_folder),
        (FIELD_BG2EE_GAME_FOLDER, &step1.bg2ee_game_folder),
        (FIELD_IWDEE_GAME_FOLDER, &step1.iwdee_game_folder),
        (FIELD_GLOBAL_MODS_FOLDER, &step1.global_mods_folder),
        (FIELD_MODS_ARCHIVE_FOLDER, &step1.mods_archive_folder),
        (FIELD_MODS_BACKUP_FOLDER, &step1.mods_backup_folder),
    ];
    for (name, value) in &folder_fields {
        report.fields.insert(*name, check_path(name, value));
    }

    report.fields.insert(
        FIELD_WEIDU_BINARY,
        check_path(FIELD_WEIDU_BINARY, &step1.weidu_binary),
    );
    report.fields.insert(
        FIELD_MOD_INSTALLER_BINARY,
        check_path(FIELD_MOD_INSTALLER_BINARY, &step1.mod_installer_binary),
    );

    report.overall_ok = state_validation::settings_paths_ok(step1);
    report.issue_count = report
        .fields
        .values()
        .filter(|status| {
            matches!(
                status,
                PathStatus::Warning { .. } | PathStatus::Error { .. }
            )
        })
        .count();

    report
}

#[must_use]
pub fn run_for_field(step1: &Step1State, field: &'static str) -> PathStatus {
    let value = match field {
        FIELD_BGEE_GAME_FOLDER => &step1.bgee_game_folder,
        FIELD_BG2EE_GAME_FOLDER => &step1.bg2ee_game_folder,
        FIELD_IWDEE_GAME_FOLDER => &step1.iwdee_game_folder,
        FIELD_EET_BGEE_GAME_FOLDER => &step1.eet_bgee_game_folder,
        FIELD_EET_BG2EE_GAME_FOLDER => &step1.eet_bg2ee_game_folder,
        FIELD_GLOBAL_MODS_FOLDER => &step1.global_mods_folder,
        FIELD_MODS_ARCHIVE_FOLDER => &step1.mods_archive_folder,
        FIELD_MODS_BACKUP_FOLDER => &step1.mods_backup_folder,
        FIELD_WEIDU_LOG_FOLDER => &step1.weidu_log_folder,
        FIELD_WEIDU_BINARY => &step1.weidu_binary,
        FIELD_MOD_INSTALLER_BINARY => &step1.mod_installer_binary,
        _ => return PathStatus::Empty,
    };
    check_path(field, value)
}

fn check_path(field: &str, value: &str) -> PathStatus {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return PathStatus::Empty;
    }
    match field_role(field) {
        FieldRole::Game => check_game_folder(trimmed, field),
        FieldRole::Working => check_working_folder(trimmed),
        FieldRole::Binary => check_binary(trimmed),
    }
}

fn check_game_folder(value: &str, field: &str) -> PathStatus {
    let path = Path::new(value);
    if !path.exists() {
        return PathStatus::Error {
            reason: "path does not exist".to_string(),
        };
    }
    if !path.is_dir() {
        return PathStatus::Error {
            reason: "not a folder".to_string(),
        };
    }
    let has_chitin = path.join("chitin.key").is_file();
    let has_lang = path.join("lang").is_dir();
    if !has_chitin || !has_lang {
        return PathStatus::Warning {
            reason: "no chitin.key/lang \u{2014} not a recognizable game install".to_string(),
        };
    }
    let game = match field {
        FIELD_BGEE_GAME_FOLDER | FIELD_EET_BGEE_GAME_FOLDER => SourceGame::Bgee,
        FIELD_BG2EE_GAME_FOLDER | FIELD_EET_BG2EE_GAME_FOLDER => SourceGame::Bg2ee,
        _ => SourceGame::Iwdee,
    };
    let report = source_check::inspect(path, game);
    let sod = report.sod.describe();
    if report.residue.is_clean() {
        PathStatus::Ok {
            detail: Some(if sod.is_empty() {
                "clean install".to_string()
            } else {
                format!("clean install \u{00B7} {sod}")
            }),
        }
    } else {
        let residue = report.residue.describe();
        PathStatus::Modded {
            detail: if sod.is_empty() {
                residue
            } else {
                format!("{residue} \u{00B7} {sod}")
            },
        }
    }
}

fn check_working_folder(value: &str) -> PathStatus {
    let path = Path::new(value);
    if !path.exists() {
        return PathStatus::Warning {
            reason: "will be created on first install".to_string(),
        };
    }
    if !path.is_dir() {
        return PathStatus::Error {
            reason: "not a folder".to_string(),
        };
    }
    if path.join("chitin.key").is_file() {
        return PathStatus::Warning {
            reason: "looks like a game install \u{2014} pick an empty folder".to_string(),
        };
    }
    PathStatus::Ok { detail: None }
}

fn check_binary(value: &str) -> PathStatus {
    let path = Path::new(value);
    if path.is_absolute() {
        if path.is_file() {
            PathStatus::Ok { detail: None }
        } else {
            PathStatus::Error {
                reason: "binary not found at that path".to_string(),
            }
        }
    } else {
        resolve_on_path(value).map_or_else(
            || PathStatus::Error {
                reason: "not on $PATH — install or specify the full path".to_string(),
            },
            |resolved| PathStatus::Ok {
                detail: Some(resolved.display().to_string()),
            },
        )
    }
}

#[must_use]
pub fn resolve_on_path(name: &str) -> Option<std::path::PathBuf> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let direct = dir.join(trimmed);
        if direct.is_file() {
            return Some(direct);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{trimmed}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{FIELD_BGEE_GAME_FOLDER, PathStatus, check_path, run_now};
    use crate::app::state::Step1State;

    struct TempFixture {
        path: std::path::PathBuf,
    }

    impl TempFixture {
        fn new(name: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_source_check_test_validate_now_{name}_{}_{id}",
                std::process::id()
            ));
            std::fs::create_dir_all(path.join("lang")).unwrap();
            std::fs::write(path.join("chitin.key"), b"clean key data").unwrap();
            Self { path }
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn modded_bgee_folder_reports_modded_status() {
        let fixture = TempFixture::new("modded");
        std::fs::write(fixture.path.join("WeiDU.log"), b"log").unwrap();
        let status = check_path(FIELD_BGEE_GAME_FOLDER, &fixture.path.display().to_string());
        assert!(matches!(status, PathStatus::Modded { .. }));
    }

    #[test]
    fn clean_bgee_folder_reports_ok_with_sod_absent() {
        let fixture = TempFixture::new("clean");
        let status = check_path(FIELD_BGEE_GAME_FOLDER, &fixture.path.display().to_string());
        assert_eq!(
            status,
            PathStatus::Ok {
                detail: Some("clean install \u{00B7} SoD not found".to_string())
            }
        );
    }

    #[test]
    fn hidden_eet_fields_never_count_as_issues() {
        let missing = std::env::temp_dir().join("bio_validate_now_test_missing_eet_bg2ee");
        let step1 = Step1State {
            eet_bg2ee_game_folder: missing.display().to_string(),
            weidu_binary: String::new(),
            mod_installer_binary: String::new(),
            ..Step1State::default()
        };
        let report = run_now(&step1);
        assert_eq!(report.issue_count, 0);
        assert!(
            !report
                .fields
                .contains_key(super::FIELD_EET_BGEE_GAME_FOLDER)
        );
        assert!(
            !report
                .fields
                .contains_key(super::FIELD_EET_BG2EE_GAME_FOLDER)
        );
    }
}
