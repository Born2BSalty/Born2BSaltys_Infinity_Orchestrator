use super::*;
use crate::app::state::{Step2ComponentState, Step2ModState, WizardState};
use crate::registry::workspace_model::{ModlistWorkspaceState, SelectionRecovery};
use crate::ui::workspace::workspace_state_loader::{
    extract_workspace_state_from_wizard, sync_step3_from_step2_if_changed,
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

fn saved(id: i64, language: u8) -> ComponentRef {
    ComponentRef {
        tp2: "TEST/TEST.TP2".to_string(),
        id,
        language,
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
        ..Default::default()
    }
}

fn recovery_app(game: &str) -> OrchestratorApp {
    let mut app = OrchestratorApp::new_isolated_for_test("language-recovery-regression");
    app.workspace_view.modlist_id = "RECOVERY".to_string();
    app.workspace_view.loaded_workspace_id = Some("RECOVERY".to_string());
    app.wizard_state.step1.game_install = game.to_string();
    app.wizard_state.step2.last_scan_report = Some(crate::app::state::Step2ScanReport::default());
    app.workspace_state
        .insert("RECOVERY".to_string(), ModlistWorkspaceState::default());
    app
}

fn notice(
    app: &OrchestratorApp,
    original: ComponentRef,
    language: u8,
    tab: &str,
) -> LanguageRecoveryNotice {
    LanguageRecoveryNotice {
        owner: "RECOVERY".to_string(),
        generation: app.workspace_view.step2.recovery_scan_generation,
        rows: vec![LanguageRecoveryRow {
            tab: tab.to_string(),
            original,
            available_language: Some(language),
            reason: String::new(),
        }],
    }
}

#[test]
fn language_recovery_model_legacy_and_unresolved_only_roundtrip() {
    let legacy: ModlistWorkspaceState = serde_json::from_str("{}").unwrap();
    assert!(legacy.selection_recovery.is_empty());
    assert!(legacy.is_empty());
    assert!(
        !serde_json::to_string(&legacy)
            .unwrap()
            .contains("selection_recovery")
    );
    let mut workspace = legacy;
    let mut original = saved(11, 0);
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
fn language_recovery_merge_keeps_reorder_new_rows_and_real_deselection() {
    let unresolved = saved(11, 0);
    let mut prior = ModlistWorkspaceState {
        order_bgee: vec![saved(1, 1), unresolved.clone(), saved(2, 1), saved(3, 1)],
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
        [saved(2, 1), unresolved, saved(1, 1), saved(4, 1)]
    );
    assert_eq!(
        extract_workspace_state_from_wizard(&wizard, &extracted),
        extracted
    );
}

#[test]
fn language_recovery_equal_positions_keep_sequence_without_duplicates() {
    let a = saved(10, 0);
    let b = saved(11, 0);
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
        ..Default::default()
    };
    let expected = vec![saved(1, 1), a, b, saved(2, 1)];
    let merged = merge_unresolved_order(vec![saved(1, 1), saved(2, 1)], Some(&mut state));
    assert_eq!(merged, expected);
    assert_eq!(merge_unresolved_order(merged, Some(&mut state)), expected);
}

#[test]
fn language_recovery_iwdee_alias_does_not_extract_bgee_unresolved() {
    let mut prior = ModlistWorkspaceState {
        order_bgee: vec![saved(90, 0)],
        order_iwdee: vec![saved(10, 0)],
        ..Default::default()
    };
    prior
        .selection_recovery
        .insert("BGEE".to_string(), recovery(saved(90, 0), 0));
    prior
        .selection_recovery
        .insert("IWDEE".to_string(), recovery(saved(10, 0), 0));
    let mut wizard = WizardState::default();
    wizard.step1.game_install = "IWDEE".to_string();
    wizard.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(2, 1, Some(1))])];
    sync_step3_from_step2_if_changed(&mut wizard);
    let extracted = extract_workspace_state_from_wizard(&wizard, &prior);
    assert_eq!(extracted.order_iwdee, [saved(10, 0), saved(2, 1)]);
    assert_eq!(extracted.order_bgee, prior.order_bgee);
    assert_eq!(
        extracted.selection_recovery["BGEE"],
        prior.selection_recovery["BGEE"]
    );
}

