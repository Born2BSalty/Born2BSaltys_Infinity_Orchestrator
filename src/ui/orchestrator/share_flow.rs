// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use tracing::warn;

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
        let local = mod_installer_parser::capture_local_recognition_rules().unwrap_or_else(|err| {
            warn!(
                target = "orchestrator",
                "Share for {id} uses no local parser recognition rules: {err}"
            );
            mod_installer_parser::ParserRecognitionRules::new()
        });
        self.code_for_share_with_rules(id, &local)
    }

    #[cfg(test)]
    pub(crate) fn code_for_share_with_phrases(
        &mut self,
        id: &str,
        local: &[String],
    ) -> Result<Option<String>, String> {
        let rules = mod_installer_parser::recognition_rules_with_legacy(
            &mod_installer_parser::ParserRecognitionRules::new(),
            local,
        )?;
        self.code_for_share_with_rules(id, &rules)
    }

    pub(crate) fn code_for_share_with_rules(
        &mut self,
        id: &str,
        local: &mod_installer_parser::ParserRecognitionRules,
    ) -> Result<Option<String>, String> {
        mod_installer_parser::validate_recognition_rules(local)?;
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
        if let Err(err) = crate::app::modlist_share::decode_share_payload(&original_code) {
            let workspace = self
                .workspace_state
                .get(id)
                .cloned()
                .or_else(|| WorkspaceStore::new_for_id(id).load().ok())
                .unwrap_or_default();
            share_export::workspace_recognition_rules("", &workspace)?;
            warn!(target = "orchestrator", "Share keeps stored code: {err}");
            return Ok(Some(original_code));
        }
        let (workspace, is_active) = self.workspace_for_share(id, &original_code, local)?;
        let mut code = merge_workspace_into_code(&original_code, workspace.as_ref(), local)?;
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
        code: &str,
        local: &mod_installer_parser::ParserRecognitionRules,
    ) -> Result<
        (
            Option<crate::registry::workspace_model::ModlistWorkspaceState>,
            bool,
        ),
        String,
    > {
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
        let selected_rules = if is_active {
            let current = mod_installer_parser::recognition_rules_with_legacy(
                &self.wizard_state.parser_recognition_rules,
                &self.wizard_state.parser_choice_phrases,
            )?;
            share_export::active_workspace_recognition_rules(
                code,
                &workspace.clone().unwrap_or_default(),
                &current,
            )?
        } else {
            share_export::workspace_recognition_rules(code, &workspace.clone().unwrap_or_default())?
        };
        mod_installer_parser::union_recognition_rules(&[&selected_rules, local])?;
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
            if let Some(workspace) = workspace.as_mut() {
                workspace.parser_choice_phrases = Some(
                    selected_rules
                        .get("choice_phrase")
                        .cloned()
                        .unwrap_or_default(),
                );
                workspace.parser_recognition_rules = Some(selected_rules);
            }
        }
        Ok((workspace, is_active))
    }

    fn persist_shared_workspace(
        &mut self,
        id: &str,
        mut workspace: crate::registry::workspace_model::ModlistWorkspaceState,
        code: &str,
        is_active: bool,
    ) -> Result<(), String> {
        let payload = crate::app::modlist_share::decode_share_payload(code)?;
        let rules = mod_installer_parser::recognition_rules_with_legacy(
            &payload.parser_recognition_rules,
            &payload.parser_choice_phrases,
        )?;
        let phrases = rules.get("choice_phrase").cloned().unwrap_or_default();
        workspace.parser_recognition_rules = Some(rules.clone());
        workspace.parser_choice_phrases = Some(phrases.clone());
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
        if is_active {
            self.wizard_state.parser_recognition_rules = rules;
            self.wizard_state.parser_choice_phrases = phrases;
        }
        Ok(())
    }
}

