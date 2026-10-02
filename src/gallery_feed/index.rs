// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use serde::Deserialize;

use crate::app::modlist_biolist::read_share_code_from_bytes;
use crate::app::modlist_share::preview_modlist_share_code;
use crate::registry::model::Game;
use crate::ui::install::gallery::catalog::requirements_for;

pub(crate) const MAX_COVER_BYTES: usize = 150 * 1024;
pub(crate) const MAX_DESCRIPTION_CHARS: usize = 500;
pub(crate) const MAX_NAME_CHARS: usize = 80;
pub(crate) const MAX_AUTHOR_CHARS: usize = 80;
pub(crate) const MAX_VERSION_CHARS: usize = 20;
pub(crate) const MAX_TAG_COUNT: usize = 6;
pub(crate) const MAX_TAG_CHARS: usize = 20;
pub(crate) const MAX_CODE_BYTES: usize = 256 * 1024;
pub(crate) const MIN_COVER_WIDTH: u32 = 460;
pub(crate) const MAX_COVER_WIDTH: u32 = 920;
const COVER_ASPECT_HEIGHT: u32 = 215;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntryMeta {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) game: String,
    pub(crate) featured: bool,
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) requirements: Option<String>,
    #[serde(default)]
    pub(crate) order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexFile {
    pub(crate) entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) game: String,
    pub(crate) featured: bool,
    pub(crate) version: String,
    pub(crate) requirements: Option<String>,
    pub(crate) biolist: String,
    pub(crate) cover: Option<String>,
    pub(crate) order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedEntry {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub game: Game,
    pub featured: bool,
    pub version: String,
    pub requirements: String,
    pub code: String,
    pub cover_png: Option<Vec<u8>>,
    pub game_version: Option<String>,
    pub order: i32,
}

#[must_use]
pub(crate) fn game_from_index_label(label: &str) -> Option<Game> {
    match label {
        "BGEE" => Some(Game::BGEE),
        "BG2EE" => Some(Game::BG2EE),
        "IWDEE" => Some(Game::IWDEE),
        "EET" => Some(Game::EET),
        _ => None,
    }
}

#[must_use]
pub(crate) const fn index_label_for_game(game: Game) -> &'static str {
    game.to_legacy_string()
}

pub(crate) fn is_valid_id(id: &str) -> bool {
    let len = id.chars().count();
    len > 0
        && len <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

pub(crate) const fn cover_matches_aspect(width: u32, height: u32) -> bool {
    let expected_height = width * COVER_ASPECT_HEIGHT / MIN_COVER_WIDTH;
    let diff = height.abs_diff(expected_height);
    let tolerance = expected_height * 2 / 100;
    diff <= tolerance
}

pub(crate) fn validate_cover_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_COVER_BYTES {
        return Err("cover.png is over 150 KB".to_string());
    }
    let decoded = image::load_from_memory(bytes)
        .map_err(|err| format!("cover.png is not a decodable PNG: {err}"))?;
    let width = decoded.width();
    let height = decoded.height();
    if !(MIN_COVER_WIDTH..=MAX_COVER_WIDTH).contains(&width) {
        return Err(format!(
            "cover.png width {width} is outside {MIN_COVER_WIDTH}..={MAX_COVER_WIDTH}"
        ));
    }
    if !cover_matches_aspect(width, height) {
        return Err(format!(
            "cover.png is {width}x{height}, which is not within 2% of the 460:215 aspect"
        ));
    }
    Ok(())
}

fn char_count_in_range(text: &str, max: usize) -> bool {
    let count = text.chars().count();
    count > 0 && count <= max
}

pub(crate) fn text_field_errors(
    name: &str,
    author: &str,
    version: &str,
    description: &str,
    tags: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();
    if !char_count_in_range(name, MAX_NAME_CHARS) {
        errors.push(format!("name must be 1 to {MAX_NAME_CHARS} characters"));
    }
    if !char_count_in_range(author, MAX_AUTHOR_CHARS) {
        errors.push(format!("author must be 1 to {MAX_AUTHOR_CHARS} characters"));
    }
    if !char_count_in_range(version, MAX_VERSION_CHARS) {
        errors.push(format!(
            "version must be 1 to {MAX_VERSION_CHARS} characters"
        ));
    }
    if !char_count_in_range(description, MAX_DESCRIPTION_CHARS) {
        errors.push(format!(
            "description must be 1 to {MAX_DESCRIPTION_CHARS} characters"
        ));
    }
    if tags.len() > MAX_TAG_COUNT {
        errors.push(format!("at most {MAX_TAG_COUNT} tags are allowed"));
    }
    for tag in tags {
        if !char_count_in_range(tag, MAX_TAG_CHARS) {
            errors.push(format!(
                "tag \"{tag}\" must be 1 to {MAX_TAG_CHARS} characters"
            ));
        }
    }
    errors
}

