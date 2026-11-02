// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::language_names::{
    UNTAGGED_LANGUAGE, canonical_language_name, display_language_name, preferred_language_name,
};

#[test]
fn canonical_names_cover_every_step1_language() {
    for (locale, name) in [
        ("en_US", "english"),
        ("de_DE", "german"),
        ("fr_FR", "french"),
        ("es_ES", "spanish"),
        ("it_IT", "italian"),
        ("pl_PL", "polish"),
        ("pt_BR", "portuguese"),
        ("cs_CZ", "czech"),
        ("tr_TR", "turkish"),
        ("uk_UA", "ukrainian"),
        ("ru_RU", "russian"),
        ("zh_CN", "chinese"),
    ] {
        assert_eq!(preferred_language_name(locale), name, "locale {locale}");
        assert_eq!(canonical_language_name(name), name, "label {name}");
    }
    for (label, name) in [
        ("English", "english"),
        ("American English", "english"),
        ("Polski", "polish"),
        ("Deutsch", "german"),
        ("Français", "french"),
        ("  Polish (by Cherub)  ", "polish"),
        ("Русский", "russian"),
        ("Українська", "ukrainian"),
        ("中文", "chinese"),
        ("简体中文", "chinese"),
        ("繁體中文", "chinese"),
        ("Čeština", "czech"),
        ("Türkçe", "turkish"),
        ("Português", "portuguese"),
        ("Español", "spanish"),
        ("English (UK)", "english"),
    ] {
        assert_eq!(canonical_language_name(label), name, "label {label}");
    }
    assert_eq!(preferred_language_name(""), UNTAGGED_LANGUAGE);
    assert_eq!(preferred_language_name("xx_YY"), UNTAGGED_LANGUAGE);
}

#[test]
fn unknown_label_keeps_its_own_name() {
    assert_eq!(canonical_language_name("  Klingon "), "klingon");
    assert_eq!(canonical_language_name(""), UNTAGGED_LANGUAGE);
    assert_eq!(canonical_language_name("   "), UNTAGGED_LANGUAGE);
    assert_eq!(display_language_name("polish"), "Polish");
    assert_eq!(display_language_name(""), "");
}
