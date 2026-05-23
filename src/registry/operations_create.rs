// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io;
use std::path::PathBuf;

use chrono::Utc;

use crate::app::modlist_share::ForkAncestor;
use crate::registry::errors::RegistryError;
use crate::registry::ids::new_modlist_id;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};

#[derive(Clone, Copy)]
pub(crate) struct ForkedModlistInput<'a> {
    pub(crate) name: &'a str,
    pub(crate) game: Game,
    pub(crate) destination: &'a str,
    pub(crate) user_name: &'a str,
    pub(crate) parent_name: &'a str,
    pub(crate) parent_author: &'a str,
    pub(crate) parent_forked_from: &'a [ForkAncestor],
    pub(crate) parent_mod_count: u32,
    pub(crate) parent_component_count: u32,
}

pub fn create_modlist(
    name: &str,
    game: Game,
    destination: &str,
    registry: &mut ModlistRegistry,
) -> Result<ModlistEntry, RegistryError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let id = new_modlist_id();
    let now = Utc::now();

    let entry = ModlistEntry {
        id: id.clone(),
        name: trimmed.to_string(),
        game,
        destination_folder: destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

pub(crate) fn create_forked_modlist(
    input: ForkedModlistInput<'_>,
    registry: &mut ModlistRegistry,
) -> Result<ModlistEntry, RegistryError> {
    let trimmed = input.name.trim();
    if trimmed.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let id = new_modlist_id();
    let now = Utc::now();

    let author = {
        let t = input.user_name.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };

    let mut forked_from = input.parent_forked_from.to_vec();
    forked_from.push(ForkAncestor {
        name: input.parent_name.to_string(),
        author: input.parent_author.to_string(),
    });

    let entry = ModlistEntry {
        id: id.clone(),
        name: trimmed.to_string(),
        game: input.game,
        destination_folder: input.destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        author,
        forked_from,
        mod_count: input.parent_mod_count,
        component_count: input.parent_component_count,
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn fork_input<'a>(
        name: &'a str,
        game: Game,
        destination: &'a str,
        user_name: &'a str,
        parent_name: &'a str,
        parent_author: &'a str,
        parent_forked_from: &'a [ForkAncestor],
    ) -> ForkedModlistInput<'a> {
        ForkedModlistInput {
            name,
            game,
            destination,
            user_name,
            parent_name,
            parent_author,
            parent_forked_from,
            parent_mod_count: 0,
            parent_component_count: 0,
        }
    }

    #[test]
    fn create_inserts_in_progress_entry_and_returns_it() {
        let mut reg = ModlistRegistry::default();
        let entry =
            create_modlist("Tactical EET 2026", Game::EET, "D:\\eet", &mut reg).expect("create ok");

        assert_eq!(entry.name, "Tactical EET 2026");
        assert_eq!(entry.game, Game::EET);
        assert_eq!(entry.destination_folder, "D:\\eet");
        assert_eq!(entry.state, ModlistState::InProgress);
        assert_eq!(entry.id.len(), 12, "ULID-style 12-char id");
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&entry.id).unwrap().name, "Tactical EET 2026");

        assert_eq!(entry.author, None);
        assert!(entry.forked_from.is_empty());
    }

    #[test]
    fn workspace_relpath_is_modlists_id_workspace_json() {
        let mut reg = ModlistRegistry::default();
        let entry = create_modlist("X", Game::BGEE, "", &mut reg).expect("ok");
        assert_eq!(
            entry.workspace_file_relpath,
            PathBuf::from("modlists")
                .join(&entry.id)
                .join("workspace.json")
        );
    }

    #[test]
    fn name_and_destination_are_trimmed() {
        let mut reg = ModlistRegistry::default();
        let entry =
            create_modlist("   Spaced Name   ", Game::BG2EE, "  /x  ", &mut reg).expect("ok");
        assert_eq!(entry.name, "Spaced Name");
        assert_eq!(entry.destination_folder, "/x");
    }

    #[test]
    fn empty_name_is_rejected_and_registry_untouched() {
        let mut reg = ModlistRegistry::default();
        let err = create_modlist("   ", Game::EET, "/x", &mut reg).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert!(reg.entries.is_empty(), "no entry added on rejection");
    }

    #[test]
    fn each_create_gets_a_distinct_id() {
        let mut reg = ModlistRegistry::default();
        let a = create_modlist("A", Game::EET, "", &mut reg).expect("a");
        let b = create_modlist("B", Game::EET, "", &mut reg).expect("b");
        assert_ne!(a.id, b.id, "ids must be unique");
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn iwdee_game_is_preserved() {
        let mut reg = ModlistRegistry::default();
        let entry = create_modlist("Icewind", Game::IWDEE, "/iwd", &mut reg).expect("ok");
        assert_eq!(entry.game, Game::IWDEE);
        assert_eq!(reg.find(&entry.id).unwrap().game, Game::IWDEE);
    }

    #[test]
    fn fork_of_a_root_appends_the_immediate_parent_only() {
        let mut reg = ModlistRegistry::default();
        let child = create_forked_modlist(
            fork_input(
                "My EET fork",
                Game::EET,
                "D:\\fork",
                "  @me  ",
                "Born2BSalty's EET",
                "@b2bs",
                &[],
            ),
            &mut reg,
        )
        .expect("fork ok");

        assert_eq!(child.name, "My EET fork");
        assert_eq!(child.game, Game::EET);
        assert_eq!(child.destination_folder, "D:\\fork");
        assert_eq!(child.state, ModlistState::InProgress);
        assert_eq!(child.author.as_deref(), Some("@me"), "author trimmed");
        assert_eq!(child.forked_from.len(), 1);
        assert_eq!(child.forked_from[0].name, "Born2BSalty's EET");
        assert_eq!(child.forked_from[0].author, "@b2bs");
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&child.id).unwrap().forked_from.len(), 1);
    }

    #[test]
    fn fork_of_a_fork_is_append_only_credit_preserved() {
        let parent_chain = vec![
            ForkAncestor {
                name: "Original".to_string(),
                author: "@root".to_string(),
            },
            ForkAncestor {
                name: "Mid".to_string(),
                author: "@mid".to_string(),
            },
        ];
        let mut reg = ModlistRegistry::default();
        let child = create_forked_modlist(
            fork_input(
                "Deep fork",
                Game::BG2EE,
                "/d",
                "@forker",
                "Parent build",
                "@parent",
                &parent_chain,
            ),
            &mut reg,
        )
        .expect("ok");

        assert_eq!(child.forked_from.len(), 3, "parent chain + parent");

        assert_eq!(child.forked_from[0].name, "Original");
        assert_eq!(child.forked_from[0].author, "@root");
        assert_eq!(child.forked_from[1].name, "Mid");
        assert_eq!(child.forked_from[1].author, "@mid");

        assert_eq!(child.forked_from[2].name, "Parent build");
        assert_eq!(child.forked_from[2].author, "@parent");

        assert!(
            !child.forked_from.iter().any(|a| a.name == "Deep fork"),
            "a modlist's own identity must never appear in its own forked_from"
        );
    }

    #[test]
    fn empty_user_name_yields_none_author() {
        let mut reg = ModlistRegistry::default();
        let child = create_forked_modlist(
            fork_input("F", Game::EET, "", "   ", "P", "@p", &[]),
            &mut reg,
        )
        .expect("ok");
        assert_eq!(child.author, None);

        assert_eq!(child.forked_from.len(), 1);
        assert_eq!(child.forked_from[0].author, "@p");
    }

    #[test]
    fn fork_empty_name_is_rejected_and_registry_untouched() {
        let mut reg = ModlistRegistry::default();
        let err = create_forked_modlist(
            fork_input("  ", Game::EET, "/x", "@me", "P", "@p", &[]),
            &mut reg,
        )
        .unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert!(reg.entries.is_empty(), "no entry added on rejection");
    }

    #[test]
    fn fork_gets_distinct_id_and_workspace_relpath() {
        let mut reg = ModlistRegistry::default();
        let a = create_forked_modlist(
            fork_input("A", Game::EET, "", "@m", "P", "@p", &[]),
            &mut reg,
        )
        .expect("a");
        let b = create_forked_modlist(
            fork_input("B", Game::EET, "", "@m", "P", "@p", &[]),
            &mut reg,
        )
        .expect("b");
        assert_ne!(a.id, b.id, "fork ids must be unique");
        assert_eq!(
            a.workspace_file_relpath,
            PathBuf::from("modlists").join(&a.id).join("workspace.json")
        );
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn parent_chain_is_not_aliased_or_mutated_by_caller() {
        let parent_chain = vec![ForkAncestor {
            name: "Root".to_string(),
            author: "@r".to_string(),
        }];
        let mut reg = ModlistRegistry::default();
        let _ = create_forked_modlist(
            fork_input("C", Game::EET, "", "@m", "P", "@p", &parent_chain),
            &mut reg,
        )
        .expect("ok");
        assert_eq!(parent_chain.len(), 1, "caller's parent chain untouched");
        assert_eq!(parent_chain[0].name, "Root");
    }

    use std::io::{Read as _, Write as _};

    use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};

    const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    const SHARE_PREFIX: &str = "BIO-MODLIST-V1:";

    fn b64url_encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            out.push(B64URL[(b0 >> 2) as usize] as char);
            out.push(B64URL[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                out.push(B64URL[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
            }
            if chunk.len() > 2 {
                out.push(B64URL[(b2 & 0b0011_1111) as usize] as char);
            }
        }
        out
    }

    fn b64url_decode(text: &str) -> Vec<u8> {
        let mut values = Vec::new();
        for ch in text.chars().filter(|c| !c.is_whitespace()) {
            match ch {
                'A'..='Z' => values.push(ch as u8 - b'A'),
                'a'..='z' => values.push(ch as u8 - b'a' + 26),
                '0'..='9' => values.push(ch as u8 - b'0' + 52),
                '-' => values.push(62),
                '_' => values.push(63),
                _ => panic!("non-base64url char in base BIO code: {ch}"),
            }
        }
        let remainder = values.len() % 4;
        assert_ne!(remainder, 1, "invalid base64url length in base code");
        if remainder != 0 {
            values.extend(std::iter::repeat_n(64, 4 - remainder));
        }
        let mut out = Vec::with_capacity(values.len() / 4 * 3);
        for chunk in values.chunks(4) {
            let mut pad = 0;
            for value in chunk {
                if *value == 64 {
                    pad += 1;
                }
            }
            let c0 = chunk[0];
            let c1 = chunk[1];
            let c2 = if chunk[2] == 64 { 0 } else { chunk[2] };
            let c3 = if chunk[3] == 64 { 0 } else { chunk[3] };
            out.push((c0 << 2) | (c1 >> 4));
            if pad < 2 {
                out.push(((c1 & 0b0000_1111) << 4) | (c2 >> 2));
            }
            if pad == 0 {
                out.push(((c2 & 0b0000_0011) << 6) | c3);
            }
        }
        out
    }

    fn zlib_deflate(bytes: &[u8]) -> Vec<u8> {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(bytes).expect("deflate write");
        e.finish().expect("deflate finish")
    }

    fn zlib_inflate(bytes: &[u8]) -> Vec<u8> {
        let mut d = ZlibDecoder::new(bytes);
        let mut out = Vec::new();
        d.read_to_end(&mut out).expect("inflate");
        out
    }

    fn minimal_export_state() -> crate::app::state::WizardState {
        let mut st = crate::app::state::WizardState::default();

        st.step3.bgee_items = vec![crate::app::state::Step3ItemState {
            tp_file: "EEFIXPACK.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "EEFixPack".to_string(),
            component_label: "Core Fixes".to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }];
        st
    }

    fn mint_forked_provenance_code(
        name: &str,
        author: &str,
        forked_from: &[ForkAncestor],
        allow_auto_install: bool,
    ) -> String {
        let base = crate::app::modlist_share::export_modlist_share_code(&minimal_export_state())
            .expect("BIO base export must succeed for the minimal state");

        let encoded = base
            .strip_prefix(SHARE_PREFIX)
            .expect("BIO base code must carry the BIO-MODLIST-V1: prefix");
        let json_bytes = zlib_inflate(&b64url_decode(encoded));
        let mut payload: serde_json::Value =
            serde_json::from_slice(&json_bytes).expect("BIO payload must be JSON");

        let obj = payload
            .as_object_mut()
            .expect("BIO payload must be a JSON object");
        obj.insert("name".to_string(), serde_json::json!(name));
        obj.insert("author".to_string(), serde_json::json!(author));
        obj.insert(
            "forked_from".to_string(),
            serde_json::json!(
                forked_from
                    .iter()
                    .map(|a| serde_json::json!({ "name": a.name, "author": a.author }))
                    .collect::<Vec<_>>()
            ),
        );
        obj.insert(
            "allow_auto_install".to_string(),
            serde_json::json!(allow_auto_install),
        );

        let re_json = serde_json::to_vec(&payload).expect("re-serialize");
        format!("{SHARE_PREFIX}{}", b64url_encode(&zlib_deflate(&re_json)))
    }

    #[test]
    fn mint_emits_a_bio_decodable_forked_provenance_code() {
        let lineage = vec![
            ForkAncestor {
                name: "Born2BSalty's EET Basics".to_string(),
                author: "@b2bs".to_string(),
            },
            ForkAncestor {
                name: "EET Tactical Mid".to_string(),
                author: "@olim".to_string(),
            },
        ];
        let code =
            mint_forked_provenance_code("Tactical EET 2026 (shared)", "@b2bs", &lineage, true);

        let preview = crate::app::modlist_share::preview_modlist_share_code(&code)
            .expect("BIO must decode the minted code (proves the envelope is bit-correct)");
        assert_eq!(preview.name.as_deref(), Some("Tactical EET 2026 (shared)"));
        assert_eq!(preview.author.as_deref(), Some("@b2bs"));
        assert!(
            preview.allow_auto_install,
            "injected allow_auto_install=true must survive the round-trip"
        );
        assert_eq!(preview.forked_from.len(), 2, "2-deep lineage preserved");
        assert_eq!(preview.forked_from[0].name, "Born2BSalty's EET Basics");
        assert_eq!(preview.forked_from[0].author, "@b2bs");
        assert_eq!(preview.forked_from[1].name, "EET Tactical Mid");
        assert_eq!(preview.forked_from[1].author, "@olim");

        println!("\n=== 7c MINTED BIO-MODLIST-V1 (forked provenance) ===");
        println!("{code}");
        println!("=== end minted code ===\n");
    }
}
