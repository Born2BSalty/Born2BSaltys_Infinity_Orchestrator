// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use tracing::warn;

use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::errors::RegistryError;
use crate::registry::model::Game;
use crate::registry::operations_create::{ForkedModlistInput, create_forked_modlist};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::destination_default::default_destination;
use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::state_install::{DestChoice, InstallStage, PreviewTab};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

/// Where the fork sub-flow should route after the mint succeeds.
///
/// Carries the minted modlist id so the caller can flip its UI stage to
/// the Downloading screen and (later) navigate the workspace using the
/// same id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkMintReport {
    pub modlist_id: String,
}

const PARENT_FALLBACK_NAME: &str = "Shared modlist";

/// Mint the forked modlist and arm the shared Install-pipeline state.
///
/// Consumes:
/// - `create_screen_state.fork_preview` (must be Some — caller's
///   responsibility to have parsed it).
/// - `create_screen_state.modlist_name` (the fork's chosen name; falls
///   back to "<parent> (fork)" when blank).
/// - `create_screen_state.destination` (falls back to the
///   `default_destination(name)` only when blank).
/// - `create_screen_state.destination_choice` (Clear / Backup; cleared
///   once consumed so a subsequent Cancel→retry does not double-fire).
///
/// Produces a registered fork entry, an on-disk `workspace.json`, the
/// associated `WorkspaceStore`, and populates the orchestrator's
/// `install_screen_state.destination` / `import_code` /
/// `destination_choice` so the shared `stage_downloading::render_live`
/// pipeline (arm + skip + stream + extract) runs against the fork's
/// destination on subsequent frames. Returns the minted modlist id so
/// the caller can route to the Workspace once the pipeline completes.
pub fn mint_and_arm(orchestrator: &mut OrchestratorApp) -> Result<ForkMintReport, ForkMintError> {
    let preview = orchestrator
        .create_screen_state
        .fork_preview
        .clone()
        .ok_or(ForkMintError::NoParsedPreview)?;

    let (parent_name, parent_author, game, fork_name, dest, code, choice) =
        derive_inputs(&preview, &orchestrator.create_screen_state);

    let user_name = orchestrator.redesign_settings.user_name.clone();

    let entry = create_forked_modlist(
        ForkedModlistInput {
            name: &fork_name,
            game,
            destination: &dest,
            user_name: &user_name,
            parent_name: &parent_name,
            parent_author: &parent_author,
            parent_forked_from: &preview.forked_from,
        },
        &mut orchestrator.registry,
    )
    .map_err(ForkMintError::Registry)?;

    let modlist_id = entry.id.clone();

    let canonical_store = WorkspaceStore::new_for_id(&entry.id);
    let workspace_state = ModlistWorkspaceState {
        pending_destination_prep: choice,
        ..Default::default()
    };
    if let Err(err) = canonical_store.save(&workspace_state) {
        warn!(
            target = "orchestrator",
            "Create fork: writing canonical workspace.json for {} failed: {err}", entry.id
        );
    }
    orchestrator
        .workspace_state
        .insert(entry.id.clone(), workspace_state);
    orchestrator
        .workspace_stores
        .insert(entry.id.clone(), canonical_store);

    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "Create fork: atomic registry persist for {} failed: {err}", entry.id
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(std::time::Instant::now());

    {
        let st = &mut orchestrator.install_screen_state;
        st.clear_preview();
        st.destination = dest;
        st.import_code = code;
        st.destination_choice = choice;
        st.parsed_preview = Some(preview);
        st.preview_cached = true;
        st.active_preview_tab = PreviewTab::default();
        st.stage = InstallStage::Downloading;
    }
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.pending_reinstall_id = None;
    orchestrator.active_install_modlist_id = Some(modlist_id.clone());

    Ok(ForkMintReport { modlist_id })
}

fn derive_inputs(
    preview: &ModlistSharePreview,
    state: &CreateScreenState,
) -> (
    String,
    String,
    Game,
    String,
    String,
    String,
    Option<DestChoice>,
) {
    let parent_name = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(PARENT_FALLBACK_NAME)
        .to_string();
    let parent_author = preview.author.as_deref().unwrap_or("").trim().to_string();
    let game = Game::from_legacy_string(&preview.game_install);
    let fork_name = {
        let n = state.modlist_name.trim();
        if n.is_empty() {
            format!("{parent_name} (fork)")
        } else {
            n.to_string()
        }
    };
    let dest = {
        let d = state.destination.trim();
        if d.is_empty() {
            default_destination(&fork_name)
        } else {
            d.to_string()
        }
    };
    let code = state.fork_code.trim().to_string();
    let choice = state.destination_choice;
    (
        parent_name,
        parent_author,
        game,
        fork_name,
        dest,
        code,
        choice,
    )
}