#[test]
fn language_recovery_candidates_reject_duplicate_malformed_disabled_and_exact() {
    let original = saved(11, 0);
    let unique = mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)]);
    assert_eq!(
        candidate(&original, std::slice::from_ref(&unique)),
        Ok((0, 0, 1))
    );
    assert!(
        candidate(&original, &[unique.clone(), unique.clone()])
            .unwrap_err()
            .contains("ambiguous")
    );
    let mut invalid = unique.clone();
    invalid.components[0].raw_line = "~OTHER/OTHER.TP2~ #1 #11 // wrong path".to_string();
    assert!(
        candidate(&original, &[invalid])
            .unwrap_err()
            .contains("invalid")
    );
    let mut disabled = unique;
    disabled.components[0].disabled = true;
    disabled.components[0].disabled_reason = Some("blocked fixture".to_string());
    assert!(candidate(&original, &[disabled]).is_err());
    assert!(
        candidate(
            &original,
            &[mod_state("TEST/TEST.TP2", vec![scanned(11, 0, None)])]
        )
        .is_err()
    );
    assert!(candidate(&original, &[]).unwrap_err().contains("mod"));
}

#[test]
fn language_recovery_action_revalidates_owner_generation_scan_and_candidate() {
    for mutation in 0..6 {
        let mut app = recovery_app("BGEE");
        let original = saved(11, 0);
        app.workspace_state
            .get_mut("RECOVERY")
            .unwrap()
            .selection_recovery
            .insert("BGEE".to_string(), recovery(original.clone(), 0));
        app.wizard_state.step2.bgee_mods =
            vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
        let action = notice(&app, original, 1, "BGEE");
        match mutation {
            0 => app.workspace_view.modlist_id = "OTHER".to_string(),
            1 => app.workspace_view.step2.recovery_scan_generation += 1,
            2 => app.wizard_state.step2.is_scanning = true,
            3 => app.wizard_state.step2.bgee_mods.clear(),
            4 => app.wizard_state.step2.bgee_mods[0].components[0].disabled = true,
            _ => app.wizard_state.step2.last_scan_report = None,
        }
        let before = app.workspace_state["RECOVERY"].clone();
        apply_recovery(&mut app, action);
        assert_eq!(
            app.workspace_state["RECOVERY"], before,
            "mutation {mutation}"
        );
        assert!(
            app.wizard_state
                .step2
                .bgee_mods
                .iter()
                .flat_map(|m| &m.components)
                .all(|c| !c.checked)
        );
    }
}

#[test]
fn language_recovery_acceptance_retains_answers_and_persists_new_identity() {
    let mut app = recovery_app("BGEE");
    let mut original = saved(11, 0);
    original.wlb_inputs = Some("yes,  ,C:\\old path".to_string());
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone(), saved(12, 0)];
    workspace.selection_recovery.insert(
        "BGEE".to_string(),
        SelectionRecovery {
            unresolved: vec![
                UnresolvedSelection {
                    original: original.clone(),
                    position: 0,
                },
                UnresolvedSelection {
                    original: saved(12, 0),
                    position: 1,
                },
            ],
            ..Default::default()
        },
    );
    let mut row = scanned(11, 1, None);
    row.raw_line
        .push_str(" // @wlb-inputs: old-language-marker");
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![row])];
    let action = notice(&app, original.clone(), 1, "BGEE");
    apply_recovery(&mut app, action);
    let selected = &app.wizard_state.step2.bgee_mods[0].components[0];
    assert!(selected.checked);
    assert_eq!(selected.selected_order, Some(1));
    assert!(!selected.raw_line.contains("@wlb-inputs"));
    assert_eq!(
        app.workspace_state["RECOVERY"].order_bgee,
        [saved(11, 1), saved(12, 0)]
    );
    let retained = &app.workspace_state["RECOVERY"].selection_recovery["BGEE"];
    assert_eq!(retained.unresolved.len(), 1);
    assert_eq!(retained.retained_answers[0].original, original);
    assert_eq!(retained.retained_answers[0].target, saved(11, 1));
    let fresh = app
        .wizard_state
        .step3
        .bgee_items
        .iter_mut()
        .find(|item| !item.is_parent)
        .unwrap();
    fresh.raw_line.push_str(" // @wlb-inputs: fresh");
    let extracted =
        extract_workspace_state_from_wizard(&app.wizard_state, &app.workspace_state["RECOVERY"]);
    assert_eq!(
        extracted.selection_recovery["BGEE"].retained_answers,
        retained.retained_answers
    );
    assert_eq!(extracted.order_bgee[0].wlb_inputs.as_deref(), Some("fresh"));
    let path = std::env::temp_dir().join(format!("recovery-answers-{}.json", std::process::id()));
    let store = crate::registry::store_workspace::WorkspaceStore::new_with_path(path);
    store.save(&extracted).unwrap();
    assert_eq!(store.load().unwrap(), extracted);
}

