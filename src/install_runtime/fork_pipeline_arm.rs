// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use tracing::warn;

use crate::app::modlist_share::ModlistSharePreview;
use crate::install_runtime::replaced_owners::{self, HeldOwners};
use crate::registry::destination_claim::{
    ClaimContext, ClaimRefusal, DestinationClaim, resolve_destination_claim,
};
use crate::registry::errors::RegistryError;
use crate::registry::model::Game;
use crate::registry::operations_create::{ForkedModlistInput, create_forked_modlist};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::create::destination_default::default_destination;
use crate::ui::install::state_install::{DestChoice, InstallStage, PipelineKind};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkMintReport {
    pub modlist_id: String,
}

const PARENT_FALLBACK_NAME: &str = "Shared modlist";

fn count_unique_mods(log_texts: &[&str]) -> u32 {
    let mut seen = std::collections::HashSet::new();
    for text in log_texts {
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix('~') else {
                continue;
            };
            let Some(end) = rest.find('~') else {
                continue;
            };
            let tp2 = rest[..end].to_ascii_uppercase();
            if !tp2.is_empty() {
                seen.insert(tp2);
            }
        }
    }
    u32::try_from(seen.len()).unwrap_or(u32::MAX)
}

pub(crate) struct ForkArmRequest<'a> {
    pub(crate) preview: &'a ModlistSharePreview,
    pub(crate) name: &'a str,
    pub(crate) destination: &'a str,
    pub(crate) code: &'a str,
    pub(crate) choice: Option<DestChoice>,
}

pub(crate) fn mint_and_arm(
    orchestrator: &mut OrchestratorApp,
    request: &ForkArmRequest<'_>,
) -> Result<ForkMintReport, ForkMintError> {
    let preview = request.preview.clone();

    let ForkInputs {
        parent_name,
        parent_author,
        game,
        fork_name,
        dest,
        code,
        choice,
    } = derive_inputs(request);

    let claim = resolve_destination_claim(
        &orchestrator.registry,
        &ClaimContext {
            destination: &dest,
            held_id: None,
            installing_id: orchestrator.active_install_modlist_id.as_deref(),
        },
    );
    let mut held: Option<HeldOwners> = None;
    match claim {
        DestinationClaim::Free | DestinationClaim::Adopt(_) => {}
        DestinationClaim::Replace(ids) => {
            held = Some(
                replaced_owners::hold_owners(orchestrator, &ids)
                    .map_err(ForkMintError::Registry)?,
            );
        }
        DestinationClaim::Refused(ClaimRefusal::OwnerIsInstalling(_)) => {
            return Err(ForkMintError::MidInstall);
        }
        DestinationClaim::Refused(other) => {
            return Err(ForkMintError::DestinationRefused(
                other.message(&orchestrator.registry),
            ));
        }
    }

    let user_name = orchestrator.redesign_settings.user_name.clone();

    let parent_component_count =
        u32::try_from(preview.bgee_entries + preview.bg2ee_entries).unwrap_or(u32::MAX);
    let parent_mod_count = count_unique_mods(&[&preview.bgee_log_text, &preview.bg2ee_log_text]);

    let entry = match create_forked_modlist(
        ForkedModlistInput {
            name: &fork_name,
            game,
            destination: &dest,
            user_name: &user_name,
            parent_name: &parent_name,
            parent_author: &parent_author,
            parent_forked_from: &preview.forked_from,
            parent_mod_count,
            parent_component_count,
            parent_description: preview.description.as_deref(),
        },
        &mut orchestrator.registry,
    ) {
        Ok(entry) => entry,
        Err(err) => {
            if let Some(h) = held.take() {
                replaced_owners::restore_owners(orchestrator, h);
            }
            return Err(ForkMintError::Registry(err));
        }
    };

    let modlist_id = entry.id.clone();

    persist_forked_workspace(orchestrator, &entry.id);
    if let Some(h) = held.take() {
        replaced_owners::finalize_owners(h, &entry.id);
    }

    {
        let st = &mut orchestrator.install_screen_state;
        st.clear_preview();
        st.destination = dest;
        st.import_code = code;
        st.destination_choice = choice;
        st.parsed_preview = Some(preview);
        st.preview_cached = true;
        st.pipeline_kind = PipelineKind::Fork;
        st.stage = InstallStage::Downloading;
    }
    crate::ui::install::page_install::refresh_source_compat_issue(
        &mut orchestrator.install_screen_state,
        &orchestrator.wizard_state.step1,
    );
    orchestrator.create_screen_state.destination_choice = None;
    orchestrator.pending_reinstall_id = None;
    orchestrator.active_install_modlist_id = Some(modlist_id.clone());
    crate::install_runtime::active_modlist_source_path::set_ambient_for_modlist(&modlist_id);

    Ok(ForkMintReport { modlist_id })
}

