// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::Utc;
use tracing::{info, warn};

use crate::app::modlist_share::ModlistSharePreview;
use crate::install_runtime::replaced_owners;
use crate::install_runtime::start_hooks::{self, InstallButtonVariant};
use crate::registry::destination_claim::{
    ClaimContext, DestinationClaim, resolve_destination_claim,
};
use crate::registry::errors::RegistryError;
use crate::registry::ids::new_modlist_id;
use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

const FALLBACK_NAME: &str = "Shared modlist";

pub(crate) fn register_install_modlist_paste(
    preview: &ModlistSharePreview,
    destination: &str,
    registry: &mut ModlistRegistry,
) -> Result<ModlistEntry, RegistryError> {
    let name = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_NAME)
        .to_string();
    if name.trim().is_empty() {
        return Err(RegistryError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "modlist name cannot be empty",
        )));
    }

    let game = Game::from_legacy_string(&preview.game_install);

    let author = preview
        .author
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let description = preview
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let id = new_modlist_id();
    let now = Utc::now();

    let entry = ModlistEntry {
        id: id.clone(),
        name,
        game,
        destination_folder: destination.trim().to_string(),
        state: ModlistState::InProgress,
        creation_date: now,
        last_touched_date: now,
        author,
        description,
        forked_from: preview.forked_from.clone(),
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

fn replace_existing_destination_entries(
    orchestrator: &mut OrchestratorApp,
    old_ids: &[String],
    destination: &str,
) -> Result<(String, bool), String> {
    let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
        warn!(
            target = "orchestrator",
            "early_mint_modlist_id: destination is owned by {old_ids:?} but there is no \
             parsed preview — cannot replace it"
        );
        return Err(format!(
            "the modlist at {destination} could not be replaced: the share code has no preview"
        ));
    };

    let old_names: Vec<String> = old_ids
        .iter()
        .filter_map(|id| orchestrator.registry.find(id).map(|e| e.name.clone()))
        .collect();

    let held = replaced_owners::hold_owners(orchestrator, old_ids)
        .map_err(|err| format!("the modlist at {destination} could not be replaced: {err}"))?;
    orchestrator.pending_replaced_entry = Some(held);

    let entry =
        match register_install_modlist_paste(&preview, destination, &mut orchestrator.registry) {
            Ok(e) => e,
            Err(err) => {
                restore_pending_replaced_entry(orchestrator);
                warn!(
                    target = "orchestrator",
                    "early_mint_modlist_id: register_install_modlist_paste failed while \
                     replacing {old_ids:?}: {err}"
                );
                return Err(format!(
                    "'{}' could not be replaced: {err}",
                    old_names.join("', '")
                ));
            }
        };
    if let Some(replaced) = orchestrator.pending_replaced_entry.as_mut() {
        replaced.replacement_id = Some(entry.id.clone());
    }
    persist_new_install_workspace(orchestrator, &entry);
    info!(
        target = "orchestrator",
        "early_mint_modlist_id: entries {old_ids:?} were replaced by {} \"{}\" at {destination}",
        entry.id,
        entry.name
    );
    Ok((entry.id, true))
}

pub fn early_mint_modlist_id(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
    held_id: Option<&str>,
) -> Result<Option<(String, bool)>, String> {
    let claim = resolve_destination_claim(
        &orchestrator.registry,
        &ClaimContext {
            destination,
            held_id,
            installing_id: orchestrator.active_install_modlist_id.as_deref(),
        },
    );
    match claim {
        DestinationClaim::Adopt(id) => Ok(Some((id, false))),
        DestinationClaim::Free => {
            let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
                warn!(
                    target = "orchestrator",
                    "early_mint_modlist_id: no parsed preview — cannot create early entry"
                );
                return Ok(None);
            };
            let entry = match register_install_modlist_paste(
                &preview,
                destination,
                &mut orchestrator.registry,
            ) {
                Ok(e) => e,
                Err(err) => {
                    warn!(
                        target = "orchestrator",
                        "early_mint_modlist_id: register_install_modlist_paste failed: {err}"
                    );
                    return Ok(None);
                }
            };
            persist_new_install_workspace(orchestrator, &entry);
            info!(
                target = "orchestrator",
                "early_mint_modlist_id: minted net-new entry {} before import", entry.id
            );
            Ok(Some((entry.id, true)))
        }
        DestinationClaim::Replace(ids) => {
            replace_existing_destination_entries(orchestrator, &ids, destination).map(Some)
        }
        DestinationClaim::Refused(refusal) => Err(refusal.message(&orchestrator.registry)),
    }
}

