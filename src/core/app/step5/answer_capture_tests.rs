use crate::app::game_authority::GameSlot;
use crate::app::state::{Step2ComponentState, Step2ModState, WizardState};
use crate::app::step5::answer_capture::{
    AnswerOrigin, CaptureComponentKey, CaptureError, CaptureEvent, CaptureRunKey, CaptureSession,
    ComponentOutcome, ConfirmedAnswers, apply_confirmed, frozen_components,
};
use crate::app::step5::scripted_inputs::load_from_logs;
use crate::app::wlb_inputs::{decode_answers, marker_spec, parse_identity};

fn run() -> CaptureRunKey {
    CaptureRunKey {
        run_id: 42,
        workspace_id: Some("CAPTURE00001".to_string()),
        generation: 7,
        game_install: "EET".to_string(),
    }
}

fn key(slot: GameSlot, language: u8) -> CaptureComponentKey {
    CaptureComponentKey {
        slot,
        identity: parse_identity(&format!("~MOD/MOD.TP2~ #{language} #17 // Label"))
            .expect("identity"),
    }
}

fn begin(session: &mut CaptureSession, component: &CaptureComponentKey) {
    session.observe_event(CaptureEvent::Phase(component.slot));
    session.observe_event(CaptureEvent::BeginComponent(vec![component.clone()]));
}

fn prompt(session: &mut CaptureSession, block: &str, position: u64, retry: bool) {
    session.observe_event(CaptureEvent::Prompt {
        block_key: block.to_string(),
        output_position: position,
        explicit_retry: retry,
    });
}

fn send(session: &mut CaptureSession, value: &str, origin: AnswerOrigin) {
    let occurrence = session.current_occurrence().expect("fresh prompt");
    assert!(
        session
            .record_sent(&occurrence, value, origin)
            .expect("checked send")
    );
}

fn finish(
    session: &mut CaptureSession,
    component: &CaptureComponentKey,
    outcome: ComponentOutcome,
) {
    session.observe_event(CaptureEvent::FinishComponent {
        component: component.clone(),
        outcome,
    });
}

