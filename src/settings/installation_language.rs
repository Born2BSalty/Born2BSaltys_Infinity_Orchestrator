// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) const INSTALLATION_LANGUAGES: &[(&str, &str)] = &[
    ("en_US", "English"),
    ("de_DE", "German"),
    ("fr_FR", "French"),
    ("es_ES", "Spanish"),
    ("it_IT", "Italian"),
    ("pl_PL", "Polish"),
    ("pt_BR", "Portuguese"),
    ("cs_CZ", "Czech"),
    ("tr_TR", "Turkish"),
    ("uk_UA", "Ukrainian"),
    ("ru_RU", "Russian"),
];

pub(crate) fn normalize_installation_locale(value: &str) -> String {
    let identity = value.trim().replace('-', "_").to_ascii_lowercase();
    INSTALLATION_LANGUAGES
        .iter()
        .find_map(|(locale, _)| locale.eq_ignore_ascii_case(&identity).then_some(*locale))
        .unwrap_or(&identity)
        .to_string()
}

pub(crate) fn installation_language_label(value: &str) -> String {
    if value.trim().is_empty() {
        return "Automatic (game language)".to_string();
    }
    let identity = normalize_installation_locale(value);
    INSTALLATION_LANGUAGES
        .iter()
        .find_map(|(locale, label)| (*locale == identity).then_some(*label))
        .unwrap_or(value)
        .to_string()
}
