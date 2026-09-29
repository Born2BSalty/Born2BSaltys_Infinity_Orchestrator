// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path,
};
use crate::app::game_authority;
use crate::app::state::{Step2State, WeiduLogImport};
use crate::app::step2_action::Step2Action;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step_action_dispatch::handle_step2_via_bio;
use crate::ui::workspace::versions::versions_drawer::close_versions_drawer;

pub fn apply_weidu_log_selection_for_orchestrator(orchestrator: &mut OrchestratorApp, bgee: bool) {
    let (current, tab) = if bgee {
        (
            resolve_bgee_weidu_log_path(&orchestrator.wizard_state.step1),
            game_authority::first_slot_tab(&orchestrator.wizard_state.step1.game_install),
        )
    } else {
        (
            resolve_bg2_weidu_log_path(&orchestrator.wizard_state.step1),
            game_authority::TAB_BG2EE,
        )
    };

    let Some(path) = pick_weidu_log_file(current.as_deref(), tab) else {
        return;
    };

    apply_picked_weidu_log(orchestrator, bgee, path);
}

pub fn apply_picked_weidu_log(orchestrator: &mut OrchestratorApp, bgee: bool, path: PathBuf) {
    let picked_str = path.to_string_lossy().to_string();
    if bgee {
        orchestrator.wizard_state.step1.bgee_log_file = picked_str;
    } else {
        orchestrator.wizard_state.step1.bg2ee_log_file = picked_str;
    }
    orchestrator
        .bio_settings_last_dirty_at
        .get_or_insert_with(Instant::now);

    apply_weidu_log_selection_from_path(&mut orchestrator.wizard_state, bgee, Some(path));

    if bgee {
        orchestrator.wizard_state.step3.bgee_items.clear();
    } else {
        orchestrator.wizard_state.step3.bg2ee_items.clear();
    }
}

