// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use crate::app::mod_installer_parser;
use crate::registry::share_author::{user_name_is_valid, workspace_marks_own_list};
use crate::registry::share_export::{self, ShareMeta};
use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

impl OrchestratorApp {
    pub(crate) fn list_is_own(&self, id: &str) -> bool {
        self.workspace_state.get(id).map_or_else(
            || {
                WorkspaceStore::new_for_id(id)
                    .load()
                    .is_ok_and(|workspace| workspace_marks_own_list(&workspace))
            },
            workspace_marks_own_list,
        )
    }

    pub(crate) fn share_name_needed(&self, id: &str) -> bool {
        self.redesign_settings.user_name.trim().is_empty() && self.list_is_own(id)
    }

    pub(crate) fn code_for_share(&mut self, id: &str) -> Result<Option<String>, String> {
        let local = mod_installer_parser::capture_local_choice_phrases()?;
        self.code_for_share_with_phrases(id, &local)
    }

    pub(crate) fn code_for_share_with_phrases(
        &mut self,
        id: &str,
        local: &[String],
    ) -> Result<Option<String>, String> {
        let Some(mut candidate) = self.registry.find(id).cloned() else {
            return Ok(None);
        };
        let Some(original_code) = candidate
            .latest_share_code
            .clone()
            .filter(|code| !code.trim().is_empty())
        else {
            return Ok(None);
        };
        let (workspace, is_active) = self.workspace_for_share(id);
        let workspace_phrases = workspace
            .as_ref()
            .and_then(|state| state.parser_choice_phrases.as_deref())
            .unwrap_or(&[]);
        let embedded = crate::app::modlist_share::preview_modlist_share_code(&original_code)?
            .parser_choice_phrases;
        let phrases = mod_installer_parser::union_phrases(&[&embedded, workspace_phrases, local])?;
        let mut code = if phrases == embedded {
            original_code.clone()
        } else {
            share_export::enrich_share_code_with_phrases(&original_code, &phrases)?
        };
        if let Some(workspace) = &workspace {
            code = share_export::overlay_workspace_wlb(&code, workspace)?;
        }
        let typed = self.share_name_buffer.trim().to_string();
        let save_name =
            user_name_is_valid(&typed) && self.redesign_settings.user_name.trim().is_empty();
        let user_name = if save_name {
            typed.as_str()
        } else {
            self.redesign_settings.user_name.as_str()
        };
        let prior_author = candidate.author.clone();
        if workspace.as_ref().is_some_and(workspace_marks_own_list) && !user_name.trim().is_empty()
        {
            candidate.author = Some(user_name.trim().to_string());
            code = share_export::set_packed_identity(
                &code,
                &ShareMeta::from_entry(&candidate, false),
            )?;
        }
        if let Some(refreshed) = workspace {
            self.persist_shared_workspace(id, refreshed, &code, is_active)?;
        }
        if save_name {
            self.notification_manager
                .success(format!("Saved your name to Settings: {typed}"));
            self.redesign_settings.user_name = typed;
        }
        self.share_name_buffer.clear();
        if let Some(entry) = self.registry.find_mut(id) {
            let author_changed = prior_author != candidate.author;
            entry.author = candidate.author;
            if code != original_code {
                entry.set_latest_share_code(code.clone());
            }
            if author_changed || code != original_code {
                self.persistence_cycle.mark_registry_dirty(Instant::now());
            }
        }
        Ok(Some(code))
    }

