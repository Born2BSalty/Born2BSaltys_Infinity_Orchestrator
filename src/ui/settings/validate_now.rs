// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `validate_now` — synchronous role-aware path validation for the Settings
// → Paths sub-tab + Phase 5's Home page on-startup validation.
//
// Per SPEC §11.2 (updated):
//   - Game folders (BGEE/BG2EE/IWDEE bindings) must contain `chitin.key`
//     and a `lang/` subfolder — the Infinity Engine install marker.
//   - Working folders (Mods archive/Mods backup/Temp) must NOT look like
//     game installs (no `chitin.key`); empty is fine (auto-created on first
//     install); existing-but-file is an error.
//
// **No BIO source modifications.** BIO's `state_validation_fs::check_game_dir`
// is `pub(super)`, so the chitin.key + lang/ check is re-implemented here
// (~5 lines) rather than escalated to a carve-out. Read-only use of public
// `bio::app::state_validation::is_step1_valid` for the aggregate ok gate.

use std::path::Path;

use crate::app::state::Step1State;
use crate::app::state_validation;
use crate::ui::settings::state_settings::{PathStatus, ValidationReport};

// ── Field name constants (keys into `ValidationReport::fields`) ─────────────

pub const FIELD_BGEE_GAME_FOLDER: &str = "bgee_game_folder";
pub const FIELD_BG2EE_GAME_FOLDER: &str = "bg2ee_game_folder";
pub const FIELD_IWDEE_GAME_FOLDER: &str = "iwdee_game_folder";
pub const FIELD_EET_BGEE_GAME_FOLDER: &str = "eet_bgee_game_folder";
pub const FIELD_EET_BG2EE_GAME_FOLDER: &str = "eet_bg2ee_game_folder";
pub const FIELD_MODS_FOLDER: &str = "mods_folder";
pub const FIELD_MODS_ARCHIVE_FOLDER: &str = "mods_archive_folder";
pub const FIELD_MODS_BACKUP_FOLDER: &str = "mods_backup_folder";
pub const FIELD_WEIDU_LOG_FOLDER: &str = "weidu_log_folder";
pub const FIELD_WEIDU_BINARY: &str = "weidu_binary";
pub const FIELD_MOD_INSTALLER_BINARY: &str = "mod_installer_binary";

// ── Field roles ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum FieldRole {
    /// Game install folder — must contain `chitin.key` + `lang/`.
    Game,
    /// Working folder (archive/backup/temp) — should NOT look like a game
    /// install; empty is fine (auto-created).
    Working,
    /// Binary on disk or on PATH.
    Binary,
}

fn field_role(field: &str) -> FieldRole {
    match field {
        FIELD_BGEE_GAME_FOLDER
        | FIELD_BG2EE_GAME_FOLDER
        | FIELD_IWDEE_GAME_FOLDER
        | FIELD_EET_BGEE_GAME_FOLDER
        | FIELD_EET_BG2EE_GAME_FOLDER => FieldRole::Game,
        FIELD_MODS_FOLDER
        | FIELD_MODS_ARCHIVE_FOLDER
        | FIELD_MODS_BACKUP_FOLDER
        | FIELD_WEIDU_LOG_FOLDER => FieldRole::Working,
        FIELD_WEIDU_BINARY | FIELD_MOD_INSTALLER_BINARY => FieldRole::Binary,
        _ => FieldRole::Working,
    }
}

