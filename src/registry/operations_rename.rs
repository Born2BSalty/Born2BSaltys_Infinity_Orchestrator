// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io;

use crate::registry::errors::RegistryError;
use crate::registry::model::ModlistRegistry;

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
}