    fn workspace_for_share(
        &mut self,
        id: &str,
    ) -> (
        Option<crate::registry::workspace_model::ModlistWorkspaceState>,
        bool,
    ) {
        let mut workspace = self
            .workspace_state
            .get(id)
            .cloned()
            .or_else(|| WorkspaceStore::new_for_id(id).load().ok());
        let active_id = self
            .workspace_view
            .loaded_workspace_id
            .as_deref()
            .or(self.active_install_modlist_id.as_deref());
        let is_active = active_id == Some(id);
        if is_active {
            crate::ui::workspace::workspace_state_loader::sync_step3_from_step2_if_changed(
                &mut self.wizard_state,
            );
            workspace = Some(
                crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
                    &self.wizard_state,
                    &workspace.unwrap_or_default(),
                ),
            );
        }
        (workspace, is_active)
    }

    fn persist_shared_workspace(
        &mut self,
        id: &str,
        mut workspace: crate::registry::workspace_model::ModlistWorkspaceState,
        code: &str,
        is_active: bool,
    ) -> Result<(), String> {
        workspace.last_share_code = Some(code.to_string());
        let store = self
            .workspace_stores
            .get(id)
            .cloned()
            .unwrap_or_else(|| WorkspaceStore::new_for_id(id));
        store
            .save(&workspace)
            .map_err(|err| format!("Saving shared WLB workspace failed: {err}"))?;
        self.persistence_cycle
            .last_saved_workspaces
            .insert(id.to_string(), workspace.clone());
        if self.workspace_state.contains_key(id) || is_active {
            self.workspace_state.insert(id.to_string(), workspace);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::modlist_share::{encode_share_payload_text, preview_modlist_share_code};
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use crate::registry::store_workspace::WorkspaceStore;
    use crate::registry::workspace_model::ModlistWorkspaceState;
    use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

    const REGISTRY_DIRTY_KEY: &str = "::registry::";

    fn code_by(author: Option<&str>) -> String {
        let author_line = author.map_or_else(String::new, |a| format!(r#","author": "{a}""#));
        encode_share_payload_text(&format!(
            r#"{{
                "format_version": 1,
                "game_install": "BGEE",
                "install_mode": "start_from_scratch",
                "weidu_logs": {{ "bgee": "~MOD/MOD.TP2~ #0 #0 // A component" }},
                "name": "My BGEE run"{author_line}
            }}"#
        ))
        .expect("mint code")
    }

    fn app_with_entry(tag: &str, id: &str, author: Option<&str>) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.registry.entries.push(ModlistEntry {
            id: id.to_string(),
            name: "My BGEE run".to_string(),
            game: Game::BGEE,
            state: ModlistState::Installed,
            author: author.map(str::to_string),
            latest_share_code: Some(code_by(author)),
            ..Default::default()
        });
        app
    }

    fn own_workspace() -> ModlistWorkspaceState {
        ModlistWorkspaceState {
            scratch_mods_folder: Some("D:\\my run\\mods".to_string()),
            ..Default::default()
        }
    }

    fn isolated_store(app: &OrchestratorApp, id: &str) -> WorkspaceStore {
        let store = WorkspaceStore::new_for_id(id);
        let root = app
            .isolated_test_config_root
            .as_ref()
            .expect("the isolated app owns a temp config root");
        assert!(
            store.path().starts_with(root),
            "the workspace path {} must sit under the temp config root {}",
            store.path().display(),
            root.display()
        );
        store
    }

    fn packed_author(code: &str) -> Option<String> {
        preview_modlist_share_code(code)
            .expect("the code decodes")
            .author
    }

    fn fixture_capture(
        app: &mut OrchestratorApp,
        id: &str,
        parser: &std::path::Path,
    ) -> Result<Option<String>, String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let result =
                crate::app::mod_installer_parser::with_parser_path_for_test(parser, || {
                    app.code_for_share(id)
                });
            if !result.as_ref().is_err_and(|error| error.contains("busy"))
                || std::time::Instant::now() >= deadline
            {
                return result;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn code_for_share_saves_the_typed_name_and_stamps_an_own_list() {
        let mut app = app_with_entry("share-flow-own", "OWNLIST00001", None);
        app.workspace_state
            .insert("OWNLIST00001".to_string(), own_workspace());
        app.redesign_settings.user_name.clear();
        app.share_name_buffer = "  @typed  ".to_string();

        let code = app
            .code_for_share_with_phrases("OWNLIST00001", &[])
            .expect("valid fixture export")
            .expect("a code to share");

        assert_eq!(app.redesign_settings.user_name, "@typed");
        assert_eq!(app.share_name_buffer, "");
        let history = app.notification_manager.history();
        assert_eq!(history.len(), 1);
        assert_eq!(
            history.back().unwrap().text,
            "Saved your name to Settings: @typed"
        );
        let entry = app.registry.find("OWNLIST00001").unwrap();
        assert_eq!(entry.author.as_deref(), Some("@typed"));
        assert_eq!(entry.latest_share_code.as_deref(), Some(code.as_str()));
        assert_eq!(packed_author(&code).as_deref(), Some("@typed"));
        assert!(
            app.persistence_cycle
                .last_dirty_at
                .contains_key(REGISTRY_DIRTY_KEY)
        );
    }

    #[test]
    fn code_for_share_rewrites_the_code_file_for_an_own_list() {
        use crate::registry::share_code_file::IMPORT_CODE_FILENAME;
        let mut app = app_with_entry("share-flow-code-file", "OWNLIST00003", Some("@old"));
        let destination = app
            .isolated_test_config_root
            .as_ref()
            .expect("the isolated app owns a temp config root")
            .join("install here");
        std::fs::create_dir_all(&destination).expect("create the destination");
        let file = destination.join(IMPORT_CODE_FILENAME);
        std::fs::write(&file, "BIO-MODLIST-V1:OLD").expect("seed the old file");
        app.registry
            .find_mut("OWNLIST00003")
            .unwrap()
            .destination_folder = destination.to_string_lossy().into_owned();
        app.workspace_state
            .insert("OWNLIST00003".to_string(), own_workspace());
        app.redesign_settings.user_name = "Xgatt".to_string();

        let code = app
            .code_for_share_with_phrases("OWNLIST00003", &[])
            .expect("valid fixture export")
            .expect("a code to share");

        assert_eq!(packed_author(&code).as_deref(), Some("Xgatt"));
        assert_eq!(std::fs::read_to_string(&file).expect("file present"), code);
    }

    #[test]
    fn code_for_share_does_not_save_an_invalid_typed_name() {
        let mut app = app_with_entry("share-flow-invalid-name", "OWNLIST00004", None);
        app.workspace_state
            .insert("OWNLIST00004".to_string(), own_workspace());
        app.redesign_settings.user_name.clear();
        app.share_name_buffer = "@".to_string();

        let _code = app
            .code_for_share_with_phrases("OWNLIST00004", &[])
            .expect("valid fixture export");

        assert_eq!(app.redesign_settings.user_name, "");
        assert!(app.notification_manager.history().is_empty());
        assert_eq!(
            app.registry.find("OWNLIST00004").unwrap().author.as_deref(),
            None
        );
    }

    #[test]
    fn code_for_share_leaves_an_as_is_list_untouched() {
        let mut app = app_with_entry("share-flow-as-is", "ASISLIST0001", Some("@original"));
        app.workspace_state
            .insert("ASISLIST0001".to_string(), ModlistWorkspaceState::default());
        app.redesign_settings.user_name = "@me".to_string();
        let stored = app
            .registry
            .find("ASISLIST0001")
            .unwrap()
            .latest_share_code
            .clone();

        let code = app
            .code_for_share_with_phrases("ASISLIST0001", &[])
            .expect("valid fixture export");

        assert_eq!(code, stored);
        assert!(app.notification_manager.history().is_empty());
        let entry = app.registry.find("ASISLIST0001").unwrap();
        assert_eq!(entry.author.as_deref(), Some("@original"));
        assert_eq!(
            packed_author(code.as_deref().unwrap()).as_deref(),
            Some("@original")
        );
        assert!(app.persistence_cycle.last_dirty_at.is_empty());
    }

    #[test]
    fn code_for_share_reads_an_unopened_list_from_disk() {
        let mut app = app_with_entry("share-flow-unopened", "UNOPENED0001", Some("@old"));
        isolated_store(&app, "UNOPENED0001")
            .save(&own_workspace())
            .expect("write the workspace");
        app.redesign_settings.user_name = "@new".to_string();
        assert!(!app.workspace_state.contains_key("UNOPENED0001"));

        let code = app
            .code_for_share_with_phrases("UNOPENED0001", &[])
            .expect("valid fixture export")
            .expect("a code to share");

        assert_eq!(packed_author(&code).as_deref(), Some("@new"));
        assert_eq!(
            app.registry.find("UNOPENED0001").unwrap().author.as_deref(),
            Some("@new")
        );
        assert!(!app.workspace_state.contains_key("UNOPENED0001"));
        assert!(!app.workspace_stores.contains_key("UNOPENED0001"));
    }

    #[test]
    fn share_name_needed_only_for_own_lists_with_a_blank_name() {
        let mut app = app_with_entry("share-flow-needed", "OWNLIST00002", None);
        app.registry.entries.push(ModlistEntry {
            id: "ASISLIST0002".to_string(),
            name: "Someone's run".to_string(),
            ..Default::default()
        });
        app.workspace_state
            .insert("OWNLIST00002".to_string(), own_workspace());
        app.workspace_state
            .insert("ASISLIST0002".to_string(), ModlistWorkspaceState::default());

        app.redesign_settings.user_name = "   ".to_string();
        assert!(app.share_name_needed("OWNLIST00002"));
        assert!(!app.share_name_needed("ASISLIST0002"));

        app.redesign_settings.user_name = "@me".to_string();
        assert!(!app.share_name_needed("OWNLIST00002"));
        assert!(!app.share_name_needed("ASISLIST0002"));
    }

    #[test]
    fn unreadable_workspace_counts_as_not_own() {
        let mut app = app_with_entry("share-flow-unreadable", "BROKEN000001", Some("@old"));
        assert!(!app.list_is_own("BROKEN000001"));

        let store = isolated_store(&app, "BROKEN000001");
        std::fs::create_dir_all(store.path().parent().unwrap()).expect("mkdir");
        std::fs::write(store.path(), b"{ not json").expect("write garbage");
        assert!(!app.list_is_own("BROKEN000001"));

        app.redesign_settings.user_name.clear();
        assert!(!app.share_name_needed("BROKEN000001"));
        let stored = app
            .registry
            .find("BROKEN000001")
            .unwrap()
            .latest_share_code
            .clone();
        assert_eq!(
            app.code_for_share_with_phrases("BROKEN000001", &[])
                .expect("valid fixture export"),
            stored
        );
        assert_eq!(
            app.registry.find("BROKEN000001").unwrap().author.as_deref(),
            Some("@old")
        );
    }
    #[test]
    fn parser_sharing_outward_uses_selected_workspace_and_explicit_capture() {
        let mut app = app_with_entry("parser-sharing-outward", "PHRASEOUT001", None);
        let root = app
            .isolated_test_config_root
            .as_ref()
            .expect("fixture root")
            .clone();
        std::fs::create_dir_all(&root).expect("fixture root");
        let parser = root.join("parser.toml");
        std::fs::write(&parser, "choice_phrase = ['Local default', 'Local custom']")
            .expect("fixture");
        let workspace = ModlistWorkspaceState {
            parser_choice_phrases: Some(vec!["Selected workspace".to_string()]),
            ..Default::default()
        };
        isolated_store(&app, "PHRASEOUT001")
            .save(&workspace)
            .expect("selected workspace on disk");
        app.wizard_state.parser_choice_phrases = vec!["Unrelated active wizard".to_string()];
        let code = fixture_capture(&mut app, "PHRASEOUT001", &parser)
            .expect("capture and export")
            .expect("code");
        assert_eq!(
            preview_modlist_share_code(&code)
                .expect("preview")
                .parser_choice_phrases,
            ["Selected workspace", "Local default", "Local custom"]
        );
        let archive = crate::app::modlist_biolist::build_biolist(&code).expect("BIO transport");
        assert!(!archive.is_empty());
    }

    #[test]
    fn parser_sharing_capture_failure_preserves_code_file_author_and_settings() {
        let mut app = app_with_entry(
            "parser-sharing-capture-failure",
            "PHRASEERR001",
            Some("@original"),
        );
        app.workspace_state
            .insert("PHRASEERR001".to_string(), own_workspace());
        let root = app
            .isolated_test_config_root
            .as_ref()
            .expect("fixture root")
            .clone();
        std::fs::create_dir_all(&root).expect("fixture root");
        let parser = root.join("malformed-parser.toml");
        std::fs::write(&parser, "choice_phrase = 42").expect("fixture");
        let destination = root.join("destination");
        std::fs::create_dir_all(&destination).expect("destination");
        let file = destination.join(crate::registry::share_code_file::IMPORT_CODE_FILENAME);
        std::fs::write(&file, "unchanged code file").expect("code file");
        app.registry
            .find_mut("PHRASEERR001")
            .expect("entry")
            .destination_folder = destination.to_string_lossy().into_owned();
        app.redesign_settings.user_name.clear();
        app.share_name_buffer = "@candidate".to_string();
        let original = app
            .registry
            .find("PHRASEERR001")
            .expect("entry")
            .latest_share_code
            .clone();
        let error =
            fixture_capture(&mut app, "PHRASEERR001", &parser).expect_err("malformed capture");
        assert!(error.contains("array"));
        let entry = app.registry.find("PHRASEERR001").expect("entry");
        assert_eq!(entry.latest_share_code, original);
        assert_eq!(entry.author.as_deref(), Some("@original"));
        assert!(app.redesign_settings.user_name.is_empty());
        assert_eq!(app.share_name_buffer, "@candidate");
        assert_eq!(
            std::fs::read_to_string(file).expect("preserved file"),
            "unchanged code file"
        );
    }
}