#[test]
fn manual_wlb_current_run_mixed_prefix_commits_only_on_unique_success_once() {
    let component = key(GameSlot::First, 2);
    let mut session = CaptureSession::new(run(), vec![component.clone()]);
    begin(&mut session, &component);
    for (position, block, value, origin) in [
        (1, "A?", "A", AnswerOrigin::Scripted),
        (2, "B?", "B", AnswerOrigin::JsonFallback),
        (3, "C?", "", AnswerOrigin::Manual),
    ] {
        prompt(&mut session, block, position, false);
        send(&mut session, value, origin);
    }
    assert!(session.take_confirmed_answers().is_empty());
    finish(&mut session, &component, ComponentOutcome::Success);
    session.invalidate();
    let confirmed = session.take_confirmed_answers();
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].run, run());
    assert_eq!(confirmed[0].component, component);
    assert_eq!(confirmed[0].answers, ["A", "B", ""]);
    assert!(session.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_correction_and_explicit_retry_replace_the_logical_answer() {
    let component = key(GameSlot::First, 0);
    let mut session = CaptureSession::new(run(), vec![component.clone()]);
    begin(&mut session, &component);
    prompt(&mut session, "Question?", 1, false);
    send(&mut session, "wrong", AnswerOrigin::Manual);
    send(&mut session, "corrected", AnswerOrigin::Manual);
    prompt(&mut session, "Question?", 2, true);
    send(&mut session, "accepted", AnswerOrigin::Manual);
    finish(&mut session, &component, ComponentOutcome::Success);
    assert_eq!(session.take_confirmed_answers()[0].answers, ["accepted"]);
}

#[test]
fn manual_wlb_ambiguous_repeat_fails_closed_with_warning() {
    let component = key(GameSlot::First, 0);
    let mut session = CaptureSession::new(run(), vec![component.clone()]);
    begin(&mut session, &component);
    prompt(&mut session, "Identical?", 1, false);
    send(&mut session, "first", AnswerOrigin::Manual);
    prompt(&mut session, "Identical?", 2, false);
    if let Some(occurrence) = session.current_occurrence() {
        let _ = session.record_sent(&occurrence, "second", AnswerOrigin::Manual);
    }
    finish(&mut session, &component, ComponentOutcome::Success);
    assert!(session.take_confirmed_answers().is_empty());
    assert!(!session.take_warnings().is_empty());
    assert!(session.take_warnings().is_empty());
}

#[test]
fn manual_wlb_failed_cancelled_script_only_and_stale_sends_never_confirm() {
    for outcome in [ComponentOutcome::Failed, ComponentOutcome::Cancelled] {
        let component = key(GameSlot::First, 0);
        let mut session = CaptureSession::new(run(), vec![component.clone()]);
        begin(&mut session, &component);
        prompt(&mut session, "First?", 1, false);
        let stale = session.current_occurrence().expect("prompt");
        send(&mut session, "discard", AnswerOrigin::Manual);
        finish(&mut session, &component, outcome);
        assert!(session.take_confirmed_answers().is_empty());
        begin(&mut session, &component);
        prompt(&mut session, "New attempt?", 2, false);
        assert!(
            !session
                .record_sent(&stale, "stale", AnswerOrigin::Manual)
                .expect("ineligible")
        );
        send(&mut session, "automatic", AnswerOrigin::Scripted);
        finish(&mut session, &component, ComponentOutcome::Success);
        assert!(session.take_confirmed_answers().is_empty());
    }
}

#[test]
fn manual_wlb_missing_phase_and_ambiguous_batch_have_no_occurrence() {
    let first = key(GameSlot::First, 0);
    let second = key(GameSlot::Second, 0);
    let mut session = CaptureSession::new(run(), vec![first.clone(), second]);
    session.observe_event(CaptureEvent::BeginComponent(vec![first.clone()]));
    prompt(&mut session, "Unknown phase?", 1, false);
    assert!(session.current_occurrence().is_none());
    session.observe_event(CaptureEvent::Phase(GameSlot::First));
    session.observe_event(CaptureEvent::BeginComponent(vec![
        first,
        key(GameSlot::First, 2),
    ]));
    prompt(&mut session, "Batch?", 2, false);
    assert!(session.current_occurrence().is_none());
}

fn state() -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = "EET".to_string();
    state.step5.capture_run = Some(run());
    let component = Step2ComponentState {
        component_id: "17".to_string(),
        label: "Label".to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: "~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: old".to_string(),
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
        checked: true,
        selected_order: Some(1),
    };
    state.step2.bgee_mods.push(Step2ModState {
        name: "Mod".to_string(),
        tp_file: "MOD/MOD.TP2".to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: true,
        hidden_components: Vec::new(),
        components: vec![component],
    });
    state.step3.bgee_items =
        crate::app::controller::step3_sync::build_step3_items(&state.step2.bgee_mods);
    state
}

fn confirmed() -> ConfirmedAnswers {
    ConfirmedAnswers {
        run: run(),
        component: key(GameSlot::First, 2),
        component_attempt: 1,
        answers: vec!["a,b".to_string(), String::new()],
    }
}

#[test]
fn manual_wlb_apply_updates_both_trees_once_without_selection_changes() {
    let mut state = state();
    assert_eq!(frozen_components(&state), [key(GameSlot::First, 2)]);
    assert!(apply_confirmed(&mut state, confirmed()).expect("apply"));
    let raw = &state.step2.bgee_mods[0].components[0].raw_line;
    assert_eq!(
        decode_answers(marker_spec(raw).expect("marker")).expect("answers"),
        ["a,b", ""]
    );
    assert_eq!(
        raw,
        &state
            .step3
            .bgee_items
            .iter()
            .find(|item| !item.is_parent)
            .expect("leaf")
            .raw_line
    );
    assert_eq!(
        state.step2.bgee_mods[0].components[0].selected_order,
        Some(1)
    );
    assert_eq!(state.step5.capture_revision, 1);
    assert!(!apply_confirmed(&mut state, confirmed()).expect("idempotent"));
    assert_eq!(state.step5.capture_revision, 1);
}