fn restore_pending_replaced_entry(orchestrator: &mut OrchestratorApp) {
    if let Some(held) = orchestrator.pending_replaced_entry.take() {
        replaced_owners::restore_owners(orchestrator, held);
    }
}

pub fn rollback_early_minted_entry(orchestrator: &mut OrchestratorApp, id: &str) {
    let before = orchestrator.registry.entries.len();
    orchestrator.registry.entries.retain(|e| e.id != id);
    if orchestrator.registry.entries.len() < before {
        info!(
            target = "orchestrator",
            "rollback_early_minted_entry: removed {id}"
        );
        if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
            warn!(
                target = "orchestrator",
                "rollback_early_minted_entry: registry persist failed: {err}"
            );
        }
        orchestrator
            .persistence_cycle
            .mark_registry_dirty(std::time::Instant::now());
    }
    restore_pending_replaced_entry(orchestrator);
}

pub fn register_and_write_install_start_artifacts(
    orchestrator: &mut OrchestratorApp,
    settled_id: Option<&str>,
) -> bool {
    let destination = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();

    let Some(modlist_id) = install_start_modlist_id(orchestrator, &destination, settled_id) else {
        restore_pending_replaced_entry(orchestrator);
        return false;
    };

    let variant = InstallButtonVariant::from_step5_and_reinstall(
        &orchestrator.wizard_state,
        &modlist_id,
        orchestrator.pending_reinstall_id.as_deref(),
    );
    let chosen_code = orchestrator.install_screen_state.import_code.trim();
    let code_source = if chosen_code.is_empty() {
        orchestrator
            .registry
            .find(&modlist_id)
            .and_then(|e| e.latest_share_code.clone())
            .unwrap_or_default()
    } else {
        chosen_code.to_string()
    };
    {
        let OrchestratorApp {
            registry,
            registry_store,
            ..
        } = &mut *orchestrator;
        if let Err(err) = start_hooks::write_install_start_artifacts_with_code(
            &modlist_id,
            variant,
            &code_source,
            registry,
            registry_store,
        ) {
            warn!(
                target = "orchestrator",
                "Install start: write_install_start_artifacts_with_code for \
                 {modlist_id} failed: {err} (non-fatal — the install proceeds; \
                 SPEC §13.14 / mirrors on_install_start's handling)"
            );
        }
    }

    orchestrator.active_install_modlist_id = Some(modlist_id.clone());
    info!(
        target = "orchestrator",
        "Install start: active_install_modlist_id = {modlist_id} (the C3 \
         clean-exit flip will move it InProgress → Installed; it shows on \
         Home In-progress until then)"
    );
    finalize_pending_replaced_entry(orchestrator, &modlist_id);
    true
}

fn finalize_pending_replaced_entry(orchestrator: &mut OrchestratorApp, started_id: &str) {
    if let Some(held) = orchestrator.pending_replaced_entry.take() {
        replaced_owners::finalize_owners(held, started_id);
    }
}

