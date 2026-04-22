// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn compat_code_from_kind(kind: &str) -> String {
    if kind.eq_ignore_ascii_case("mismatch") || kind.eq_ignore_ascii_case("game_mismatch") {
        "MISMATCH".to_string()
    } else if kind.eq_ignore_ascii_case("missing_dep") {
        "REQ_MISSING".to_string()
    } else if kind.eq_ignore_ascii_case("conflict") || kind.eq_ignore_ascii_case("not_compatible") {
        "RULE_HIT".to_string()
    } else if kind.eq_ignore_ascii_case("included") {
        "INCLUDED".to_string()
    } else if kind.eq_ignore_ascii_case("order_block") {
        "ORDER_BLOCK".to_string()
    } else if kind.eq_ignore_ascii_case("conditional") {
        "CONDITIONAL".to_string()
    } else if kind.eq_ignore_ascii_case("path_requirement") {
        "PATH_REQUIREMENT".to_string()
    } else if kind.eq_ignore_ascii_case("deprecated") {
        "DEPRECATED".to_string()
    } else {
        kind.to_ascii_uppercase()
    }
}

pub(crate) fn compat_role(kind: &str, source: Option<&str>) -> String {
    let Some(source) = source.map(str::trim) else {
        return "Compatibility rule".to_string();
    };
    let lower = source.to_ascii_lowercase();
    if lower.ends_with(".toml") {
        "Rule file".to_string()
    } else if lower.ends_with(".tp2") {
        if kind.eq_ignore_ascii_case("mismatch") {
            "TP2 guard".to_string()
        } else if kind.eq_ignore_ascii_case("missing_dep") {
            "TP2 dependency check".to_string()
        } else if kind.eq_ignore_ascii_case("path_requirement") {
            "TP2 path check".to_string()
        } else {
            "TP2 relation".to_string()
        }
    } else {
        "Compatibility rule".to_string()
    }
}

pub(crate) fn display_name_from_tp2(tp2_ref: &str) -> String {
    let file = tp2_file_name(tp2_ref);
    let lower = file.to_ascii_lowercase();
    let stem = if lower.ends_with(".tp2") {
        &file[..file.len().saturating_sub(4)]
    } else {
        file.as_str()
    };
    let stem = if stem.to_ascii_lowercase().starts_with("setup-") {
        &stem[6..]
    } else {
        stem
    };
    if stem.is_empty() {
        return tp2_ref.to_string();
    }
    stem.to_string()
}

pub(crate) fn tp2_file_name(tp2_ref: &str) -> String {
    let trimmed = tp2_ref.trim().trim_matches(['~', '"']);
    let file = if let Some(idx) = trimmed.rfind(['/', '\\']) {
        &trimmed[idx + 1..]
    } else {
        trimmed
    };
    if file.is_empty() {
        tp2_ref.to_string()
    } else {
        file.to_string()
    }
}