#[test]
fn manual_wlb_apply_stale_language_slot_and_duplicate_fail_atomically() {
    for variation in 0..4 {
        let mut state = state();
        let mut confirmation = confirmed();
        match variation {
            0 => confirmation.run.generation += 1,
            1 => confirmation.component.identity.language = 0,
            2 => confirmation.component.slot = GameSlot::Second,
            _ => {
                let duplicate = state
                    .step3
                    .bgee_items
                    .iter()
                    .find(|item| !item.is_parent)
                    .expect("leaf")
                    .clone();
                state.step3.bgee_items.push(duplicate);
            }
        }
        let before2 = state.step2.bgee_mods.clone();
        let before3 = state.step3.bgee_items.clone();
        assert!(apply_confirmed(&mut state, confirmation).is_err());
        assert_eq!(state.step2.bgee_mods, before2);
        assert_eq!(state.step3.bgee_items, before3);
        assert_eq!(state.step5.capture_revision, 0);
    }
    let mut state = state();
    let mut foreign_confirmation = confirmed();
    foreign_confirmation.run.workspace_id = Some("OTHER0000001".to_string());
    assert!(matches!(
        apply_confirmed(&mut state, foreign_confirmation),
        Err(CaptureError::StaleRun)
    ));
}

#[test]
fn manual_wlb_replay_preserves_slot_language_and_rejects_duplicates_or_bad_tags() {
    let first = "~MOD/MOD.TP2~ #2 #17 // First // @wlb-inputs: bio-json-v1:[\"a,b\",\"\"]";
    let second = "~MOD/MOD.TP2~ #0 #17 // Second // @wlb-inputs: y,1";
    let logs = vec![
        (GameSlot::First, first.to_string()),
        (GameSlot::Second, second.to_string()),
    ];
    let plan = load_from_logs(&logs, vec![]).expect("typed queues");
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].component, key(GameSlot::First, 2));
    assert_eq!(plan.entries[0].answers, ["a,b", ""]);
    assert_eq!(plan.entries[1].component, key(GameSlot::Second, 0));
    assert!(load_from_logs(&[(GameSlot::First, format!("{first}\n{first}"))], vec![]).is_err());
    assert!(
        load_from_logs(
            &[(
                GameSlot::First,
                first.replace("bio-json-v1:", "bio-json-v9:")
            )],
            vec![]
        )
        .is_err()
    );
}

fn install_line(language: u8) -> String {
    format!(
        r#"[INFO mod_installer::installers] Installing mod Component {{ tp_file: "MOD/MOD.TP2", component: "17", lang: "{language}", component_name: "Label" }}"#
    )
}

fn stream_prompt(session: &mut CaptureSession, question: &str) {
    session.observe_line("[INFO] User input required");
    session.observe_line("[INFO] Question is:");
    session.observe_line(question);
}

