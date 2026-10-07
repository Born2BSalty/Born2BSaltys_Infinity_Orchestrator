// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLogImport {
    pub modlist_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteStatus {
    Wait,
    Open,
    Drop,
}

#[must_use]
pub fn note_status(note: &CreateLogImport, nav: &NavDestination, list_loaded: bool) -> NoteStatus {
    let on_the_list = matches!(
        nav,
        NavDestination::Workspace { modlist_id: Some(id) } if *id == note.modlist_id
    );
    if !on_the_list {
        NoteStatus::Drop
    } else if list_loaded {
        NoteStatus::Open
    } else {
        NoteStatus::Wait
    }
}

pub fn advance_create_log_import(orchestrator: &mut OrchestratorApp) {
    let Some(note) = orchestrator.create_log_import.as_ref() else {
        return;
    };
    let list_loaded = orchestrator.workspace_view.loaded_workspace_id.as_deref()
        == Some(note.modlist_id.as_str());
    match note_status(note, &orchestrator.nav, list_loaded) {
        NoteStatus::Wait => {}
        NoteStatus::Drop => orchestrator.create_log_import = None,
        NoteStatus::Open => {
            orchestrator.create_log_import = None;
            orchestrator.workspace_view.step2.weidu_log_import_form =
                Some(WeiduLogImportForm::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str) -> CreateLogImport {
        CreateLogImport {
            modlist_id: id.to_string(),
        }
    }

    fn workspace(id: &str) -> NavDestination {
        NavDestination::Workspace {
            modlist_id: Some(id.to_string()),
        }
    }

    #[test]
    fn note_waits_until_the_list_is_loaded() {
        let note = note("LIST1");
        let nav = workspace("LIST1");
        assert_eq!(note_status(&note, &nav, false), NoteStatus::Wait);
        assert_eq!(note_status(&note, &nav, true), NoteStatus::Open);

        let mut app = OrchestratorApp::new_isolated_for_test("createnotewait");
        app.create_log_import = Some(note.clone());
        app.nav = nav;
        app.workspace_view.loaded_workspace_id = Some("OLD".to_string());
        advance_create_log_import(&mut app);
        assert_eq!(app.create_log_import.as_ref(), Some(&note));
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
    }

    #[test]
    fn note_opens_the_import_drawer_once_the_list_is_loaded() {
        let mut app = OrchestratorApp::new_isolated_for_test("createnoteopen");
        app.create_log_import = Some(note("LIST1"));
        app.nav = workspace("LIST1");
        app.workspace_view.loaded_workspace_id = Some("LIST1".to_string());

        advance_create_log_import(&mut app);

        let form = app
            .workspace_view
            .step2
            .weidu_log_import_form
            .as_ref()
            .expect("the import drawer is open");
        assert!(form.fetch_missing);
        assert_eq!(form, &WeiduLogImportForm::default());
        assert_eq!(app.create_log_import, None);
    }

    #[test]
    fn note_drops_when_leaving_the_workspace() {
        let note = note("LIST1");
        for nav in [
            NavDestination::Home,
            NavDestination::Create,
            NavDestination::Workspace { modlist_id: None },
            workspace("OTHER"),
        ] {
            assert_eq!(note_status(&note, &nav, false), NoteStatus::Drop);
            assert_eq!(note_status(&note, &nav, true), NoteStatus::Drop);
        }

        let mut app = OrchestratorApp::new_isolated_for_test("createnote");
        app.create_log_import = Some(note.clone());
        app.nav = workspace("LIST1");
        advance_create_log_import(&mut app);
        assert_eq!(app.create_log_import.as_ref(), Some(&note));
        app.nav = NavDestination::Home;
        app.workspace_view.loaded_workspace_id = Some("LIST1".to_string());
        advance_create_log_import(&mut app);
        assert_eq!(app.create_log_import, None);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
    }
}