/// Failure modes for [`mint_and_arm`].
#[derive(Debug)]
pub enum ForkMintError {
    NoParsedPreview,
    Registry(RegistryError),
}

impl std::fmt::Display for ForkMintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoParsedPreview => {
                write!(f, "fork import requested without a parsed parent preview")
            }
            Self::Registry(err) => write!(f, "registry: {err}"),
        }
    }
}

impl std::error::Error for ForkMintError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[must_use]
pub fn fork_workspace_relpath(modlist_id: &str) -> PathBuf {
    PathBuf::from("modlists")
        .join(modlist_id)
        .join("workspace.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modlist_share::ForkAncestor;
    use crate::registry::model::ModlistRegistry;
    use crate::ui::create::state_create::{CreateScreenState, StartingPoint};

    fn preview(name: Option<&str>, author: Option<&str>, game: &str) -> ModlistSharePreview {
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
            forked_from: Vec::new(),
        }
    }

    fn state(modlist_name: &str, destination: &str, code: &str) -> CreateScreenState {
        CreateScreenState {
            modlist_name: modlist_name.to_string(),
            destination: destination.to_string(),
            destination_choice: None,
            fork_code: code.to_string(),
            starting_point: StartingPoint::Import,
            ..CreateScreenState::new()
        }
    }

    #[test]
    fn derive_inputs_uses_user_modlist_name_when_present() {
        let p = preview(Some("Parent name"), Some("@p"), "EET");
        let s = state("My fork", "D:\\fork", "BIO-MODLIST-V1:CODE");
        let (parent_name, parent_author, game, fork_name, dest, code, choice) =
            derive_inputs(&p, &s);
        assert_eq!(parent_name, "Parent name");
        assert_eq!(parent_author, "@p");
        assert_eq!(game, Game::EET);
        assert_eq!(fork_name, "My fork", "user's modlist_name MUST win");
        assert_eq!(dest, "D:\\fork", "user's destination MUST win");
        assert_eq!(code, "BIO-MODLIST-V1:CODE");
        assert_eq!(choice, None);
    }

    #[test]
    fn derive_inputs_falls_back_to_parent_fork_when_name_blank() {
        let p = preview(Some("Parent"), None, "BGEE");
        let s = state("   ", "D:\\dest", "code");
        let (_, _, _, fork_name, _, _, _) = derive_inputs(&p, &s);
        assert_eq!(fork_name, "Parent (fork)");
    }

    #[test]
    fn derive_inputs_falls_back_to_default_destination_when_dest_blank() {
        let p = preview(Some("Parent"), None, "BGEE");
        let s = state("My fork", "  ", "code");
        let (_, _, _, fork_name, dest, _, _) = derive_inputs(&p, &s);
        assert_eq!(fork_name, "My fork");
        // default_destination slugifies — "My fork" ⇒ "my-fork" — so
        // assert the slug ends the path rather than the raw name.
        assert!(
            dest.ends_with("my-fork"),
            "default_destination(name) ends with the slugified fork name; got {dest}"
        );
    }

    #[test]
    fn derive_inputs_uses_shared_modlist_fallback_when_parent_name_absent() {
        let p = preview(None, None, "BGEE");
        let s = state("My fork", "D:\\dest", "code");
        let (parent_name, _, _, _, _, _, _) = derive_inputs(&p, &s);
        assert_eq!(parent_name, "Shared modlist");
    }

    #[test]
    fn derive_inputs_carries_destination_choice() {
        let p = preview(Some("P"), None, "EET");
        let mut s = state("F", "D:\\d", "c");
        s.destination_choice = Some(DestChoice::Backup);
        let (_, _, _, _, _, _, choice) = derive_inputs(&p, &s);
        assert_eq!(choice, Some(DestChoice::Backup));
    }

    #[test]
    fn create_forked_modlist_appends_parent_to_lineage() {
        let mut reg = ModlistRegistry::default();
        let existing_chain = vec![ForkAncestor {
            name: "Original".to_string(),
            author: "@root".to_string(),
        }];
        let input = ForkedModlistInput {
            name: "my fork",
            game: Game::EET,
            destination: "D:\\fork",
            user_name: "@me",
            parent_name: "ParentMod",
            parent_author: "@parent",
            parent_forked_from: &existing_chain,
        };
        let entry = create_forked_modlist(input, &mut reg).expect("ok");
        assert_eq!(entry.forked_from.len(), 2);
        assert_eq!(entry.forked_from[0].name, "Original");
        assert_eq!(entry.forked_from[1].name, "ParentMod");
        assert_eq!(entry.forked_from[1].author, "@parent");
        assert_eq!(entry.author.as_deref(), Some("@me"));
    }

    #[test]
    fn fork_workspace_relpath_is_modlists_id_workspace_json() {
        let p = fork_workspace_relpath("ABC123");
        assert_eq!(p, PathBuf::from("modlists/ABC123/workspace.json"));
    }
}