#[test]
fn manual_wlb_stream_orders_paired_headers_and_mixed_actual_prefix() {
    let component = key(GameSlot::First, 2);
    let mut session = CaptureSession::new(run(), vec![component.clone()]);
    session.observe_line("Beginning pre EET install process");
    session.observe_line(&install_line(2));
    for (question, answer, origin) in [
        ("First?", "script", AnswerOrigin::Scripted),
        ("Second?", "json", AnswerOrigin::JsonFallback),
        ("Third?", "manual", AnswerOrigin::Manual),
    ] {
        stream_prompt(&mut session, question);
        assert_eq!(
            session
                .current_occurrence()
                .expect("fresh complete block")
                .block_key,
            question
        );
        send(&mut session, answer, origin);
    }
    assert!(session.take_confirmed_answers().is_empty());
    session.observe_line("SUCCESSFULLY INSTALLED Label");
    session.invalidate();
    let answers = session.take_confirmed_answers();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].answers, ["script", "json", "manual"]);
    assert_eq!(answers[0].component, component);
    assert!(session.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_stream_retry_replaces_and_ambiguous_repeat_suppresses() {
    for explicit_retry in [false, true] {
        let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
        session.observe_line("Beginning pre EET install process");
        session.observe_line(&install_line(2));
        stream_prompt(&mut session, "Choice?");
        send(&mut session, "wrong", AnswerOrigin::Manual);
        if explicit_retry {
            session.observe_line("Invalid answer, please retry");
        }
        stream_prompt(&mut session, "Choice?");
        let occurrence = session.current_occurrence().expect("new occurrence");
        let eligible = session
            .record_sent(&occurrence, "right", AnswerOrigin::Manual)
            .expect("valid answer");
        assert_eq!(eligible, explicit_retry);
        session.observe_line("SUCCESSFULLY INSTALLED Label");
        let confirmed = session.take_confirmed_answers();
        if explicit_retry {
            assert_eq!(confirmed[0].answers, ["right"]);
        } else {
            assert!(confirmed.is_empty());
            assert!(!session.take_warnings().is_empty());
        }
    }
}

#[test]
fn manual_wlb_stream_unknown_phase_wrong_language_and_unproven_success_are_ineligible() {
    for variation in 0..4 {
        let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
        if variation != 0 {
            session.observe_line("Beginning pre EET install process");
        }
        session.observe_line(&install_line(if variation == 1 { 0 } else { 2 }));
        stream_prompt(&mut session, "Choice?");
        if variation < 2 {
            assert!(session.current_occurrence().is_none());
        } else {
            send(&mut session, "answer", AnswerOrigin::Manual);
        }
        session.observe_line(if variation == 3 {
            "installation failed"
        } else {
            "SUCCESSFULLY INSTALLED Unknown"
        });
        session.invalidate();
        assert!(session.take_confirmed_answers().is_empty());
    }
}

#[test]
fn manual_wlb_scripted_failed_send_keeps_queue_and_capture_empty() {
    use crate::app::state::WizardState;
    use crate::app::step5::answer_capture::{
        CaptureComponentKey, CaptureEvent, CaptureRunKey, CaptureSession, ReplayEntry, ReplayPlan,
    };
    use crate::app::terminal::EmbeddedTerminal;
    use crate::app::wlb_inputs::parse_identity;
    let component = CaptureComponentKey {
        slot: GameSlot::First,
        identity: parse_identity("~MOD/MOD.TP2~ #0 #17 // Label").expect("identity"),
    };
    let mut session = CaptureSession::new(
        CaptureRunKey {
            run_id: 1,
            workspace_id: None,
            generation: 1,
            game_install: "BGEE".to_string(),
        },
        vec![component.clone()],
    );
    session.observe_event(CaptureEvent::BeginComponent(vec![component.clone()]));
    session.observe_event(CaptureEvent::Prompt {
        block_key: "Question?".to_string(),
        output_position: 1,
        explicit_retry: false,
    });
    let mut terminal = EmbeddedTerminal::new().expect("terminal");
    terminal.set_capture_session(session);
    assert_eq!(
        terminal.set_scripted_inputs(ReplayPlan {
            entries: vec![ReplayEntry {
                component,
                answers: vec!["first".to_string(), "unused".to_string()]
            }],
            legacy_fallback: vec![]
        }),
        2
    );
    let mut state = WizardState::default();
    assert!(!crate::app::step5::scripted::send_scripted(
        &mut state,
        &mut terminal,
        "fixture",
        "Question?",
        "text",
        "cycle",
        1
    ));
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("first")
    );
    assert!(terminal.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_explicit_retry_invalidates_previous_candidate_before_replacement_send() {
    let component = key(GameSlot::First, 2);
    let mut session = CaptureSession::new(run(), vec![component.clone()]);
    begin(&mut session, &component);
    prompt(&mut session, "Question?", 1, false);
    send(&mut session, "rejected", AnswerOrigin::Manual);
    prompt(&mut session, "Question?", 2, true);
    finish(&mut session, &component, ComponentOutcome::Success);
    assert!(
        session.take_confirmed_answers().is_empty(),
        "retry without a successful replacement send must not publish the rejected candidate"
    );
}

#[test]
fn manual_wlb_stream_batch_with_multiple_frozen_owners_suppresses_capture() {
    let mut session = CaptureSession::new(
        run(),
        vec![key(GameSlot::First, 0), key(GameSlot::First, 2)],
    );
    session.observe_line("Beginning pre EET install process");
    session.observe_line(r#"[INFO mod_installer::installers] Installing mod WeiduBatchedComponents [Component { tp_file: "MOD/MOD.TP2", component: "17", lang: "0", component_name: "One" }, Component { tp_file: "MOD/MOD.TP2", component: "17", lang: "2", component_name: "Two" }]"#);
    stream_prompt(&mut session, "Ambiguous owner?");
    assert!(session.current_occurrence().is_none());
    session.observe_line("SUCCESSFULLY INSTALLED One");
    assert!(session.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_stream_retry_without_successful_replacement_never_confirms_rejected_answer() {
    let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
    session.observe_line("Beginning pre EET install process");
    session.observe_line(&install_line(2));
    stream_prompt(&mut session, "Question?");
    send(&mut session, "rejected", AnswerOrigin::Manual);
    session.observe_line("Invalid answer, please retry");
    stream_prompt(&mut session, "Question?");
    session.observe_line("SUCCESSFULLY INSTALLED Label");
    assert!(session.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_stream_rejection_without_new_header_cannot_confirm() {
    let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
    session.observe_line("Beginning pre EET install process");
    session.observe_line(&install_line(2));
    stream_prompt(&mut session, "Question?");
    send(&mut session, "rejected", AnswerOrigin::Manual);
    session.observe_line("Invalid answer, please retry");
    session.observe_line("SUCCESSFULLY INSTALLED Label");
    assert!(session.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_stream_rejection_immediately_retires_stale_occurrence() {
    let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
    session.observe_line("Beginning pre EET install process");
    session.observe_line(&install_line(2));
    stream_prompt(&mut session, "Question?");
    let old = session.current_occurrence().expect("question");
    send(&mut session, "rejected", AnswerOrigin::Manual);
    session.observe_line("Invalid answer, please retry");
    assert!(session.current_occurrence().is_none());
    assert!(
        !session
            .record_sent(&old, "stale", AnswerOrigin::Manual)
            .expect("stale send")
    );
}

#[test]
fn manual_wlb_stream_rejection_distinct_replacement_clears_manual_provenance() {
    for origin in [AnswerOrigin::Manual, AnswerOrigin::Scripted] {
        let mut session = CaptureSession::new(run(), vec![key(GameSlot::First, 2)]);
        session.observe_line("Beginning pre EET install process");
        session.observe_line(&install_line(2));
        stream_prompt(&mut session, "Valid prefix?");
        send(&mut session, "prefix", AnswerOrigin::Scripted);
        stream_prompt(&mut session, "Question?");
        send(&mut session, "rejected", AnswerOrigin::Manual);
        session.observe_line("Invalid answer, please retry");
        stream_prompt(&mut session, "Corrected question wording?");
        send(&mut session, "accepted", origin);
        session.observe_line("SUCCESSFULLY INSTALLED Label");
        let confirmed = session.take_confirmed_answers();
        if origin == AnswerOrigin::Manual {
            assert_eq!(confirmed.len(), 1);
            assert_eq!(confirmed[0].answers, ["prefix", "accepted"]);
        } else {
            assert!(confirmed.is_empty());
        }
    }
}

fn official_component(id: i64, language: u8, label: &str) -> String {
    format!(
        r#"WeiduComponent {{ tp_file: "SETUP-BARDICWONDERS.TP2", name: "BardicWonders", lang: "{language}", component: "{id}", component_name: "Bardic Wonders", sub_component: "", version: "{label}" }}"#
    )
}

fn official_key(id: i64) -> CaptureComponentKey {
    CaptureComponentKey {
        slot: GameSlot::First,
        identity: parse_identity(&format!(
            "~BARDICWONDERS/SETUP-BARDICWONDERS.TP2~ #0 #{id} // Bardic Wonders"
        ))
        .expect("official identity"),
    }
}

fn official_start(session: &mut CaptureSession, component: &str) {
    session.observe_line(&format!(
        "[2026-10-29T01:15:20Z INFO mod_installer::installers] Installing mod WeiduBatchedComponents([{component}])"
    ));
    session.observe_line("[2026-10-29T01:15:29Z INFO mod_installer::runner] User Input required");
    session.observe_line("[2026-10-29T01:15:29Z INFO mod_installer::runner] Question is");
    session.observe_line("Copying and patching 1 file ...");
    session.observe_line("OPTIONAL: Make Garrick a Troubadour?");
    session.observe_line("Please choose one of the following:");
    session.observe_line("[1] Yes");
    session.observe_line("[2] No");
    session.observe_line("Please select 1 or 2 and press Enter.");
}

#[test]
fn manual_wlb_real_checked_completion_saves_and_exports_actual_manual_answers() {
    use crate::app::modlist_share::{base64url_decode, encode_share_payload_text};
    use crate::registry::share_export::overlay_workspace_wlb;
    use crate::registry::store_workspace::WorkspaceStore;
    use crate::registry::workspace_model::ModlistWorkspaceState;
    use crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard;
    use std::io::Read;

    let run = CaptureRunKey {
        game_install: "BGEE".to_string(),
        ..run()
    };
    let component = official_component(1008, 0, "Troubadour Kit");
    let mut session = CaptureSession::new(run.clone(), vec![official_key(1008)]);
    official_start(&mut session, &component);
    send(&mut session, "1", AnswerOrigin::Manual);
    session.observe_line(
        "[INFO mod_installer::parser] SUCCESSFULLY INSTALLED Bardic Wonders: Troubadour Kit",
    );
    assert!(
        session.take_confirmed_answers().is_empty(),
        "a shared mod-name prefix is not unique success evidence"
    );
    session.observe_line(&format!(
        "[INFO mod_installer::installers] Installed mod {component}"
    ));
    let mut answers = session.take_confirmed_answers();
    assert_eq!(
        answers.len(),
        1,
        "real checked completion must confirm the manual answer"
    );
    assert_eq!(answers[0].component, official_key(1008));
    assert_eq!(answers[0].answers, ["1"]);
    assert!(session.take_confirmed_answers().is_empty());

    let mut state = state();
    state.step1.game_install = "BGEE".to_string();
    state.step5.capture_run = Some(run);
    state.parser_choice_phrases = vec!["1) yes".to_string()];
    state.step2.bgee_mods[0].tp_file = "BARDICWONDERS/SETUP-BARDICWONDERS.TP2".to_string();
    let row = &mut state.step2.bgee_mods[0].components[0];
    row.component_id = "1008".to_string();
    row.raw_line =
        "~BARDICWONDERS/SETUP-BARDICWONDERS.TP2~ #0 #1008 // Bardic Wonders: Troubadour Kit"
            .to_string();
    state.step3.bgee_items =
        crate::app::controller::step3_sync::build_step3_items(&state.step2.bgee_mods);
    assert!(apply_confirmed(&mut state, answers.remove(0)).expect("apply real completion"));
    let workspace = extract_workspace_state_from_wizard(&state, &ModlistWorkspaceState::default());
    assert_eq!(workspace.order_bgee[0].wlb_inputs.as_deref(), Some("1"));
    let root = std::env::temp_dir().join(format!(
        "bio-real-completion-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let store = WorkspaceStore::new_with_path(root.join("workspace.json"));
    store.save(&workspace).expect("isolated save");
    let reopened = store.load().expect("reopen");
    let original = serde_json::json!({"format_version":1,"game_install":"BGEE","weidu_logs":{"bgee":"~BARDICWONDERS/SETUP-BARDICWONDERS.TP2~ #0 #1008 // Bardic Wonders: Troubadour Kit\n~BP-BGT-WORLDMAP/SETUP-BP-BGT-WORLDMAP.TP2~ #0 #0 // Worldmap // @wlb-inputs: 2"},"parser_choice_phrases":["1) yes"],"future_key":{"opaque":42}});
    let code = encode_share_payload_text(&original.to_string()).expect("code");
    let code = overlay_workspace_wlb(&code, &reopened).expect("export captured answer");
    let bytes =
        base64url_decode(code.strip_prefix("BIO-MODLIST-V1:").expect("prefix")).expect("decode");
    let mut text = String::new();
    flate2::read::ZlibDecoder::new(bytes.as_slice())
        .read_to_string(&mut text)
        .expect("inflate");
    let exported: serde_json::Value = serde_json::from_str(&text).expect("payload");
    let mut expected = original;
    expected["weidu_logs"]["bgee"] = serde_json::json!(
        "~BARDICWONDERS/SETUP-BARDICWONDERS.TP2~ #0 #1008 // Bardic Wonders: Troubadour Kit // @wlb-inputs: 1\n~BP-BGT-WORLDMAP/SETUP-BP-BGT-WORLDMAP.TP2~ #0 #0 // Worldmap // @wlb-inputs: 2"
    );
    assert_eq!(exported, expected);
    std::fs::remove_file(store.path()).expect("remove fixture file");
    std::fs::remove_dir(root).expect("remove empty fixture directory");
}

#[test]
fn manual_wlb_real_checked_completion_keeps_same_mod_component_attempts_separate() {
    let run = CaptureRunKey {
        game_install: "BGEE".to_string(),
        ..run()
    };
    let mut session = CaptureSession::new(run, vec![official_key(1008), official_key(2009)]);
    for (id, label, answer) in [
        (1008, "Troubadour Kit", "1"),
        (2009, "Mage/Bard multiclass", "2"),
    ] {
        let component = official_component(id, 0, label);
        official_start(&mut session, &component);
        send(&mut session, answer, AnswerOrigin::Manual);
        session.observe_line(&format!(
            "[INFO mod_installer::parser] SUCCESSFULLY INSTALLED Bardic Wonders: {label}"
        ));
        assert!(session.take_confirmed_answers().is_empty());
        session.observe_line(&format!(
            "[INFO mod_installer::installers] Installed mod {component}"
        ));
        let confirmed = session.take_confirmed_answers();
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].component, official_key(id));
        assert_eq!(confirmed[0].answers, [answer]);
    }
}

#[test]
fn manual_wlb_real_checked_completion_rejects_mismatched_or_untrusted_events() {
    let component = official_component(1008, 0, "Troubadour Kit");
    let good = format!("[INFO mod_installer::installers] Installed mod {component}");
    let invalid = [
        good.replace("SETUP-BARDICWONDERS.TP2", "OTHER.TP2"),
        good.replace("component: \"1008\"", "component: \"2009\""),
        good.replace("lang: \"0\"", "lang: \"1\""),
        good.replace("lang: \"0\", ", ""),
        good.replace("mod_installer::installers", "other::module"),
        format!("{good} with warnings: not clean"),
    ];
    for completion in invalid {
        let run = CaptureRunKey {
            game_install: "BGEE".to_string(),
            ..run()
        };
        let mut session = CaptureSession::new(run, vec![official_key(1008)]);
        official_start(&mut session, &component);
        send(&mut session, "1", AnswerOrigin::Manual);
        session.observe_line(&completion);
        assert!(
            session.take_confirmed_answers().is_empty(),
            "must not confirm {completion}"
        );
    }
}

#[test]
fn manual_wlb_real_checked_completion_does_not_bless_failed_or_scripted_attempts() {
    let component = official_component(1008, 0, "Troubadour Kit");
    for origin in [AnswerOrigin::Manual, AnswerOrigin::Scripted] {
        let run = CaptureRunKey {
            game_install: "BGEE".to_string(),
            ..run()
        };
        let mut session = CaptureSession::new(run, vec![official_key(1008)]);
        official_start(&mut session, &component);
        send(&mut session, "1", origin);
        if origin == AnswerOrigin::Manual {
            session.observe_line("ERROR Installing [Bardic Wonders: Troubadour Kit]");
        }
        session.observe_line(&format!(
            "[INFO mod_installer::installers] Installed mod {component}"
        ));
        assert!(session.take_confirmed_answers().is_empty());
    }
}
