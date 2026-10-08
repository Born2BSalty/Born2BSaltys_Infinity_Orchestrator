// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::compat_tp2_blocks::tp2_preamble_code;
use crate::app::tp2_component_begin::code_part;

const MOD_FOLDER_UPPER: &str = "%MOD_FOLDER%";
const MOD_FOLDER_LOWER: &str = "%mod_folder%";
const LANGUAGE_TOKEN: &str = "%LANGUAGE%";
const FALLBACK_LANGUAGE: &str = "english";
const LONG_TILDE: &str = "~~~~~";
const BYTE_ORDER_MARK: &[u8] = &[0xEF, 0xBB, 0xBF];

pub(super) struct ModLayout {
    mod_dir: PathBuf,
    folder_name: String,
}

impl ModLayout {
    pub(super) fn for_tp2(tp2_path: &Path) -> Self {
        let mod_dir = tp2_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let folder_name = mod_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            mod_dir,
            folder_name,
        }
    }

    pub(super) fn mod_dir(&self) -> &Path {
        &self.mod_dir
    }

    pub(super) fn with_mod_folder(&self, raw: &str) -> String {
        raw.replace('\\', "/")
            .replace(MOD_FOLDER_UPPER, &self.folder_name)
            .replace(MOD_FOLDER_LOWER, &self.folder_name)
    }

    pub(super) fn resolve(&self, relative: &str, including_dir: &Path) -> Option<PathBuf> {
        let relative = relative.trim();
        if relative.is_empty() {
            return None;
        }
        self.mod_dir
            .parent()
            .map(|mods_dir| mods_dir.join(relative))
            .into_iter()
            .chain([self.mod_dir.join(relative), including_dir.join(relative)])
            .find(|candidate| candidate.is_file())
    }
}

pub(super) fn decode_text(mut bytes: Vec<u8>) -> String {
    if bytes.starts_with(BYTE_ORDER_MARK) {
        bytes.drain(..BYTE_ORDER_MARK.len());
    }
    String::from_utf8(bytes)
        .unwrap_or_else(|err| err.into_bytes().into_iter().map(char::from).collect())
}

pub(super) fn tra_map_for_tp2(
    tp2_path: &Path,
    tp2_text: &str,
    preferred_lang: Option<&str>,
) -> HashMap<String, String> {
    let code = tp2_preamble_code(tp2_text)
        .into_iter()
        .map(code_part)
        .collect::<Vec<_>>()
        .join("\n");
    let blocks = language_blocks(&code);
    let Some(block) = chosen_block(&blocks, preferred_lang) else {
        return HashMap::new();
    };
    let layout = ModLayout::for_tp2(tp2_path);
    let mut map = HashMap::new();
    for tra in &block.tra_paths {
        let relative = layout
            .with_mod_folder(tra)
            .replace(LANGUAGE_TOKEN, &block.dir);
        let Some(path) = layout.resolve(&relative, layout.mod_dir()) else {
            continue;
        };
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        map.extend(parse_tra_entries(&decode_text(bytes)));
    }
    map
}

struct LanguageBlock {
    dir: String,
    tra_paths: Vec<String>,
}

enum Token<'a> {
    Word(&'a str),
    Text(&'a str),
}

struct Tokens<'a> {
    text: &'a str,
}

impl<'a> Tokens<'a> {
    fn next_token(&mut self) -> Option<Token<'a>> {
        let trimmed = self.text.trim_start();
        if trimmed.is_empty() {
            self.text = trimmed;
            return None;
        }
        if let Some((value, used)) = quoted_string(trimmed) {
            self.text = &trimmed[used..];
            return Some(Token::Text(value));
        }
        if trimmed.starts_with(['~', '"']) {
            self.text = "";
            return None;
        }
        let end = trimmed
            .find(|c: char| c.is_whitespace() || c == '~' || c == '"')
            .unwrap_or(trimmed.len());
        self.text = &trimmed[end..];
        Some(Token::Word(&trimmed[..end]))
    }
}

fn language_blocks(code: &str) -> Vec<LanguageBlock> {
    let mut tokens = Tokens { text: code };
    let mut blocks = Vec::new();
    let mut pending = tokens.next_token();
    while let Some(token) = pending {
        pending = tokens.next_token();
        let Token::Word(word) = token else {
            continue;
        };
        if !word.eq_ignore_ascii_case("LANGUAGE") {
            continue;
        }
        let mut strings = Vec::<String>::new();
        while let Some(Token::Text(value)) = pending {
            strings.push(value.to_string());
            pending = tokens.next_token();
        }
        let mut strings = strings.into_iter().skip(1);
        if let Some(dir) = strings.next() {
            blocks.push(LanguageBlock {
                dir,
                tra_paths: strings.collect(),
            });
        }
    }
    blocks
}

