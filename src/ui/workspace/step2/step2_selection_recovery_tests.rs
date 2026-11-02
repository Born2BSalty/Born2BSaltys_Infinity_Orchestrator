use super::*;
use crate::app::state::{Step2ComponentState, Step2ModState, WizardState};
use crate::registry::workspace_model::{ModlistWorkspaceState, SelectionRecovery};
use crate::ui::workspace::step2::step2_rescan_reconcile::{
    arm_post_download_snapshot, reconcile_on_scan_complete, snapshot_current_selection,
};
use crate::ui::workspace::workspace_state_loader::{
    extract_wlb_inputs, extract_workspace_state_from_wizard, sync_step3_from_step2_if_changed,
};

fn comp(id: &str, checked: bool, order: Option<usize>) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: String::new(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: format!("~FIXTURE/FIXTURE.TP2~ #0 #{id} // Fixture"),
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
        checked,
        selected_order: order,
    }
}

fn mod_state(tp: &str, mut comps: Vec<Step2ComponentState>) -> Step2ModState {
    for component in &mut comps {
        if component.raw_line.starts_with("~FIXTURE/FIXTURE.TP2~") {
            component.raw_line = format!("~{tp}~ #0 #{} // Fixture", component.component_id);
        }
    }
    Step2ModState {
        name: tp.to_string(),
        tp_file: tp.to_string(),
        tp2_path: String::new(),
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
        components: comps,
    }
}

fn saved(id: i64) -> ComponentRef {
    ComponentRef {
        tp2: "TEST/TEST.TP2".to_string(),
        id,
        wlb_inputs: None,
    }
}

fn scanned(id: i64, language: u8, selected: Option<usize>) -> Step2ComponentState {
    let mut row = comp(&id.to_string(), selected.is_some(), selected);
    row.raw_line = format!("~TEST/TEST.TP2~ #{language} #{id} // Component {id}");
    row
}

fn recovery(original: ComponentRef, position: usize) -> SelectionRecovery {
    SelectionRecovery {
        unresolved: vec![UnresolvedSelection { original, position }],
    }
}

fn recovery_app(game: &str) -> OrchestratorApp {
    let mut app = OrchestratorApp::new_isolated_for_test("selection-recovery-regression");
    app.workspace_view.modlist_id = "RECOVERY".to_string();
    app.workspace_view.loaded_workspace_id = Some("RECOVERY".to_string());
    app.wizard_state.step1.game_install = game.to_string();
    app.wizard_state.step2.last_scan_report = Some(crate::app::state::Step2ScanReport::default());
    app.workspace_state
        .insert("RECOVERY".to_string(), ModlistWorkspaceState::default());
    app
}

fn rescan(app: &mut OrchestratorApp) {
    app.workspace_view.step2.resume_pending = true;
    assert!(reconcile_on_scan_complete(app));
}

fn step3_ids(app: &OrchestratorApp) -> Vec<String> {
    app.wizard_state
        .step3
        .bgee_items
        .iter()
        .filter(|item| !item.is_parent)
        .map(|item| item.component_id.clone())
        .collect()
}

struct TempRoot(std::path::PathBuf);

