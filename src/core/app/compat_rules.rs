// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

pub(crate) use super::compat_rules_model::{
    COMPAT_RULES_SCHEMA_VERSION, CompatRule, CompatRulesFile, StringOrMany,
};
use crate::platform_defaults::app_config_file;

const COMPAT_RULES_LEGACY_USER_FILE_NAME: &str = "step2_compat_rules.toml";
const COMPAT_RULES_USER_FILE_NAME: &str = "step2_compat_rules_user.toml";
const COMPAT_RULES_DEFAULT_FILE_NAME: &str = "step2_compat_rules_default.toml";

#[derive(Debug, Clone)]
pub(crate) struct CompatRulesFileInventory {
    pub(crate) role: String,
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) parse_status: String,
    pub(crate) schema_version: Option<u32>,
    pub(crate) total_rules: usize,
    pub(crate) enabled_rules: usize,
    pub(crate) loaded_rules: usize,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompatRulesInventory {
    pub(crate) default_path: String,
    pub(crate) user_path: String,
    pub(crate) total_loaded_rules: usize,
    pub(crate) files: Vec<CompatRulesFileInventory>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompatRulesLoad {
    pub(crate) rules: Vec<CompatRule>,
    pub(crate) error: Option<String>,
}

pub(crate) fn compat_rules_user_path() -> PathBuf {
    app_config_file(COMPAT_RULES_USER_FILE_NAME, "config")
}

pub(crate) fn compat_rules_legacy_user_path() -> PathBuf {
    app_config_file(COMPAT_RULES_LEGACY_USER_FILE_NAME, "config")
}

pub(crate) fn compat_rules_default_path() -> PathBuf {
    app_config_file(COMPAT_RULES_DEFAULT_FILE_NAME, "config")
}

pub(crate) fn ensure_compat_rules_files() -> std::io::Result<()> {
    let default_path = compat_rules_default_path();
    let user_path = compat_rules_user_path();
    let legacy_user_path = compat_rules_legacy_user_path();

    if let Some(parent) = default_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&default_path, default_step2_rules_content())?;

    if !user_path.exists() && legacy_user_path.exists() {
        fs::copy(&legacy_user_path, &user_path)?;
    }

    if !user_path.exists() {
        fs::write(&user_path, user_step2_rules_content())?;
    }

    Ok(())
}

pub(crate) fn effective_compat_rules_user_path() -> PathBuf {
    let user_path = compat_rules_user_path();
    if user_path.exists() {
        return user_path;
    }
    let legacy_path = compat_rules_legacy_user_path();
    if legacy_path.exists() {
        legacy_path
    } else {
        user_path
    }
}

pub(crate) fn rules_files_signature() -> String {
    let default_path = compat_rules_default_path();
    let user_path = effective_compat_rules_user_path();
    format!(
        "{}|{}|{}|{}",
        default_path.display(),
        cache_stamp_signature(&default_path),
        user_path.display(),
        cache_stamp_signature(&user_path)
    )
}

pub(crate) fn load_rules() -> CompatRulesLoad {
    let default_path = compat_rules_default_path();
    let user_path = effective_compat_rules_user_path();
    let default_stamp = cache_stamp(&default_path);
    let user_stamp = cache_stamp(&user_path);
    let cache = rules_cache();
    let mut cache = cache.lock().expect("compat rules cache lock poisoned");

    if let Some(entry) = cache.as_ref()
        && entry.default_path == default_path
        && entry.user_path == user_path
        && entry.default_stamp == default_stamp
        && entry.user_stamp == user_stamp
    {
        return entry.load.clone();
    }

    let default_load = load_rules_from_path(&default_path);
    let user_load = load_rules_from_path(&user_path);
    let mut rules = default_load.rules;
    rules.extend(user_load.rules);
    let load = CompatRulesLoad {
        rules,
        error: merge_load_errors(default_load.error, user_load.error),
    };
    *cache = Some(CachedRules {
        default_path,
        user_path,
        default_stamp,
        user_stamp,
        load: load.clone(),
    });
    load
}

pub(crate) fn inspect_compat_rules_inventory() -> CompatRulesInventory {
    let default_path = compat_rules_default_path();
    let user_path = effective_compat_rules_user_path();
    let loaded_rules = load_rules();

    CompatRulesInventory {
        default_path: default_path.display().to_string(),
        user_path: user_path.display().to_string(),
        total_loaded_rules: loaded_rules.rules.len(),
        files: vec![
            inspect_rules_file("default", &default_path),
            inspect_rules_file("user", &user_path),
        ],
    }
}

fn write_if_changed(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == content => Ok(()),
        _ => fs::write(path, content),
    }
}

fn default_step2_rules_content() -> &'static str {
    include_str!("../config/default_step2_compat_rules.toml")
}