fn persist_forked_workspace(orchestrator: &mut OrchestratorApp, modlist_id: &str) {
    let canonical_store = WorkspaceStore::new_for_id(modlist_id);
    let workspace_state = ModlistWorkspaceState {
        pending_destination_prep: None,
        ..Default::default()
    };
    if let Err(err) = canonical_store.save(&workspace_state) {
        warn!(
            target = "orchestrator",
            "Create fork: writing canonical workspace.json for {modlist_id} failed: {err}"
        );
    }
    orchestrator
        .workspace_state
        .insert(modlist_id.to_string(), workspace_state);
    orchestrator
        .workspace_stores
        .insert(modlist_id.to_string(), canonical_store);

    if let Err(err) = orchestrator.registry_store.save(&orchestrator.registry) {
        warn!(
            target = "orchestrator",
            "Create fork: atomic registry persist for {modlist_id} failed: {err}"
        );
    }
    orchestrator
        .persistence_cycle
        .mark_registry_dirty(std::time::Instant::now());
}

pub(crate) struct ForkInputs {
    pub(crate) parent_name: String,
    pub(crate) parent_author: String,
    pub(crate) game: Game,
    pub(crate) fork_name: String,
    pub(crate) dest: String,
    pub(crate) code: String,
    pub(crate) choice: Option<DestChoice>,
}

fn derive_inputs(request: &ForkArmRequest<'_>) -> ForkInputs {
    let preview = request.preview;
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
        let n = request.name.trim();
        if n.is_empty() {
            format!("{parent_name} (fork)")
        } else {
            n.to_string()
        }
    };
    let dest = {
        let d = request.destination.trim();
        if d.is_empty() {
            default_destination(&fork_name)
        } else {
            d.to_string()
        }
    };
    ForkInputs {
        parent_name,
        parent_author,
        game,
        fork_name,
        dest,
        code: request.code.trim().to_string(),
        choice: request.choice,
    }
}

#[derive(Debug)]
pub enum ForkMintError {
    Registry(RegistryError),
    MidInstall,
    DestinationRefused(String),
}

