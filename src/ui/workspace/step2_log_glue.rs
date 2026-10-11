// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rfd::FileDialog;

use crate::app::app_step2_log::{
    apply_weidu_log_selection_from_path, imported_log_path, logged_ticks_for_run,
    refresh_logged_missing, reseed_added_mod_pending_downloads, resolve_bg2_weidu_log_path,
    resolve_bgee_weidu_log_path, tab_for_slot, tick_arrived_logged_mods, ticked_log_line_count,
    unticked_log_lines,
};
use crate::app::game_authority;
use crate::app::state::{Step1State, Step2LogUnticked, Step2State};
use crate::app::step2_action::Step2Action;
use crate::mods::log_file::LogFile;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::{LogCheck, RescanSnapshot, WeiduLogImportForm};
use crate::ui::workspace::step_action_dispatch::handle_step2_via_bio;
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

fn slot_tab(orchestrator: &OrchestratorApp, first_slot: bool) -> &'static str {
    tab_for_slot(&orchestrator.wizard_state.step1.game_install, first_slot)
}

fn apply_recorded_log(orchestrator: &mut OrchestratorApp, first_slot: bool, path: PathBuf) -> bool {
    apply_picked_weidu_log(orchestrator, first_slot, path);
    let Some(reason) = parse_failure(&orchestrator.wizard_state.step2.scan_status) else {
        return true;
    };
    let toast = format!(
        "Could not read the {} WeiDU log: {reason}",
        slot_tab(orchestrator, first_slot)
    );
    orchestrator.notification_manager.error(toast);
    false
}

fn picked_logs(first: Option<PathBuf>, second: Option<PathBuf>) -> Vec<(bool, PathBuf)> {
    [(true, first), (false, second)]
        .into_iter()
        .filter_map(|(first_slot, path)| path.map(|path| (first_slot, path)))
        .collect()
}

fn picked_logs_read(orchestrator: &mut OrchestratorApp, picked: &[(bool, PathBuf)]) -> bool {
    for (first_slot, path) in picked {
        if let Err(err) = LogFile::from_path(path) {
            let toast = format!(
                "Could not read the {} WeiDU log: {err}",
                slot_tab(orchestrator, *first_slot)
            );
            orchestrator.notification_manager.error(toast);
            return false;
        }
    }
    true
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

struct StagedLog {
    first_slot: bool,
    target: PathBuf,
    staged: Option<PathBuf>,
}

fn staged_log_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    target.with_file_name(name)
}

fn stage_imported_log(
    step1: &Step1State,
    first_slot: bool,
    picked: &Path,
) -> Result<StagedLog, String> {
    let target = imported_log_path(step1, first_slot)
        .ok_or_else(|| "the list has no WeiDU log folder".to_string())?;
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(|err| err.to_string())?;
    }
    if same_file(picked, &target) {
        return Ok(StagedLog {
            first_slot,
            target,
            staged: None,
        });
    }
    let staged = staged_log_path(&target);
    if let Err(err) = std::fs::copy(picked, &staged) {
        let _ = std::fs::remove_file(&staged);
        return Err(err.to_string());
    }
    Ok(StagedLog {
        first_slot,
        target,
        staged: Some(staged),
    })
}