fn install_start_modlist_id(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
    settled_id: Option<&str>,
) -> Option<String> {
    if let Some(id) = settled_id.filter(|id| orchestrator.registry.find(id).is_some()) {
        info!(
            target = "orchestrator",
            "Install start: using the entry settled at arm time {id}"
        );
        return Some(id.to_string());
    }
    let claim = resolve_destination_claim(
        &orchestrator.registry,
        &ClaimContext {
            destination,
            held_id: orchestrator.pending_reinstall_id.as_deref(),
            installing_id: orchestrator.active_install_modlist_id.as_deref(),
        },
    );
    match claim {
        DestinationClaim::Adopt(id) => {
            info!(
                target = "orchestrator",
                "Install start: reusing existing registry entry {id}"
            );
            Some(id)
        }
        DestinationClaim::Free => register_new_install_start_modlist(orchestrator, destination),
        DestinationClaim::Replace(_) | DestinationClaim::Refused(_) => {
            warn!(
                target = "orchestrator",
                "Install start: destination claim is {claim:?} after arming; refusing to register"
            );
            None
        }
    }
}

fn register_new_install_start_modlist(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
) -> Option<String> {
    let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
        warn!(
            target = "orchestrator",
            "Install start: no parsed preview to register an Install-Modlist entry"
        );
        return None;
    };
    let entry =
        match register_install_modlist_paste(&preview, destination, &mut orchestrator.registry) {
            Ok(entry) => entry,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "Install start: register_install_modlist_paste failed: {err}"
                );
                return None;
            }
        };
    persist_new_install_workspace(orchestrator, &entry);
    info!(
        target = "orchestrator",
        "Install start (Install-Modlist paste): registered net-new in-progress entry {}", entry.id
    );
    Some(entry.id)
}