#[test]
fn language_recovery_mixed_survivor_order_saves_and_reopens() {
    for reordered in [false, true] {
        let mut app = recovery_app("BGEE");
        let original = saved(11, 0);
        let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
        workspace.order_bgee = vec![original.clone(), saved(1, 1), saved(2, 1)];
        workspace
            .selection_recovery
            .insert("BGEE".to_string(), recovery(original.clone(), 0));
        app.wizard_state.step2.bgee_mods = vec![mod_state(
            "TEST/TEST.TP2",
            vec![
                scanned(11, 1, None),
                scanned(1, 1, Some(if reordered { 2 } else { 1 })),
                scanned(2, 1, Some(if reordered { 1 } else { 2 })),
            ],
        )];
        sync_step3_from_step2_if_changed(&mut app.wizard_state);
        let action = notice(&app, original, 1, "BGEE");
        apply_recovery(&mut app, action);
        let active = app
            .wizard_state
            .step3
            .bgee_items
            .iter()
            .filter(|item| !item.is_parent)
            .map(|item| item.component_id.as_str())
            .collect::<Vec<_>>();
        let expected = if reordered {
            ["11", "2", "1"]
        } else {
            ["11", "1", "2"]
        };
        assert_eq!(active, expected);
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
        assert_eq!(loaded.order_bgee[0].language, 1);
        assert!(loaded.selection_recovery.is_empty());
    }
}

#[test]
fn language_recovery_exact_zero_one_two_keep_answers_and_ignore_unselected() {
    for language in [0, 1, 2] {
        let mut app = recovery_app("BGEE");
        let mut original = saved(11, language);
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
                language,
                selected_order: Some(1),
                wlb_inputs: original.wlb_inputs.clone(),
            }],
            ..Default::default()
        });
        app.workspace_view.step2.was_scanning = true;
        app.workspace_view.step2.resume_pending = true;
        assert!(
            crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(
                &mut app
            )
        );
        let rows = &app.wizard_state.step2.bgee_mods[0].components;
        assert!(rows[0].checked);
        assert!(!rows[1].checked);
        assert_eq!(rows[0].selected_order, Some(1));
        assert_eq!(
            crate::ui::workspace::workspace_state_loader::extract_wlb_inputs(&rows[0].raw_line),
            original.wlb_inputs
        );
        let extracted = extract_workspace_state_from_wizard(
            &app.wizard_state,
            &app.workspace_state["RECOVERY"],
        );
        assert_eq!(extracted.order_bgee, [original]);
        assert!(extracted.selection_recovery.is_empty());
    }
}

#[test]
fn language_recovery_no_report_and_warm_download_snapshot_retain_intent() {
    let mut app = recovery_app("BGEE");
    let original = saved(11, 0);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone()];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original.clone(), 0));
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![scanned(1, 1, Some(1))])];
    crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
    let snapshot = app.workspace_view.step2.rescan_snapshot.clone().unwrap();
    assert!(
        snapshot
            .bgee
            .iter()
            .any(|entry| entry.component_id == "11" && entry.language == 0)
    );
    app.wizard_state.step2.last_scan_report = None;
    assert!(
        crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(&mut app)
    );
    assert_eq!(app.workspace_view.step2.rescan_snapshot, Some(snapshot));
    assert_eq!(
        app.workspace_state["RECOVERY"].order_bgee,
        [original, saved(1, 1)]
    );
    crate::ui::workspace::step2::step2_rescan_reconcile::arm_post_download_snapshot(&mut app);
    assert!(
        app.workspace_view
            .step2
            .pending_update_download_snapshot
            .as_ref()
            .unwrap()
            .bgee
            .iter()
            .any(|entry| entry.component_id == "11" && entry.language == 0)
    );
}