/// Run all per-field checks against `step1` and produce a fresh
/// `ValidationReport`. Per-row results carry their own message strings —
/// callers don't need to know about field roles to render the hint.
pub fn run_now(step1: &Step1State) -> ValidationReport {
    let mut report = ValidationReport::default();

    let folder_fields: [(&'static str, &str); 9] = [
        (FIELD_BGEE_GAME_FOLDER, &step1.bgee_game_folder),
        (FIELD_BG2EE_GAME_FOLDER, &step1.bg2ee_game_folder),
        (FIELD_IWDEE_GAME_FOLDER, &step1.iwdee_game_folder),
        (FIELD_EET_BGEE_GAME_FOLDER, &step1.eet_bgee_game_folder),
        (FIELD_EET_BG2EE_GAME_FOLDER, &step1.eet_bg2ee_game_folder),
        (FIELD_MODS_FOLDER, &step1.mods_folder),
        (FIELD_MODS_ARCHIVE_FOLDER, &step1.mods_archive_folder),
        (FIELD_MODS_BACKUP_FOLDER, &step1.mods_backup_folder),
        (FIELD_WEIDU_LOG_FOLDER, &step1.weidu_log_folder),
    ];
    for (name, value) in folder_fields.iter() {
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

    report.overall_ok = state_validation::is_step1_valid(step1);
    // Count visible "not ok" rows (Warning + Error) — this matches what the
    // user sees in the per-row hints, not BIO's minimum-required gate set.
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

/// Re-run validation for a single field. Used by `validate_debounce::tick`
/// when only one field's debounce has elapsed.
pub fn run_for_field(step1: &Step1State, field: &'static str) -> PathStatus {
    let value = match field {
        FIELD_BGEE_GAME_FOLDER => &step1.bgee_game_folder,
        FIELD_BG2EE_GAME_FOLDER => &step1.bg2ee_game_folder,
        FIELD_IWDEE_GAME_FOLDER => &step1.iwdee_game_folder,
        FIELD_EET_BGEE_GAME_FOLDER => &step1.eet_bgee_game_folder,
        FIELD_EET_BG2EE_GAME_FOLDER => &step1.eet_bg2ee_game_folder,
        FIELD_MODS_FOLDER => &step1.mods_folder,
        FIELD_MODS_ARCHIVE_FOLDER => &step1.mods_archive_folder,
        FIELD_MODS_BACKUP_FOLDER => &step1.mods_backup_folder,
        FIELD_WEIDU_LOG_FOLDER => &step1.weidu_log_folder,
        FIELD_WEIDU_BINARY => &step1.weidu_binary,
        FIELD_MOD_INSTALLER_BINARY => &step1.mod_installer_binary,
        _ => return PathStatus::Empty,
    };
    check_path(field, value)
}

// ── Role-specific checks ───────────────────────────────────────────────────

fn check_path(field: &str, value: &str) -> PathStatus {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return PathStatus::Empty;
    }
    match field_role(field) {
        FieldRole::Game => check_game_folder(trimmed),
        FieldRole::Working => check_working_folder(trimmed),
        FieldRole::Binary => check_binary(trimmed),
    }
}

fn check_game_folder(value: &str) -> PathStatus {
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
    // Same marker pair as BIO's `state_validation_fs::check_game_dir`.
    let has_chitin = path.join("chitin.key").is_file();
    let has_lang = path.join("lang").is_dir();
    if !has_chitin || !has_lang {
        return PathStatus::Warning {
            reason: "no chitin.key/lang \u{2014} not a recognizable game install".to_string(),
        };
    }
    PathStatus::Ok { detail: None }
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
    // Defensive: the user may have pointed a working folder at their game
    // install by mistake. Flag, but don't block — they can ignore the warn.
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
        // Bare name — actually scan $PATH so we don't lie to the user with a
        // green "ok" when nothing is installed system-wide. If the user
        // typed `weidu` but `weidu` isn't on PATH, the install will fail —
        // surface that here.
        match resolve_on_path(value) {
            Some(resolved) => PathStatus::Ok {
                detail: Some(resolved.display().to_string()),
            },
            None => PathStatus::Error {
                reason: "not on $PATH — install or specify the full path".to_string(),
            },
        }
    }
}

/// Search `$PATH` for an executable named `name`. On Windows also tries
/// `<name>.exe`. Returns the first matching absolute path, or `None`.
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