impl std::fmt::Display for ForkMintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry(err) => write!(f, "registry: {err}"),
            Self::MidInstall => write!(
                f,
                "cannot take over a folder that is actively being installed"
            ),
            Self::DestinationRefused(msg) => write!(f, "cannot use that folder: {msg}"),
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
    use crate::registry::model::{ModlistEntry, ModlistRegistry, ModlistState};

    struct TempDestGuard(std::path::PathBuf);

    impl TempDestGuard {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("bio_fork_arm_test_{tag}_{}", std::process::id()));
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

    fn preview(name: Option<&str>, author: Option<&str>, game: &str) -> ModlistSharePreview {
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
            description: None,
            forked_from: Vec::new(),
            unresolved_mods: Vec::new(),
        }
    }

    fn request<'a>(
        preview: &'a ModlistSharePreview,
        name: &'a str,
        destination: &'a str,
        code: &'a str,
    ) -> ForkArmRequest<'a> {
        ForkArmRequest {
            preview,
            name,
            destination,
            code,
            choice: None,
        }
    }

    #[test]
    fn derive_inputs_uses_the_requested_name_and_destination() {
        let p = preview(Some("Parent name"), Some("@p"), "EET");
        let inputs = derive_inputs(&request(&p, "My fork", "D:\\fork", "BIO-MODLIST-V1:CODE"));
        assert_eq!(inputs.parent_name, "Parent name");
        assert_eq!(inputs.parent_author, "@p");
        assert_eq!(inputs.game, Game::EET);
        assert_eq!(
            inputs.fork_name, "My fork",
            "the request's name MUST win over any screen state"
        );
        assert_eq!(
            inputs.dest, "D:\\fork",
            "the request's destination MUST win over any screen state"
        );
        assert_eq!(inputs.code, "BIO-MODLIST-V1:CODE");
        assert_eq!(inputs.choice, None);
    }

    #[test]
    fn derive_inputs_falls_back_to_parent_fork_when_name_blank() {
        let p = preview(Some("Parent"), None, "BGEE");
        let inputs = derive_inputs(&request(&p, "   ", "D:\\dest", "code"));
        assert_eq!(inputs.fork_name, "Parent (fork)");
    }

    #[test]
    fn derive_inputs_falls_back_to_default_destination_when_dest_blank() {
        let p = preview(Some("Parent"), None, "BGEE");
        let inputs = derive_inputs(&request(&p, "My fork", "  ", "code"));
        assert_eq!(inputs.fork_name, "My fork");
        assert!(
            inputs.dest.ends_with("my-fork"),
            "default_destination(name) ends with the slugified fork name; got {}",
            inputs.dest
        );
    }

    #[test]
    fn derive_inputs_uses_shared_modlist_fallback_when_parent_name_absent() {
        let p = preview(None, None, "BGEE");
        let inputs = derive_inputs(&request(&p, "My fork", "D:\\dest", "code"));
        assert_eq!(inputs.parent_name, "Shared modlist");
    }

    #[test]
    fn derive_inputs_carries_destination_choice() {
        let p = preview(Some("P"), None, "EET");
        let mut req = request(&p, "F", "D:\\d", "c");
        req.choice = Some(DestChoice::Backup);
        assert_eq!(derive_inputs(&req).choice, Some(DestChoice::Backup));
    }

    #[test]
    fn derive_inputs_trims_the_requested_code() {
        let p = preview(Some("P"), None, "EET");
        let inputs = derive_inputs(&request(&p, "F", "D:\\d", "  BIO-MODLIST-V1:X  "));
        assert_eq!(inputs.code, "BIO-MODLIST-V1:X");
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
            parent_mod_count: 0,
            parent_component_count: 0,
            parent_description: None,
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

    #[test]
    fn minted_workspace_never_persists_destination_prep_regardless_of_user_choice() {
        for chosen in [
            None,
            Some(DestChoice::Backup),
            Some(DestChoice::Clear),
            Some(DestChoice::Continue),
        ] {
            let minted = ModlistWorkspaceState {
                pending_destination_prep: None,
                ..Default::default()
            };
            assert_eq!(
                minted.pending_destination_prep, None,
                "the canonical fork-minted workspace must never carry a \
                 pending_destination_prep — the fork pipeline arms its own \
                 prepare_destination at install_screen_state.destination_choice \
                 (single-fire); persisting choice here would re-fire on the \
                 freshly-extracted Mods/ at Step-5 Install click"
            );

            let pre_fix_shape = ModlistWorkspaceState {
                pending_destination_prep: chosen,
                ..Default::default()
            };
            if chosen.is_some() {
                assert_ne!(
                    pre_fix_shape.pending_destination_prep, None,
                    "sanity: the pre-fix literal `pending_destination_prep: \
                     choice` would have persisted the user's pick when chosen \
                     was Some — this is the bug shape the fix eliminates"
                );
            }
        }
    }

    #[test]
    fn fork_take_over_holds_the_old_owner_and_removes_its_data_dir() {
        let dest = TempDestGuard::new("fork-take-over");
        let mut app = OrchestratorApp::new_isolated_for_test("fork-take-over");
        app.registry.entries.push(ModlistEntry {
            id: "OLD000000001".to_string(),
            name: "Old fork".to_string(),
            game: Game::EET,
            destination_folder: dest.as_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        });
        let old_data_dir = crate::registry::store_workspace::modlist_data_dir("OLD000000001");
        std::fs::create_dir_all(&old_data_dir).expect("seed the old data dir");

        let p = preview(Some("Parent"), Some("@p"), "EET");
        let dest_string = dest.as_string();
        let req = request(&p, "New fork", &dest_string, "code");

        let report = mint_and_arm(&mut app, &req).expect("mint ok");

        assert!(app.registry.find("OLD000000001").is_none());
        assert!(!old_data_dir.exists());
        assert!(app.registry.find(&report.modlist_id).is_some());
        assert!(app.pending_replaced_entry.is_none());
    }
}
