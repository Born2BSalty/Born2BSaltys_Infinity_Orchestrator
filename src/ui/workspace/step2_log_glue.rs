// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path,
};
use crate::app::game_authority;
use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::{Step2State, WeiduLogImport};
use crate::app::step2_action::Step2Action;
use crate::mods::log_file::LogFile;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step_action_dispatch::handle_step2_via_bio;
use crate::ui::workspace::versions::versions_drawer::close_versions_drawer;
use crate::ui::workspace::workspace_header::save_draft;

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
    if !record_logs_parse(&record) {
        orchestrator.wizard_state.step2.weidu_log_import = None;
        return;
    }
    orchestrator.mark_workspace_dirty();
    if !form.fetch_missing
        || orchestrator
            .wizard_state
            .step2
            .log_pending_downloads
            .is_empty()
    {
        orchestrator.wizard_state.step2.weidu_log_import = None;
        save_draft(orchestrator);
        finish_weidu_log_import(orchestrator, &record);
        return;
    }
    orchestrator.wizard_state.step2.weidu_log_import = Some(record);
    handle_step2_via_bio(Step2Action::OpenUpdatePopup, orchestrator);
    let step2 = &mut orchestrator.wizard_state.step2;
    step2.versions_ui.auto_check_pending = true;
    step2.update_selected_has_run = false;
    step2.weidu_log_import_awaiting_check = true;
    step2.versions_ui.log_pending_scope = true;
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

#[must_use]
pub(crate) fn applied_toast_text(
    selected: usize,
    missing_mods: usize,
    not_selectable: usize,
) -> String {
    let missing_part = if missing_mods > 0 {
        format!(", {missing_mods} mods not on disk")
    } else {
        String::new()
    };
    let not_selectable_part = if not_selectable > 0 {
        format!(", {not_selectable} components not selectable")
    } else {
        String::new()
    };
    format!("WeiDU logs applied: {selected} components selected{missing_part}{not_selectable_part}")
}

fn not_selectable_count(record: &WeiduLogImport, step2: &Step2State, selected: usize) -> usize {
    let missing: HashSet<String> = step2
        .log_pending_downloads
        .iter()
        .map(|pending| normalize_mod_download_tp2(&pending.tp_file))
        .collect();
    let on_disk_lines = record
        .first
        .iter()
        .chain(record.second.iter())
        .filter_map(|path| LogFile::from_path(path).ok())
        .map(|log| {
            log.components()
                .iter()
                .filter(|component| {
                    !missing.contains(&normalize_mod_download_tp2(&component.tp_file))
                })
                .count()
        })
        .sum::<usize>();
    on_disk_lines.saturating_sub(selected)
}

#[must_use]
pub(crate) const fn weidu_log_import_check_settled(step2: &Step2State, scan_rx_live: bool) -> bool {
    step2.weidu_log_import.is_some()
        && step2.weidu_log_import_awaiting_check
        && !step2.pending_weidu_log_reapply
        && step2.update_selected_popup_open
        && !step2.versions_ui.auto_check_pending
        && step2.update_selected_has_run
        && !step2.update_selected_check_running
        && !step2.update_selected_download_running
        && !step2.update_selected_extract_running
        && !step2.is_scanning
        && !scan_rx_live
}

#[must_use]
pub(crate) const fn weidu_log_import_check_found_nothing_to_fetch(step2: &Step2State) -> bool {
    step2.update_selected_update_assets.is_empty()
        && step2.update_selected_failed_sources.is_empty()
        && step2
            .update_selected_exact_version_failed_sources
            .is_empty()
}

fn record_logs_parse(record: &WeiduLogImport) -> bool {
    record
        .first
        .iter()
        .chain(record.second.iter())
        .all(|path| LogFile::from_path(path).is_ok())
}