impl TempRoot {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let root = Self(std::env::temp_dir().join(format!(
            "bio_selection_recovery_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir_all(&root.0).expect("create temp root");
        root
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn selection_recovery_model_legacy_and_unresolved_only_roundtrip() {
    let legacy: ModlistWorkspaceState = serde_json::from_str("{}").unwrap();
    assert!(legacy.selection_recovery.is_empty());
    assert!(legacy.is_empty());
    assert!(
        !serde_json::to_string(&legacy)
            .unwrap()
            .contains("selection_recovery")
    );
    let mut workspace = legacy;
    let mut original = saved(11);
    original.wlb_inputs = Some("yes,  ,C:\\old path".to_string());
    workspace
        .selection_recovery
        .insert("IWDEE".to_string(), recovery(original, 4));
    assert!(!workspace.is_empty());
    assert_eq!(
        serde_json::from_str::<ModlistWorkspaceState>(&serde_json::to_string(&workspace).unwrap())
            .unwrap(),
        workspace
    );
}

#[test]
fn retained_answers_in_old_json_loads_and_is_dropped_on_save() {
    let root = TempRoot::new();
    let path = root.0.join("workspace.json");
    std::fs::write(
        &path,
        r#"{"order_bgee":[{"tp2":"TEST/TEST.TP2","id":11,"language":0,"wlb_inputs":"yes"}],"selection_recovery":{"BGEE":{"unresolved":[{"original":{"tp2":"TEST/TEST.TP2","id":11,"language":0,"wlb_inputs":"yes"},"position":0}],"retained_answers":[{"original":{"tp2":"TEST/TEST.TP2","id":12,"language":0,"wlb_inputs":"old"},"target":{"tp2":"TEST/TEST.TP2","id":12,"language":1}}]}}}"#,
    )
    .unwrap();
    let store = crate::registry::store_workspace::WorkspaceStore::new_with_path(path.clone());
    let loaded = store.load().expect("old workspace loads");
    let mut original = saved(11);
    original.wlb_inputs = Some("yes".to_string());
    assert_eq!(loaded.order_bgee, [original.clone()]);
    assert_eq!(loaded.selection_recovery["BGEE"], recovery(original, 0));
    store.save(&loaded).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(!written.contains("retained_answers"), "got: {written}");
    assert!(!written.contains("\"language\""), "got: {written}");
    assert_eq!(store.load().unwrap(), loaded);
}

#[test]
fn selection_recovery_merge_keeps_reorder_new_rows_and_real_deselection() {
    let unresolved = saved(11);
    let mut prior = ModlistWorkspaceState {
        order_bgee: vec![saved(1), unresolved.clone(), saved(2), saved(3)],
        ..Default::default()
    };
    prior
        .selection_recovery
        .insert("BGEE".to_string(), recovery(unresolved.clone(), 1));
    let mut wizard = WizardState::default();
    wizard.step1.game_install = "BGEE".to_string();
    wizard.step2.bgee_mods = vec![mod_state(
        "TEST/TEST.TP2",
        vec![
            scanned(1, 1, Some(2)),
            scanned(2, 1, Some(1)),
            scanned(3, 1, None),
            scanned(4, 1, Some(3)),
        ],
    )];
    sync_step3_from_step2_if_changed(&mut wizard);
    let extracted = extract_workspace_state_from_wizard(&wizard, &prior);
    assert_eq!(
        extracted.order_bgee,
        [saved(2), unresolved, saved(1), saved(4)]
    );
    assert_eq!(
        extract_workspace_state_from_wizard(&wizard, &extracted),
        extracted
    );
}

#[test]
fn selection_recovery_equal_positions_keep_sequence_without_duplicates() {
    let a = saved(10);
    let b = saved(11);
    let mut state = SelectionRecovery {
        unresolved: vec![
            UnresolvedSelection {
                original: a.clone(),
                position: 1,
            },
            UnresolvedSelection {
                original: b.clone(),
                position: 1,
            },
        ],
    };
    let expected = vec![saved(1), a, b, saved(2)];
    let merged = merge_unresolved_order(vec![saved(1), saved(2)], Some(&mut state));
    assert_eq!(merged, expected);
    assert_eq!(merge_unresolved_order(merged, Some(&mut state)), expected);
}

#[test]
fn include_unresolved_keeps_first_of_duplicate_entries() {
    let app = recovery_app("BGEE");
    let entry = |tp2: &str, id: &str, order: usize| RescanSelection {
        tp2_upper: tp2.to_string(),
        component_id: id.to_string(),
        selected_order: Some(order),
        wlb_inputs: None,
    };
    let mut snapshot = RescanSnapshot {
        bgee: vec![
            entry("TEST/TEST.TP2", "3", 1),
            entry("TEST/TEST.TP2", "1", 2),
            entry("test/test.tp2", "3", 3),
        ],
        ..Default::default()
    };
    include_unresolved(&app, &mut snapshot);
    assert_eq!(
        snapshot
            .bgee
            .iter()
            .map(|entry| (entry.component_id.as_str(), entry.selected_order))
            .collect::<Vec<_>>(),
        [("3", Some(1)), ("1", Some(2))]
    );
}

#[test]
fn selection_recovery_iwdee_alias_does_not_extract_bgee_unresolved() {
    let mut prior = ModlistWorkspaceState {
        order_bgee: vec![saved(90)],
        order_iwdee: vec![saved(10)],
        ..Default::default()
    };
    prior
        .selection_recovery
        .insert("BGEE".to_string(), recovery(saved(90), 0));
    prior
        .selection_recovery
        .insert("IWDEE".to_string(), recovery(saved(10), 0));
    let mut wizard = WizardState::default();
    wizard.step1.game_install = "IWDEE".to_string();
    wizard.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(2, 1, Some(1))])];
    sync_step3_from_step2_if_changed(&mut wizard);
    let extracted = extract_workspace_state_from_wizard(&wizard, &prior);
    assert_eq!(extracted.order_iwdee, [saved(10), saved(2)]);
    assert_eq!(extracted.order_bgee, prior.order_bgee);
    assert_eq!(
        extracted.selection_recovery["BGEE"],
        prior.selection_recovery["BGEE"]
    );
}

#[test]
fn unresolved_reason_names_missing_duplicate_malformed_and_disabled_and_clears_unique() {
    let original = saved(11);
    for language in [0, 1, 2] {
        for selected in [None, Some(1)] {
            let unique = mod_state("TEST/TEST.TP2", vec![scanned(11, language, selected)]);
            assert_eq!(
                unresolved_reason(&original, std::slice::from_ref(&unique)),
                None
            );
        }
    }
    let unique = mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)]);
    assert_eq!(
        unresolved_reason(&original, &[unique.clone(), unique.clone()]).as_deref(),
        Some("ambiguous component candidates")
    );
    let mut invalid = unique.clone();
    invalid.components[0].raw_line = "~OTHER/OTHER.TP2~ #1 #11 // wrong path".to_string();
    assert_eq!(
        unresolved_reason(&original, &[invalid]).as_deref(),
        Some("invalid component identity")
    );
    let mut disabled = unique;
    disabled.components[0].disabled = true;
    disabled.components[0].disabled_reason = Some("blocked fixture".to_string());
    assert_eq!(
        unresolved_reason(&original, &[disabled]).as_deref(),
        Some("blocked fixture")
    );
    assert_eq!(
        unresolved_reason(
            &original,
            &[mod_state("TEST/TEST.TP2", vec![scanned(12, 1, None)])]
        )
        .as_deref(),
        Some("component no longer present")
    );
    assert_eq!(
        unresolved_reason(&original, &[]).as_deref(),
        Some("mod no longer present")
    );
}

