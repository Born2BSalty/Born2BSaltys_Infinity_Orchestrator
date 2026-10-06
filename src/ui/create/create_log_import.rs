// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::mods::log_file::LogFile;
use crate::ui::create::state_create::LogCheck;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step2::step2_log_confirm::WeiduLogImportRow;
use crate::ui::workspace::step2_log_glue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLogImport {
    pub modlist_id: String,
    pub form: WeiduLogImportForm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteStatus {
    Wait,
    Apply,
    Drop,
}

#[must_use]
pub fn check_log(path: &Path) -> LogCheck {
    match LogFile::from_path(path) {
        Ok(log) if !log.is_empty() => LogCheck::Valid {
            components: log.len(),
            mods: log.mod_count(),
        },
        _ => LogCheck::NotALog,
    }
}

#[must_use]
pub fn start_allowed(
    rows: &[WeiduLogImportRow],
    first: Option<&LogCheck>,
    second: Option<&LogCheck>,
) -> bool {
    let shown = || {
        rows.iter()
            .map(move |row| if row.first_slot { first } else { second })
    };
    shown().any(|check| matches!(check, Some(LogCheck::Valid { .. })))
        && !shown().any(|check| matches!(check, Some(LogCheck::NotALog)))
}

#[must_use]
pub fn carried_form(rows: &[WeiduLogImportRow], form: &WeiduLogImportForm) -> WeiduLogImportForm {
    let shows_first = rows.iter().any(|row| row.first_slot);
    let shows_second = rows.iter().any(|row| !row.first_slot);
    WeiduLogImportForm {
        first: form.first.clone().filter(|_| shows_first),
        second: form.second.clone().filter(|_| shows_second),
        fetch_missing: form.fetch_missing,
    }
}

#[must_use]
pub fn note_status(
    note: &CreateLogImport,
    nav: &NavDestination,
    list_loaded: bool,
    scan_completed: bool,
    scan_report_present: bool,
) -> NoteStatus {
    let on_the_list = matches!(
        nav,
        NavDestination::Workspace { modlist_id: Some(id) } if *id == note.modlist_id
    );
    if !on_the_list {
        NoteStatus::Drop
    } else if list_loaded && scan_completed && scan_report_present {
        NoteStatus::Apply
    } else {
        NoteStatus::Wait
    }
}

pub fn advance_create_log_import(orchestrator: &mut OrchestratorApp, scan_completed: bool) {
    let Some(note) = orchestrator.create_log_import.as_ref() else {
        return;
    };
    let list_loaded = orchestrator.workspace_view.loaded_workspace_id.as_deref()
        == Some(note.modlist_id.as_str());
    let status = note_status(
        note,
        &orchestrator.nav,
        list_loaded,
        scan_completed,
        orchestrator.wizard_state.step2.last_scan_report.is_some(),
    );
    match status {
        NoteStatus::Wait => {}
        NoteStatus::Drop => orchestrator.create_log_import = None,
        NoteStatus::Apply => {
            if let Some(note) = orchestrator.create_log_import.take() {
                step2_log_glue::import_weidu_logs(orchestrator, note.form);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::ui::workspace::step2::step2_log_confirm::weidu_log_import_rows;

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "bio_create_log_import_{tag}_{}_{}",
                std::process::id(),
                TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            let guard = Self(root);
            std::fs::create_dir_all(&guard.0).expect("create the test root");
            guard
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, text).expect("write the test file");
            path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const VALID: LogCheck = LogCheck::Valid {
        components: 3,
        mods: 2,
    };

    fn note(id: &str) -> CreateLogImport {
        CreateLogImport {
            modlist_id: id.to_string(),
            form: WeiduLogImportForm::default(),
        }
    }

    fn workspace(id: &str) -> NavDestination {
        NavDestination::Workspace {
            modlist_id: Some(id.to_string()),
        }
    }

    #[test]
    fn check_log_counts_components_and_mods() {
        let root = TempRoot::new("valid");
        let path = root.write(
            "weidu.log",
            "// Log of Currently Installed WeiDU Mods\n\
             ~EEFIXPACK/SETUP-EEFIXPACK.TP2~ #0 #0 // Core Fixes: 1.0\n\
             ~EEFIXPACK/SETUP-EEFIXPACK.TP2~ #0 #1 // Extra Fixes: 1.0\n\
             ~BG1UB/BG1UB.TP2~ #0 #0 // Ice Island: 1.0\n",
        );
        assert_eq!(
            check_log(&path),
            LogCheck::Valid {
                components: 3,
                mods: 2
            }
        );
    }

    #[test]
    fn a_file_without_component_lines_is_not_a_weidu_log() {
        let root = TempRoot::new("nolines");
        let path = root.write("notes.txt", "shopping list\n// eggs\nmilk\n");
        assert_eq!(check_log(&path), LogCheck::NotALog);
    }

    #[test]
    fn an_unreadable_file_is_not_a_weidu_log() {
        let root = TempRoot::new("unreadable");
        assert_eq!(check_log(&root.0.join("missing.log")), LogCheck::NotALog);
        let broken = root.write("broken.log", "~~ #0 #0 // nothing\n");
        assert_eq!(check_log(&broken), LogCheck::NotALog);
    }

    #[test]
    fn start_needs_one_valid_row_and_no_invalid_row() {
        let eet = weidu_log_import_rows("EET");
        let bad = LogCheck::NotALog;
        assert!(!start_allowed(&eet, None, None));
        assert!(start_allowed(&eet, Some(&VALID), None));
        assert!(start_allowed(&eet, None, Some(&VALID)));
        assert!(!start_allowed(&eet, Some(&VALID), Some(&bad)));
        assert!(!start_allowed(&eet, Some(&bad), Some(&VALID)));
        assert!(!start_allowed(&eet, Some(&bad), None));
        assert!(start_allowed(&eet, Some(&VALID), Some(&VALID)));

        let bgee = weidu_log_import_rows("BGEE");
        assert!(!start_allowed(&bgee, None, None));
        assert!(start_allowed(&bgee, Some(&VALID), None));
        assert!(!start_allowed(&bgee, Some(&bad), None));
        assert!(start_allowed(&bgee, Some(&VALID), Some(&bad)));
        assert!(!start_allowed(&bgee, None, Some(&VALID)));
    }

    #[test]
    fn hidden_rows_are_not_carried() {
        let form = WeiduLogImportForm {
            first: Some(PathBuf::from("bgee.log")),
            second: Some(PathBuf::from("bg2ee.log")),
            fetch_missing: false,
        };
        assert_eq!(carried_form(&weidu_log_import_rows("EET"), &form), form);
        assert_eq!(
            carried_form(&weidu_log_import_rows("BGEE"), &form),
            WeiduLogImportForm {
                first: Some(PathBuf::from("bgee.log")),
                second: None,
                fetch_missing: false,
            }
        );
        assert_eq!(
            carried_form(&weidu_log_import_rows("BG2EE"), &form),
            WeiduLogImportForm {
                first: None,
                second: Some(PathBuf::from("bg2ee.log")),
                fetch_missing: false,
            }
        );
        assert_eq!(
            carried_form(&weidu_log_import_rows("IWDEE"), &form).second,
            None
        );
        assert!(
            carried_form(
                &weidu_log_import_rows("BGEE"),
                &WeiduLogImportForm::default()
            )
            .fetch_missing
        );
    }

    #[test]
    fn note_waits_until_a_finished_scan_with_a_report() {
        let note = note("LIST1");
        let nav = workspace("LIST1");
        assert_eq!(
            note_status(&note, &nav, true, false, false),
            NoteStatus::Wait
        );
        assert_eq!(
            note_status(&note, &nav, true, false, true),
            NoteStatus::Wait
        );
        assert_eq!(
            note_status(&note, &nav, true, true, false),
            NoteStatus::Wait
        );
    }

    #[test]
    fn note_applies_on_the_lists_finished_scan() {
        let note = note("LIST1");
        assert_eq!(
            note_status(&note, &workspace("LIST1"), true, true, true),
            NoteStatus::Apply
        );
        assert_eq!(
            note_status(&note, &workspace("LIST2"), true, true, true),
            NoteStatus::Drop
        );
    }

    #[test]
    fn note_waits_until_the_list_is_loaded() {
        let note = note("LIST1");
        let nav = workspace("LIST1");
        assert_eq!(
            note_status(&note, &nav, false, true, true),
            NoteStatus::Wait
        );
        assert_eq!(
            note_status(&note, &nav, true, true, true),
            NoteStatus::Apply
        );
    }

    #[test]
    fn a_previous_lists_scan_does_not_apply_the_note() {
        let mut app = OrchestratorApp::new_isolated_for_test("createnoteprev");
        let note = note("LIST1");
        app.create_log_import = Some(note.clone());
        app.nav = workspace("LIST1");
        app.workspace_view.loaded_workspace_id = Some("OLD".to_string());
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        let toasts_before = app.notification_manager.history().len();
        advance_create_log_import(&mut app, true);
        assert_eq!(app.create_log_import.as_ref(), Some(&note));
        assert_eq!(app.wizard_state.step2.weidu_log_import, None);
        assert_eq!(app.notification_manager.history().len(), toasts_before);
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
            assert_eq!(
                note_status(&note, &nav, false, false, false),
                NoteStatus::Drop
            );
            assert_eq!(note_status(&note, &nav, true, true, true), NoteStatus::Drop);
        }

        let mut app = OrchestratorApp::new_isolated_for_test("createnote");
        app.create_log_import = Some(note.clone());
        app.nav = workspace("LIST1");
        advance_create_log_import(&mut app, false);
        assert_eq!(app.create_log_import.as_ref(), Some(&note));
        app.nav = NavDestination::Home;
        advance_create_log_import(&mut app, true);
        assert_eq!(app.create_log_import, None);
    }
}
