// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::install::weidu_scan;
use crate::ui::state::Step1State;

pub(super) fn detect_preferred_game_locale(step1: &Step1State) -> String {
    for base in documents_roots() {
        for profile in profile_dir_names(&step1.game_install) {
            let path = base.join(profile).join("Baldur.lua");
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(locale) = parse_language_text_value(&content) {
                return locale;
            }
        }
    }
    "en_US".to_string()
}

pub(super) fn candidate_language_ids(
    weidu: &Path,
    tp2: &Path,
    game_dir: &Path,
    work_dir: &Path,
    preferred_locale: &str,
) -> Vec<String> {
    let mut langs = weidu_scan::list_languages_for_game(weidu, tp2, game_dir, work_dir)
        .unwrap_or_default();
    if langs.is_empty() {
        langs = weidu_scan::list_languages(weidu, tp2).unwrap_or_default();
    }

    if langs.is_empty() {
        return vec!["0".to_string()];
    }

    let locale = preferred_locale.to_ascii_lowercase();
    let locale_key = locale
        .split(['_', '-'])
        .next()
        .unwrap_or_default()
        .to_string();
    let preferred_hints = language_hints_for_locale(&locale_key);
    let english_hints = language_hints_for_locale("en");
    let wants_english = locale_key.is_empty() || locale_key == "en";

    let mut preferred = Vec::<String>::new();
    let mut english = Vec::<String>::new();
    let mut others = Vec::<String>::new();
    for entry in langs {
        let id = entry.id;
        let label = entry.label.to_ascii_lowercase();
        let locale_matches_label =
            !wants_english && (!locale_key.is_empty() && label.contains(&locale_key));
        if !wants_english && (contains_any_hint(&label, &preferred_hints) || locale_matches_label) {
            preferred.push(id);
        } else if contains_any_hint(&label, &english_hints) {
            english.push(id);
        } else {
            others.push(id);
        }
    }

    let mut ordered = Vec::<String>::new();
    if wants_english {
        ordered.extend(english);
        ordered.extend(others);
    } else {
        ordered.extend(preferred);
        ordered.extend(english);
        ordered.extend(others);
    }

    let mut deduped = Vec::<String>::new();
    for id in ordered {
        if !deduped.iter().any(|v| v == &id) {
            deduped.push(id);
        }
    }

    let mut preferred = deduped;
    if preferred.is_empty() {
        preferred.push("0".to_string());
    }
    preferred
}

fn contains_any_hint(text: &str, hints: &[&str]) -> bool {
    hints.iter().any(|h| !h.is_empty() && text.contains(h))
}

fn language_hints_for_locale(locale_key: &str) -> Vec<&'static str> {
    match locale_key {
        "en" => vec!["english", "en_us", "en-gb", "american english"],
        "de" => vec!["german", "deutsch", "de_de"],
        "ru" => vec!["russian", "ru_ru", "рус"],
        "fr" => vec!["french", "fr_fr", "francais", "français"],
        "es" => vec!["spanish", "es_es", "español", "espanol"],
        "it" => vec!["italian", "it_it", "italiano"],
        "pl" => vec!["polish", "pl_pl", "polski"],
        "pt" => vec!["portuguese", "pt_br", "pt_pt", "português", "portugues"],
        "cs" => vec!["czech", "cs_cz", "čeština", "cestina"],
        "tr" => vec!["turkish", "tr_tr", "türkçe", "turkce"],
        "uk" => vec!["ukrainian", "uk_ua", "україн"],
        _ => vec![],
    }
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