#[test]
fn language_change_rescan_keeps_selection_and_order() {
    let mut app = recovery_app("BGEE");
    let mut answered = saved(11);
    answered.wlb_inputs = Some("y,1".to_string());
    app.workspace_state.get_mut("RECOVERY").unwrap().order_bgee = vec![saved(12), answered];
    let mut first = scanned(11, 0, Some(2));
    first.raw_line.push_str(" // @wlb-inputs: y,1");
    app.wizard_state.step2.bgee_mods = vec![mod_state(
        "TEST/TEST.TP2",
        vec![first, scanned(12, 0, Some(1)), scanned(13, 0, None)],
    )];
    snapshot_current_selection(&mut app);
    app.wizard_state.step2.bgee_mods = vec![mod_state(
        "TEST/TEST.TP2",
        vec![
            scanned(11, 1, None),
            scanned(12, 1, None),
            scanned(13, 1, None),
        ],
    )];
    rescan(&mut app);
    let rows = &app.wizard_state.step2.bgee_mods[0].components;
    assert!(rows[0].checked && rows[1].checked && !rows[2].checked);
    assert_eq!(rows[0].selected_order, Some(2));
    assert_eq!(rows[1].selected_order, Some(1));
    assert_eq!(
        extract_wlb_inputs(&rows[0].raw_line).as_deref(),
        Some("y,1")
    );
    assert_eq!(step3_ids(&app), ["12", "11"]);
    let workspace = &app.workspace_state["RECOVERY"];
    assert!(workspace.selection_recovery.is_empty());
    assert_eq!(
        workspace
            .order_bgee
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        [12, 11]
    );
    assert!(app.workspace_view.step2.rescan_drop_warning.is_none());
    let extracted = extract_workspace_state_from_wizard(&app.wizard_state, workspace);
    assert_eq!(extracted.order_bgee, workspace.order_bgee);
}