fn user_step2_rules_content() -> &'static str {
    include_str!("../config/user_step2_compat_rules.toml")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRules {
    default_path: PathBuf,
    user_path: PathBuf,
    default_stamp: FileCacheStamp,
    user_stamp: FileCacheStamp,
    load: CompatRulesLoad,
}

fn rules_cache() -> &'static Mutex<Option<CachedRules>> {
    static CACHE: OnceLock<Mutex<Option<CachedRules>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn cache_stamp(path: &PathBuf) -> FileCacheStamp {
    match fs::metadata(path) {
        Ok(meta) => FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
        Err(_) => FileCacheStamp {
            modified: None,
            len: 0,
        },
    }
}

fn cache_stamp_signature(path: &PathBuf) -> String {
    let stamp = cache_stamp(path);
    let modified = stamp
        .modified
        .and_then(|value| value.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().to_string())
        .unwrap_or_default();
    format!("{modified}:{}", stamp.len)
}

fn load_rules_from_path(path: &PathBuf) -> CompatRulesLoad {
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return CompatRulesLoad {
                rules: Vec::new(),
                error: Some(format!(
                    "compat rules load failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    let parsed = match toml::from_str::<CompatRulesFile>(&content) {
        Ok(value) => value,
        Err(err) => {
            return CompatRulesLoad {
                rules: Vec::new(),
                error: Some(format!(
                    "compat rules parse failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    let CompatRulesFile {
        schema_version,
        rules,
    } = parsed;
    let _schema_version = schema_version.unwrap_or(COMPAT_RULES_SCHEMA_VERSION);
    CompatRulesLoad {
        rules: rules
            .into_iter()
            .filter(|rule| {
                rule.enabled
                    && !rule.r#mod.trimmed_items().is_empty()
                    && !rule.kind.trim().is_empty()
            })
            .map(|mut rule| {
                rule.loaded_from = Some(path.to_string_lossy().to_string());
                rule
            })
            .collect(),
        error: None,
    }
}

fn merge_load_errors(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("{left} | {right}")),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

pub(crate) fn compat_rule_source_path(rule: &CompatRule) -> String {
    let fallback = rule.loaded_from.clone().unwrap_or_else(|| {
        effective_compat_rules_user_path()
            .to_string_lossy()
            .to_string()
    });
    let Some(source) = rule.source.as_deref().map(str::trim) else {
        return fallback;
    };
    if source.is_empty()
        || source.eq_ignore_ascii_case("step2_compat_rules.toml")
        || source.eq_ignore_ascii_case("step2_compat_rules")
        || source.eq_ignore_ascii_case("step2_compat_rules_user.toml")
        || source.eq_ignore_ascii_case("step2_compat_rules_user")
        || source.eq_ignore_ascii_case("step2_compat_rules_default.toml")
        || source.eq_ignore_ascii_case("step2_compat_rules_default")
    {
        fallback
    } else {
        source.to_string()
    }
}

pub(crate) fn compat_rule_source_bucket(rule: &CompatRule) -> String {
    let Some(loaded_from) = rule.loaded_from.as_deref() else {
        return "unknown".to_string();
    };
    let loaded_from = normalize_path_key(loaded_from);
    let default_path = normalize_path_key(&compat_rules_default_path().display().to_string());
    if loaded_from == default_path {
        return "default".to_string();
    }
    let user_path = normalize_path_key(&effective_compat_rules_user_path().display().to_string());
    if loaded_from == user_path {
        return "user".to_string();
    }
    "external".to_string()
}

fn inspect_rules_file(role: &str, path: &PathBuf) -> CompatRulesFileInventory {
    let exists = path.is_file();
    let mut inventory = CompatRulesFileInventory {
        role: role.to_string(),
        path: path.display().to_string(),
        exists,
        parse_status: if exists { "unknown" } else { "missing" }.to_string(),
        schema_version: None,
        total_rules: 0,
        enabled_rules: 0,
        loaded_rules: 0,
        error: None,
    };
    if !exists {
        return inventory;
    }
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            inventory.parse_status = "read_error".to_string();
            inventory.error = Some(err.to_string());
            return inventory;
        }
    };
    let parsed = match toml::from_str::<CompatRulesFile>(&content) {
        Ok(value) => value,
        Err(err) => {
            inventory.parse_status = "parse_error".to_string();
            inventory.error = Some(err.to_string());
            return inventory;
        }
    };
    inventory.parse_status = "ok".to_string();
    inventory.schema_version = parsed.schema_version;
    inventory.total_rules = parsed.rules.len();
    inventory.enabled_rules = parsed.rules.iter().filter(|rule| rule.enabled).count();
    inventory.loaded_rules = parsed
        .rules
        .iter()
        .filter(|rule| {
            rule.enabled && !rule.r#mod.trimmed_items().is_empty() && !rule.kind.trim().is_empty()
        })
        .count();
    inventory
}

fn normalize_path_key(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}
