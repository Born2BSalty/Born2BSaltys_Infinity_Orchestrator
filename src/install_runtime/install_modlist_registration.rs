// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use chrono::Utc;
use tracing::{info, warn};

use crate::app::modlist_share::ModlistSharePreview;
use crate::install_runtime::start_hooks::{self, InstallButtonVariant};
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
        forked_from: preview.forked_from.clone(),
        workspace_file_relpath: PathBuf::from("modlists").join(&id).join("workspace.json"),
        ..Default::default()
    };

    registry.entries.push(entry.clone());
    Ok(entry)
}

fn existing_entry_id_for_destination(
    registry: &ModlistRegistry,
    destination: &str,
) -> Option<String> {
    let dest = destination.trim();
    if dest.is_empty() {
        return None;
    }
    registry
        .entries
        .iter()
        .find(|e| e.destination_folder.trim() == dest)
        .map(|e| e.id.clone())
}

/// Mints or reuses a modlist registry entry before `prepare_install_dirs_and_maybe_import`.
///
/// Setting the ambient before the import write ensures per-modlist isolation for
/// paste-and-install before the import runs. Returns `(id, freshly_minted)` where `freshly_minted`
/// is `true` for a brand-new entry and `false` for a reused one. Callers must roll
/// back a freshly-minted entry if the subsequent import fails.
pub fn early_mint_modlist_id(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
) -> Option<(String, bool)> {
    // Reinstall and destination-dedup reuse: not freshly minted.
    if let Some(id) = orchestrator
        .pending_reinstall_id
        .as_ref()
        .filter(|id| orchestrator.registry.find(id).is_some())
        .cloned()
    {
        return Some((id, false));
    }
    if let Some(id) = existing_entry_id_for_destination(&orchestrator.registry, destination) {
        return Some((id, false));
    }

    // Fresh mint.
    let Some(preview) = orchestrator.install_screen_state.parsed_preview.clone() else {
        warn!(
            target = "orchestrator",
            "early_mint_modlist_id: no parsed preview — cannot create early entry"
        );
        return None;
    };
    let entry =
        match register_install_modlist_paste(&preview, destination, &mut orchestrator.registry) {
            Ok(e) => e,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "early_mint_modlist_id: register_install_modlist_paste failed: {err}"
                );
                return None;
            }
        };
    persist_new_install_workspace(orchestrator, &entry);
    info!(
        target = "orchestrator",
        "early_mint_modlist_id: minted net-new entry {} before import", entry.id
    );
    Some((entry.id, true))
}

/// Rolls back a freshly-minted entry created by `early_mint_modlist_id` when the
/// subsequent import fails. Has no effect on reused (destination-deduped) entries.
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
}

pub fn register_and_write_install_start_artifacts(orchestrator: &mut OrchestratorApp) -> bool {
    let destination = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();

    let Some(modlist_id) = install_start_modlist_id(orchestrator, &destination) else {
        return false;
    };

    let variant = InstallButtonVariant::from_step5_and_reinstall(
        &orchestrator.wizard_state,
        &modlist_id,
        orchestrator.pending_reinstall_id.as_deref(),
    );
    let code_source = orchestrator
        .registry
        .find(&modlist_id)
        .and_then(|e| e.latest_share_code.clone())
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| orchestrator.install_screen_state.import_code.clone());
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
    true
}

fn install_start_modlist_id(
    orchestrator: &mut OrchestratorApp,
    destination: &str,
) -> Option<String> {
    if let Some(id) = orchestrator
        .pending_reinstall_id
        .as_ref()
        .filter(|id| orchestrator.registry.find(id).is_some())
        .cloned()
    {
        info!(
            target = "orchestrator",
            "Install start (Reinstall): reusing existing registry entry {id}"
        );
        return Some(id);
    }
    if let Some(id) = existing_entry_id_for_destination(&orchestrator.registry, destination) {
        info!(
            target = "orchestrator",
            "Install start (Install-Modlist paste): reusing already-registered entry {id}"
        );
        return Some(id);
    }
    register_new_install_start_modlist(orchestrator, destination)
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

    fn preview(
        name: Option<&str>,
        game: &str,
        author: Option<&str>,
        forked_from: Vec<ForkAncestor>,
    ) -> ModlistSharePreview {
        ModlistSharePreview {
            bio_version: "x".to_string(),
            game_install: game.to_string(),
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
            forked_from,
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
    fn existing_entry_id_for_destination_matches_trimmed_nonempty_only() {
        let mut reg = ModlistRegistry::default();
        let p = preview(Some("A"), "EET", None, vec![]);
        let a = register_install_modlist_paste(&p, "D:\\dest one", &mut reg).expect("a");

        assert_eq!(
            existing_entry_id_for_destination(&reg, "  D:\\dest one  "),
            Some(a.id)
        );

        assert_eq!(existing_entry_id_for_destination(&reg, "D:\\other"), None);

        assert_eq!(existing_entry_id_for_destination(&reg, ""), None);
        assert_eq!(existing_entry_id_for_destination(&reg, "   "), None);
    }
}