#[test]
fn missing_component_still_unresolved_with_reason() {
    let mut app = recovery_app("BGEE");
    let original = saved(11);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone(), saved(1)];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original.clone(), 0));
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![scanned(1, 1, Some(1))])];
    snapshot_current_selection(&mut app);
    rescan(&mut app);
    let workspace = &app.workspace_state["RECOVERY"];
    assert_eq!(workspace.order_bgee, [original.clone(), saved(1)]);
    assert_eq!(workspace.selection_recovery["BGEE"], recovery(original, 0));
    assert_eq!(
        app.workspace_view.step2.rescan_drop_warning.as_deref(),
        Some("1 saved component(s) remain unresolved")
    );
    let text = &app.notification_manager.history().back().unwrap().text;
    assert_eq!(
        text,
        "Some saved selections could not be restored\nBGEE / TEST/TEST.TP2 #11: component no longer present\nUnresolved selections stay saved. You can keep editing and return after another scan."
    );
}

#[test]
fn selection_recovery_later_language_match_restores_mixed_survivor_order_and_reopens() {
    for reordered in [false, true] {
        let mut app = recovery_app("BGEE");
        let original = saved(11);
        let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
        workspace.order_bgee = vec![original.clone(), saved(1), saved(2)];
        workspace
            .selection_recovery
            .insert("BGEE".to_string(), recovery(original, 0));
        app.wizard_state.step2.bgee_mods = vec![mod_state(
            "TEST/TEST.TP2",
            vec![
                scanned(11, 1, None),
                scanned(1, 1, Some(if reordered { 2 } else { 1 })),
                scanned(2, 1, Some(if reordered { 1 } else { 2 })),
            ],
        )];
        sync_step3_from_step2_if_changed(&mut app.wizard_state);
        snapshot_current_selection(&mut app);
        rescan(&mut app);
        let expected = if reordered {
            ["11", "2", "1"]
        } else {
            ["11", "1", "2"]
        };
        assert_eq!(step3_ids(&app), expected);
        crate::ui::workspace::workspace_header::save_draft(&mut app);
        eframe::App::on_exit(&mut app, None);
        let store = &app.workspace_stores["RECOVERY"];
        let loaded = store.load().unwrap();
        let expected_ids = expected.map(|id| id.parse::<i64>().unwrap());
        assert_eq!(
            loaded
                .order_bgee
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert!(loaded.selection_recovery.is_empty());
    }
}

#[test]
fn selection_recovery_exact_match_keeps_answers_and_ignores_unselected() {
    for language in [0, 1, 2] {
        let mut app = recovery_app("BGEE");
        let mut original = saved(11);
        original.wlb_inputs = Some("yes,  ,unchanged".to_string());
        app.workspace_state.get_mut("RECOVERY").unwrap().order_bgee = vec![original.clone()];
        app.wizard_state.step2.bgee_mods = vec![mod_state(
            "TEST/TEST.TP2",
            vec![scanned(11, language, None), scanned(12, language, None)],
        )];
        app.workspace_view.step2.rescan_snapshot = Some(RescanSnapshot {
            bgee: vec![RescanSelection {
                tp2_upper: original.tp2.clone(),
                component_id: "11".to_string(),
                selected_order: Some(1),
                wlb_inputs: original.wlb_inputs.clone(),
            }],
            ..Default::default()
        });
        app.workspace_view.step2.was_scanning = true;
        rescan(&mut app);
        let rows = &app.wizard_state.step2.bgee_mods[0].components;
        assert!(rows[0].checked);
        assert!(!rows[1].checked);
        assert_eq!(rows[0].selected_order, Some(1));
        assert_eq!(extract_wlb_inputs(&rows[0].raw_line), original.wlb_inputs);
        let extracted = extract_workspace_state_from_wizard(
            &app.wizard_state,
            &app.workspace_state["RECOVERY"],
        );
        assert_eq!(extracted.order_bgee, [original]);
        assert!(extracted.selection_recovery.is_empty());
    }
}

#[test]
fn selection_recovery_no_report_and_warm_download_snapshot_retain_intent() {
    let mut app = recovery_app("BGEE");
    let original = saved(11);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone()];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original.clone(), 0));
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![scanned(1, 1, Some(1))])];
    snapshot_current_selection(&mut app);
    let snapshot = app.workspace_view.step2.rescan_snapshot.clone().unwrap();
    assert!(snapshot.bgee.iter().any(|entry| entry.component_id == "11"));
    app.wizard_state.step2.last_scan_report = None;
    assert!(reconcile_on_scan_complete(&mut app));
    assert_eq!(app.workspace_view.step2.rescan_snapshot, Some(snapshot));
    assert_eq!(
        app.workspace_state["RECOVERY"].order_bgee,
        [original, saved(1)]
    );
    arm_post_download_snapshot(&mut app);
    assert!(
        app.workspace_view
            .step2
            .pending_update_download_snapshot
            .as_ref()
            .unwrap()
            .bgee
            .iter()
            .any(|entry| entry.component_id == "11")
    );
}

