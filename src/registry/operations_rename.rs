// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io;

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;
use crate::registry::share_export::{ShareMeta, set_packed_identity};

pub const MAX_DESCRIPTION_CHARS: usize = 500;

pub fn edit_modlist(
    id: &str,
    new_name: &str,
    description: &str,
    registry: &mut ModlistRegistry,
) -> Result<(), RegistryError> {
    let trimmed_name = new_name.trim();
    if trimmed_name.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let trimmed_description = description.trim();
    if trimmed_description.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "description is limited to 500 characters",
        )));
    }

    let Some(entry) = registry.find_mut(id) else {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no modlist with id {id}"),
        )));
    };

    entry.name = trimmed_name.to_string();
    entry.description = Some(trimmed_description.to_string()).filter(|s| !s.is_empty());

    if let Some(code) = entry.latest_share_code.clone()
        && let Ok(stamped) = set_packed_identity(&code, &ShareMeta::from_entry(entry, false))
    {
        entry.set_latest_share_code(stamped);
    }

    Ok(())
}

pub fn rename_modlist(
    id: &str,
    new_name: &str,
    registry: &mut ModlistRegistry,
) -> Result<(), RegistryError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let Some(entry) = registry.find_mut(id) else {
        return Err(RegistryError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no modlist with id {id}"),
        )));
    };

    entry.name = trimmed.to_string();
    if let Some(code) = entry.latest_share_code.clone()
        && let Ok(stamped) = set_packed_identity(&code, &ShareMeta::from_entry(entry, false))
    {
        entry.set_latest_share_code(stamped);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use std::path::PathBuf;

    fn reg_with(id: &str, name: &str, dest: &str) -> ModlistRegistry {
        let mut r = ModlistRegistry::default();
        r.entries.push(ModlistEntry {
            id: id.to_string(),
            name: name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            workspace_file_relpath: PathBuf::from(format!("modlists/{id}/workspace.json")),
            ..Default::default()
        });
        r
    }

    fn minimal_share_code(name: &str) -> String {
        let json = format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": {{ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }},
                "name": "{name}"
            }}"#
        );
        crate::app::modlist_share::encode_share_payload_text(&json).expect("mint code")
    }

    #[test]
    fn edit_modlist_sets_name_and_description_and_rebakes_the_code() {
        let mut r = reg_with("ABC000000000", "old name", "/install/here");
        r.find_mut("ABC000000000").unwrap().latest_share_code =
            Some(minimal_share_code("old name"));

        edit_modlist("ABC000000000", "New Name", "BG2EE with the fixpack", &mut r)
            .expect("edit ok");

        let e = r.find("ABC000000000").unwrap();
        assert_eq!(e.name, "New Name");
        assert_eq!(e.description.as_deref(), Some("BG2EE with the fixpack"));

        let preview = crate::app::modlist_share::preview_modlist_share_code(
            e.latest_share_code.as_deref().unwrap(),
        )
        .expect("preview");
        assert_eq!(preview.name.as_deref(), Some("New Name"));
        assert_eq!(
            preview.description.as_deref(),
            Some("BG2EE with the fixpack")
        );
    }

    #[test]
    fn edit_modlist_rewrites_the_code_file() {
        use crate::registry::share_code_file::IMPORT_CODE_FILENAME;
        use std::sync::atomic::{AtomicU64, Ordering};

        static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

        struct TempRoot(PathBuf);
        impl Drop for TempRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let root = TempRoot(std::env::temp_dir().join(format!(
            "bio_edit_code_file_{}_{}",
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir_all(&root.0).expect("create the destination");
        let file = root.0.join(IMPORT_CODE_FILENAME);
        std::fs::write(&file, "BIO-MODLIST-V1:OLD").expect("seed the old file");
        let mut r = reg_with("ABC000000000", "old name", &root.0.to_string_lossy());
        r.find_mut("ABC000000000").unwrap().latest_share_code =
            Some(minimal_share_code("old name"));

        edit_modlist("ABC000000000", "New Name", "", &mut r).expect("edit ok");

        let stored = r
            .find("ABC000000000")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("code kept");
        let on_disk = std::fs::read_to_string(&file).expect("file present");
        assert_eq!(on_disk, stored);
        let preview =
            crate::app::modlist_share::preview_modlist_share_code(&on_disk).expect("preview");
        assert_eq!(preview.name.as_deref(), Some("New Name"));
    }

    #[test]
    fn edit_modlist_rejects_an_empty_name() {
        let mut r = reg_with("ABC000000000", "keep", "");
        let err = edit_modlist("ABC000000000", "   ", "desc", &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert_eq!(r.find("ABC000000000").unwrap().name, "keep");
    }

    #[test]
    fn edit_modlist_rejects_a_long_description() {
        let mut r = reg_with("ABC000000000", "keep", "");
        let too_long = "x".repeat(501);
        let err = edit_modlist("ABC000000000", "keep", &too_long, &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }
        assert_eq!(r.find("ABC000000000").unwrap().description, None);
    }

    #[test]
    fn edit_modlist_with_a_blank_description_clears_it() {
        let mut r = reg_with("ABC000000000", "keep", "");
        r.find_mut("ABC000000000").unwrap().description = Some("old description".to_string());

        edit_modlist("ABC000000000", "keep", "   ", &mut r).expect("edit ok");

        assert_eq!(r.find("ABC000000000").unwrap().description, None);
    }

    #[test]
    fn edit_modlist_keeps_a_non_bio_code_untouched() {
        let mut r = reg_with("ABC000000000", "old", "");
        r.find_mut("ABC000000000").unwrap().latest_share_code =
            Some("not a share code".to_string());

        edit_modlist("ABC000000000", "New Name", "desc", &mut r).expect("edit ok");

        let e = r.find("ABC000000000").unwrap();
        assert_eq!(e.name, "New Name");
        assert_eq!(e.description.as_deref(), Some("desc"));
        assert_eq!(e.latest_share_code.as_deref(), Some("not a share code"));
    }

    #[test]
    fn rename_updates_name_only() {
        let mut r = reg_with("ABC000000000", "old name", "/install/here");
        rename_modlist("ABC000000000", "new name", &mut r).expect("rename ok");
        let e = r.find("ABC000000000").unwrap();
        assert_eq!(e.name, "new name");
    }

    #[test]
    fn rename_never_touches_destination_or_workspace_path() {
        let mut r = reg_with("ABC000000000", "old", "/games/eet-install");
        let dest_before = r.find("ABC000000000").unwrap().destination_folder.clone();
        let ws_before = r
            .find("ABC000000000")
            .unwrap()
            .workspace_file_relpath
            .clone();

        rename_modlist("ABC000000000", "Totally Different Name", &mut r).expect("ok");

        let e = r.find("ABC000000000").unwrap();
        assert_eq!(
            e.destination_folder, dest_before,
            "destination_folder must be unchanged (SPEC §2.2)"
        );
        assert_eq!(
            e.workspace_file_relpath, ws_before,
            "workspace_file_relpath must be unchanged (SPEC §2.2)"
        );
        assert_eq!(e.name, "Totally Different Name");
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut r = reg_with("ID0000000000", "x", "");
        rename_modlist("ID0000000000", "   spaced name   ", &mut r).expect("ok");
        assert_eq!(r.find("ID0000000000").unwrap().name, "spaced name");
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut r = reg_with("ID0000000000", "keep", "");
        let err = rename_modlist("ID0000000000", "   ", &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            other => panic!("expected Io(InvalidInput), got {other:?}"),
        }

        assert_eq!(r.find("ID0000000000").unwrap().name, "keep");
    }

    #[test]
    fn unknown_id_is_not_found() {
        let mut r = reg_with("REAL00000000", "real", "");
        let err = rename_modlist("GONE00000000", "x", &mut r).unwrap_err();
        match err {
            RegistryError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            other => panic!("expected Io(NotFound), got {other:?}"),
        }
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.find("REAL00000000").unwrap().name, "real");
    }

    #[test]
    fn rename_to_same_name_is_ok_noop() {
        let mut r = reg_with("SAME00000000", "Same Name", "");
        rename_modlist("SAME00000000", "Same Name", &mut r).expect("ok");
        assert_eq!(r.find("SAME00000000").unwrap().name, "Same Name");
    }

    #[test]
    fn rename_rebakes_the_entry_identity_and_keeps_the_rest_of_the_code() {
        use crate::app::modlist_share::ForkAncestor;
        let mut r = reg_with("ABC000000000", "EET Essentials", "/install/here");
        {
            let e = r.find_mut("ABC000000000").unwrap();
            e.latest_share_code = Some(minimal_share_code("EET Essentials"));
            e.author = Some("Xgatt".to_string());
            e.forked_from = vec![ForkAncestor {
                name: "EET Essentials".to_string(),
                author: "BIO Team".to_string(),
            }];
        }
        let before = crate::app::modlist_share::preview_modlist_share_code(
            r.find("ABC000000000")
                .unwrap()
                .latest_share_code
                .as_deref()
                .unwrap(),
        )
        .expect("preview");

        rename_modlist("ABC000000000", "EET Essentials ++++", &mut r).expect("rename ok");

        let e = r.find("ABC000000000").unwrap();
        let after = crate::app::modlist_share::preview_modlist_share_code(
            e.latest_share_code.as_deref().unwrap(),
        )
        .expect("preview");
        assert_eq!(after.name.as_deref(), Some("EET Essentials ++++"));
        assert_eq!(after.author.as_deref(), Some("Xgatt"));
        assert_eq!(after.forked_from, e.forked_from);
        assert_eq!(after.bgee_log_text, before.bgee_log_text);
        assert_eq!(after.bgee_entries, before.bgee_entries);
    }

    #[test]
    fn rename_without_a_cached_code_leaves_it_absent() {
        let mut r = reg_with("ABC000000000", "old", "/install/here");
        rename_modlist("ABC000000000", "new", &mut r).expect("rename ok");
        assert_eq!(r.find("ABC000000000").unwrap().latest_share_code, None);
    }
}
