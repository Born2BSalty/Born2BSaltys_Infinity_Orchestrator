// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub const UNTAGGED_LANGUAGE: &str = "english";

const LANGUAGE_NAMES: [(&str, &str); 12] = [
    ("de", "german"),
    ("ru", "russian"),
    ("fr", "french"),
    ("es", "spanish"),
    ("it", "italian"),
    ("pl", "polish"),
    ("pt", "portuguese"),
    ("cs", "czech"),
    ("tr", "turkish"),
    ("uk", "ukrainian"),
    ("zh", "chinese"),
    ("en", "english"),
];

#[must_use]
pub fn canonical_language_name(label: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return UNTAGGED_LANGUAGE.to_string();
    }
    let lowered = trimmed.to_lowercase();
    LANGUAGE_NAMES
        .iter()
        .find(|(key, _)| {
            contains_any_hint(&lowered, &language_hints_for_locale(key))
                || native_script_hints(key)
                    .iter()
                    .any(|hint| lowered.contains(hint))
        })
        .map_or(lowered, |(_, name)| (*name).to_string())
}

fn native_script_hints(locale_key: &str) -> &'static [&'static str] {
    match locale_key {
        "ru" => &["рус", "русский"],
        "fr" => &["français"],
        "es" => &["español"],
        "pt" => &["português"],
        "cs" => &["čeština"],
        "tr" => &["türkçe"],
        "uk" => &["україн"],
        "zh" => &["中文", "简体", "繁體"],
        _ => &[],
    }
}

#[must_use]
pub fn preferred_language_name(locale: &str) -> String {
    let key = locale_key(locale);
    LANGUAGE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map_or(UNTAGGED_LANGUAGE, |(_, name)| *name)
        .to_string()
}

#[must_use]
pub fn display_language_name(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[must_use]
pub(crate) fn locale_key(locale: &str) -> String {
    locale
        .to_ascii_lowercase()
        .split(['_', '-'])
        .next()
        .unwrap_or_default()
        .to_string()
}

#[must_use]
pub(crate) fn contains_any_hint(text: &str, hints: &[&str]) -> bool {
    let text_tokens = tokenize_language_label(text);
    let normalized_text = normalize_language_phrase(text);
    hints.iter().any(|hint| {
        let hint = normalize_language_phrase(hint);
        !hint.is_empty()
            && (normalized_text == hint
                || normalized_text.contains(&hint)
                || text_tokens.iter().any(|token| token == &hint))
    })
}

#[must_use]
pub(crate) fn matches_locale_token(label: &str, locale_key: &str) -> bool {
    if locale_key.is_empty() {
        return false;
    }
    tokenize_language_label(label)
        .into_iter()
        .any(|token| token == locale_key)
}

fn tokenize_language_label(label: &str) -> Vec<String> {
    label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn normalize_language_phrase(value: &str) -> String {
    let tokens = tokenize_language_label(value);
    if tokens.is_empty() {
        String::new()
    } else {
        format!(" {} ", tokens.join(" "))
    }
}

#[must_use]
pub(crate) fn language_hints_for_locale(locale_key: &str) -> Vec<&'static str> {
    match locale_key {
        "en" => vec!["english", "en", "en_us", "en-gb", "american english"],
        "de" => vec!["german", "deutsch", "de_de"],
        "ru" => vec!["russian", "ru_ru", "рус"],
        "fr" => vec!["french", "fr_fr", "francais", "français"],
        "es" => vec!["spanish", "es", "es_es", "es-es", "español", "espanol"],
        "it" => vec!["italian", "it_it", "italiano"],
        "pl" => vec!["polish", "pl_pl", "polski"],
        "pt" => vec!["portuguese", "pt_br", "pt_pt", "português", "portugues"],
        "cs" => vec!["czech", "cs_cz", "čeština", "cestina"],
        "tr" => vec!["turkish", "tr_tr", "türkçe", "turkce"],
        "uk" => vec!["ukrainian", "uk_ua", "україн"],
        "zh" => vec!["zh", "zh_cn", "zh_tw", "chinese", "schinese", "tchinese"],
        _ => vec![],
    }
}