fn chosen_block<'b>(
    blocks: &'b [LanguageBlock],
    preferred_lang: Option<&str>,
) -> Option<&'b LanguageBlock> {
    let mut wanted = language_candidates(preferred_lang);
    wanted.push(FALLBACK_LANGUAGE.to_string());
    wanted
        .iter()
        .find_map(|candidate| {
            blocks
                .iter()
                .find(|block| language_dir_matches(&block.dir, candidate))
        })
        .or_else(|| blocks.first())
}

fn language_dir_matches(dir: &str, candidate: &str) -> bool {
    let dir = dir.trim().trim_end_matches(['/', '\\']);
    let last = dir.rsplit(['/', '\\']).next().unwrap_or(dir);
    dir.eq_ignore_ascii_case(candidate) || last.eq_ignore_ascii_case(candidate)
}

fn language_candidates(preferred_lang: Option<&str>) -> Vec<String> {
    let Some(raw) = preferred_lang else {
        return Vec::new();
    };
    let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
    if normalized.is_empty() {
        return Vec::new();
    }
    let mut out = vec![normalized.clone()];
    if let Some(short) = normalized.split('_').next()
        && short != normalized
    {
        out.push(short.to_string());
    }
    let named: Vec<String> = out
        .iter()
        .filter_map(|code| language_name(code))
        .map(str::to_string)
        .collect();
    for name in named {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

fn language_name(code: &str) -> Option<&'static str> {
    match code {
        "en" | "en_us" | "en_gb" => Some("english"),
        "pl" | "pl_pl" => Some("polish"),
        "de" | "de_de" => Some("german"),
        "fr" | "fr_fr" => Some("french"),
        "it" | "it_it" => Some("italian"),
        "es" | "es_es" => Some("spanish"),
        _ => None,
    }
}

fn parse_tra_entries(text: &str) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    let mut rest = skip_trivia(text);
    while !rest.is_empty() {
        let Some((key, value, after)) = tra_entry(rest) else {
            break;
        };
        entries.insert(key, value);
        rest = skip_trivia(after);
    }
    entries
}

fn tra_entry(text: &str) -> Option<(String, String, &str)> {
    let reference = text.strip_prefix('@')?;
    let sign = usize::from(reference.starts_with('-'));
    let digits = reference[sign..]
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(reference.len() - sign);
    if digits == 0 {
        return None;
    }
    let end = sign + digits;
    let key = format!("@{}", &reference[..end]);
    let after_equals = skip_trivia(&reference[end..]).strip_prefix('=')?;
    let value_start = skip_trivia(after_equals);
    let (value, used) = tra_string(value_start)?;
    let mut rest = &value_start[used..];
    loop {
        let next = skip_trivia(rest);
        if let Some((_, used)) = tra_string(next) {
            rest = &next[used..];
        } else if let Some(sound) = next.strip_prefix('[') {
            let close = sound.find(']')?;
            rest = &sound[close + 1..];
        } else {
            break;
        }
    }
    Some((key, value.replace("\r\n", "\n").trim().to_string(), rest))
}

fn skip_trivia(text: &str) -> &str {
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start();
        if let Some(comment) = trimmed.strip_prefix("//") {
            rest = comment.find('\n').map_or("", |end| &comment[end..]);
        } else if let Some(comment) = trimmed.strip_prefix("/*") {
            rest = comment.find("*/").map_or("", |end| &comment[end + 2..]);
        } else {
            return trimmed;
        }
    }
}

fn tra_string(text: &str) -> Option<(&str, usize)> {
    quoted_string(text).or_else(|| enclosed(text, "%"))
}

fn quoted_string(text: &str) -> Option<(&str, usize)> {
    if text.starts_with(LONG_TILDE) {
        return enclosed(text, LONG_TILDE);
    }
    enclosed(text, "~").or_else(|| enclosed(text, "\""))
}

