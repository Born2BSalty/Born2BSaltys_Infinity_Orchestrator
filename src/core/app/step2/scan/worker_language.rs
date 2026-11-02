// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::language_names::{
    contains_any_hint, language_hints_for_locale, locale_key, matches_locale_token,
};
use crate::app::state::Step1State;
use crate::install::weidu_scan::{self, LanguageEntry};

#[derive(Debug, Clone)]
pub struct PreferredLocaleInfo {
    pub locale: String,
    pub source: String,
    pub baldur_lua_path: Option<PathBuf>,
}

#[must_use]
pub fn detect_preferred_game_locale(step1: &Step1State) -> PreferredLocaleInfo {
    if !step1.language.trim().is_empty() {
        return PreferredLocaleInfo {
            locale: step1.language.trim().to_string(),
            source: "step1_language".to_string(),
            baldur_lua_path: None,
        };
    }

    for base in documents_roots() {
        for profile in profile_dir_names(&step1.game_install) {
            let path = base.join(profile).join("Baldur.lua");
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(locale) = parse_language_text_value(&content) {
                return PreferredLocaleInfo {
                    locale,
                    source: "Baldur.lua".to_string(),
                    baldur_lua_path: Some(path),
                };
            }
        }
    }
    PreferredLocaleInfo {
        locale: "en_US".to_string(),
        source: "fallback_default".to_string(),
        baldur_lua_path: None,
    }
}

pub(super) fn candidate_language_ids(
    weidu: &Path,
    tp2: &Path,
    game_dir: &Path,
    work_dir: &Path,
    preferred_locale: &str,
) -> Result<Vec<LanguageEntry>, String> {
    let langs = match weidu_scan::list_languages_for_game(weidu, tp2, game_dir, work_dir) {
        Ok(langs) if !langs.is_empty() => langs,
        Ok(_) => weidu_scan::list_languages(weidu, tp2)
            .map_err(|err| format!("failed to list languages for {}: {err}", tp2.display()))?,
        Err(game_err) => {
            weidu_scan::list_languages(weidu, tp2).map_err(|fallback_err| {
                format!(
                    "failed to list languages for {} with game context ({game_err}) and without game context ({fallback_err})",
                    tp2.display()
                )
            })?
        }
    };
    Ok(order_language_entries(langs, preferred_locale))
}

pub(super) fn order_language_entries(
    langs: Vec<LanguageEntry>,
    preferred_locale: &str,
) -> Vec<LanguageEntry> {
    let locale_key = locale_key(preferred_locale);
    let preferred_hints = language_hints_for_locale(&locale_key);
    let english_hints = language_hints_for_locale("en");
    let wants_english = locale_key.is_empty() || locale_key == "en";

    let mut preferred = Vec::<LanguageEntry>::new();
    let mut english = Vec::<LanguageEntry>::new();
    let mut others = Vec::<LanguageEntry>::new();
    for entry in langs {
        let label = entry.label.to_ascii_lowercase();
        let locale_matches_label = !wants_english && matches_locale_token(&label, &locale_key);
        if !wants_english && (contains_any_hint(&label, &preferred_hints) || locale_matches_label) {
            preferred.push(entry);
        } else if contains_any_hint(&label, &english_hints) {
            english.push(entry);
        } else {
            others.push(entry);
        }
    }

    let mut ordered = Vec::<LanguageEntry>::new();
    if !wants_english {
        ordered.extend(preferred);
    }
    ordered.extend(english);
    ordered.extend(others);

    let mut deduped = Vec::<LanguageEntry>::new();
    for entry in ordered {
        if !deduped.iter().any(|kept| kept.id == entry.id) {
            deduped.push(entry);
        }
    }

    if deduped.is_empty() {
        deduped.push(LanguageEntry {
            id: "0".to_string(),
            label: String::new(),
        });
    }
    deduped
}

fn profile_dir_names(game_install: &str) -> Vec<&'static str> {
    match game_install {
        "BG2EE" => vec!["Baldur's Gate II - Enhanced Edition"],
        "IWDEE" => vec!["Icewind Dale - Enhanced Edition"],
        "PSTEE" => vec!["Planescape Torment - Enhanced Edition"],
        "EET" => vec![
            "Baldur's Gate - Enhanced Edition Trilogy",
            "Baldur's Gate II - Enhanced Edition",
        ],
        _ => vec![
            "Baldur's Gate - Enhanced Edition",
            "Baldur's Gate Enhanced Edition",
        ],
    }
}

fn documents_roots() -> Vec<PathBuf> {
    let mut roots = Vec::<PathBuf>::new();
    let mut push_root = |p: PathBuf| {
        if !roots.iter().any(|e| e == &p) {
            roots.push(p);
        }
    };

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_root(home.join("Documents"));
    }
    if let Some(user_profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        push_root(user_profile.join("Documents"));
    }
    if let Some(one_drive) = env::var_os("OneDrive").map(PathBuf::from) {
        push_root(one_drive.join("Documents"));
    }
    roots
}

fn parse_language_text_value(content: &str) -> Option<String> {
    for line in content.lines() {
        if !line.contains("SetPrivateProfileString") {
            continue;
        }
        let parts = extract_quoted_strings(line);
        if parts.len() < 3 {
            continue;
        }
        if parts[0].eq_ignore_ascii_case("Language") && parts[1].eq_ignore_ascii_case("Text") {
            let v = parts[2].trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_quoted_strings(line: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote != b'\'' && quote != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        if i <= bytes.len() {
            out.push(line[start..i].to_string());
        }
        i += 1;
    }
    out
}