fn persist_new_install_workspace(orchestrator: &mut OrchestratorApp, entry: &ModlistEntry) {
    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let empty = ModlistWorkspaceState::default();
    if let Err(err) = canonical_store.save(&empty) {
        warn!(
            target = "orchestrator",
            "Install start: writing canonical workspace.json for {} failed: {err}", entry.id
        );
    }
    orchestrator.workspace_state.insert(entry.id.clone(), empty);
    orchestrator
        .workspace_stores
        .insert(entry.id.clone(), canonical_store);
    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "Install start: atomic registry persist for the new Install-Modlist entry {} failed: {err}",
            entry.id
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(std::time::Instant::now());
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::app::modlist_share::ForkAncestor;

    struct TempDestGuard(PathBuf);

    impl TempDestGuard {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("bio_install_reg_test_{tag}_{}", std::process::id()));
            Self(path)
        }

        fn as_string(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempDestGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn preview(
        name: Option<&str>,
        game: &str,
        author: Option<&str>,
        forked_from: Vec<ForkAncestor>,
    ) -> ModlistSharePreview {
        preview_with_description(name, game, author, None, forked_from)
    }

    fn preview_with_description(
        name: Option<&str>,
        game: &str,
        author: Option<&str>,
        description: Option<&str>,
        forked_from: Vec<ForkAncestor>,
    ) -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "x".to_string(),
            game_install: game.to_string(),
            game_version: None,
            install_mode: "build-from-scanned-mods".to_string(),
            bgee_entries: 0,
            bg2ee_entries: 0,
            has_source_overrides: false,
            has_installed_refs: false,
            bgee_log_text: String::new(),
            bg2ee_log_text: String::new(),
            source_overrides_text: String::new(),
            installed_refs_text: String::new(),
            mod_config_count: 0,
            mod_configs_text: String::new(),
            allow_auto_install: true,
            name: name.map(str::to_string),
            author: author.map(str::to_string),
            description: description.map(str::to_string),
            forked_from,
            unresolved_mods: Vec::new(),
        }
    }

    #[test]
    fn registers_in_progress_entry_from_preview_packed_name_and_game() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("Tactical EET 2026"), "EET", Some("@b2bs"), vec![]);
        let e = register_install_modlist_paste(&p, "  D:\\eet  ", &mut reg).expect("register ok");

        assert_eq!(e.name, "Tactical EET 2026");
        assert_eq!(e.game, Game::EET, "game = the payload's game (SPEC §4)");
        assert_eq!(e.destination_folder, "D:\\eet", "destination trimmed");
        assert_eq!(
            e.state,
            ModlistState::InProgress,
            "a pasted-code install is in-progress until it succeeds (SPEC §13.1)"
        );
        assert_eq!(
            e.id.len(),
            12,
            "the create_modlist ULID-style id convention"
        );
        assert_eq!(e.author.as_deref(), Some("@b2bs"), "the code's own author");
        assert!(
            e.forked_from.is_empty(),
            "a non-forked code carries no lineage"
        );
        assert_eq!(
            e.workspace_file_relpath,
            PathBuf::from("modlists").join(&e.id).join("workspace.json"),
            "the exact create_modlist workspace_file_relpath convention"
        );
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.find(&e.id).unwrap().name, "Tactical EET 2026");
    }

    #[test]
    fn honest_fallback_name_when_code_has_no_packed_name() {
        let mut reg = ModlistRegistry::default();
        let p = preview(None, "BGEE", None, vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.name, "Shared modlist");
        assert_eq!(e.game, Game::BGEE);
        assert_eq!(e.author, None, "no packed author ⇒ None");
    }

    #[test]
    fn empty_or_whitespace_packed_name_falls_back_not_errors() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("   "), "EET", Some("  "), vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.name, "Shared modlist");
        assert_eq!(e.author, None, "whitespace author ⇒ None");
    }

    #[test]
    fn carries_the_pasted_codes_lineage_verbatim_for_credit() {
        let lineage = vec![
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
        let p = preview(Some("Shared deep build"), "BG2EE", Some("@sharer"), lineage);
        let e = register_install_modlist_paste(&p, "/d", &mut reg).expect("ok");

        assert_eq!(e.forked_from.len(), 2, "the code's chain carried verbatim");
        assert_eq!(e.forked_from[0].name, "Original");
        assert_eq!(e.forked_from[0].author, "@root");
        assert_eq!(e.forked_from[1].name, "Mid");
        assert_eq!(e.forked_from[1].author, "@mid");
        assert_eq!(
            e.author.as_deref(),
            Some("@sharer"),
            "the entry's own author = the code's author (the sharer)"
        );

        assert!(
            !e.forked_from.iter().any(|a| a.name == "Shared deep build"),
            "a modlist's own identity must never appear in its own forked_from"
        );
    }

    #[test]
    fn unknown_game_string_defaults_to_bgee_like_create() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("X"), "???", None, vec![]);
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("ok");
        assert_eq!(e.game, Game::BGEE);
    }

    #[test]
    fn each_registration_gets_a_distinct_id() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("A"), "EET", None, vec![]);
        let a = register_install_modlist_paste(&p, "/a", &mut reg).expect("a");
        let b = register_install_modlist_paste(&p, "/b", &mut reg).expect("b");
        assert_ne!(
            a.id, b.id,
            "ids must be unique (the create_modlist ids convention)"
        );
        assert_eq!(reg.entries.len(), 2);
    }

    #[test]
    fn paste_registration_carries_the_description() {
        let mut reg = ModlistRegistry::default();
        let p = preview_with_description(
            Some("Tactical EET 2026"),
            "EET",
            Some("@b2bs"),
            Some("A tactical build with fixpack"),
            vec![],
        );
        let e = register_install_modlist_paste(&p, "/x", &mut reg).expect("register ok");
        assert_eq!(
            e.description.as_deref(),
            Some("A tactical build with fixpack")
        );

        let blank = preview_with_description(Some("X"), "EET", None, Some("   "), vec![]);
        let e2 = register_install_modlist_paste(&blank, "/y", &mut reg).expect("register ok");
        assert_eq!(e2.description, None, "blank description ⇒ None");
    }

    fn minimal_share_payload(name: &str) -> String {
        format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": {{ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }},
                "name": "{name}"
            }}"#
        )
    }

    fn minimal_share_code(name: &str) -> String {
        crate::app::modlist_share::encode_share_payload_text(&minimal_share_payload(name))
            .expect("mint code")
    }

    fn app_with_dest_entry(tag: &str, dest: &str, entry_name: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.registry.entries.push(ModlistEntry {
            id: "EXISTING00001".to_string(),
            name: entry_name.to_string(),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app
    }

    #[test]
    fn installing_into_an_owned_destination_replaces_the_old_entry() {
        let mut app = app_with_dest_entry("replace-dest", "D:\\dest", "EET Essentials");
        {
            let old = app.registry.find_mut("EXISTING00001").unwrap();
            old.description = Some("Old description".to_string());
            old.latest_share_code = Some(minimal_share_code("EET Essentials"));
        }
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("EET Essentials 2"),
            "EET",
            None,
            Some("Catalog description"),
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let (id, minted) = early_mint_modlist_id(&mut app, "D:\\dest", None)
            .expect("replaced")
            .expect("minted");

        assert!(minted, "a replace mints a genuinely fresh entry");
        assert_ne!(id, "EXISTING00001", "the old id is retired, not reused");
        assert!(
            app.registry.find("EXISTING00001").is_none(),
            "the old entry is gone from the registry"
        );
        let at_dest: Vec<_> = app
            .registry
            .entries
            .iter()
            .filter(|e| e.destination_folder.trim() == "D:\\dest")
            .collect();
        assert_eq!(
            at_dest.len(),
            1,
            "exactly one entry now owns the destination"
        );
        let replaced = at_dest[0];
        assert_eq!(replaced.id, id);
        assert_eq!(replaced.name, "EET Essentials 2");
        assert_eq!(
            replaced.latest_share_code, None,
            "a freshly minted entry carries no stale stored code"
        );
        assert_eq!(replaced.description.as_deref(), Some("Catalog description"));

        let pending = app
            .pending_replaced_entry
            .as_ref()
            .expect("the replaced entry survives, pending finalization");
        assert_eq!(pending.entries.len(), 1);
        assert_eq!(pending.entries[0].1.id, "EXISTING00001");
        assert_eq!(pending.entries[0].1.name, "EET Essentials");
    }

    #[test]
    fn rolling_back_a_replace_restores_the_old_entry() {
        let mut app = app_with_dest_entry("replace-rollback", "D:\\dest", "EET Essentials");
        {
            let old = app.registry.find_mut("EXISTING00001").unwrap();
            old.description = Some("Old description".to_string());
            old.latest_share_code = Some(minimal_share_code("EET Essentials"));
        }
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("EET Essentials 2"),
            "EET",
            None,
            Some("Catalog description"),
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let (new_id, minted) = early_mint_modlist_id(&mut app, "D:\\dest", None)
            .expect("replaced")
            .expect("minted");
        assert!(minted);

        rollback_early_minted_entry(&mut app, &new_id);

        assert!(
            app.registry.find(&new_id).is_none(),
            "the freshly minted entry is gone after rollback"
        );
        let restored = app
            .registry
            .find("EXISTING00001")
            .expect("the replaced entry is restored");
        assert_eq!(restored.name, "EET Essentials");
        assert_eq!(restored.description.as_deref(), Some("Old description"));
        assert_eq!(
            app.registry
                .entries
                .iter()
                .position(|e| e.id == "EXISTING00001"),
            Some(0),
            "restored at its original index"
        );
        assert!(
            app.pending_replaced_entry.is_none(),
            "the pending replace is cleared after rollback"
        );
    }

    #[test]
    fn install_start_finalizes_the_replace() {
        let dest = TempDestGuard::new("replace-finalize");
        let mut app = app_with_dest_entry("replace-finalize", &dest.as_string(), "EET Essentials");
        {
            let old = app.registry.find_mut("EXISTING00001").unwrap();
            old.description = Some("Old description".to_string());
            old.latest_share_code = Some(minimal_share_code("EET Essentials"));
        }
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("EET Essentials 2"),
            "EET",
            None,
            Some("Catalog description"),
            vec![],
        ));
        app.install_screen_state.destination = dest.as_string();

        let (new_id, minted) = early_mint_modlist_id(&mut app, &dest.as_string(), None)
            .expect("replaced")
            .expect("minted");
        assert!(minted);
        let old_data_dir = crate::registry::store_workspace::modlist_data_dir("EXISTING00001");
        assert!(
            app.pending_replaced_entry.is_some(),
            "a replace leaves a pending replaced entry"
        );
        std::fs::create_dir_all(&old_data_dir).expect("seed the old data dir");
        assert!(old_data_dir.is_dir());

        app.pending_reinstall_id = Some(new_id.clone());
        let ok = register_and_write_install_start_artifacts(&mut app, Some(new_id.as_str()));

        assert!(ok, "install-start artifacts registered");
        assert!(
            app.pending_replaced_entry.is_none(),
            "the pending replace is cleared once the new install starts"
        );
        assert!(
            !old_data_dir.exists(),
            "the replaced entry's data dir is gone once the new install starts"
        );
    }

    #[test]
    fn install_start_holds_the_chosen_code_over_the_stored_one() {
        let dest = TempDestGuard::new("held");
        let mut app = OrchestratorApp::new_isolated_for_test("held-code");
        let stored_code = minimal_share_code("Stored Name");
        let chosen_code = crate::app::modlist_share::encode_share_payload_text(
            r#"{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": { "bgee": "~CHOSEN/CHOSEN.TP2~ #0 #0 // The chosen one" },
                "name": "Chosen Name"
            }"#,
        )
        .expect("mint code");
        app.registry.entries.push(ModlistEntry {
            id: "HELDCODE0001".to_string(),
            name: "Stored Name".to_string(),
            game: Game::BGEE,
            destination_folder: dest.as_string(),
            state: ModlistState::InProgress,
            latest_share_code: Some(stored_code),
            ..Default::default()
        });
        app.install_screen_state.import_code = chosen_code;
        app.install_screen_state.destination = dest.as_string();
        app.pending_reinstall_id = Some("HELDCODE0001".to_string());

        let ok = register_and_write_install_start_artifacts(&mut app, None);

        assert!(ok, "install-start artifacts registered");
        let held = app
            .registry
            .find("HELDCODE0001")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("held code present");
        let preview = crate::app::modlist_share::preview_modlist_share_code(&held)
            .expect("held code decodes");
        assert!(
            preview.bgee_log_text.contains("CHOSEN.TP2"),
            "the user-chosen code's content wins over the entry's previously stored one"
        );
        assert_eq!(
            preview.name.as_deref(),
            Some("Stored Name"),
            "the code's name is the entry's name"
        );
        assert!(
            !preview.allow_auto_install,
            "install-start always flips allow_auto_install off"
        );
    }

    #[test]
    fn reinstall_equivalence_holds_the_stored_code_when_import_code_mirrors_it() {
        let dest = TempDestGuard::new("held2");
        let mut app = OrchestratorApp::new_isolated_for_test("held-code-reinstall");
        let stored_code = minimal_share_code("Stored Name");
        app.registry.entries.push(ModlistEntry {
            id: "HELDCODE0002".to_string(),
            name: "Stored Name".to_string(),
            game: Game::BGEE,
            destination_folder: dest.as_string(),
            state: ModlistState::Installed,
            latest_share_code: Some(stored_code.clone()),
            ..Default::default()
        });
        app.install_screen_state.import_code = stored_code;
        app.install_screen_state.destination = dest.as_string();
        app.pending_reinstall_id = Some("HELDCODE0002".to_string());

        let ok = register_and_write_install_start_artifacts(&mut app, None);

        assert!(ok, "install-start artifacts registered");
        let held = app
            .registry
            .find("HELDCODE0002")
            .unwrap()
            .latest_share_code
            .clone()
            .expect("held code present");
        let preview = crate::app::modlist_share::preview_modlist_share_code(&held)
            .expect("held code decodes");
        assert_eq!(preview.name.as_deref(), Some("Stored Name"));
        assert!(!preview.allow_auto_install);
    }

    #[test]
    fn fork_owned_destination_is_adopted_not_replaced() {
        let mut app = app_with_dest_entry("fork-adopt", "D:\\dest", "My fork");
        app.active_install_modlist_id = Some("EXISTING00001".to_string());
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("Parent"),
            "EET",
            None,
            Some("Parent description"),
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let result = early_mint_modlist_id(&mut app, "D:\\dest", Some("EXISTING00001"));

        assert_eq!(
            result,
            Ok(Some(("EXISTING00001".to_string(), false))),
            "the fork's own owned destination is adopted, never replaced"
        );
        assert_eq!(app.registry.entries.len(), 1);
        assert_eq!(app.registry.find("EXISTING00001").unwrap().name, "My fork");
        assert!(app.pending_replaced_entry.is_none());
    }

    #[test]
    fn installing_owner_at_arm_time_is_an_arm_error() {
        let mut app = app_with_dest_entry("busy-arm", "D:\\dest", "Busy List");
        app.active_install_modlist_id = Some("EXISTING00001".to_string());
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("New name"),
            "EET",
            None,
            None,
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let result = early_mint_modlist_id(&mut app, "D:\\dest", None);

        assert!(
            matches!(result, Err(ref msg) if msg.contains("is installing into this folder right now")),
            "got {result:?}"
        );
        assert_eq!(app.registry.entries.len(), 1);
        assert!(app.registry.find("EXISTING00001").is_some());
        assert!(app.pending_replaced_entry.is_none());
    }

    #[test]
    fn replacing_two_owners_removes_both_and_restores_both() {
        let mut app = OrchestratorApp::new_isolated_for_test("replace-two-owners");
        app.registry.entries.push(ModlistEntry {
            id: "OWNERAAAAAA1".to_string(),
            name: "Owner A".to_string(),
            game: Game::EET,
            destination_folder: "D:\\dest".to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app.registry.entries.push(ModlistEntry {
            id: "OWNERBBBBBB2".to_string(),
            name: "Owner B".to_string(),
            game: Game::EET,
            destination_folder: "D:\\dest".to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("New name"),
            "EET",
            None,
            None,
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let (new_id, minted) = early_mint_modlist_id(&mut app, "D:\\dest", None)
            .expect("replaced")
            .expect("minted");
        assert!(minted);
        assert!(app.registry.find("OWNERAAAAAA1").is_none());
        assert!(app.registry.find("OWNERBBBBBB2").is_none());
        assert!(app.registry.find(&new_id).is_some());
        assert_eq!(app.registry.entries.len(), 1);

        rollback_early_minted_entry(&mut app, &new_id);

        assert!(app.registry.find(&new_id).is_none());
        let restored_a = app
            .registry
            .entries
            .iter()
            .position(|e| e.id == "OWNERAAAAAA1")
            .expect("owner A restored");
        let restored_b = app
            .registry
            .entries
            .iter()
            .position(|e| e.id == "OWNERBBBBBB2")
            .expect("owner B restored");
        assert_eq!(restored_a, 0);
        assert_eq!(restored_b, 1);
        assert_eq!(app.registry.entries.len(), 2);
    }

    #[test]
    fn failed_replace_is_an_arm_error_not_an_adoption() {
        let mut app = app_with_dest_entry("replace-noprev", "D:\\dest", "EET Essentials");
        app.install_screen_state.parsed_preview = None;
        app.active_install_modlist_id = None;

        let result = early_mint_modlist_id(&mut app, "D:\\dest", None);

        assert!(
            matches!(result, Err(ref msg) if msg.contains("could not be replaced")),
            "got {result:?}"
        );
        assert!(app.registry.find("EXISTING00001").is_some());
        assert!(app.pending_replaced_entry.is_none());
    }

    struct TempFileGuard(PathBuf);

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn removal_save_failure_restores_the_owner_with_its_folder() {
        let blocker = std::env::temp_dir().join(format!(
            "bio_replace_savefail_{}_blocker",
            std::process::id()
        ));
        std::fs::write(&blocker, b"not a directory").expect("blocker file");
        let _guard = TempFileGuard(blocker.clone());

        let mut app = app_with_dest_entry("replace-savefail", "D:\\dest", "EET Essentials");
        app.registry_store =
            crate::registry::store::RegistryStore::new_with_path(blocker.join("modlists.json"));
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("EET Essentials 2"),
            "EET",
            None,
            None,
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        let result = early_mint_modlist_id(&mut app, "D:\\dest", None);

        assert!(
            matches!(result, Err(ref msg) if msg.contains("could not be replaced")),
            "got {result:?}"
        );
        let restored = app
            .registry
            .find("EXISTING00001")
            .expect("the owner whose removal failed is back");
        assert_eq!(restored.destination_folder, "D:\\dest");
        assert_eq!(restored.name, "EET Essentials");
        assert_eq!(app.registry.entries.len(), 1);
        assert!(app.pending_replaced_entry.is_none());
    }

    #[test]
    fn install_start_failure_restores_the_replaced_entry() {
        let mut app = app_with_dest_entry("replace-restore", "D:\\dest", "EET Essentials");
        {
            let old = app.registry.find_mut("EXISTING00001").unwrap();
            old.description = Some("Old description".to_string());
            old.latest_share_code = Some(minimal_share_code("EET Essentials"));
        }
        app.install_screen_state.parsed_preview = Some(preview_with_description(
            Some("EET Essentials 2"),
            "EET",
            None,
            Some("Catalog description"),
            vec![],
        ));
        app.install_screen_state.destination = "D:\\dest".to_string();

        early_mint_modlist_id(&mut app, "D:\\dest", None)
            .expect("replaced")
            .expect("minted");

        app.install_screen_state.parsed_preview = None;
        app.install_screen_state.destination = "D:\\elsewhere".to_string();

        let ok = register_and_write_install_start_artifacts(&mut app, None);

        assert!(!ok, "install start could not resolve a modlist id");
        assert!(app.pending_replaced_entry.is_none());
        let restored = app
            .registry
            .find("EXISTING00001")
            .expect("the replaced entry is restored");
        assert_eq!(restored.name, "EET Essentials");
        assert_eq!(restored.destination_folder, "D:\\dest");
        assert_eq!(app.registry.entries[0].id, "EXISTING00001");
        assert_eq!(
            app.registry.entries.len(),
            1,
            "the orphan replacement entry is retired with the restore"
        );
    }

    struct TempDirGuard(PathBuf);

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finalize_refuses_a_blank_id() {
        let mut app = OrchestratorApp::new_isolated_for_test("finalize-blank");
        let keep_dir = crate::registry::store_workspace::modlist_data_dir("KEEP00000001");
        std::fs::create_dir_all(&keep_dir).expect("mkdir");
        let guard = TempDirGuard(keep_dir.clone());
        app.pending_replaced_entry = Some(replaced_owners::HeldOwners {
            entries: vec![(
                0,
                ModlistEntry {
                    id: String::new(),
                    ..Default::default()
                },
            )],
            replacement_id: None,
        });

        finalize_pending_replaced_entry(&mut app, "NEWID0000001");

        assert!(
            keep_dir.is_dir(),
            "a sibling folder must survive a blank id's refusal"
        );
        assert!(app.pending_replaced_entry.is_none());
        drop(guard);
    }
}