#[must_use]
pub(crate) fn index_entry_from_meta(folder: &str, meta: EntryMeta, has_cover: bool) -> IndexEntry {
    IndexEntry {
        id: meta.id,
        name: meta.name,
        author: meta.author,
        description: meta.description,
        tags: meta.tags,
        game: meta.game,
        featured: meta.featured,
        version: meta.version,
        requirements: meta.requirements,
        biolist: format!("{folder}/modlist.biolist"),
        cover: has_cover.then(|| format!("{folder}/cover.png")),
        order: meta.order,
    }
}

pub(crate) fn entries_from_index<'a>(
    index: IndexFile,
    resolve: &dyn Fn(&str) -> Option<&'a [u8]>,
) -> Vec<FeedEntry> {
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut result = Vec::new();

    for entry in index.entries {
        if !is_valid_id(&entry.id) || !seen_ids.insert(entry.id.clone()) {
            continue;
        }
        let Some(game) = game_from_index_label(&entry.game) else {
            continue;
        };
        if !text_field_errors(
            &entry.name,
            &entry.author,
            &entry.version,
            &entry.description,
            &entry.tags,
        )
        .is_empty()
        {
            continue;
        }
        let Some(biolist_bytes) = resolve(&entry.biolist) else {
            continue;
        };
        let Ok(code) = read_share_code_from_bytes(biolist_bytes) else {
            continue;
        };
        if code.len() > MAX_CODE_BYTES {
            continue;
        }
        let Ok(preview) = preview_modlist_share_code(&code) else {
            continue;
        };
        if preview.game_install != index_label_for_game(game) {
            continue;
        }
        let cover_png = match entry.cover.as_deref() {
            None => None,
            Some(path) => {
                let Some(cover_bytes) = resolve(path) else {
                    continue;
                };
                if validate_cover_bytes(cover_bytes).is_err() {
                    continue;
                }
                Some(cover_bytes.to_vec())
            }
        };
        let requirements = match entry.requirements {
            Some(requirements) if !requirements.is_empty() => requirements,
            _ => requirements_for(game).to_string(),
        };
        let game_version = preview.game_version.clone();
        result.push(FeedEntry {
            id: entry.id,
            name: entry.name,
            author: entry.author,
            description: entry.description,
            tags: entry.tags,
            game,
            featured: entry.featured,
            version: entry.version,
            requirements,
            code,
            cover_png,
            game_version,
            order: entry.order,
        });
    }

    result.sort_by_key(|entry| (entry.order, !entry.featured, entry.name.to_lowercase()));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_biolist::build_biolist;
    use crate::ui::install::gallery::catalog::entries;
    use std::collections::HashMap;
    use std::io::Cursor;

    fn entry(id: &str, name: &str, featured: bool, game: &str, biolist: &str) -> IndexEntry {
        IndexEntry {
            id: id.to_string(),
            name: name.to_string(),
            author: "Author".to_string(),
            description: "A description.".to_string(),
            tags: vec!["Tag".to_string()],
            game: game.to_string(),
            featured,
            version: "1.0.0".to_string(),
            requirements: None,
            biolist: biolist.to_string(),
            cover: None,
            order: 0,
        }
    }

    fn valid_code() -> String {
        entries()[0].code.clone()
    }

    fn cover_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::new(width, height);
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode png");
        buffer.into_inner()
    }

    fn resolver<'a>(files: &'a HashMap<String, Vec<u8>>) -> impl Fn(&str) -> Option<&'a [u8]> + 'a {
        move |path: &str| files.get(path).map(Vec::as_slice)
    }

    #[test]
    fn entries_from_index_accepts_a_valid_two_entry_index() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes.clone());
        files.insert("entries/two.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![
                entry(
                    "eet-essentials-two",
                    "EET Essentials",
                    true,
                    "EET",
                    "entries/one.biolist",
                ),
                entry(
                    "second-list",
                    "Second List",
                    false,
                    "EET",
                    "entries/two.biolist",
                ),
            ],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn entries_from_index_drops_an_entry_with_a_bad_id() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let long_id = "a".repeat(65);
        let index = IndexFile {
            entries: vec![
                entry("Uppercase", "One", true, "EET", "entries/one.biolist"),
                entry(&long_id, "Two", true, "EET", "entries/one.biolist"),
                entry("", "Three", true, "EET", "entries/one.biolist"),
            ],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_a_duplicate_id_keeping_the_first() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![
                entry("dup-id", "First", true, "EET", "entries/one.biolist"),
                entry("dup-id", "Second", true, "EET", "entries/one.biolist"),
            ],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "First");
    }

    #[test]
    fn entries_from_index_drops_an_unknown_game_label() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry(
                "bad-game",
                "One",
                true,
                "SOMEGAME",
                "entries/one.biolist",
            )],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_with_an_oversized_code() {
        let oversized_code = "x".repeat(MAX_CODE_BYTES + 1);
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("share-code.txt", options)
            .expect("start file");
        std::io::Write::write_all(&mut writer, oversized_code.as_bytes()).expect("write");
        let biolist_bytes = writer.finish().expect("finish").into_inner();
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry("big-code", "One", true, "EET", "entries/one.biolist")],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_with_text_over_the_limits() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let mut long_description = entry(
            "long-description",
            "One",
            true,
            "EET",
            "entries/one.biolist",
        );
        long_description.description = "d".repeat(MAX_DESCRIPTION_CHARS + 1);
        let mut many_tags = entry("many-tags", "Two", true, "EET", "entries/one.biolist");
        many_tags.tags = (0..=MAX_TAG_COUNT).map(|n| format!("tag{n}")).collect();
        let mut empty_name = entry("empty-name", "", true, "EET", "entries/one.biolist");
        empty_name.name = String::new();
        let index = IndexFile {
            entries: vec![
                long_description,
                many_tags,
                empty_name,
                entry("fine", "Three", true, "EET", "entries/one.biolist"),
            ],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "fine");
    }

    #[test]
    fn entries_from_index_drops_an_entry_whose_code_does_not_preview() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("share-code.txt", options)
            .expect("start file");
        std::io::Write::write_all(&mut writer, b"not-a-real-code").expect("write");
        let biolist_bytes = writer.finish().expect("finish").into_inner();
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry("bad-code", "One", true, "EET", "entries/one.biolist")],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_whose_game_does_not_match_its_code() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry(
                "mismatched-game",
                "One",
                true,
                "BGEE",
                "entries/one.biolist",
            )],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_whose_biolist_is_missing() {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let index = IndexFile {
            entries: vec![entry(
                "missing-biolist",
                "One",
                true,
                "EET",
                "entries/missing.biolist",
            )],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_whose_biolist_is_not_a_zip() {
        let mut files = HashMap::new();
        files.insert(
            "entries/one.biolist".to_string(),
            b"not a zip file".to_vec(),
        );
        let index = IndexFile {
            entries: vec![entry(
                "not-a-zip",
                "One",
                true,
                "EET",
                "entries/one.biolist",
            )],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_oversized_cover() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        files.insert(
            "entries/oversized.png".to_string(),
            vec![0u8; MAX_COVER_BYTES + 1],
        );
        let mut fixture = entry("oversized-cover", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/oversized.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_a_cover_with_the_wrong_aspect() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        files.insert("entries/wrong.png".to_string(), cover_png(460, 100));
        let mut fixture = entry("wrong-aspect", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/wrong.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_keeps_a_cover_at_460_by_215() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        files.insert("entries/good.png".to_string(), cover_png(460, 215));
        let mut fixture = entry("good-cover", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/good.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 1);
        assert!(result[0].cover_png.is_some());
    }

    #[test]
    fn entries_from_index_keeps_a_cover_referenced_by_path() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        files.insert("entries/cover.png".to_string(), cover_png(460, 215));
        let mut fixture = entry("cover-by-path", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/cover.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].cover_png.as_deref(),
            Some(files.get("entries/cover.png").unwrap().as_slice())
        );
    }

    #[test]
    fn entries_from_index_drops_a_cover_narrower_than_460() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        files.insert("entries/narrow.png".to_string(), cover_png(400, 187));
        let mut fixture = entry("narrow-cover", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/narrow.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_drops_an_entry_whose_cover_file_is_missing() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let mut fixture = entry("missing-cover", "One", true, "EET", "entries/one.biolist");
        fixture.cover = Some("entries/missing.png".to_string());
        let index = IndexFile {
            entries: vec![fixture],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn entries_from_index_sorts_featured_first_then_name_case_insensitive() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![
                entry("z-entry", "zebra", false, "EET", "entries/one.biolist"),
                entry("a-entry", "Apple", true, "EET", "entries/one.biolist"),
                entry("m-entry", "Mango", false, "EET", "entries/one.biolist"),
                entry("b-entry", "banana", true, "EET", "entries/one.biolist"),
            ],
        };
        let result = entries_from_index(index, &resolver(&files));
        let names: Vec<&str> = result.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "banana", "Mango", "zebra"]);
    }

    #[test]
    fn entries_from_index_sorts_by_order_then_featured_then_name() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let mut entry_a = entry("a-entry", "A", true, "EET", "entries/one.biolist");
        entry_a.order = 1;
        let mut entry_z = entry("z-entry", "Z", true, "EET", "entries/one.biolist");
        entry_z.order = 0;
        let mut entry_b = entry("b-entry", "B", false, "EET", "entries/one.biolist");
        entry_b.order = 0;
        let index = IndexFile {
            entries: vec![entry_a, entry_z, entry_b],
        };
        let result = entries_from_index(index, &resolver(&files));
        let names: Vec<&str> = result.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["Z", "B", "A"]);
    }

    #[test]
    fn feed_entry_carries_the_codes_game_version() {
        let tagged_json = r#"{
            "format_version": 1,
            "bio_version": "0.1.0-test",
            "game_install": "EET",
            "game_version": "2.6",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~MOD\\MOD.TP2~ #0 #0 // A component: 1.0" }
        }"#;
        let tagged_code =
            crate::app::modlist_share::encode_share_payload_text(tagged_json).expect("encode");
        let biolist_bytes = build_biolist(&tagged_code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/tagged.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry(
                "tagged-entry",
                "Tagged",
                true,
                "EET",
                "entries/tagged.biolist",
            )],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].game_version.as_deref(), Some("2.6"));

        let untagged_code = valid_code();
        let untagged_bytes = build_biolist(&untagged_code).expect("build biolist");
        let mut untagged_files = HashMap::new();
        untagged_files.insert("entries/one.biolist".to_string(), untagged_bytes);
        let untagged_index = IndexFile {
            entries: vec![entry(
                "untagged-entry",
                "Untagged",
                true,
                "EET",
                "entries/one.biolist",
            )],
        };
        let untagged_result = entries_from_index(untagged_index, &resolver(&untagged_files));
        assert_eq!(untagged_result.len(), 1);
        assert_eq!(untagged_result[0].game_version, None);
    }

    #[test]
    fn requirements_fall_back_to_the_game_default_when_absent() {
        let code = valid_code();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let mut files = HashMap::new();
        files.insert("entries/one.biolist".to_string(), biolist_bytes);
        let index = IndexFile {
            entries: vec![entry("no-reqs", "One", true, "EET", "entries/one.biolist")],
        };
        let result = entries_from_index(index, &resolver(&files));
        assert_eq!(result[0].requirements, requirements_for(Game::EET));
    }

    #[test]
    fn index_entry_from_meta_sets_the_paths() {
        let meta = EntryMeta {
            id: "an-id".to_string(),
            name: "Name".to_string(),
            author: "Author".to_string(),
            description: "A description.".to_string(),
            tags: vec!["Tag".to_string()],
            game: "EET".to_string(),
            featured: true,
            version: "1.0.0".to_string(),
            requirements: None,
            order: 0,
        };
        let with_cover = index_entry_from_meta("an-id", meta.clone(), true);
        assert_eq!(with_cover.biolist, "an-id/modlist.biolist");
        assert_eq!(with_cover.cover, Some("an-id/cover.png".to_string()));

        let without_cover = index_entry_from_meta("an-id", meta, false);
        assert_eq!(without_cover.biolist, "an-id/modlist.biolist");
        assert_eq!(without_cover.cover, None);
    }

    #[test]
    fn description_char_cap_matches_the_edit_modlist_dialog_limit() {
        assert_eq!(MAX_DESCRIPTION_CHARS, 500);
    }

    #[test]
    fn game_labels_round_trip() {
        for game in [Game::BGEE, Game::BG2EE, Game::IWDEE, Game::EET] {
            let label = index_label_for_game(game);
            assert_eq!(game_from_index_label(label), Some(game));
        }
        assert_eq!(game_from_index_label("NOPE"), None);
    }
}
