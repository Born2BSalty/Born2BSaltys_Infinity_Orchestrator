// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use crate::registry::share_author::{
    stamp_current_author, user_name_is_valid, workspace_marks_own_list,
};
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

    pub(crate) fn code_for_share(&mut self, id: &str) -> Option<String> {
        let typed = self.share_name_buffer.trim().to_string();
        if user_name_is_valid(&typed) && self.redesign_settings.user_name.trim().is_empty() {
            self.notification_manager
                .success(format!("Saved your name to Settings: {typed}"));
            self.redesign_settings.user_name = typed;
        }
        self.share_name_buffer.clear();

        if self.list_is_own(id) {
            let user_name = self.redesign_settings.user_name.clone();
            if let Some(entry) = self.registry.find_mut(id)
                && stamp_current_author(entry, &user_name)
            {
                self.persistence_cycle.mark_registry_dirty(Instant::now());
            }
        }

        self.registry
            .find(id)
            .and_then(|entry| entry.latest_share_code.clone())
            .filter(|code| !code.trim().is_empty())
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

    #[test]
    fn code_for_share_saves_the_typed_name_and_stamps_an_own_list() {
        let mut app = app_with_entry("share-flow-own", "OWNLIST00001", None);
        app.workspace_state
            .insert("OWNLIST00001".to_string(), own_workspace());
        app.redesign_settings.user_name.clear();
        app.share_name_buffer = "  @typed  ".to_string();

        let code = app.code_for_share("OWNLIST00001").expect("a code to share");

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

        let code = app.code_for_share("OWNLIST00003").expect("a code to share");

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

        let _code = app.code_for_share("OWNLIST00004");

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

        let code = app.code_for_share("ASISLIST0001");

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

        let code = app.code_for_share("UNOPENED0001").expect("a code to share");

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
        assert_eq!(app.code_for_share("BROKEN000001"), stored);
        assert_eq!(
            app.registry.find("BROKEN000001").unwrap().author.as_deref(),
            Some("@old")
        );
    }
}
