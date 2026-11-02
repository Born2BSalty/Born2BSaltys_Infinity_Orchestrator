use super::*;
use crate::app::state::{Step2ComponentState, Step2ModState, Step2ScanReport};
use crate::settings::redesign_fields::UiLanguage;

#[test]
fn combination_language_rescan_gate_requires_a_matching_completed_scan() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("matching-locale-completion");
    app.workspace_view.modlist_id = "MATCHING".to_string();
    app.workspace_state.insert(
        "MATCHING".to_string(),
        crate::registry::workspace_model::ModlistWorkspaceState::default(),
    );
    app.wizard_state.step1.game_install = "BGEE".to_string();
    app.wizard_state.step1.language = "en_US".to_string();
    app.wizard_state.step2.bgee_mods =
        vec![mod_state("TEST/TEST.TP2", vec![comp("1", true, Some(1))])];
    for (locale, stale) in [("de_DE", "en_US"), ("en_US", "de_DE")] {
        assert!(app.select_mod_installation_language(locale));
        assert!(app.mod_language_rescan_required);
        app.workspace_view.current_step =
            crate::ui::workspace::state_workspace::WorkspaceStep::Step5;
        assert!(crate::ui::workspace::workspace_view::enforce_mod_language_rescan_gate(&mut app));
        assert_eq!(
            app.workspace_view.current_step,
            crate::ui::workspace::state_workspace::WorkspaceStep::Step2
        );
        crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
        app.wizard_state.step2.last_scan_report = Some(Step2ScanReport {
            preferred_locale: stale.to_string(),
            ..Default::default()
        });
        assert!(
            crate::ui::workspace::step2::step2_rescan_reconcile::reconcile_on_scan_complete(
                &mut app
            )
        );
        assert!(app.mod_language_rescan_required);
        crate::ui::workspace::step2::step2_rescan_reconcile::snapshot_current_selection(&mut app);
        let (sender, receiver) = std::sync::mpsc::channel();
        app.step2_scan_rx = Some(receiver);
        app.wizard_state.step2.is_scanning = true;
        sender
            .send(Step2ScanEvent::Finished {
                bgee_mods: vec![mod_state("TEST/TEST.TP2", vec![comp("1", false, None)])],
                bg2ee_mods: Vec::new(),
                mod_languages: crate::app::scan::ModInstallLanguages::default(),
                report: Box::new(Step2ScanReport {
                    preferred_locale: locale.to_string(),
                    ..Default::default()
                }),
            })
            .unwrap();
        app.poll_step2_channels();
        app.reconcile_scan_and_advance_log_imports();
        assert!(!app.mod_language_rescan_required);
        assert!(app.wizard_state.step2.bgee_mods[0].components[0].checked);
        assert_eq!(
            app.wizard_state.step2.bgee_mods[0].components[0].selected_order,
            Some(1)
        );
    }
}

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

#[test]
fn mod_language_setter_detaches_old_queued_events_and_preserves_selected_answers() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-detach");
    app.wizard_state.step1.language = "en_US".to_string();
    app.redesign_settings.language = UiLanguage::English;
    let mut component = comp("1", true, Some(7));
    component.raw_line.push_str(" WLB_INPUTS=kept");
    app.wizard_state.step2.bgee_mods = vec![mod_state("TEST/TEST.TP2", vec![component])];
    let original = app.wizard_state.step2.bgee_mods.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    app.step2_scan_rx = Some(receiver);
    let cancel = Arc::new(AtomicBool::new(false));
    app.step2_cancel = Some(Arc::clone(&cancel));
    app.wizard_state.step2.is_scanning = true;
    app.step2_progress_queue
        .push_back((1, 2, "old progress".to_string()));
    sender
        .send(Step2ScanEvent::Preview {
            bgee_mods: Vec::new(),
            bg2ee_mods: Vec::new(),
            total: 0,
        })
        .unwrap();
    sender
        .send(Step2ScanEvent::Finished {
            bgee_mods: Vec::new(),
            bg2ee_mods: Vec::new(),
            mod_languages: crate::app::scan::ModInstallLanguages::default(),
            report: Box::new(Step2ScanReport {
                preferred_locale: "en_US".to_string(),
                ..Default::default()
            }),
        })
        .unwrap();
    sender
        .send(Step2ScanEvent::Failed("obsolete".to_string()))
        .unwrap();
    sender.send(Step2ScanEvent::Canceled).unwrap();
    assert!(app.select_mod_installation_language(" DE-de "));
    assert!(cancel.load(std::sync::atomic::Ordering::Relaxed));
    assert!(app.step2_scan_rx.is_none());
    assert!(app.step2_cancel.is_none());
    assert!(app.step2_progress_queue.is_empty());
    assert!(!app.wizard_state.step2.is_scanning);
    assert!(app.wizard_state.step2.last_scan_report.is_none());
    assert!(app.mod_language_rescan_required);
    app.poll_step2_channels();
    assert_eq!(app.wizard_state.step2.bgee_mods, original);
    assert_eq!(app.wizard_state.step1.language, "de_DE");
    assert_eq!(app.redesign_settings.language, UiLanguage::English);
}

