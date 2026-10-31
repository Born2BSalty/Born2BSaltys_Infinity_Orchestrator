// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::OnceLock;

use crate::gallery_feed::index::{
    EntryMeta, FeedEntry, IndexFile, entries_from_index, index_entry_from_meta,
};

include!(concat!(env!("OUT_DIR"), "/gallery_snapshot.rs"));

pub(crate) fn entries_from_files<'a>(files: &[(&'a str, &'a [u8])]) -> Vec<FeedEntry> {
    let mut folders: Vec<&str> = files
        .iter()
        .filter_map(|(key, _)| key.strip_suffix("/entry.json"))
        .collect();
    folders.sort_unstable();

    let mut index_entries = Vec::new();
    for folder in folders {
        let meta_key = format!("{folder}/entry.json");
        let Some((_, bytes)) = files.iter().find(|(key, _)| *key == meta_key) else {
            continue;
        };
        let meta: EntryMeta = match serde_json::from_slice(bytes) {
            Ok(meta) => meta,
            Err(err) => {
                tracing::warn!("gallery folder {folder} entry.json failed to parse: {err}");
                continue;
            }
        };
        if meta.id != folder {
            tracing::warn!(
                "gallery folder {folder} has a mismatched id \"{}\"",
                meta.id
            );
            continue;
        }
        let has_cover = files
            .iter()
            .any(|(key, _)| *key == format!("{folder}/cover.png"));
        index_entries.push(index_entry_from_meta(folder, meta, has_cover));
    }

    entries_from_index(
        IndexFile {
            entries: index_entries,
        },
        &|path| {
            files
                .iter()
                .find(|(key, _)| *key == path)
                .map(|(_, bytes)| *bytes)
        },
    )
}

#[must_use]
pub(crate) fn snapshot_entries() -> &'static [FeedEntry] {
    static ENTRIES: OnceLock<Vec<FeedEntry>> = OnceLock::new();
    ENTRIES
        .get_or_init(|| entries_from_files(SNAPSHOT_FILES))
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_biolist::build_biolist;
    use crate::app::modlist_share::preview_modlist_share_code;
    use crate::registry::model::Game;
    use crate::ui::install::gallery::catalog;
    use std::io::Cursor;

    fn cover_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::new(width, height);
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode png");
        buffer.into_inner()
    }

    #[test]
    fn snapshot_holds_every_embedded_folder_in_gallery_order() {
        let entries = snapshot_entries();
        let mut ids: Vec<&str> = entries.iter().map(|entry| entry.id.as_str()).collect();
        let mut folders: Vec<&str> = SNAPSHOT_FILES
            .iter()
            .filter_map(|(key, _)| key.strip_suffix("/entry.json"))
            .collect();
        ids.sort_unstable();
        folders.sort_unstable();
        assert_ne!(folders.len(), 0);
        assert_eq!(ids, folders);
        let order_keys: Vec<(i32, bool, String)> = entries
            .iter()
            .map(|entry| (entry.order, !entry.featured, entry.name.to_lowercase()))
            .collect();
        let mut sorted_keys = order_keys.clone();
        sorted_keys.sort();
        assert_eq!(order_keys, sorted_keys);
        assert_eq!(
            entries.first().expect("gallery is non-empty").id,
            "eet-plus-fixes"
        );
        assert_eq!(
            entries.last().expect("gallery is non-empty").id,
            "born2bsalty-personal-modlist"
        );
        for entry in entries {
            assert!(
                preview_modlist_share_code(&entry.code).is_ok(),
                "{} must preview",
                entry.id
            );
        }
    }

    #[test]
    fn snapshot_entries_with_a_cover_carry_its_bytes() {
        let entries = snapshot_entries();
        let mut any_cover = false;
        for entry in entries {
            let cover_key = format!("{}/cover.png", entry.id);
            let expects_cover = SNAPSHOT_FILES.iter().any(|(key, _)| *key == cover_key);
            any_cover |= expects_cover;
            assert_eq!(
                entry.cover_png.is_some(),
                expects_cover,
                "{} cover presence mismatch",
                entry.id
            );
        }
        assert!(any_cover, "at least one entry must carry a cover");
    }

    #[test]
    fn entries_from_files_builds_an_entry_from_a_synthetic_table() {
        let code = catalog::entries()[0].code.clone();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let cover_bytes = cover_png(460, 215);
        let entry_json = br#"{"id":"synthetic","name":"Synthetic","author":"Author","description":"A description.","tags":["Tag"],"game":"EET","featured":true,"version":"1.0.0"}"#;
        let files: Vec<(&str, &[u8])> = vec![
            ("synthetic/entry.json", entry_json.as_slice()),
            ("synthetic/modlist.biolist", biolist_bytes.as_slice()),
            ("synthetic/cover.png", cover_bytes.as_slice()),
        ];
        let entries = entries_from_files(&files);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "synthetic");
        assert_eq!(entries[0].game, Game::EET);
        assert!(entries[0].cover_png.is_some());
    }

    #[test]
    fn entries_from_files_skips_a_folder_whose_meta_id_differs() {
        let code = catalog::entries()[0].code.clone();
        let biolist_bytes = build_biolist(&code).expect("build biolist");
        let entry_json = br#"{"id":"other","name":"Name","author":"Author","description":"A description.","tags":[],"game":"EET","featured":true,"version":"1.0.0"}"#;
        let files: Vec<(&str, &[u8])> = vec![
            ("folder/entry.json", entry_json.as_slice()),
            ("folder/modlist.biolist", biolist_bytes.as_slice()),
        ];
        assert_eq!(entries_from_files(&files).len(), 0);
    }

    #[test]
    fn entries_from_files_skips_unparsable_meta() {
        let files: Vec<(&str, &[u8])> = vec![("broken/entry.json", b"not json".as_slice())];
        assert_eq!(entries_from_files(&files).len(), 0);
    }

    #[test]
    fn entries_from_files_ignores_a_biolist_without_meta() {
        let files: Vec<(&str, &[u8])> = vec![("x/modlist.biolist", b"anything".as_slice())];
        assert_eq!(entries_from_files(&files).len(), 0);
    }
}