#[must_use]
pub fn pick_weidu_log_file(current: Option<&Path>, tab: &str) -> Option<PathBuf> {
    let mut dialog = FileDialog::new()
        .add_filter("WeiDU Log", &["log"])
        .set_title(format!("Select {tab} WeiDU log"));
    if let Some(cur) = current
        && let Some(dir) = cur.parent()
    {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_file()
}

#[must_use]
fn parse_failure(status: &str) -> Option<&str> {
    status.strip_prefix("Failed to parse log: ")
}

fn apply_recorded_log(orchestrator: &mut OrchestratorApp, bgee: bool, path: PathBuf) {
    apply_picked_weidu_log(orchestrator, bgee, path);
    let Some(reason) = parse_failure(&orchestrator.wizard_state.step2.scan_status) else {
        return;
    };
    let tab = if bgee {
        game_authority::first_slot_tab(&orchestrator.wizard_state.step1.game_install)
    } else {
        game_authority::TAB_BG2EE
    };
    let toast = format!("Could not read the {tab} WeiDU log: {reason}");
    orchestrator.notification_manager.error(toast);
}

fn apply_recorded_logs(orchestrator: &mut OrchestratorApp, record: &WeiduLogImport) {
    if let Some(first) = record.first.clone() {
        apply_recorded_log(orchestrator, true, first);
    }
    if let Some(second) = record.second.clone() {
        apply_recorded_log(orchestrator, false, second);
    }
}

pub fn import_weidu_logs(orchestrator: &mut OrchestratorApp, form: WeiduLogImportForm) {
    let record = WeiduLogImport {
        first: form.first,
        second: form.second,
    };
    apply_recorded_logs(orchestrator, &record);
    orchestrator.wizard_state.step2.weidu_log_import = Some(record);
    orchestrator.mark_workspace_dirty();
    handle_step2_via_bio(Step2Action::OpenUpdatePopup, orchestrator);
    orchestrator
        .wizard_state
        .step2
        .versions_ui
        .auto_check_pending = true;
}

#[must_use]
pub(crate) const fn weidu_log_reapply_ready(step2: &Step2State, scan_rx_live: bool) -> bool {
    step2.pending_weidu_log_reapply
        && !step2.is_scanning
        && !scan_rx_live
        && !step2.update_selected_check_running
        && !step2.update_selected_download_running
        && !step2.update_selected_extract_running
}

fn checked_component_count(step2: &Step2State) -> usize {
    step2
        .bgee_mods
        .iter()
        .chain(step2.bg2ee_mods.iter())
        .flat_map(|mod_state| mod_state.components.iter())
        .filter(|component| component.checked)
        .count()
}

fn reapply_toast_text(selected: usize, still_missing: usize) -> String {
    if still_missing == 0 {
        format!("WeiDU logs applied: {selected} components selected")
    } else {
        format!(
            "WeiDU logs applied: {selected} components selected, {still_missing} mods still not on disk"
        )
    }
}

pub fn advance_pending_weidu_log_reapply(orchestrator: &mut OrchestratorApp) {
    if !matches!(
        orchestrator.nav,
        NavDestination::Workspace {
            modlist_id: Some(_)
        }
    ) {
        orchestrator.wizard_state.step2.weidu_log_import = None;
        orchestrator.wizard_state.step2.pending_weidu_log_reapply = false;
        return;
    }
    if !weidu_log_reapply_ready(
        &orchestrator.wizard_state.step2,
        orchestrator.step2_scan_rx.is_some(),
    ) {
        return;
    }
    let record = orchestrator.wizard_state.step2.weidu_log_import.take();
    orchestrator.wizard_state.step2.pending_weidu_log_reapply = false;
    let Some(record) = record else {
        return;
    };
    apply_recorded_logs(orchestrator, &record);
    orchestrator.mark_workspace_dirty();
    close_versions_drawer(&mut orchestrator.wizard_state.step2);
    let step2 = &orchestrator.wizard_state.step2;
    let toast = reapply_toast_text(
        checked_component_count(step2),
        step2.log_pending_downloads.len(),
    );
    orchestrator.notification_manager.success(toast);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("bio_logimport_test_{}_{n}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp root");
            Self { path }
        }

        fn write_log(&self, name: &str, text: &str) -> PathBuf {
            let path = self.path.join(name);
            std::fs::write(&path, text).expect("write log");
            path
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    const TWO_COMPONENT_LOG: &str =
        "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n";

    #[test]
    fn reapply_waits_for_every_busy_flag() {
        let mut step2 = Step2State {
            pending_weidu_log_reapply: true,
            ..Step2State::default()
        };
        assert!(weidu_log_reapply_ready(&step2, false));
        assert!(!weidu_log_reapply_ready(&step2, true));

        step2.is_scanning = true;
        assert!(!weidu_log_reapply_ready(&step2, false));
        step2.is_scanning = false;

        step2.update_selected_check_running = true;
        assert!(!weidu_log_reapply_ready(&step2, false));
        step2.update_selected_check_running = false;

        step2.update_selected_download_running = true;
        assert!(!weidu_log_reapply_ready(&step2, false));
        step2.update_selected_download_running = false;

        step2.update_selected_extract_running = true;
        assert!(!weidu_log_reapply_ready(&step2, false));
        step2.update_selected_extract_running = false;

        assert!(weidu_log_reapply_ready(&step2, false));
        step2.pending_weidu_log_reapply = false;
        assert!(!weidu_log_reapply_ready(&step2, false));
    }

    #[test]
    fn reapply_consumes_the_record_and_closes_the_drawer() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-reapply");
        app.nav = NavDestination::Workspace {
            modlist_id: Some("LOGIMPORTFIX".to_string()),
        };
        app.wizard_state.step2.weidu_log_import = Some(WeiduLogImport {
            first: Some(log_path),
            second: None,
        });
        app.wizard_state.step2.pending_weidu_log_reapply = true;
        app.wizard_state.step2.update_selected_popup_open = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        let step2 = &app.wizard_state.step2;
        assert_eq!(step2.weidu_log_import, None);
        assert!(!step2.pending_weidu_log_reapply);
        assert!(!step2.update_selected_popup_open);
        assert!(
            step2
                .log_pending_downloads
                .iter()
                .any(|pending| pending.tp_file.eq_ignore_ascii_case("SETUP-X.TP2")),
            "pending downloads: {:?}",
            step2.log_pending_downloads
        );
        assert_eq!(app.notification_manager.history().len(), toasts_before + 1);
    }

    #[test]
    fn import_records_both_paths_and_opens_the_drawer() {
        let root = TempRoot::new();
        let first = root.write_log("bgee.log", TWO_COMPONENT_LOG);
        let second = root.write_log("bg2ee.log", TWO_COMPONENT_LOG);
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-import");
        app.wizard_state.step1.game_install = "EET".to_string();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(first.clone()),
                second: Some(second.clone()),
            },
        );

        assert_eq!(
            app.wizard_state.step2.weidu_log_import,
            Some(WeiduLogImport {
                first: Some(first),
                second: Some(second),
            })
        );
        assert!(app.wizard_state.step2.update_selected_popup_open);
    }

    #[test]
    fn import_forces_the_source_check_even_when_the_selection_looks_fresh() {
        let root = TempRoot::new();
        let first = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-forces-check");
        let step2 = &mut app.wizard_state.step2;
        step2.update_selected_has_run = true;
        step2.update_selected_last_was_full_selection = true;
        step2.update_selected_last_selection_signature =
            Some(crate::app::state::update_selection_signature(step2));

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(first),
                second: None,
            },
        );

        assert!(app.wizard_state.step2.versions_ui.auto_check_pending);
    }

    #[test]
    fn reapply_outside_a_workspace_drops_the_record() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-outside-workspace");
        app.nav = NavDestination::Home;
        app.wizard_state.step2.weidu_log_import = Some(WeiduLogImport {
            first: Some(log_path),
            second: None,
        });
        app.wizard_state.step2.pending_weidu_log_reapply = true;
        app.wizard_state.step2.update_selected_popup_open = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        let step2 = &app.wizard_state.step2;
        assert_eq!(step2.weidu_log_import, None);
        assert!(!step2.pending_weidu_log_reapply);
        assert!(step2.update_selected_popup_open);
        assert!(step2.log_pending_downloads.is_empty());
        assert_eq!(app.notification_manager.history().len(), toasts_before);
    }

    #[test]
    fn parse_failure_strips_the_prefix() {
        assert_eq!(
            parse_failure("Failed to parse log: bad line"),
            Some("bad line")
        );
        assert_eq!(parse_failure("BGEE selected from log: 2"), None);
    }

    #[test]
    fn a_log_that_fails_to_parse_raises_an_error_toast() {
        let root = TempRoot::new();
        let broken = root.write_log("weidu.log", "~not a weidu log\n");
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-parse-failure");
        app.wizard_state.step1.game_install = "BGEE".to_string();
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(broken),
                second: None,
            },
        );

        let errors = app
            .notification_manager
            .history()
            .iter()
            .skip(toasts_before)
            .filter(|record| record.kind == egui_toast::ToastKind::Error)
            .map(|record| record.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(errors.len(), 1, "errors: {errors:?}");
        assert!(
            errors[0].starts_with("Could not read the BGEE WeiDU log"),
            "toast: {}",
            errors[0]
        );
    }
}
