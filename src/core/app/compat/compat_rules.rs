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
pub(crate) const BUILT_IN_RULES_LABEL: &str = "BIO default (built in)";

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

pub(crate) fn ensure_compat_rules_files() -> std::io::Result<()> {
    let user_path = compat_rules_user_path();
    let legacy_user_path = compat_rules_legacy_user_path();

    if let Some(parent) = user_path.parent() {
        fs::create_dir_all(parent)?;
    }

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
    let user_path = effective_compat_rules_user_path();
    format!(
        "builtin|{}|{}",
        user_path.display(),
        cache_stamp_signature(&user_path)
    )
}

pub(crate) fn load_rules() -> CompatRulesLoad {
    let user_path = effective_compat_rules_user_path();
    let user_stamp = cache_stamp(&user_path);
    let cache = rules_cache();
    let mut cache = cache.lock().expect("compat rules cache lock poisoned");

    if let Some(entry) = cache.as_ref()
        && entry.user_path == user_path
        && entry.user_stamp == user_stamp
    {
        return entry.load.clone();
    }

    let built_in = built_in_rules_load();
    let user_load = load_rules_from_path(&user_path);
    let mut rules = built_in.rules.clone();
    rules.extend(user_load.rules);
    let load = CompatRulesLoad {
        rules,
        error: merge_load_errors(built_in.error.clone(), user_load.error),
    };
    *cache = Some(CachedRules {
        user_path,
        user_stamp,
        load: load.clone(),
    });
    load
}

pub(crate) fn inspect_compat_rules_inventory() -> CompatRulesInventory {
    let user_path = effective_compat_rules_user_path();
    let loaded_rules = load_rules();

    CompatRulesInventory {
        default_path: BUILT_IN_RULES_LABEL.to_string(),
        user_path: user_path.display().to_string(),
        total_loaded_rules: loaded_rules.rules.len(),
        files: vec![
            inspect_rules_content(
                "default",
                BUILT_IN_RULES_LABEL.to_string(),
                default_step2_rules_content(),
            ),
            inspect_rules_file("user", &user_path),
        ],
    }
}

fn built_in_rules_load() -> &'static CompatRulesLoad {
    static BUILT_IN: OnceLock<CompatRulesLoad> = OnceLock::new();
    BUILT_IN.get_or_init(|| {
        load_rules_from_content(default_step2_rules_content(), BUILT_IN_RULES_LABEL)
    })
}

const fn default_step2_rules_content() -> &'static str {
    include_str!("../../config/default_step2_compat_rules.toml")
}

const fn user_step2_rules_content() -> &'static str {
    include_str!("../../config/user_step2_compat_rules.toml")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedRules {
    user_path: PathBuf,
    user_stamp: FileCacheStamp,
    load: CompatRulesLoad,
}

fn rules_cache() -> &'static Mutex<Option<CachedRules>> {
    static CACHE: OnceLock<Mutex<Option<CachedRules>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn cache_stamp(path: &PathBuf) -> FileCacheStamp {
    fs::metadata(path).map_or(
        FileCacheStamp {
            modified: None,
            len: 0,
        },
        |meta| FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
    )
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
    load_rules_from_content(&content, &path.to_string_lossy())
}

fn load_rules_from_content(content: &str, loaded_from: &str) -> CompatRulesLoad {
    let parsed = match toml::from_str::<CompatRulesFile>(content) {
        Ok(value) => value,
        Err(err) => {
            return CompatRulesLoad {
                rules: Vec::new(),
                error: Some(format!(
                    "compat rules parse failed for {loaded_from}: {err}"
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
                rule.loaded_from = Some(loaded_from.to_string());
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
    if loaded_from == BUILT_IN_RULES_LABEL {
        return "default".to_string();
    }
    let loaded_from = normalize_path_key(loaded_from);
    let user_path = normalize_path_key(&effective_compat_rules_user_path().display().to_string());
    if loaded_from == user_path {
        return "user".to_string();
    }
    "external".to_string()
}

fn inspect_rules_file(role: &str, path: &PathBuf) -> CompatRulesFileInventory {
    let exists = path.is_file();
    if !exists {
        return CompatRulesFileInventory {
            role: role.to_string(),
            path: path.display().to_string(),
            exists,
            parse_status: "missing".to_string(),
            schema_version: None,
            total_rules: 0,
            enabled_rules: 0,
            loaded_rules: 0,
            error: None,
        };
    }
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return CompatRulesFileInventory {
                role: role.to_string(),
                path: path.display().to_string(),
                exists,
                parse_status: "read_error".to_string(),
                schema_version: None,
                total_rules: 0,
                enabled_rules: 0,
                loaded_rules: 0,
                error: Some(err.to_string()),
            };
        }
    };
    inspect_rules_content(role, path.display().to_string(), &content)
}

fn inspect_rules_content(role: &str, path: String, content: &str) -> CompatRulesFileInventory {
    let mut inventory = CompatRulesFileInventory {
        role: role.to_string(),
        path,
        exists: true,
        parse_status: "unknown".to_string(),
        schema_version: None,
        total_rules: 0,
        enabled_rules: 0,
        loaded_rules: 0,
        error: None,
    };
    let parsed = match toml::from_str::<CompatRulesFile>(content) {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::platform_defaults::{clear_config_dir_override_if, set_config_dir_override};

    struct TempRootGuard(PathBuf);

    impl Drop for TempRootGuard {
        fn drop(&mut self) {
            clear_config_dir_override_if(&self.0);
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temp_root(tag: &str) -> TempRootGuard {
        let root = std::env::temp_dir().join(format!(
            "bio_compat_rules_{tag}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&root).expect("temp root");
        set_config_dir_override(Some(root.clone()));
        TempRootGuard(root)
    }

    #[test]
    fn built_in_rules_load_without_a_default_file() {
        let guard = temp_root("no_default");
        assert!(!guard.0.join("step2_compat_rules_default.toml").exists());
        let expected = load_rules_from_content(default_step2_rules_content(), BUILT_IN_RULES_LABEL);
        assert_ne!(expected.rules.len(), 0);
        let loaded = load_rules();
        let built_in_count = loaded
            .rules
            .iter()
            .filter(|rule| rule.loaded_from.as_deref() == Some(BUILT_IN_RULES_LABEL))
            .count();
        assert_eq!(built_in_count, expected.rules.len());
    }

    #[test]
    fn built_in_rules_are_bucketed_default() {
        let guard = temp_root("bucket");
        assert!(!guard.0.join("step2_compat_rules_default.toml").exists());
        let loaded = load_rules();
        let rule = loaded
            .rules
            .iter()
            .find(|rule| rule.loaded_from.as_deref() == Some(BUILT_IN_RULES_LABEL))
            .expect("at least one built-in rule loaded");
        assert_eq!(compat_rule_source_bucket(rule), "default");
    }

    #[test]
    fn signature_ignores_missing_default_file() {
        let guard = temp_root("signature");
        assert!(!guard.0.join("step2_compat_rules_default.toml").exists());
        let signature = rules_files_signature();
        assert!(signature.starts_with("builtin|"));
        assert!(
            !signature
                .to_ascii_lowercase()
                .contains("step2_compat_rules_default")
        );
    }
}