#[cfg(test)]
mod parser_sharing_tests {
    use super::*;

    #[test]
    fn parser_sharing_biolist_snapshot_and_index_keep_canonical_phrase_metadata() {
        let phrases = vec!["Default".to_string(), " Custom 世界 ".to_string()];
        let original = &crate::ui::install::gallery::catalog::entries()[0].code;
        let code =
            crate::registry::share_export::enrich_share_code_with_phrases(original, &phrases)
                .expect("enrich");
        let bytes = crate::app::modlist_biolist::build_biolist(&code).expect("BIO export");
        assert_eq!(
            crate::app::modlist_biolist::read_share_code_from_bytes(&bytes)
                .expect("canonical code"),
            code
        );
        let meta = br#"{"id":"parser-sharing","name":"Parser sharing","author":"Author","description":"Parser sharing fixture","tags":[],"game":"EET","featured":false,"version":"1.0.0"}"#;
        let entries = entries_from_files(&[
            ("parser-sharing/entry.json", meta.as_slice()),
            ("parser-sharing/modlist.biolist", bytes.as_slice()),
        ]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].code, code);
        assert_eq!(
            crate::app::modlist_share::preview_modlist_share_code(&entries[0].code)
                .expect("gallery preview")
                .parser_choice_phrases,
            phrases
        );
        assert_eq!(
            crate::app::modlist_share::decode_share_payload(&entries[0].code)
                .expect("import payload")
                .parser_choice_phrases,
            phrases
        );
    }

    #[test]
    fn manual_wlb_gallery_folder_snapshot_and_index_keep_enriched_canonical_code() {
        use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
        let original = crate::app::modlist_share::encode_share_payload_text(r#"{"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: stale"},"parser_choice_phrases":["Custom"],"opaque":{"keep":42}}"#).expect("code");
        let workspace = ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "MOD/MOD.TP2".to_string(),
                id: 17,
                language: 2,
                wlb_inputs: Some("bio-json-v1:[\"a,b\",\"\"]".to_string()),
            }],
            ..Default::default()
        };
        let code = crate::registry::share_export::overlay_workspace_wlb(&original, &workspace)
            .expect("overlay");
        let bytes = crate::app::modlist_biolist::build_biolist(&code).expect("archive");
        let meta = br#"{"id":"manual-wlb","name":"Manual WLB","author":"Author","description":"Manual WLB fixture","tags":[],"game":"BGEE","featured":false,"version":"1.0.0"}"#;
        let entries = entries_from_files(&[
            ("manual-wlb/entry.json", meta.as_slice()),
            ("manual-wlb/modlist.biolist", bytes.as_slice()),
        ]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].code, code);
        let parsed: EntryMeta = serde_json::from_slice(meta).expect("metadata");
        let index = IndexFile {
            entries: vec![index_entry_from_meta("manual-wlb", parsed, false)],
        };
        let indexed = entries_from_index(index, &|path| {
            (path == "manual-wlb/modlist.biolist").then_some(bytes.as_slice())
        });
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].code, code);
        let payload =
            crate::app::modlist_share::decode_share_payload(&indexed[0].code).expect("payload");
        assert_eq!(payload.parser_choice_phrases, ["Custom"]);
        assert!(
            payload
                .weidu_logs
                .bgee
                .expect("log")
                .contains("bio-json-v1:")
        );
        let root =
            std::env::temp_dir().join(format!("bio-manual-wlb-gallery-{}", std::process::id()));
        let folder = root.join("manual-wlb");
        std::fs::create_dir_all(&folder).expect("directory");
        std::fs::write(folder.join("entry.json"), meta).expect("metadata file");
        std::fs::write(folder.join("modlist.biolist"), &bytes).expect("archive file");
        assert_eq!(crate::gallery_feed::folder::check_folder(&root), Ok(1));
        assert_eq!(
            crate::app::modlist_biolist::read_share_code(&folder.join("modlist.biolist"))
                .expect("canonical"),
            code
        );
        std::fs::remove_file(folder.join("entry.json")).expect("remove metadata");
        std::fs::remove_file(folder.join("modlist.biolist")).expect("remove archive");
        std::fs::remove_dir(folder).expect("remove folder");
        std::fs::remove_dir(root).expect("remove root");
    }
}