#[test]
fn language_recovery_failed_preview_save_and_fresh_load_keep_original_intent() {
    let mut app = recovery_app("BGEE");
    let mut original = saved(11, 0);
    original.wlb_inputs = Some("retain me".to_string());
    app.workspace_state.get_mut("RECOVERY").unwrap().order_bgee = vec![original.clone()];
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 0, Some(1))])];
    app.wizard_state.step2.bgee_mods[0].components[0]
        .raw_line
        .push_str(" // @wlb-inputs: retain me");
    crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
    app.wizard_state.step2.last_scan_report = None;
    assert!(
        crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(&mut app)
    );
    crate::ui::workspace::workspace_header::save_draft(&mut app);
    eframe::App::on_exit(&mut app, None);
    let reopened = app.workspace_stores["RECOVERY"].load().unwrap();
    assert_eq!(reopened.order_bgee, [original.clone()]);
    assert_eq!(
        reopened.selection_recovery["BGEE"].unresolved[0].original,
        original
    );
    app.workspace_state.insert("RECOVERY".to_string(), reopened);
    crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
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
fn language_recovery_keeps_survivor_answers_fresher_in_step3() {
    let mut app = recovery_app("BGEE");
    let original = saved(11, 0);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone(), saved(1, 1)];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original.clone(), 0));
    let mut survivor = scanned(1, 1, Some(1));
    survivor.raw_line.push_str(" // @wlb-inputs: stale");
    app.wizard_state.step2.bgee_mods = vec![mod_state(
        "TEST/TEST.TP2",
        vec![scanned(11, 1, None), survivor],
    )];
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
    let action = notice(&app, original, 1, "BGEE");
    apply_recovery(&mut app, action);
    let extracted =
        extract_workspace_state_from_wizard(&app.wizard_state, &app.workspace_state["RECOVERY"]);
    assert_eq!(extracted.order_bgee[0], saved(11, 1));
    assert_eq!(
        extracted.order_bgee[1].wlb_inputs.as_deref(),
        Some("fresh-survivor")
    );
}

#[test]
fn language_recovery_eet_mixed_action_resolves_both_tabs_and_keeps_disabled() {
    let mut app = recovery_app("EET");
    let first = saved(11, 0);
    let second = saved(22, 0);
    let blocked = saved(23, 0);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![first.clone()];
    workspace.order_bg2ee = vec![second.clone(), blocked.clone()];
    workspace.order_iwdee = vec![saved(90, 2)];
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
                    original: blocked.clone(),
                    position: 1,
                },
            ],
            ..Default::default()
        },
    );
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 1, None)])];
    let mut disabled = scanned(23, 1, None);
    disabled.disabled = true;
    app.wizard_state.step2.bg2ee_mods = vec![mod_state(
        "TEST/TEST.TP2",
        vec![scanned(22, 2, None), disabled],
    )];
    let mut action = notice(&app, first, 1, "BGEE");
    action.rows.extend([
        LanguageRecoveryRow {
            tab: "BG2EE".to_string(),
            original: second,
            available_language: Some(2),
            reason: String::new(),
        },
        LanguageRecoveryRow {
            tab: "BG2EE".to_string(),
            original: blocked.clone(),
            available_language: Some(1),
            reason: String::new(),
        },
    ]);
    apply_recovery(&mut app, action);
    let workspace = &app.workspace_state["RECOVERY"];
    assert_eq!(workspace.order_bgee, [saved(11, 1)]);
    assert_eq!(workspace.order_bg2ee, [saved(22, 2), blocked.clone()]);
    assert_eq!(workspace.order_iwdee, [saved(90, 2)]);
    assert!(!workspace.selection_recovery.contains_key("BGEE"));
    assert_eq!(
        workspace.selection_recovery["BG2EE"].unresolved[0].original,
        blocked
    );
    assert!(app.wizard_state.step2.bg2ee_mods[0].components[0].checked);
    assert!(!app.wizard_state.step2.bg2ee_mods[0].components[1].checked);
}

#[test]
fn language_recovery_later_exact_match_resolves_then_deselection_stays_removed() {
    let mut app = recovery_app("BGEE");
    let original = saved(11, 0);
    let workspace = app.workspace_state.get_mut("RECOVERY").unwrap();
    workspace.order_bgee = vec![original.clone()];
    workspace
        .selection_recovery
        .insert("BGEE".to_string(), recovery(original, 0));
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![scanned(11, 0, None)])];
    crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
    assert!(
        crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(&mut app)
    );
    assert!(app.wizard_state.step2.bgee_mods[0].components[0].checked);
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