fn merge_workspace_into_code(
    original_code: &str,
    workspace: Option<&crate::registry::workspace_model::ModlistWorkspaceState>,
    local: &mod_installer_parser::ParserRecognitionRules,
) -> Result<String, String> {
    let selected = share_export::workspace_recognition_rules(
        original_code,
        &workspace.cloned().unwrap_or_default(),
    )?;
    let rules = mod_installer_parser::union_recognition_rules(&[&selected, local])?;
    let overlaid = workspace
        .map_or_else(
            || Ok(original_code.to_string()),
            |workspace| share_export::overlay_workspace_wlb(original_code, workspace),
        )
        .unwrap_or_else(|err| {
            warn!(
                target = "orchestrator",
                "Share keeps stored component selections: {err}"
            );
            original_code.to_string()
        });
    let payload = crate::app::modlist_share::decode_share_payload(&overlaid)?;
    let embedded = mod_installer_parser::recognition_rules_with_legacy(
        &payload.parser_recognition_rules,
        &payload.parser_choice_phrases,
    )?;
    if rules == embedded {
        Ok(overlaid)
    } else {
        share_export::replace_share_code_recognition_rules(&overlaid, &rules)
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
        assert_ne!(archive, [] as [u8; 0]);
    }

    #[test]
    fn parser_sharing_capture_failure_shares_without_local_phrases() {
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
        app.redesign_settings.user_name = "@sharer".to_string();
        let code = fixture_capture(&mut app, "PHRASEERR001", &parser)
            .expect("a malformed parser config does not block sharing")
            .expect("shared code");
        assert!(code.starts_with("BIO-MODLIST-V1:"));
        assert_eq!(
            std::fs::read_to_string(&file).expect("import code file"),
            code
        );
    }

    #[test]
    fn parser_rules_home_share_uses_selected_stored_map_and_capture_without_active_wizard_leak() {
        let mut app = app_with_entry("parser-rules-home", "RULEHOME0001", None);
        let root = app.isolated_test_config_root.as_ref().unwrap().clone();
        std::fs::create_dir_all(&root).unwrap();
        let parser = root.join("parser.toml");
        std::fs::write(&parser, "choice_phrase = ['local phrase']\nchoice_words = ['local word']\nfinished = ['local complete']\n").unwrap();
        let selected = [
            (
                "choice_words".to_string(),
                vec!["selected word".to_string()],
            ),
            (
                "finished".to_string(),
                vec!["selected complete".to_string()],
            ),
        ]
        .into_iter()
        .collect();
        isolated_store(&app, "RULEHOME0001")
            .save(&ModlistWorkspaceState {
                parser_recognition_rules: Some(selected),
                ..Default::default()
            })
            .unwrap();
        app.wizard_state.parser_recognition_rules = std::iter::once((
            "choice_words".to_string(),
            vec!["unrelated active wizard".to_string()],
        ))
        .collect();
        let code = fixture_capture(&mut app, "RULEHOME0001", &parser)
            .unwrap()
            .unwrap();
        let preview = preview_modlist_share_code(&code).unwrap();
        assert_eq!(
            preview.parser_recognition_rules["choice_words"],
            ["selected word", "local word"]
        );
        assert_eq!(
            preview.parser_recognition_rules["finished"],
            ["selected complete", "local complete"]
        );
        assert_eq!(preview.parser_choice_phrases, ["local phrase"]);
        let stored = isolated_store(&app, "RULEHOME0001").load().unwrap();
        let recovered =
            crate::registry::share_export::workspace_recognition_rules(&code, &stored).unwrap();
        assert_eq!(recovered, preview.parser_recognition_rules);
        assert_eq!(
            stored.parser_recognition_rules.as_ref().unwrap()["choice_words"],
            ["selected word", "local word"]
        );
        let cleared = ModlistWorkspaceState {
            parser_recognition_rules: Some(
                crate::app::mod_installer_parser::ParserRecognitionRules::new(),
            ),
            parser_choice_phrases: Some(vec!["stale mirror".to_string()]),
            ..Default::default()
        };
        app.workspace_state
            .insert("RULEHOME0001".to_string(), cleared);
        let code = app
            .code_for_share_with_rules(
                "RULEHOME0001",
                &crate::app::mod_installer_parser::ParserRecognitionRules::new(),
            )
            .unwrap()
            .unwrap();
        let preview = preview_modlist_share_code(&code).unwrap();
        assert!(preview.parser_recognition_rules.is_empty());
        assert_eq!(preview.parser_choice_phrases, [] as [String; 0]);
    }

    #[test]
    fn parser_rules_home_share_rejects_invalid_capture_before_mutating_selected_code() {
        let mut app = app_with_entry("parser-rules-home-invalid", "RULEHOME0002", Some("@owner"));
        let before = app
            .registry
            .find("RULEHOME0002")
            .unwrap()
            .latest_share_code
            .clone();
        let invalid =
            std::iter::once(("bell_character".to_string(), vec!["!".to_string()])).collect();
        assert!(
            app.code_for_share_with_rules("RULEHOME0002", &invalid)
                .is_err()
        );
        assert_eq!(
            app.registry.find("RULEHOME0002").unwrap().latest_share_code,
            before
        );
    }

    #[test]
    fn parser_rules_home_share_rejects_invalid_selected_workspace_without_rewriting_disk() {
        let mut app = app_with_entry(
            "parser-rules-invalid-selected",
            "RULEHOME0003",
            Some("@owner"),
        );
        let invalid = ModlistWorkspaceState {
            parser_recognition_rules: Some(
                std::iter::once(("unknown_field".to_string(), vec!["keep raw".to_string()]))
                    .collect(),
            ),
            ..Default::default()
        };
        let store = isolated_store(&app, "RULEHOME0003");
        store.save(&invalid).unwrap();
        let before_disk = std::fs::read(store.path()).unwrap();
        let before_code = app
            .registry
            .find("RULEHOME0003")
            .unwrap()
            .latest_share_code
            .clone();
        assert!(
            app.code_for_share_with_rules(
                "RULEHOME0003",
                &crate::app::mod_installer_parser::ParserRecognitionRules::new()
            )
            .is_err()
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before_disk);
        assert_eq!(
            app.registry.find("RULEHOME0003").unwrap().latest_share_code,
            before_code
        );
    }
}