fn discard_staged_logs(staged: &[StagedLog]) {
    for log in staged {
        if let Some(path) = &log.staged {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn stage_imported_logs(
    step1: &Step1State,
    picked: Vec<(bool, PathBuf)>,
) -> Result<Vec<StagedLog>, (bool, String)> {
    let mut staged = Vec::new();
    for (first_slot, path) in picked {
        match stage_imported_log(step1, first_slot, &path) {
            Ok(log) => staged.push(log),
            Err(err) => {
                discard_staged_logs(&staged);
                return Err((first_slot, err));
            }
        }
    }
    Ok(staged)
}

fn commit_staged_logs(staged: &[StagedLog]) -> Result<Vec<(bool, PathBuf)>, (bool, String)> {
    let mut kept = Vec::new();
    for (index, log) in staged.iter().enumerate() {
        if let Some(path) = &log.staged
            && let Err(err) = std::fs::rename(path, &log.target)
        {
            discard_staged_logs(&staged[index..]);
            return Err((log.first_slot, err.to_string()));
        }
        kept.push((log.first_slot, log.target.clone()));
    }
    Ok(kept)
}

fn keep_imported_logs(
    orchestrator: &mut OrchestratorApp,
    picked: Vec<(bool, PathBuf)>,
) -> Option<Vec<(bool, PathBuf)>> {
    let kept = stage_imported_logs(&orchestrator.wizard_state.step1, picked)
        .and_then(|staged| commit_staged_logs(&staged));
    match kept {
        Ok(kept) => Some(kept),
        Err((first_slot, err)) => {
            let toast = format!(
                "Could not save the {} WeiDU log: {err}",
                slot_tab(orchestrator, first_slot)
            );
            orchestrator.notification_manager.error(toast);
            None
        }
    }
}

pub fn import_weidu_logs(orchestrator: &mut OrchestratorApp, form: WeiduLogImportForm) {
    let WeiduLogImportForm {
        first,
        second,
        fetch_missing,
        ..
    } = form;
    let picked = picked_logs(first, second);
    if !picked_logs_read(orchestrator, &picked) {
        return;
    }
    let Some(kept) = keep_imported_logs(orchestrator, picked) else {
        return;
    };
    orchestrator.wizard_state.step2.log_apply.lines.clear();
    for (first_slot, path) in kept {
        if !apply_recorded_log(orchestrator, first_slot, path) {
            return;
        }
    }
    refresh_logged_missing(&mut orchestrator.wizard_state);
    reseed_added_mod_pending_downloads(&mut orchestrator.wizard_state);
    orchestrator.mark_workspace_dirty();
    save_draft(orchestrator);
    announce_log_results(orchestrator);
    if !fetch_missing || orchestrator.wizard_state.step2.logged_missing_count() == 0 {
        return;
    }
    handle_step2_via_bio(Step2Action::OpenUpdatePopup, orchestrator);
    let step2 = &mut orchestrator.wizard_state.step2;
    step2.versions_ui.log_pending_scope = true;
    step2.versions_ui.auto_check_pending = true;
    step2.update_selected_has_run = false;
}

pub(crate) fn tick_arrived_logged_mods_and_announce(
    orchestrator: &mut OrchestratorApp,
    arrived: &BTreeSet<String>,
) {
    if !tick_arrived_logged_mods(&mut orchestrator.wizard_state, arrived) {
        return;
    }
    orchestrator.mark_workspace_dirty();
    save_draft(orchestrator);
    announce_log_results(orchestrator);
}

pub(crate) fn arm_pending_log_ticks(step2: &mut Step2State, download_was_running: bool) {
    if download_was_running || !step2.update_selected_download_running {
        return;
    }
    let arrived = logged_ticks_for_run(step2);
    step2.pending_log_ticks.extend(arrived);
}

pub(crate) fn drop_log_ticks_outside_workspace(orchestrator: &mut OrchestratorApp) {
    if !matches!(
        orchestrator.nav,
        NavDestination::Workspace {
            modlist_id: Some(_)
        }
    ) {
        orchestrator.wizard_state.step2.pending_log_ticks.clear();
    }
}

pub(crate) const fn forget_import_callouts_on_scan_end(
    orchestrator: &mut OrchestratorApp,
    scan_ended: bool,
) {
    if !scan_ended && !step2_scan_running(orchestrator) {
        return;
    }
    if let Some(form) = orchestrator
        .workspace_view
        .step2
        .weidu_log_import_form
        .as_mut()
    {
        form.first_callout = None;
        form.second_callout = None;
    }
}

fn announce_log_results(orchestrator: &mut OrchestratorApp) {
    let step2 = &orchestrator.wizard_state.step2;
    let applied = applied_toast_text(ticked_log_line_count(step2));
    let unticked = grouped_unticked_text(&unticked_log_lines(step2));
    orchestrator
        .wizard_state
        .step2
        .scan_status
        .clone_from(&applied);
    orchestrator.notification_manager.success(applied);
    if let Some(unticked) = unticked {
        orchestrator.notification_manager.warn_persistent(unticked);
    }
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
fn applied_toast_text(selected: usize) -> String {
    let plural = if selected == 1 { "" } else { "s" };
    format!("WeiDU logs applied: {selected} component{plural} selected")
}

#[must_use]
pub(crate) fn grouped_unticked_text(unticked: &[Step2LogUnticked]) -> Option<String> {
    if unticked.is_empty() {
        return None;
    }
    let mut groups: Vec<((&str, &str), Vec<&str>)> = Vec::new();
    for entry in unticked {
        let key = (entry.mod_name.as_str(), entry.reason.as_str());
        if let Some((_, ids)) = groups.iter_mut().find(|(group, _)| *group == key) {
            ids.push(entry.component_id.as_str());
        } else {
            groups.push((key, vec![entry.component_id.as_str()]));
        }
    }
    let mut lines = vec![format!(
        "{} logged component(s) could not be ticked",
        unticked.len()
    )];
    lines.extend(groups.into_iter().map(|((mod_name, reason), ids)| {
        let ids = ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{mod_name}: {reason}: {ids}")
    }));
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::app::state::{
        LogPendingOrigin, Step2ComponentState, Step2ModState, Step2ScanReport, Step2UpdateAsset,
    };
    use crate::ui::workspace::state_workspace::{ImportCallout, RescanSelection};
    use crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete;
    use crate::ui::workspace::versions::versions_drawer::close_versions_drawer;

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

        fn folder(&self, name: &str) -> String {
            self.path.join(name).to_string_lossy().to_string()
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    const TWO_COMPONENT_LOG: &str =
        "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n";

    const ARRIVAL_LOG: &str = "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n~Y/SETUP-Y.TP2~ #0 #3 // Y three\n";

    const LIST_ID: &str = "LOGIMPORTFLOW";

    fn unchecked_component(component_id: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: component_id.to_string(),
            label: format!("Component {component_id}"),
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

    fn scanned_mod(name: &str, component_ids: &[&str]) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: format!("SETUP-{name}.TP2"),
            tp2_path: format!("{name}/SETUP-{name}.TP2"),
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
            components: component_ids
                .iter()
                .map(|id| unchecked_component(id))
                .collect(),
        }
    }

    fn scanned_x_mod() -> Step2ModState {
        scanned_mod("X", &["1", "2"])
    }

    fn bgee_import_app(tag: &str, root: &TempRoot) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.nav = NavDestination::Workspace {
            modlist_id: Some(LIST_ID.to_string()),
        };
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app.wizard_state.step1.bgee_log_folder = root.folder("slot_bgee");
        app
    }

    fn eet_import_app(tag: &str, root: &TempRoot) -> OrchestratorApp {
        let mut app = bgee_import_app(tag, root);
        app.wizard_state.step1.game_install = "EET".to_string();
        app.wizard_state.step1.eet_bgee_log_folder = root.folder("slot_eet_bgee");
        app.wizard_state.step1.eet_bg2ee_log_folder = root.folder("slot_eet_bg2ee");
        app
    }

    fn import_form(first: Option<PathBuf>, fetch_missing: bool) -> WeiduLogImportForm {
        WeiduLogImportForm {
            first,
            fetch_missing,
            ..WeiduLogImportForm::default()
        }
    }

    fn toasts_since(app: &OrchestratorApp, before: usize) -> Vec<(egui_toast::ToastKind, String)> {
        app.notification_manager
            .history()
            .iter()
            .skip(before)
            .map(|record| (record.kind, record.text.clone()))
            .collect()
    }

    fn success_text(selected: &str) -> (egui_toast::ToastKind, String) {
        (
            egui_toast::ToastKind::Success,
            format!("WeiDU logs applied: {selected} selected"),
        )
    }

    const MISSING_X_WARNING: &str =
        "2 logged component(s) could not be ticked\nSETUP-X.TP2: mod not on disk: #1, #2";

    fn missing_x_toasts() -> Vec<(egui_toast::ToastKind, String)> {
        vec![
            success_text("0 components"),
            (
                egui_toast::ToastKind::Warning,
                MISSING_X_WARNING.to_string(),
            ),
        ]
    }

    #[test]
    fn import_copies_each_log_to_its_tab_imported_log() {
        let root = TempRoot::new();
        let picked = root.write_log("picked_bgee.log", TWO_COMPONENT_LOG);
        let mut app = bgee_import_app("logimport-copy-bgee", &root);

        import_weidu_logs(&mut app, import_form(Some(picked.clone()), false));

        let kept = imported_log_path(&app.wizard_state.step1, true).expect("BGEE slot");
        assert!(kept.starts_with(&root.path), "{}", kept.display());
        assert_eq!(
            std::fs::read(&kept).expect("kept log"),
            std::fs::read(&picked).expect("picked log")
        );

        let first = root.write_log("eet_bgee.log", TWO_COMPONENT_LOG);
        let second = root.write_log("eet_bg2ee.log", "~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n");
        let mut app = eet_import_app("logimport-copy-eet", &root);
        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(first.clone()),
                second: Some(second.clone()),
                fetch_missing: false,
                ..WeiduLogImportForm::default()
            },
        );

        let step1 = &app.wizard_state.step1;
        let first_kept = imported_log_path(step1, true).expect("EET first slot");
        let second_kept = imported_log_path(step1, false).expect("EET second slot");
        assert_ne!(first_kept, second_kept);
        for (kept, picked) in [(first_kept, first), (second_kept, second)] {
            assert!(kept.starts_with(&root.path), "{}", kept.display());
            assert!(kept.ends_with("imported.log"), "{}", kept.display());
            assert_eq!(
                std::fs::read(&kept).expect("kept log"),
                std::fs::read(&picked).expect("picked log")
            );
        }
    }

    #[test]
    fn a_log_that_cannot_be_copied_stops_the_import() {
        let root = TempRoot::new();
        let picked = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let blocker = root.write_log("blocker", "a file, not a folder");
        let mut app = bgee_import_app("logimport-copy-fails", &root);
        app.wizard_state.step1.bgee_log_folder = blocker.join("slot").to_string_lossy().to_string();
        app.wizard_state.step2.bgee_mods = vec![scanned_x_mod()];
        app.wizard_state.step2.bgee_mods[0].components[0].checked = true;
        let trees_before = app.wizard_state.step2.bgee_mods.clone();
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(picked), true));

        assert_eq!(app.wizard_state.step2.bgee_mods, trees_before);
        assert!(!app.wizard_state.step2.update_selected_popup_open);
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(toasts.len(), 1, "toasts: {toasts:?}");
        assert_eq!(toasts[0].0, egui_toast::ToastKind::Error);
        assert!(
            toasts[0]
                .1
                .starts_with("Could not save the BGEE WeiDU log: "),
            "toast: {}",
            toasts[0].1
        );
    }

    #[test]
    fn a_failed_second_copy_leaves_the_first_imported_log_unchanged() {
        let root = TempRoot::new();
        let first = root.write_log("eet_bgee.log", TWO_COMPONENT_LOG);
        let second = root.write_log("eet_bg2ee.log", "~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n");
        let mut app = eet_import_app("logimport-second-copy-fails", &root);
        let first_kept = imported_log_path(&app.wizard_state.step1, true).expect("first slot");
        let second_kept = imported_log_path(&app.wizard_state.step1, false).expect("second slot");
        let earlier = "~Z/SETUP-Z.TP2~ #0 #5 // Z five\n";
        std::fs::create_dir_all(first_kept.parent().expect("first folder")).expect("first folder");
        std::fs::write(&first_kept, earlier).expect("earlier first log");
        std::fs::create_dir_all(staged_log_path(&second_kept)).expect("blocking folder");
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(
            &mut app,
            WeiduLogImportForm {
                first: Some(first),
                second: Some(second),
                fetch_missing: false,
                ..WeiduLogImportForm::default()
            },
        );

        assert_eq!(
            std::fs::read_to_string(&first_kept).expect("first log"),
            earlier
        );
        assert!(!staged_log_path(&first_kept).exists());
        assert!(!second_kept.exists());
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(toasts.len(), 1, "toasts: {toasts:?}");
        assert_eq!(toasts[0].0, egui_toast::ToastKind::Error);
        assert!(
            toasts[0]
                .1
                .starts_with("Could not save the BG2EE WeiDU log: "),
            "toast: {}",
            toasts[0].1
        );
    }

    #[test]
    fn import_toasts_at_once_even_with_missing_mods() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = bgee_import_app("logimport-toasts-at-once", &root);
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(log_path), true));

        assert!(app.wizard_state.step2.logged_missing_count() > 0);
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts());
    }

    #[test]
    fn import_opens_versions_on_missing_only_with_the_switch_on_and_missing_mods() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);

        let mut app = bgee_import_app("logimport-versions-on", &root);
        app.wizard_state.step2.update_selected_has_run = true;
        app.wizard_state
            .step2
            .update_selected_last_was_full_selection = true;
        app.wizard_state
            .step2
            .update_selected_last_selection_signature = Some(
            crate::app::state::update_selection_signature(&app.wizard_state.step2),
        );
        import_weidu_logs(&mut app, import_form(Some(log_path.clone()), true));
        let step2 = &app.wizard_state.step2;
        assert!(step2.update_selected_popup_open);
        assert!(step2.versions_ui.log_pending_scope);
        assert!(step2.versions_ui.auto_check_pending);
        assert!(!step2.update_selected_has_run);

        let mut app = bgee_import_app("logimport-versions-switch-off", &root);
        import_weidu_logs(&mut app, import_form(Some(log_path.clone()), false));
        assert!(!app.wizard_state.step2.update_selected_popup_open);
        assert!(!app.wizard_state.step2.versions_ui.log_pending_scope);

        let mut app = bgee_import_app("logimport-versions-nothing-missing", &root);
        app.wizard_state.step2.bgee_mods = vec![scanned_x_mod()];
        import_weidu_logs(&mut app, import_form(Some(log_path), true));
        assert_eq!(app.wizard_state.step2.logged_missing_count(), 0);
        assert!(!app.wizard_state.step2.update_selected_popup_open);
        assert!(!app.wizard_state.step2.versions_ui.log_pending_scope);
    }

    #[test]
    fn status_line_matches_the_green_toast() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = bgee_import_app("logimport-status-line", &root);
        app.wizard_state.step2.bgee_mods = vec![scanned_x_mod()];
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(log_path), true));

        let success = toasts_since(&app, toasts_before)
            .into_iter()
            .find(|(kind, _)| *kind == egui_toast::ToastKind::Success)
            .expect("a green toast");
        assert_eq!(app.wizard_state.step2.scan_status, success.1);
    }

    const SAVE_DRAFT_ID: &str = "LOGIMPORTSAVE";

    fn app_with_a_saved_list(tag: &str, root: &TempRoot) -> OrchestratorApp {
        let mut app = bgee_import_app(tag, root);
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
        app.wizard_state.step2.bgee_mods = vec![scanned_x_mod()];
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
    fn import_with_nothing_missing_saves_the_draft_without_versions() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-nothing-missing", &root);
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(log_path), true));

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        assert_eq!(step2.logged_missing_count(), 0);
        assert_eq!(
            toasts_since(&app, toasts_before),
            vec![success_text("2 components")]
        );
        assert_the_draft_was_saved_with_the_counts(&app);
    }

    #[test]
    fn a_new_import_drops_lines_from_an_earlier_import() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_saved_list("logimport-drops-earlier", &root);
        app.wizard_state.step2.log_apply.lines = vec![crate::app::state::Step2LogLine {
            game_tab: "BG2EE".to_string(),
            mod_label: "SETUP-Z.TP2".to_string(),
            component_id: "0".to_string(),
            outcome: crate::app::state::Step2LogLineOutcome::NoMod,
        }];
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(log_path), true));

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
            vec![success_text("2 components")]
        );
    }

    #[test]
    fn an_unreadable_log_stops_the_import() {
        let root = TempRoot::new();
        let log_path = root.path.join("missing").join("weidu.log");
        let mut app = bgee_import_app("logimport-unreadable-stops", &root);
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(log_path), true));

        let step2 = &app.wizard_state.step2;
        assert!(!step2.update_selected_popup_open);
        assert!(!step2.versions_ui.log_pending_scope);
        let kept = imported_log_path(&app.wizard_state.step1, true).expect("slot");
        assert!(!kept.exists(), "no log is kept: {}", kept.display());
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
        let mut app = bgee_import_app("logimport-parse-failure", &root);
        let toasts_before = app.notification_manager.history().len();

        import_weidu_logs(&mut app, import_form(Some(broken), true));

        let errors = toasts_since(&app, toasts_before)
            .into_iter()
            .filter(|(kind, _)| *kind == egui_toast::ToastKind::Error)
            .map(|(_, text)| text)
            .collect::<Vec<_>>();
        assert_eq!(errors.len(), 1, "errors: {errors:?}");
        assert!(
            errors[0].starts_with("Could not read the BGEE WeiDU log"),
            "toast: {}",
            errors[0]
        );
    }

    #[test]
    fn the_success_toast_names_the_selected_count() {
        assert_eq!(
            applied_toast_text(5),
            "WeiDU logs applied: 5 components selected"
        );
        assert_eq!(
            applied_toast_text(1),
            "WeiDU logs applied: 1 component selected"
        );
        assert_eq!(
            applied_toast_text(0),
            "WeiDU logs applied: 0 components selected"
        );
    }

    fn unticked(mod_name: &str, component_id: &str, reason: &str) -> Step2LogUnticked {
        Step2LogUnticked {
            game_tab: "BGEE".to_string(),
            mod_name: mod_name.to_string(),
            component_id: component_id.to_string(),
            reason: reason.to_string(),
        }
    }

    #[test]
    fn grouped_toast_lists_every_component_per_mod_and_reason() {
        assert_eq!(grouped_unticked_text(&[]), None);
        let entries = [
            unticked("SETUP-Y.TP2", "0", "mod not on disk"),
            unticked("X", "9", "component not in this version"),
            unticked("SETUP-Y.TP2", "3", "mod not on disk"),
            unticked("X", "4", "excluded by compatibility rules"),
            unticked("SETUP-Y.TP2", "12", "mod not on disk"),
            unticked("X", "11", "component not in this version"),
        ];
        assert_eq!(
            grouped_unticked_text(&entries).as_deref(),
            Some(
                "6 logged component(s) could not be ticked\n\
                 SETUP-Y.TP2: mod not on disk: #0, #3, #12\n\
                 X: component not in this version: #9, #11\n\
                 X: excluded by compatibility rules: #4"
            )
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

    fn selection(tp2_upper: &str, component_id: &str, order: usize) -> RescanSelection {
        RescanSelection {
            tp2_upper: tp2_upper.to_string(),
            component_id: component_id.to_string(),
            selected_order: Some(order),
            wlb_inputs: None,
        }
    }

    fn app_before_the_fetch(tag: &str, root: &TempRoot) -> OrchestratorApp {
        let mut app = bgee_import_app(tag, root);
        let slot = PathBuf::from(&app.wizard_state.step1.bgee_log_folder);
        std::fs::create_dir_all(&slot).expect("slot folder");
        std::fs::write(slot.join("imported.log"), ARRIVAL_LOG).expect("imported log");
        app.wizard_state.step2.bgee_mods = vec![scanned_x_mod(), scanned_mod("Z", &["5"])];
        crate::app::app_step2_log::refresh_logged_missing(&mut app.wizard_state);
        app
    }

    fn y_key() -> String {
        crate::app::mod_downloads::normalize_mod_download_tp2("SETUP-Y.TP2")
    }

    fn finish_the_rescan_with_y_on_disk(app: &mut OrchestratorApp) {
        app.wizard_state.step2.bgee_mods = vec![
            scanned_x_mod(),
            scanned_mod("Y", &["0", "3"]),
            scanned_mod("Z", &["5"]),
        ];
        app.workspace_view.step2.rescan_snapshot = Some(RescanSnapshot {
            bgee: vec![
                selection("SETUP-X.TP2", "1", 1),
                selection("SETUP-Z.TP2", "5", 2),
            ],
            bg2ee: Vec::new(),
        });
        app.workspace_view.step2.was_scanning = true;
        app.wizard_state.step2.is_scanning = false;
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
        assert!(reconcile_on_scan_complete(app));
    }

    fn checked_ids(app: &OrchestratorApp, name: &str) -> Vec<String> {
        app.wizard_state
            .step2
            .bgee_mods
            .iter()
            .filter(|mod_state| mod_state.name == name)
            .flat_map(|mod_state| mod_state.components.iter())
            .filter(|component| component.checked)
            .map(|component| component.component_id.clone())
            .collect()
    }

    fn asset(tp_file: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: tp_file.to_string(),
            source_id: "primary".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{tp_file}.zip"),
            asset_url: format!("https://example.test/{tp_file}.zip"),
            installed_source_ref: None,
        }
    }

    #[test]
    fn a_fetch_ticks_only_the_arrived_logged_mods() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-fetch-ticks", &root);
        assert_eq!(
            app.wizard_state.step2.logged_missing_keys(),
            BTreeSet::from([y_key()])
        );
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from([y_key()]);
        let toasts_before = app.notification_manager.history().len();

        finish_the_rescan_with_y_on_disk(&mut app);

        assert_eq!(checked_ids(&app, "Y"), vec!["0", "3"]);
        assert_eq!(
            checked_ids(&app, "X"),
            vec!["1"],
            "X #2 was unticked by hand and stays unticked"
        );
        assert_eq!(
            checked_ids(&app, "Z"),
            vec!["5"],
            "a hand tick outside the log stays ticked"
        );
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
        let toasts = toasts_since(&app, toasts_before);
        assert_eq!(toasts, vec![success_text("2 components")]);
        assert_eq!(app.wizard_state.step2.scan_status, toasts[0].1);
    }

    #[test]
    fn a_fetch_from_the_all_scope_also_ticks_arrived_logged_mods() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-fetch-all-scope", &root);
        let step2 = &mut app.wizard_state.step2;
        step2.update_selected_update_assets = vec![asset("SETUP-Z.TP2"), asset("SETUP-Y.TP2")];
        step2.update_selected_download_scope = None;
        step2.update_selected_download_running = true;
        arm_pending_log_ticks(step2, false);
        assert_eq!(step2.pending_log_ticks, BTreeSet::from([y_key()]));

        step2.pending_log_ticks.clear();
        step2.update_selected_download_scope = Some("z".to_string());
        arm_pending_log_ticks(step2, false);
        assert!(
            step2.pending_log_ticks.is_empty(),
            "a card fetch of a mod on disk ticks nothing"
        );

        step2.update_selected_download_scope = None;
        arm_pending_log_ticks(step2, true);
        assert!(
            step2.pending_log_ticks.is_empty(),
            "a refused start leaves the ticks down"
        );
        arm_pending_log_ticks(step2, false);
        step2.update_selected_download_running = false;

        finish_the_rescan_with_y_on_disk(&mut app);
        assert_eq!(checked_ids(&app, "Y"), vec!["0", "3"]);
    }

    #[test]
    fn a_failed_fetch_ticks_nothing_and_clears_the_pending_ticks() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-fetch-fails", &root);
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from([y_key()]);
        app.wizard_state.step2.update_selected_download_running = true;
        let (tx, rx) = std::sync::mpsc::channel();
        drop(tx);
        let mut rx = Some(rx);
        let _ = crate::app::app_step2_update_download::poll_step2_update_download(
            &mut app.wizard_state,
            &mut rx,
        );
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
        let toasts_before = app.notification_manager.history().len();

        finish_the_rescan_with_y_on_disk(&mut app);

        assert_eq!(checked_ids(&app, "Y"), Vec::<String>::new());
        assert!(
            toasts_since(&app, toasts_before)
                .iter()
                .all(|(kind, _)| *kind != egui_toast::ToastKind::Success)
        );
    }

    fn end_the_scan(app: &mut OrchestratorApp, report: Option<Step2ScanReport>) {
        app.wizard_state.step2.bgee_mods = vec![
            scanned_x_mod(),
            scanned_mod("Y", &["0", "3"]),
            scanned_mod("Z", &["5"]),
        ];
        app.workspace_view.step2.was_scanning = true;
        app.wizard_state.step2.is_scanning = false;
        app.wizard_state.step2.last_scan_report = report;
        assert!(reconcile_on_scan_complete(app));
    }

    #[test]
    fn a_cancelled_rescan_keeps_the_pending_log_ticks_for_the_next_scan() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-cancelled-rescan", &root);
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from([y_key()]);
        app.workspace_view.step2.rescan_snapshot = Some(RescanSnapshot {
            bgee: vec![selection("SETUP-X.TP2", "1", 1)],
            bg2ee: Vec::new(),
        });

        end_the_scan(&mut app, None);

        assert_eq!(
            app.wizard_state.step2.pending_log_ticks,
            BTreeSet::from([y_key()])
        );
        assert_eq!(checked_ids(&app, "Y"), Vec::<String>::new());

        finish_the_rescan_with_y_on_disk(&mut app);

        assert_eq!(checked_ids(&app, "Y"), vec!["0", "3"]);
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
    }

    #[test]
    fn a_fetch_after_a_cancelled_rescan_keeps_the_earlier_pending_ticks() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-fetch-after-cancel", &root);
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from([y_key()]);
        end_the_scan(&mut app, None);

        let step2 = &mut app.wizard_state.step2;
        step2.update_selected_update_assets = vec![asset("SETUP-Z.TP2")];
        step2.update_selected_download_scope = Some("z".to_string());
        step2.update_selected_download_running = true;
        arm_pending_log_ticks(step2, false);
        assert_eq!(step2.pending_log_ticks, BTreeSet::from([y_key()]));
        step2.update_selected_download_running = false;

        finish_the_rescan_with_y_on_disk(&mut app);

        assert_eq!(checked_ids(&app, "Y"), vec!["0", "3"]);
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
    }

    #[test]
    fn the_log_tick_runs_without_a_rescan_snapshot() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-tick-no-snapshot", &root);
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from([y_key()]);
        assert!(app.workspace_view.step2.rescan_snapshot.is_none());

        end_the_scan(&mut app, Some(Step2ScanReport::default()));

        assert_eq!(checked_ids(&app, "Y"), vec!["0", "3"]);
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
    }

    #[test]
    fn pending_log_ticks_are_dropped_outside_a_workspace() {
        let mut app = OrchestratorApp::new_isolated_for_test("logimport-ticks-outside");
        app.wizard_state.step2.pending_log_ticks = BTreeSet::from(["y".to_string()]);
        app.nav = NavDestination::Workspace {
            modlist_id: Some(LIST_ID.to_string()),
        };
        drop_log_ticks_outside_workspace(&mut app);
        assert!(!app.wizard_state.step2.pending_log_ticks.is_empty());

        app.nav = NavDestination::Home;
        drop_log_ticks_outside_workspace(&mut app);
        assert!(app.wizard_state.step2.pending_log_ticks.is_empty());
    }

    #[test]
    fn added_mod_entries_are_never_counted_as_missing_from_logs() {
        let root = TempRoot::new();
        let mut app = app_before_the_fetch("logimport-added-not-counted", &root);
        app.wizard_state.step2.log_pending_downloads.push(
            crate::app::state::Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: "gadget.tp2".to_string(),
                label: "Gadget".to_string(),
                requested_version: None,
                origin: LogPendingOrigin::AddedMod,
            },
        );
        assert_eq!(
            app.wizard_state.step2.logged_missing_keys(),
            BTreeSet::from([y_key()])
        );
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

    fn app_with_a_queued_import(
        tag: &str,
        root: &TempRoot,
        log_path: PathBuf,
        queued: bool,
    ) -> OrchestratorApp {
        let mut app = bgee_import_app(tag, root);
        app.workspace_view.loaded_workspace_id = Some(LIST_ID.to_string());
        let view = &mut app.workspace_view.step2;
        view.weidu_log_import_form = Some(import_form(Some(log_path), false));
        view.weidu_log_import_queued = queued;
        app
    }

    #[test]
    fn a_queued_import_waits_while_the_scan_runs() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-wait", &root, log_path, true);
        app.wizard_state.step2.is_scanning = true;
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
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
        let mut app = app_with_a_queued_import("logimport-queue-run", &root, log_path, true);
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts());
    }

    #[test]
    fn a_cancelled_scan_clears_the_wait_and_keeps_the_drawer() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app = app_with_a_queued_import("logimport-queue-cancel", &root, log_path, true);
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
        let mut app = app_with_a_queued_import("logimport-queue-none", &root, log_path, false);
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
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
        let mut app = app_with_a_queued_import("logimport-queue-leave", &root, log_path, true);
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
        app.nav = NavDestination::Home;
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, true);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    #[test]
    fn a_queued_import_ends_when_no_scan_is_running_without_an_edge() {
        let root = TempRoot::new();
        let log_path = root.write_log("weidu.log", TWO_COMPONENT_LOG);
        let mut app =
            app_with_a_queued_import("logimport-queue-noedge", &root, log_path.clone(), true);
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport::default());
        assert!(!step2_scan_running(&app));
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, false);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert_eq!(app.workspace_view.step2.weidu_log_import_form, None);
        assert_eq!(toasts_since(&app, toasts_before), missing_x_toasts());

        let mut app =
            app_with_a_queued_import("logimport-queue-noedge-noreport", &root, log_path, true);
        app.wizard_state.step2.last_scan_report = None;
        let toasts_before = app.notification_manager.history().len();

        advance_queued_weidu_log_import(&mut app, false);

        assert!(!app.workspace_view.step2.weidu_log_import_queued);
        assert!(app.workspace_view.step2.weidu_log_import_form.is_some());
        assert_eq!(app.wizard_state.step2.log_pending_downloads.len(), 0);
        assert_eq!(toasts_since(&app, toasts_before), Vec::new());
    }

    fn app_with_drawer_callouts(tag: &str) -> (OrchestratorApp, ImportCallout) {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        let callout = ImportCallout::Counted(crate::app::app_step2_log::LogShortfall {
            missing_mods: 1,
            unselectable_components: 2,
        });
        app.workspace_view.step2.weidu_log_import_form = Some(WeiduLogImportForm {
            first_callout: Some(callout),
            second_callout: Some(ImportCallout::Unreadable),
            ..WeiduLogImportForm::default()
        });
        (app, callout)
    }

    fn drawer_callouts(app: &OrchestratorApp) -> (Option<ImportCallout>, Option<ImportCallout>) {
        let form = app
            .workspace_view
            .step2
            .weidu_log_import_form
            .as_ref()
            .expect("the drawer stays");
        (form.first_callout, form.second_callout)
    }

    #[test]
    fn a_cancelled_or_running_scan_forgets_the_drawer_callouts() {
        let (mut app, _) = app_with_drawer_callouts("logimport-callouts-cancelled");
        app.workspace_view.step2.was_scanning = true;
        app.wizard_state.step2.is_scanning = false;
        app.wizard_state.step2.last_scan_report = None;
        let ended = reconcile_on_scan_complete(&mut app);
        forget_import_callouts_on_scan_end(&mut app, ended);
        assert_eq!(drawer_callouts(&app), (None, None));

        let (mut app, _) = app_with_drawer_callouts("logimport-callouts-running");
        app.wizard_state.step2.is_scanning = true;
        forget_import_callouts_on_scan_end(&mut app, false);
        assert_eq!(drawer_callouts(&app), (None, None));
    }

    #[test]
    fn a_finished_scan_forgets_the_drawer_callouts() {
        let (mut app, shortfall) = app_with_drawer_callouts("logimport-callouts-forgotten");

        forget_import_callouts_on_scan_end(&mut app, false);
        let form = app.workspace_view.step2.weidu_log_import_form.clone();
        assert_eq!(form.and_then(|form| form.first_callout), Some(shortfall));

        forget_import_callouts_on_scan_end(&mut app, true);
        assert_eq!(drawer_callouts(&app), (None, None));
    }
}