#[test]
fn selection_recovery_failed_preview_save_and_fresh_load_keep_original_intent() {
    let mut app = recovery_app("BGEE");
    let mut original = saved(11);
    original.wlb_inputs = Some("retain me".to_string());
    app.workspace_state.get_mut("RECOVERY").unwrap().order_bgee = vec![original.clone()];
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 0, Some(1))])];
    app.wizard_state.step2.bgee_mods[0].components[0]
        .raw_line
        .push_str(" // @wlb-inputs: retain me");
    snapshot_current_selection(&mut app);
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
    app.wizard_state.step2.last_scan_report = None;
    assert!(reconcile_on_scan_complete(&mut app));
    crate::ui::workspace::workspace_header::save_draft(&mut app);
    eframe::App::on_exit(&mut app, None);
    let reopened = app.workspace_stores["RECOVERY"].load().unwrap();
    assert_eq!(reopened.order_bgee, [original.clone()]);
    assert_eq!(
        reopened.selection_recovery["BGEE"].unresolved[0].original,
        original
    );
    app.workspace_state.insert("RECOVERY".to_string(), reopened);
    snapshot_current_selection(&mut app);
    assert_eq!(
        app.workspace_view
            .step2
            .rescan_snapshot
            .as_ref()
            .unwrap()
            .bgee
            .len(),
        1
    );
}

#[test]
fn selection_recovery_merge_keeps_survivor_answers_fresher_in_step3() {
    let mut app = recovery_app("BGEE");
    let original = saved(11);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone(), saved(1)];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original.clone(), 0));
    let mut survivor = scanned(1, 1, Some(1));
    survivor.raw_line.push_str(" // @wlb-inputs: stale");
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![survivor])];
    sync_step3_from_step2_if_changed(&mut app.wizard_state);
    let item = app
        .wizard_state
        .step3
        .bgee_items
        .iter_mut()
        .find(|item| !item.is_parent)
        .unwrap();
    item.raw_line =
        crate::app::wlb_inputs::replace_marker(&item.raw_line, "fresh-survivor").unwrap();
    let extracted =
        extract_workspace_state_from_wizard(&app.wizard_state, &app.workspace_state["RECOVERY"]);
    assert_eq!(extracted.order_bgee[0], original);
    assert_eq!(
        extracted.order_bgee[1].wlb_inputs.as_deref(),
        Some("fresh-survivor")
    );
}