fn settle_weidu_log_import_check(orchestrator: &mut OrchestratorApp) {
    let step2 = &mut orchestrator.wizard_state.step2;
    step2.weidu_log_import_awaiting_check = false;
    if !weidu_log_import_check_found_nothing_to_fetch(step2) {
        return;
    }
    let Some(record) = step2.weidu_log_import.clone() else {
        return;
    };
    if !record_logs_parse(&record) {
        return;
    }
    apply_recorded_logs(orchestrator, &record);
    orchestrator.mark_workspace_dirty();
    let step2 = &mut orchestrator.wizard_state.step2;
    if step2.log_pending_downloads.is_empty() {
        step2.weidu_log_import = None;
        save_draft(orchestrator);
        finish_weidu_log_import(orchestrator, &record);
    }
}

fn finish_weidu_log_import(orchestrator: &mut OrchestratorApp, record: &WeiduLogImport) {
    close_versions_drawer(&mut orchestrator.wizard_state.step2);
    let step2 = &orchestrator.wizard_state.step2;
    let selected = checked_component_count(step2);
    let toast = applied_toast_text(
        selected,
        step2.log_pending_downloads.len(),
        not_selectable_count(record, step2, selected),
    );
    orchestrator.notification_manager.success(toast);
}

pub fn advance_pending_weidu_log_reapply(orchestrator: &mut OrchestratorApp) {
    if !matches!(
        orchestrator.nav,
        NavDestination::Workspace {
            modlist_id: Some(_)
        }
    ) {
        let step2 = &mut orchestrator.wizard_state.step2;
        step2.weidu_log_import = None;
        step2.pending_weidu_log_reapply = false;
        step2.weidu_log_import_awaiting_check = false;
        return;
    }
    let scan_rx_live = orchestrator.step2_scan_rx.is_some();
    if weidu_log_import_check_settled(&orchestrator.wizard_state.step2, scan_rx_live) {
        settle_weidu_log_import_check(orchestrator);
        return;
    }
    if !weidu_log_reapply_ready(&orchestrator.wizard_state.step2, scan_rx_live) {
        return;
    }
    let record = orchestrator.wizard_state.step2.weidu_log_import.take();
    orchestrator.wizard_state.step2.pending_weidu_log_reapply = false;
    let Some(record) = record else {
        return;
    };
    apply_recorded_logs(orchestrator, &record);
    orchestrator.mark_workspace_dirty();
    save_draft(orchestrator);
    finish_weidu_log_import(orchestrator, &record);
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

    fn app_awaiting_the_forced_check(tag: &str, log_path: PathBuf) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.nav = NavDestination::Workspace {
            modlist_id: Some("LOGIMPORTCHECK".to_string()),
        };
        let step2 = &mut app.wizard_state.step2;
        step2.weidu_log_import = Some(WeiduLogImport {
            first: Some(log_path),
            second: None,
        });
        step2.weidu_log_import_awaiting_check = true;
        step2.update_selected_popup_open = true;
        step2.update_selected_has_run = true;
        app
    }

    #[test]
    fn import_with_nothing_to_fetch_reapplies_then_closes_the_drawer_and_toasts() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", "// log of current files installed\n");
        let mut app = app_awaiting_the_forced_check("logimport-nofetch", log_path);
        let toasts_before = app.notification_manager.history().len();

        app.wizard_state.step2.update_selected_has_run = false;
        advance_pending_weidu_log_reapply(&mut app);
        assert!(app.wizard_state.step2.weidu_log_import_awaiting_check);

        app.wizard_state.step2.update_selected_has_run = true;
        app.wizard_state.step2.versions_ui.auto_check_pending = true;
        advance_pending_weidu_log_reapply(&mut app);
        assert!(app.wizard_state.step2.weidu_log_import_awaiting_check);

        app.wizard_state.step2.versions_ui.auto_check_pending = false;
        advance_pending_weidu_log_reapply(&mut app);
        let step2 = &app.wizard_state.step2;
        assert!(!step2.weidu_log_import_awaiting_check);
        assert_eq!(step2.weidu_log_import, None);
        assert!(!step2.update_selected_popup_open);
        assert_eq!(app.notification_manager.history().len(), toasts_before + 1);
    }

    #[test]
    fn import_check_with_a_missing_mod_keeps_the_drawer_and_the_record_once() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_awaiting_the_forced_check("logimport-missing", log_path);
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);
        let step2 = &app.wizard_state.step2;
        assert!(!step2.weidu_log_import_awaiting_check);
        assert!(step2.weidu_log_import.is_some());
        assert!(step2.update_selected_popup_open);
        assert!(
            step2
                .log_pending_downloads
                .iter()
                .any(|pending| pending.tp_file.eq_ignore_ascii_case("SETUP-X.TP2"))
        );
        assert_eq!(app.notification_manager.history().len(), toasts_before);

        app.wizard_state.step2.log_pending_downloads.clear();
        advance_pending_weidu_log_reapply(&mut app);
        assert!(app.wizard_state.step2.weidu_log_import.is_some());
        assert!(app.wizard_state.step2.update_selected_popup_open);
        assert_eq!(app.notification_manager.history().len(), toasts_before);
    }

    const SAVE_DRAFT_ID: &str = "LOGIMPORTSAVE";

    fn unchecked_component(component_id: &str) -> crate::app::state::Step2ComponentState {
        crate::app::state::Step2ComponentState {
            component_id: component_id.to_string(),
            label: format!("X {component_id}"),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    fn scanned_x_mod() -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: "X".to_string(),
            tp_file: "SETUP-X.TP2".to_string(),
            tp2_path: "X/SETUP-X.TP2".to_string(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: vec![unchecked_component("1"), unchecked_component("2")],
        }
    }

    fn app_with_a_saved_list(tag: &str, log_path: PathBuf) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.nav = NavDestination::Workspace {
            modlist_id: Some(SAVE_DRAFT_ID.to_string()),
        };
        app.registry
            .entries
            .push(crate::registry::model::ModlistEntry {
                id: SAVE_DRAFT_ID.to_string(),
                name: "save draft on import".to_string(),
                game: crate::registry::model::Game::BGEE,
                state: crate::registry::model::ModlistState::InProgress,
                ..Default::default()
            });
        app.workspace_view.modlist_id = SAVE_DRAFT_ID.to_string();
        app.wizard_state.step1.game_install = "BGEE".to_string();
        let step2 = &mut app.wizard_state.step2;
        step2.bgee_mods = vec![scanned_x_mod()];
        step2.weidu_log_import = Some(WeiduLogImport {
            first: Some(log_path),
            second: None,
        });
        step2.update_selected_popup_open = true;
        app
    }

    fn assert_the_draft_was_saved_with_the_counts(app: &OrchestratorApp) {
        let entry = app
            .registry
            .find(SAVE_DRAFT_ID)
            .expect("the registry entry");
        assert_eq!(entry.mod_count, 1);
        assert_eq!(entry.component_count, 2);
        let store_path = app.workspace_stores[SAVE_DRAFT_ID].path().to_path_buf();
        assert!(
            store_path.starts_with(std::env::temp_dir()),
            "the workspace store must live under the isolated temp root: {}",
            store_path.display()
        );
        assert!(
            store_path.is_file(),
            "the import must write the workspace file"
        );
    }

    #[test]
    fn reapply_saves_the_draft_and_writes_the_counts() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-reapply-save", log_path);
        app.wizard_state.step2.pending_weidu_log_reapply = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        assert_eq!(app.notification_manager.history().len(), toasts_before + 1);
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_the_draft_was_saved_with_the_counts(&app);
    }

    #[test]
    fn no_fetch_completion_saves_the_draft_and_writes_the_counts() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-nofetch-save", log_path);
        let step2 = &mut app.wizard_state.step2;
        step2.weidu_log_import_awaiting_check = true;
        step2.update_selected_has_run = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        assert_eq!(app.notification_manager.history().len(), toasts_before + 1);
        assert_eq!(app.wizard_state.step2.weidu_log_import, None);
        assert_the_draft_was_saved_with_the_counts(&app);
    }

    #[test]
    fn closing_the_drawer_drops_the_awaiting_check_flag() {
        let mut step2 = Step2State {
            weidu_log_import_awaiting_check: true,
            update_selected_popup_open: true,
            ..Step2State::default()
        };
        close_versions_drawer(&mut step2);
        assert!(!step2.weidu_log_import_awaiting_check);
        assert!(!step2.update_selected_popup_open);
    }

    #[test]
    fn import_check_with_an_unreadable_log_keeps_the_drawer_and_the_record() {
        let root = TempRoot::new();
        let log_path = root.path.join("missing").join("weidu.log");
        let mut app = app_awaiting_the_forced_check("logimport-unreadable", log_path);
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);
        let step2 = &app.wizard_state.step2;
        assert!(!step2.weidu_log_import_awaiting_check);
        assert!(step2.weidu_log_import.is_some());
        assert!(step2.update_selected_popup_open);
        assert_eq!(app.notification_manager.history().len(), toasts_before);
    }

    #[test]
    fn import_check_with_failures_keeps_the_drawer_open() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", "// log of current files installed\n");
        let mut app = app_awaiting_the_forced_check("logimport-failed", log_path);
        app.wizard_state.step2.update_selected_failed_sources = vec!["x".to_string()];
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);
        let step2 = &app.wizard_state.step2;
        assert!(!step2.weidu_log_import_awaiting_check);
        assert!(step2.weidu_log_import.is_some());
        assert!(step2.update_selected_popup_open);
        assert_eq!(app.notification_manager.history().len(), toasts_before);
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
                fetch_missing: true,
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
        assert!(app.wizard_state.step2.versions_ui.log_pending_scope);
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
                ..WeiduLogImportForm::default()
            },
        );

        assert!(app.wizard_state.step2.versions_ui.auto_check_pending);
        assert!(app.wizard_state.step2.weidu_log_import_awaiting_check);
        assert!(!app.wizard_state.step2.update_selected_has_run);
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
        assert_eq!(step2.log_pending_downloads.len(), 0);
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
                ..WeiduLogImportForm::default()
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

    fn toasts_since(app: &OrchestratorApp, before: usize) -> Vec<(egui_toast::ToastKind, String)> {
        app.notification_manager
            .history()
            .iter()
            .skip(before)
            .map(|record| (record.kind, record.text.clone()))
            .collect()
    }

    fn bgee_import_app(tag: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.nav = NavDestination::Workspace {
            modlist_id: Some("LOGIMPORTFLOW".to_string()),
        };
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app
    }

    #[test]
    fn import_with_fetch_off_completes_without_versions() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = bgee_import_app("logimport-fetch-off");
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(log_path),
                second: None,
                fetch_missing: false,
            },
        );

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        assert!(!step2.weidu_log_import_awaiting_check);
        assert_eq!(step2.weidu_log_import, None);
        assert_eq!(step2.log_pending_downloads.len(), 1);
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(toasts.len(), 1, "toasts: {toasts:?}");
        assert_eq!(toasts[0].0, egui_toast::ToastKind::Success);
        assert!(
            toasts[0].1.contains("1 mods not on disk"),
            "toast: {}",
            toasts[0].1
        );
        assert_eq!(
            toasts[0].1,
            "WeiDU logs applied: 0 components selected, 1 mods not on disk"
        );
    }

    #[test]
    fn import_with_nothing_missing_completes_without_versions() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-nothing-missing", log_path.clone());
        app.wizard_state.step2.update_selected_popup_open = false;
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(log_path),
                ..WeiduLogImportForm::default()
            },
        );

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        assert!(!step2.versions_ui.auto_check_pending);
        assert_eq!(step2.weidu_log_import, None);
        assert_eq!(step2.log_pending_downloads.len(), 0);
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(
            toasts,
            vec![(
                egui_toast::ToastKind::Success,
                "WeiDU logs applied: 2 components selected".to_string()
            )]
        );
        assert_the_draft_was_saved_with_the_counts(&app);
    }

    #[test]
    fn import_with_missing_mods_opens_versions_scoped_to_them() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = bgee_import_app("logimport-missing-scoped");
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(log_path.clone()),
                ..WeiduLogImportForm::default()
            },
        );

        let step2 = &app.wizard_state.step2;
        assert!(step2.update_selected_popup_open);
        assert!(step2.versions_ui.log_pending_scope);
        assert!(step2.versions_ui.auto_check_pending);
        assert!(step2.weidu_log_import_awaiting_check);
        assert_eq!(
            step2.weidu_log_import,
            Some(WeiduLogImport {
                first: Some(log_path),
                second: None,
            })
        );
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn an_unreadable_log_stops_the_import() {
        let root = TempRoot::new();
        let log_path = root.path.join("missing").join("weidu.log");
        let mut app = bgee_import_app("logimport-unreadable-stops");
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(log_path),
                ..WeiduLogImportForm::default()
            },
        );

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        assert!(!step2.weidu_log_import_awaiting_check);
        assert_eq!(step2.weidu_log_import, None);
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(toasts.len(), 1, "toasts: {toasts:?}");
        assert_eq!(toasts[0].0, egui_toast::ToastKind::Error);
        assert!(
            toasts[0].1.starts_with("Could not read the BGEE WeiDU log"),
            "toast: {}",
            toasts[0].1
        );
    }

    #[test]
    fn applied_toast_text_drops_zero_parts() {
        assert_eq!(
            applied_toast_text(5, 0, 0),
            "WeiDU logs applied: 5 components selected"
        );
        assert_eq!(
            applied_toast_text(5, 2, 0),
            "WeiDU logs applied: 5 components selected, 2 mods not on disk"
        );
        assert_eq!(
            applied_toast_text(5, 0, 3),
            "WeiDU logs applied: 5 components selected, 3 components not selectable"
        );
        assert_eq!(
            applied_toast_text(5, 2, 3),
            "WeiDU logs applied: 5 components selected, 2 mods not on disk, 3 components not selectable"
        );
    }

    #[test]
    fn not_selectable_counts_on_disk_lines_left_unticked() {
        let root = TempRoot::new();
        let first = root.write_log(
            "bgee.log",
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n~Y/SETUP-Y.TP2~ #0 #0 // Y\n",
        );
        let second = root.write_log("bg2ee.log", "~x/setup-x.tp2~ #0 #3 // X three\n");
        let unreadable = root.path.join("missing").join("weidu.log");
        let record = WeiduLogImport {
            first: Some(first),
            second: Some(second),
        };
        let step2 = Step2State {
            log_pending_downloads: vec![crate::app::state::Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: "Y/SETUP-Y.TP2".to_string(),
                label: "Y".to_string(),
                requested_version: None,
            }],
            ..Step2State::default()
        };

        assert_eq!(not_selectable_count(&record, &step2, 0), 3);
        assert_eq!(not_selectable_count(&record, &step2, 2), 1);
        assert_eq!(not_selectable_count(&record, &step2, 3), 0);
        assert_eq!(not_selectable_count(&record, &step2, 7), 0);

        let partly_unreadable = WeiduLogImport {
            first: record.first,
            second: Some(unreadable),
        };
        assert_eq!(not_selectable_count(&partly_unreadable, &step2, 1), 1);
    }

    #[test]
    fn closing_versions_clears_the_log_pending_scope() {
        let mut step2 = Step2State {
            update_selected_popup_open: true,
            ..Step2State::default()
        };
        step2.versions_ui.log_pending_scope = true;
        close_versions_drawer(&mut step2);
        assert!(!step2.versions_ui.log_pending_scope);
    }
}
