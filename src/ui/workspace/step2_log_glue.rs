// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, resolve_bg2_weidu_log_path, resolve_bgee_weidu_log_path,
    unticked_log_lines,
};
use crate::app::game_authority;
use crate::app::state::{Step2LogUnticked, Step2State, WeiduLogImport};
use crate::app::step2_action::Step2Action;
use crate::mods::log_file::LogFile;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::{LogCheck, RescanSnapshot, WeiduLogImportForm};
use crate::ui::workspace::step_action_dispatch::handle_step2_via_bio;
use crate::ui::workspace::step2::step2_rescan_reconcile::format_component_notification;
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
pub(crate) fn check_log(path: &Path) -> LogCheck {
    match LogFile::from_path(path) {
        Ok(log) if !log.is_empty() => LogCheck::Valid {
            components: log.len(),
            mods: log.mod_count(),
        },
        _ => LogCheck::NotALog,
    }
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
    orchestrator.wizard_state.step2.log_apply.lines.clear();
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
        finish_weidu_log_import(orchestrator);
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

pub(crate) fn advance_queued_weidu_log_import(
    orchestrator: &mut OrchestratorApp,
    scan_completed: bool,
) {
    if !orchestrator.workspace_view.step2.weidu_log_import_queued {
        return;
    }
    let on_the_loaded_list = matches!(
        &orchestrator.nav,
        NavDestination::Workspace { modlist_id: Some(id) }
            if orchestrator.workspace_view.loaded_workspace_id.as_deref() == Some(id.as_str())
    );
    if !on_the_loaded_list {
        let view = &mut orchestrator.workspace_view.step2;
        view.weidu_log_import_queued = false;
        view.weidu_log_import_form = None;
        return;
    }
    if !scan_completed && step2_scan_running(orchestrator) {
        return;
    }
    orchestrator.workspace_view.step2.weidu_log_import_queued = false;
    if orchestrator.wizard_state.step2.last_scan_report.is_none() {
        return;
    }
    if let Some(form) = orchestrator
        .workspace_view
        .step2
        .weidu_log_import_form
        .take()
    {
        import_weidu_logs(orchestrator, form);
    }
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

#[must_use]
pub(crate) const fn step2_scan_running(orchestrator: &OrchestratorApp) -> bool {
    orchestrator.wizard_state.step2.is_scanning || orchestrator.step2_scan_rx.is_some()
}

#[must_use]
pub(crate) fn list_has_selection(step2: &Step2State, snapshot: Option<&RescanSnapshot>) -> bool {
    let tree_has_a_check = step2
        .bgee_mods
        .iter()
        .chain(step2.bg2ee_mods.iter())
        .flat_map(|mod_state| mod_state.components.iter())
        .any(|component| component.checked);
    tree_has_a_check
        || snapshot.is_some_and(|snapshot| !snapshot.bgee.is_empty() || !snapshot.bg2ee.is_empty())
}

#[must_use]
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
fn applied_toast_text(selected: usize) -> String {
    format!("WeiDU logs applied: {selected} components selected")
}

#[must_use]
fn unticked_toast_text(unticked: &[Step2LogUnticked]) -> Option<String> {
    if unticked.is_empty() {
        return None;
    }
    Some(format_component_notification(
        &format!("{} logged component(s) could not be ticked", unticked.len()),
        unticked.iter().map(|entry| {
            (
                entry.mod_name.as_str(),
                entry.component_id.as_str(),
                entry.reason.as_str(),
            )
        }),
    ))
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
        finish_weidu_log_import(orchestrator);
    }
}

fn finish_weidu_log_import(orchestrator: &mut OrchestratorApp) {
    close_versions_drawer(&mut orchestrator.wizard_state.step2);
    let step2 = &orchestrator.wizard_state.step2;
    let toast = applied_toast_text(checked_component_count(step2));
    let unticked = unticked_toast_text(&unticked_log_lines(step2));
    orchestrator.notification_manager.success(toast);
    if let Some(unticked) = unticked {
        orchestrator.notification_manager.warn_persistent(unticked);
    }
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
    finish_weidu_log_import(orchestrator);
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::ui::workspace::state_workspace::RescanSelection;

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
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts(0));
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
                ..WeiduLogImportForm::default()
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
                fetch_missing: false,
                ..WeiduLogImportForm::default()
            },
        );

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        assert!(!step2.weidu_log_import_awaiting_check);
        assert_eq!(step2.weidu_log_import, None);
        assert_eq!(step2.log_pending_downloads.len(), 1);
        assert_eq!(
            toasts_since(&app, toasts_before),
            missing_x_toasts(0),
            "fetch off ends with the selected count and the unticked lines"
        );
    }

    const MISSING_X_WARNING: &str = "2 logged component(s) could not be ticked\nSETUP-X.TP2 #1: mod not on disk\nSETUP-X.TP2 #2: mod not on disk";

    fn missing_x_toasts(selected: usize) -> Vec<(egui_toast::ToastKind, String)> {
        vec![
            (
                egui_toast::ToastKind::Success,
                format!("WeiDU logs applied: {selected} components selected"),
            ),
            (
                egui_toast::ToastKind::Warning,
                MISSING_X_WARNING.to_string(),
            ),
        ]
    }

    #[test]
    fn finish_raises_a_persistent_warning_listing_unticked_lines() {
        let root = TempRoot::new();
        let log_path = root.write_log(
            "weidu.log",
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #9 // X nine\n~Y/SETUP-Y.TP2~ #0 #0 // Y\n",
        );
        let mut app = app_with_a_saved_list("logimport-unticked-warning", log_path);
        app.wizard_state.step2.pending_weidu_log_reapply = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        assert_eq!(
            toasts_since(&app, toasts_before),
            vec![
                (
                    egui_toast::ToastKind::Success,
                    "WeiDU logs applied: 1 components selected".to_string()
                ),
                (
                    egui_toast::ToastKind::Warning,
                    "2 logged component(s) could not be ticked\nX #9: component not in this version\nSETUP-Y.TP2 #0: mod not on disk"
                        .to_string()
                ),
            ]
        );
    }

    #[test]
    fn finish_raises_no_warning_when_every_line_ticked() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-all-ticked", log_path);
        app.wizard_state.step2.pending_weidu_log_reapply = true;
        let toasts_before = app.notification_manager.history().len();

        advance_pending_weidu_log_reapply(&mut app);

        assert_eq!(unticked_log_lines(&app.wizard_state.step2), Vec::new());
        assert_eq!(
            toasts_since(&app, toasts_before),
            vec![(
                egui_toast::ToastKind::Success,
                "WeiDU logs applied: 2 components selected".to_string()
            )]
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
    fn a_new_import_drops_lines_from_an_earlier_import() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-drops-earlier", log_path.clone());
        app.wizard_state.step2.update_selected_popup_open = false;
        app.wizard_state.step2.log_apply.lines = vec![crate::app::state::Step2LogLine {
            game_tab: "BG2EE".to_string(),
            mod_label: "SETUP-Z.TP2".to_string(),
            component_id: "0".to_string(),
            outcome: crate::app::state::Step2LogLineOutcome::NoMod,
        }];
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(log_path),
                ..WeiduLogImportForm::default()
            },
        );

        assert!(
            app.wizard_state
                .step2
                .log_apply
                .lines
                .iter()
                .all(|line| line.game_tab != "BG2EE"),
            "lines: {:?}",
            app.wizard_state.step2.log_apply.lines
        );
        assert_eq!(
            toasts_since(&app, toasts_before),
            vec![(
                egui_toast::ToastKind::Success,
                "WeiDU logs applied: 2 components selected".to_string()
            )]
        );
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
    fn the_success_toast_reads_only_the_selected_count() {
        assert_eq!(
            applied_toast_text(5),
            "WeiDU logs applied: 5 components selected"
        );
        assert_eq!(
            applied_toast_text(0),
            "WeiDU logs applied: 0 components selected"
        );
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

    #[test]
    fn check_log_counts_components_and_mods() {
        let root = TempRoot::new();
        let path = root.write_log(
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
        let root = TempRoot::new();
        let path = root.write_log("notes.txt", "shopping list\n// eggs\nmilk\n");
        assert_eq!(check_log(&path), LogCheck::NotALog);
    }

    #[test]
    fn an_unreadable_file_is_not_a_weidu_log() {
        let root = TempRoot::new();
        assert_eq!(check_log(&root.path.join("missing.log")), LogCheck::NotALog);
        let broken = root.write_log("broken.log", "~~ #0 #0 // nothing\n");
        assert_eq!(check_log(&broken), LogCheck::NotALog);
    }

    #[test]
    fn the_replace_warning_needs_a_checked_component() {
        let mut step2 = Step2State::default();
        assert!(!list_has_selection(&step2, None));
        step2.bgee_mods = vec![scanned_x_mod()];
        assert!(!list_has_selection(&step2, None));
        step2.bgee_mods[0].components[1].checked = true;
        assert!(list_has_selection(&step2, None));
        step2.bgee_mods.clear();
        step2.bg2ee_mods = vec![scanned_x_mod()];
        step2.bg2ee_mods[0].components[0].checked = true;
        assert!(list_has_selection(&step2, None));
    }

    fn one_selection() -> RescanSelection {
        RescanSelection {
            tp2_upper: "SETUP-X.TP2".to_string(),
            component_id: "1".to_string(),
            selected_order: Some(0),
            wlb_inputs: None,
        }
    }

    #[test]
    fn the_replace_warning_counts_a_pending_rescan_snapshot() {
        let step2 = Step2State::default();
        let first_tab_only = RescanSnapshot {
            bgee: vec![one_selection()],
            bg2ee: Vec::new(),
        };
        assert!(list_has_selection(&step2, Some(&first_tab_only)));
        let second_tab_only = RescanSnapshot {
            bgee: Vec::new(),
            bg2ee: vec![one_selection()],
        };
        assert!(list_has_selection(&step2, Some(&second_tab_only)));
        assert!(!list_has_selection(&step2, None));
        assert!(!list_has_selection(
            &step2,
            Some(&RescanSnapshot::default())
        ));
    }

    const QUEUE_LIST_ID: &str = "LOGIMPORTFLOW";

    fn app_with_a_queued_import(tag: &str, log_path: PathBuf, queued: bool) -> OrchestratorApp {
        let mut app = bgee_import_app(tag);
        app.workspace_view.loaded_workspace_id = Some(QUEUE_LIST_ID.to_string());
        let view = &mut app.workspace_view.step2;
        view.weidu_log_import_form = Some(WeiduLogImportForm {
            first: Some(log_path),
            fetch_missing: false,
            ..WeiduLogImportForm::default()
        });
        view.weidu_log_import_queued = queued;
        app
    }

    #[test]
    fn a_queued_import_waits_while_the_scan_runs() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-wait", log_path, true);
        app.wizard_state.step2.is_scanning = true;
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, false);

        assert!(app.workspace_view.step2.weidu_log_import_queued);
        assert!(app.workspace_view.step2.weidu_log_import_form.is_some());
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn a_queued_import_runs_on_a_finished_scan() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-run", log_path, true);
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts(0));
    }

    #[test]
    fn a_cancelled_scan_clears_the_wait_and_keeps_the_drawer() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-cancel", log_path, true);
        app.wizard_state.step2.last_scan_report = None;
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert!(app.workspace_view.step2.weidu_log_import_form.is_some());
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn no_queue_means_no_import_on_scan_end() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-none", log_path, false);
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert!(app.workspace_view.step2.weidu_log_import_form.is_some());
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn a_queued_import_is_dropped_when_leaving_the_list() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-leave", log_path, true);
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        app.nav = NavDestination::Home;
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(app.wizard_state.step2.weidu_log_import, None);
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn a_queued_import_ends_when_no_scan_is_running_without_an_edge() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-noedge", log_path.clone(), true);
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        assert!(!step2_scan_running(&app));
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, false);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts(0));

        let mut app = app_with_a_queued_import("logimport-queue-noedge-noreport", log_path, true);
        app.wizard_state.step2.last_scan_report = None;
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, false);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert!(app.workspace_view.step2.weidu_log_import_form.is_some());
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }
}