#[test]
fn selection_recovery_eet_rescan_resolves_both_tabs_and_keeps_missing_and_disabled() {
    let root = TempRoot::new();
    let tp2_path = root.0.join("TEST.TP2");
    std::fs::write(
        &tp2_path,
        "BEGIN ~Kept~ DESIGNATED 22\nBEGIN ~Retired~ DESIGNATED 24\nDEPRECATED ~retired fixture~\n",
    )
    .unwrap();
    let mut app = recovery_app("EET");
    let first = saved(11);
    let second = saved(22);
    let missing = saved(23);
    let retired = saved(24);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![first.clone()];
    workspace.order_bg2ee = vec![second.clone(), missing.clone(), retired.clone()];
    workspace.order_iwdee = vec![saved(90)];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(first.clone(), 0));
    workspace.selection_recovery.insert(
        "BG2EE".to_string(),
        SelectionRecovery {
            unresolved: vec![
                UnresolvedSelection {
                    original: second.clone(),
                    position: 0,
                },
                UnresolvedSelection {
                    original: missing.clone(),
                    position: 1,
                },
                UnresolvedSelection {
                    original: retired.clone(),
                    position: 2,
                },
            ],
        },
    );
    snapshot_current_selection(&mut app);
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
    let mut bg2ee = mod_state(
        "TEST/TEST.TP2",
        vec![scanned(22, 2, None), scanned(24, 2, None)],
    );
    bg2ee.tp2_path = tp2_path.to_string_lossy().into_owned();
    app.wizard_state.step2.bg2ee_mods = vec![bg2ee];
    rescan(&mut app);
    let workspace = &app.workspace_state["RECOVERY"];
    assert_eq!(workspace.order_bgee, [first]);
    assert_eq!(
        workspace.order_bg2ee,
        [second, missing.clone(), retired.clone()]
    );
    assert_eq!(workspace.order_iwdee, [saved(90)]);
    assert!(!workspace.selection_recovery.contains_key("BGEE"));
    assert_eq!(
        workspace.selection_recovery["BG2EE"],
        SelectionRecovery {
            unresolved: vec![
                UnresolvedSelection {
                    original: missing,
                    position: 1,
                },
                UnresolvedSelection {
                    original: retired,
                    position: 2,
                },
            ],
        }
    );
    assert!(app.wizard_state.step2.bgee_mods[0].components[0].checked);
    let bg2ee_rows = &app.wizard_state.step2.bg2ee_mods[0].components;
    assert!(bg2ee_rows[0].checked);
    assert!(bg2ee_rows[1].disabled);
    assert_eq!(
        bg2ee_rows[1].disabled_reason.as_deref(),
        Some("retired fixture")
    );
    assert_eq!(
        app.workspace_view.step2.rescan_drop_warning.as_deref(),
        Some("2 saved component(s) remain unresolved")
    );
    let text = &app.notification_manager.history().back().unwrap().text;
    assert!(
        text.contains("BG2EE / TEST/TEST.TP2 #23: component no longer present"),
        "got: {text}"
    );
    assert!(
        text.contains("BG2EE / TEST/TEST.TP2 #24: retired fixture"),
        "got: {text}"
    );
}

#[test]
fn selection_recovery_later_exact_match_resolves_then_deselection_stays_removed() {
    let mut app = recovery_app("BGEE");
    let original = saved(11);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone()];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original, 0));
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
    snapshot_current_selection(&mut app);
    assert!(reconcile_on_scan_complete(&mut app));
    assert!(app.wizard_state.step2.bgee_mods[0].components[0].checked);
    assert_eq!(
        app.wizard_state.step2.bgee_mods[0].components[0].selected_order,
        Some(1)
    );
    assert!(
        app.workspace_state["RECOVERY"]
            .selection_recovery
            .is_empty()
    );
    app.wizard_state.step2.bgee_mods[0].components[0].checked = false;
    app.wizard_state.step2.bgee_mods[0].components[0].selected_order = None;
    sync_step3_from_step2_if_changed(&mut app.wizard_state);
    let extracted =
        extract_workspace_state_from_wizard(&app.wizard_state, &app.workspace_state["RECOVERY"]);
    assert_eq!(extracted.order_bgee, [] as [ComponentRef; 0]);
}