fn enclosed<'t>(text: &'t str, delimiter: &str) -> Option<(&'t str, usize)> {
    let body = text.strip_prefix(delimiter)?;
    let end = body.find(delimiter)?;
    Some((&body[..end], delimiter.len() * 2 + end))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{decode_text, parse_tra_entries, tra_map_for_tp2};

    static ROOT_SEQ: AtomicUsize = AtomicUsize::new(0);

    struct TempModsRoot {
        root: PathBuf,
    }

    impl TempModsRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_tra_lookup_test_{}_{}",
                std::process::id(),
                ROOT_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("mods")).expect("create temp mods root");
            Self { root }
        }

        fn write(&self, relative: &str, text: &str) -> PathBuf {
            let path = self.root.join("mods").join(relative);
            fs::create_dir_all(path.parent().expect("file has a parent")).expect("create dir");
            fs::write(&path, text).expect("write file");
            path
        }
    }

    impl Drop for TempModsRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn languages_fixture(root: &TempModsRoot, language_lines: &str) -> PathBuf {
        root.write("mymod/tra/english/setup.tra", "@1 = ~In English~");
        root.write("mymod/tra/german/setup.tra", "@1 = ~Auf Deutsch~");
        root.write("mymod/tra/french/setup.tra", "@1 = ~En francais~");
        root.write(
            "mymod/setup-mymod.tp2",
            &format!("BACKUP ~mymod/backup~\nAUTHOR ~me~\n{language_lines}\nBEGIN ~A~\n"),
        )
    }

    #[test]
    fn a_tra_entry_spans_lines_and_keeps_the_first_string() {
        let entries = parse_tra_entries(
            "@1 = ~First line\r\nsecond line~ ~Female text~ [SOUND1]\n@2 = \"Two\"\n@3 = ~~~~~a ~tilde~ inside~~~~~\n@4 = ~broken",
        );
        assert_eq!(entries["@1"], "First line\nsecond line");
        assert_eq!(entries["@2"], "Two");
        assert_eq!(entries["@3"], "a ~tilde~ inside");
        assert!(!entries.contains_key("@4"));
    }

    #[test]
    fn comments_between_entries_are_skipped() {
        let entries = parse_tra_entries(
            "// header\n@-5 = \"Negative\"\n@1 = ~One~ /* note */\n/* @2 = ~Hidden~ */\n// @3 = ~Also hidden~\n@4 = ~Four~",
        );
        assert_eq!(entries["@-5"], "Negative");
        assert_eq!(entries["@1"], "One");
        assert_eq!(entries["@4"], "Four");
        assert!(!entries.contains_key("@2"));
        assert!(!entries.contains_key("@3"));
    }

    #[test]
    fn the_preferred_language_block_wins() {
        let root = TempModsRoot::new();
        let language_lines = "LANGUAGE ~English~ ~english~ ~mymod/tra/english/setup.tra~\nLANGUAGE ~Deutsch~\n  ~german~\n  ~mymod/tra/%LANGUAGE%/setup.tra~";
        let tp2 = languages_fixture(&root, language_lines);
        let text = fs::read_to_string(&tp2).expect("read tp2");
        let map = tra_map_for_tp2(&tp2, &text, Some("de_DE"));
        assert_eq!(map["@1"], "Auf Deutsch");
    }

    #[test]
    fn a_regional_locale_picks_its_language_block() {
        let root = TempModsRoot::new();
        let language_lines = "LANGUAGE ~English~ ~english~ ~mymod/tra/english/setup.tra~\nLANGUAGE ~Deutsch~ ~german~ ~mymod/tra/german/setup.tra~\nLANGUAGE ~Francais~ ~french~ ~mymod/tra/french/setup.tra~";
        let tp2 = languages_fixture(&root, language_lines);
        let text = fs::read_to_string(&tp2).expect("read tp2");
        assert_eq!(
            tra_map_for_tp2(&tp2, &text, Some("de_AT"))["@1"],
            "Auf Deutsch"
        );
        assert_eq!(
            tra_map_for_tp2(&tp2, &text, Some("fr-CA"))["@1"],
            "En francais"
        );
    }

    #[test]
    fn a_byte_order_mark_is_stripped_from_latin1_text() {
        assert_eq!(decode_text(vec![0xEF, 0xBB, 0xBF, b'a', 0xE9]), "a\u{e9}");
        assert_eq!(decode_text(vec![0xEF, 0xBB, 0xBF, b'a']), "a");
    }

    #[test]
    fn english_is_the_fallback_language() {
        let root = TempModsRoot::new();
        let language_lines = "LANGUAGE ~Francais~ ~french~ ~mymod/tra/french/setup.tra~\nLANGUAGE ~English~ ~english~ ~%MOD_FOLDER%/tra/english/setup.tra~";
        let tp2 = languages_fixture(&root, language_lines);
        let text = fs::read_to_string(&tp2).expect("read tp2");
        let map = tra_map_for_tp2(&tp2, &text, Some("pl_PL"));
        assert_eq!(map["@1"], "In English");
    }
}