#[test]
fn mod_language_setter_normalized_reselection_keeps_worker_and_rejects_invalid() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-equivalent");
    app.wizard_state.step1.language = " DE-de ".to_string();
    let (_sender, receiver) = std::sync::mpsc::channel();
    app.step2_scan_rx = Some(receiver);
    assert!(app.select_mod_installation_language("de_DE"));
    assert!(app.step2_scan_rx.is_some());
    assert!(!app.mod_language_rescan_required);
    assert!(!app.select_mod_installation_language("unknown"));
    assert!(!app.select_mod_installation_language(""));
    assert_eq!(app.wizard_state.step1.language, "de_DE");
}

#[test]
fn mod_language_setter_rejects_each_active_or_pending_install_phase() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-install-guard");
    app.wizard_state.step1.language = "en_US".to_string();
    for phase in 0..6 {
        app.wizard_state.step5.start_install_requested = phase == 0;
        app.wizard_state.step5.prep_running = phase == 1;
        app.wizard_state.step5.install_running = phase == 2;
        app.wizard_state.step5.cancel_pending = phase == 3;
        app.wizard_state.step5.cancel_requested = phase == 4;
        let (_sender, receiver) = std::sync::mpsc::channel();
        app.step5_prep_rx = (phase == 5).then_some(receiver);
        assert!(!app.mod_installation_language_change_available());
        assert!(!app.select_mod_installation_language("de_DE"));
        assert_eq!(app.wizard_state.step1.language, "en_US");
    }
    app.step5_prep_rx = None;
    app.wizard_state.step5.cancel_requested = false;
    assert!(app.mod_installation_language_change_available());
}

#[test]
fn mod_language_settings_debounce_round_trip_exit_flush_and_failed_save_keep_memory() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-persistence");
    app.wizard_state.step1.language = "en_US".to_string();
    app.redesign_settings.language = UiLanguage::German;
    app.settings_store
        .save(&app.bio_settings_snapshot())
        .unwrap();
    assert!(app.select_mod_installation_language("de_DE"));
    let now = Instant::now();
    app.tick_bio_settings(now);
    assert_eq!(app.settings_store.load().unwrap().step1.language, "en_US");
    app.tick_bio_settings(now + Duration::from_millis(BIO_SETTINGS_DEBOUNCE_MS));
    let persisted = app.settings_store.load().unwrap();
    assert_eq!(persisted.step1.language, "de_DE");
    assert_eq!(persisted.general.language, UiLanguage::German);
    assert!(app.select_mod_installation_language("fr_FR"));
    app.flush_all_now();
    assert_eq!(app.settings_store.load().unwrap().step1.language, "fr_FR");
    let old_store = app.settings_store.clone();
    app.settings_store =
        crate::settings::store::SettingsStore::new_with_path(old_store.path().join("blocked.json"));
    assert!(app.select_mod_installation_language("de_DE"));
    app.tick_bio_settings(now + Duration::from_secs(2));
    app.tick_bio_settings(now + Duration::from_secs(4));
    assert_eq!(app.wizard_state.step1.language, "de_DE");
    assert_eq!(old_store.load().unwrap().step1.language, "fr_FR");
    app.settings_store = old_store;
}

#[test]
fn mod_language_english_german_english_persists_after_debounce_and_exit() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-round-trip-persist");
    app.wizard_state.step1.language = "en_US".to_string();
    app.redesign_settings.language = UiLanguage::German;
    app.flush_all_now();
    let start = Instant::now();
    for (index, locale) in [(1, "de_DE"), (2, "en_US")] {
        let previous = app.settings_store.load().unwrap().step1.language;
        assert!(app.select_mod_installation_language(locale));
        let now = start + Duration::from_secs(index * 3);
        app.tick_bio_settings(now);
        assert_eq!(app.settings_store.load().unwrap().step1.language, previous);
        assert_eq!(app.wizard_state.step1.language, locale);
        app.tick_bio_settings(now + Duration::from_millis(BIO_SETTINGS_DEBOUNCE_MS));
        let loaded = app.settings_store.load().unwrap();
        assert_eq!(loaded.step1.language, locale);
        assert_eq!(loaded.general.language, UiLanguage::German);
    }
    assert!(app.select_mod_installation_language("de_DE"));
    app.flush_all_now();
    assert_eq!(app.settings_store.load().unwrap().step1.language, "de_DE");
    assert!(app.select_mod_installation_language("en_US"));
    app.flush_all_now();
    assert_eq!(app.settings_store.load().unwrap().step1.language, "en_US");
}
