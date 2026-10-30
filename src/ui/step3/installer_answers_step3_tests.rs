use super::{bind_workspace, open_component, open_missing, save_current};
use crate::app::state::{
    InstallerAnswerDraft, Step2ComponentState, Step2ModState, Step2Selection, Step3ItemState,
    WizardState,
};
use crate::app::wlb_inputs;

fn component(id: &str, language: u8, checked: bool) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: format!("Choose kit {id}"),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: format!("~QUEST/SETUP-QUEST.TP2~ #{language} #{id} // Choose kit: 1.1"),
        prompt_summary: Some("Choose a kit:\n1) Yes\n2) No".to_string()),
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
        selected_order: checked.then_some(1),
    }
}

fn mod_state(components: Vec<Step2ComponentState>) -> Step2ModState {
    Step2ModState {
        name: "Quest".to_string(),
        tp_file: "setup-quest.tp2".to_string(),
        tp2_path: "QUEST/SETUP-QUEST.TP2".to_string(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: components.iter().any(|value| value.checked),
        hidden_components: Vec::new(),
        components,
    }
}

fn item(source: &Step2ComponentState) -> Step3ItemState {
    Step3ItemState {
        tp_file: "setup-quest.tp2".to_string(),
        component_id: source.component_id.clone(),
        mod_name: "Quest".to_string(),
        component_label: source.label.clone(),
        raw_line: source.raw_line.clone(),
        prompt_summary: source.prompt_summary.clone(),
        prompt_events: source.prompt_events.clone(),
        selected_order: 7,
        block_id: "quest".to_string(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn fixture() -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step3.bgee_items = vec![item(&component("11", 3, true))];
    state
}

fn with_mirror() -> WizardState {
    let mut state = fixture();
    state.step2.bgee_mods = vec![mod_state(vec![component("11", 3, true)])];
    state
}

fn open_fixture(state: &mut WizardState) {
    assert!(open_component(state, "BGEE", 0));
}

fn answers(state: &mut WizardState, values: &[(&str, bool)]) {
    let editor = state.step3.installer_answers.as_mut().unwrap();
    editor.targets[editor.active].answers = values
        .iter()
        .map(|(text, blank_enter)| InstallerAnswerDraft {
            text: (*text).to_string(),
            blank_enter: *blank_enter,
        })
        .collect();
}

fn set_saved(item: &mut Step3ItemState, values: &[String]) {
    let spec = wlb_inputs::encode_answers(values).unwrap().unwrap();
    item.raw_line = wlb_inputs::replace_marker(&item.raw_line, &spec).unwrap();
}

#[test]
fn imported_step3_component_opens_and_saves_without_scanned_mods() {
    let mut state = fixture();
    assert_eq!(state.step2.bgee_mods, [] as [Step2ModState; 0]);
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line),
        Some("2")
    );
    assert_eq!(state.step2.bgee_mods, [] as [Step2ModState; 0]);
}

#[test]
fn reopening_saved_answers_preserves_exact_values_identity_and_selection_key() {
    let mut state = with_mirror();
    let values = vec!["1".to_string(), String::new(), "  Élan,kit  ".to_string()];
    set_saved(&mut state.step3.bgee_items[0], &values);
    state.step2.bgee_mods[0].components[0].raw_line = state.step3.bgee_items[0].raw_line.clone();
    state.step2.selected = Some(Step2Selection::Component {
        game_tab: "BGEE".to_string(),
        tp_file: "setup-quest.tp2".to_string(),
        component_id: "11".to_string(),
        component_key: state.step3.bgee_items[0].raw_line.clone(),
    });
    open_fixture(&mut state);
    let draft = &state.step3.installer_answers.as_ref().unwrap().targets[0];
    assert_eq!(draft.language, 3);
    assert!(draft.answers[1].blank_enter);
    assert_eq!(draft.answers[2].text, "  Élan,kit  ");
    answers(
        &mut state,
        &[("2", false), ("", true), ("  Élan,kit  ", false)],
    );
    save_current(&mut state).unwrap();
    let raw = &state.step3.bgee_items[0].raw_line;
    assert_eq!(wlb_inputs::parse_identity(raw).unwrap().language, 3);
    assert_eq!(
        wlb_inputs::decode_answers(wlb_inputs::marker_spec(raw).unwrap()).unwrap(),
        ["2", "", "  Élan,kit  "]
    );
    assert_eq!(state.step2.bgee_mods[0].components[0].raw_line, *raw);
    assert_eq!(state.step3.bgee_items[0].selected_order, 7);
    let Some(Step2Selection::Component { component_key, .. }) = &state.step2.selected else {
        panic!("component selection must remain selected");
    };
    assert_eq!(component_key, raw);
}

#[test]
fn missing_queue_uses_relevant_game_slots_and_existing_install_order() {
    let mut state = fixture();
    state.step1.game_install = "EET".to_string();
    state.step3.bgee_items.push(item(&component("12", 3, true)));
    state.step3.bg2ee_items = vec![item(&component("13", 2, true))];
    assert!(open_missing(&mut state));
    let editor = state.step3.installer_answers.as_ref().unwrap();
    let targets: Vec<_> = editor
        .targets
        .iter()
        .map(|target| (target.game_tab.as_str(), target.component_id.as_str()))
        .collect();
    assert_eq!(targets, [("BGEE", "11"), ("BGEE", "12"), ("BG2EE", "13")]);
    assert!(editor.review_missing);
}

#[test]
fn irrelevant_game_tree_and_nonprompt_parent_rows_do_not_block_next() {
    let mut state = fixture();
    state.step1.game_install = "BG2EE".to_string();
    state.step3.bg2ee_items = vec![item(&component("12", 0, true))];
    state.step3.bg2ee_items[0].prompt_summary = None;
    let mut parent = item(&component("13", 0, true));
    parent.is_parent = true;
    state.step3.bg2ee_items.push(parent);
    assert!(!open_missing(&mut state));
    assert_eq!(state.step3.installer_answers, None);
}

#[test]
fn deliberate_blank_enter_and_tagged_exact_answers_are_already_saved() {
    for values in [
        vec![String::new()],
        vec!["  Élan,kit  ".to_string(), String::new()],
        vec!["1".to_string()],
    ] {
        let mut state = fixture();
        set_saved(&mut state.step3.bgee_items[0], &values);
        assert!(!open_missing(&mut state), "{values:?}");
        assert_eq!(state.step3.installer_answers, None);
    }
}

#[test]
fn conditional_questions_do_not_require_one_answer_per_possible_question() {
    let mut state = fixture();
    state.step3.bgee_items[0].prompt_summary =
        Some("Choose kit\nRename kit\nAccept title\nProceed with install".to_string());
    set_saved(&mut state.step3.bgee_items[0], &["1".to_string()]);
    assert!(!open_missing(&mut state));
}

#[test]
fn evaluated_inactive_prompt_branch_is_not_missing() {
    let mut state = fixture();
    state.step3.bgee_items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
        summary_line: "Only on BG2EE".to_string(),
        game_allow: vec!["bg2ee".to_string()],
        ..Default::default()
    }];
    assert!(!open_missing(&mut state));
    assert_eq!(state.step3.installer_answers, None);
}

#[test]
fn malformed_saved_answers_open_but_cannot_be_silently_overwritten() {
    let mut state = fixture();
    state.step3.bgee_items[0]
        .raw_line
        .push_str(" // @wlb-inputs: bio-json-v1:not-json");
    let before = state.step3.bgee_items.clone();
    assert!(open_missing(&mut state));
    answers(&mut state, &[("2", false)]);
    assert!(save_current(&mut state).is_err());
    assert_eq!(state.step3.bgee_items, before);
}

#[test]
fn invalid_drafts_leave_both_component_stores_untouched() {
    for invalid in ["a\nb", "a\rb", "a\0b"] {
        let mut state = with_mirror();
        open_fixture(&mut state);
        answers(&mut state, &[(invalid, false)]);
        let before = (
            state.step2.bgee_mods.clone(),
            state.step3.bgee_items.clone(),
        );
        assert!(save_current(&mut state).is_err(), "{invalid:?}");
        assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
    }
}

#[test]
fn stale_changed_removed_or_replaced_rows_cannot_overwrite_saved_data() {
    for scenario in 0..3 {
        let mut state = fixture();
        open_fixture(&mut state);
        answers(&mut state, &[("2", false)]);
        match scenario {
            0 => state.step3.bgee_items[0]
                .raw_line
                .push_str(" // changed elsewhere"),
            1 => state.step3.bgee_items.clear(),
            _ => state.step3.bgee_items[0].block_id = "replacement".to_string(),
        }
        let before = state.step3.bgee_items.clone();
        assert!(save_current(&mut state).is_err());
        assert_eq!(state.step3.bgee_items, before);
    }
}

#[test]
fn stale_or_ambiguous_step2_mirror_rejects_before_step3_changes() {
    for duplicate in [false, true] {
        let mut state = with_mirror();
        open_fixture(&mut state);
        answers(&mut state, &[("2", false)]);
        if duplicate {
            state.step2.bgee_mods.push(state.step2.bgee_mods[0].clone());
        } else {
            state.step2.bgee_mods[0].components[0]
                .raw_line
                .push_str(" // changed elsewhere");
        }
        let before = (
            state.step2.bgee_mods.clone(),
            state.step3.bgee_items.clone(),
        );
        assert!(save_current(&mut state).is_err());
        assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
    }
}

#[test]
fn repeated_step3_identity_is_not_selected_arbitrarily() {
    let mut state = fixture();
    state
        .step3
        .bgee_items
        .push(state.step3.bgee_items[0].clone());
    if open_component(&mut state, "BGEE", 0) {
        answers(&mut state, &[("2", false)]);
        let before = state.step3.bgee_items.clone();
        assert!(save_current(&mut state).is_err());
        assert_eq!(state.step3.bgee_items, before);
    }
}

#[test]
fn saving_is_isolated_by_game_slot_and_language_not_step2_active_tab() {
    let mut state = fixture();
    state.step1.game_install = "EET".to_string();
    state.step2.active_game_tab = "BG2EE".to_string();
    state.step3.bgee_items.push(item(&component("11", 4, true)));
    state.step3.bg2ee_items = state.step3.bgee_items.clone();
    let second_before = state.step3.bg2ee_items.clone();
    let language_before = state.step3.bgee_items[1].clone();
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.bg2ee_items, second_before);
    assert_eq!(state.step3.bgee_items[1], language_before);
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line),
        Some("2")
    );
}

#[test]
fn iwdee_answers_save_to_first_game_slot() {
    let mut state = fixture();
    state.step1.game_install = "IWDEE".to_string();
    state.step3.active_game_tab = "IWDEE".to_string();
    assert!(open_component(&mut state, "IWDEE", 0));
    answers(&mut state, &[("1", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line),
        Some("1")
    );
    assert_eq!(state.step3.bg2ee_items, [] as [Step3ItemState; 0]);
}

#[test]
fn scanning_or_installing_locks_saving_without_erasing_values() {
    for installing in [false, true] {
        let mut state = fixture();
        open_fixture(&mut state);
        answers(&mut state, &[("2", false)]);
        if installing {
            state.step5.install_running = true;
        } else {
            state.step2.is_scanning = true;
        }
        let before = state.step3.bgee_items.clone();
        assert!(save_current(&mut state).is_err());
        assert_eq!(state.step3.bgee_items, before);
    }
}

#[test]
fn switching_workspace_clears_drafts_with_identical_components() {
    let mut state = fixture();
    open_fixture(&mut state);
    bind_workspace(&mut state, Some("first-workspace"));
    answers(&mut state, &[("2", false)]);
    let before = state.step3.bgee_items.clone();
    bind_workspace(&mut state, Some("second-workspace"));
    assert_eq!(state.step3.installer_answers, None);
    assert_eq!(state.step3.bgee_items, before);
    assert!(save_current(&mut state).is_err());
}

#[test]
fn saving_one_missing_component_closes_and_next_reopens_only_remaining_inputs() {
    let mut state = fixture();
    state.step3.bgee_items.push(item(&component("12", 3, true)));
    assert!(open_missing(&mut state));
    bind_workspace(&mut state, Some("answer-workspace"));
    answers(&mut state, &[("1", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.installer_answers, None);
    assert_eq!(state.step3.installer_answers_workspace, None);
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line),
        Some("1")
    );
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[1].raw_line),
        None
    );
    assert!(open_missing(&mut state));
    let editor = state.step3.installer_answers.as_ref().unwrap();
    assert_eq!(editor.targets.len(), 1);
    assert_eq!(editor.targets[editor.active].component_id, "12");
    answers(&mut state, &[("", true)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.installer_answers, None);
    assert!(!open_missing(&mut state));
}

#[test]
fn removing_all_answers_clears_marker_mirror_and_selection_without_losing_metadata() {
    for empty_rows in 0..3 {
        let mut state = with_mirror();
        state.step3.bgee_items[0]
            .raw_line
            .push_str(" // keep metadata");
        let base = state.step3.bgee_items[0].raw_line.clone();
        set_saved(
            &mut state.step3.bgee_items[0],
            &["1".to_string(), "2".to_string()],
        );
        state.step2.bgee_mods[0].components[0].raw_line =
            state.step3.bgee_items[0].raw_line.clone();
        state.step2.selected = Some(Step2Selection::Component {
            game_tab: "BGEE".to_string(),
            tp_file: "setup-quest.tp2".to_string(),
            component_id: "11".to_string(),
            component_key: state.step3.bgee_items[0].raw_line.clone(),
        });
        open_fixture(&mut state);
        bind_workspace(&mut state, Some("answer-workspace"));
        let values = vec![("", false); empty_rows];
        answers(&mut state, &values);
        save_current(&mut state).unwrap();
        assert_eq!(state.step3.installer_answers, None);
        assert_eq!(state.step3.installer_answers_workspace, None);
        assert_eq!(state.step3.bgee_items[0].raw_line, base);
        assert_eq!(state.step2.bgee_mods[0].components[0].raw_line, base);
        let Some(Step2Selection::Component { component_key, .. }) = &state.step2.selected else {
            panic!("component must remain selected");
        };
        assert_eq!(component_key, &base);
        assert_eq!(wlb_inputs::marker_spec(&base), None);
        assert!(open_missing(&mut state));
    }
}

#[test]
fn an_intentional_blank_enter_is_saved_but_an_empty_unmarked_answer_clears() {
    let mut state = fixture();
    open_fixture(&mut state);
    answers(&mut state, &[("", true)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.installer_answers, None);
    let spec = wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line).unwrap();
    assert_eq!(wlb_inputs::decode_answers(spec).unwrap(), [""]);
    assert!(!open_missing(&mut state));
    open_fixture(&mut state);
    answers(&mut state, &[("", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.installer_answers, None);
    assert_eq!(
        wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line),
        None
    );
    assert!(open_missing(&mut state));
}

#[test]
fn whitespace_only_answers_remain_exact_saved_responses() {
    let mut state = fixture();
    open_fixture(&mut state);
    answers(&mut state, &[("  ", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(state.step3.installer_answers, None);
    let spec = wlb_inputs::marker_spec(&state.step3.bgee_items[0].raw_line).unwrap();
    assert_eq!(wlb_inputs::decode_answers(spec).unwrap(), ["  "]);
    assert!(!open_missing(&mut state));
}

#[test]
fn a_missing_entry_among_filled_answers_rejects_save_and_keeps_editor_open() {
    let mut state = with_mirror();
    open_fixture(&mut state);
    answers(&mut state, &[("1", false), ("", false)]);
    let before = (
        state.step2.bgee_mods.clone(),
        state.step3.bgee_items.clone(),
    );
    assert!(save_current(&mut state).is_err());
    assert!(state.step3.installer_answers.is_some());
    assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
}

#[test]
fn clearing_answers_still_rejects_stale_or_ambiguous_mirrors_before_mutation() {
    for duplicate in [false, true] {
        let mut state = with_mirror();
        set_saved(&mut state.step3.bgee_items[0], &["1".to_string()]);
        state.step2.bgee_mods[0].components[0].raw_line =
            state.step3.bgee_items[0].raw_line.clone();
        open_fixture(&mut state);
        answers(&mut state, &[]);
        if duplicate {
            state.step2.bgee_mods.push(state.step2.bgee_mods[0].clone());
        } else {
            state.step2.bgee_mods[0].components[0]
                .raw_line
                .push_str(" // changed");
        }
        let before = (
            state.step2.bgee_mods.clone(),
            state.step3.bgee_items.clone(),
        );
        assert!(save_current(&mut state).is_err());
        assert!(state.step3.installer_answers.is_some());
        assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
    }
}

#[test]
fn queued_game_conditions_follow_each_target_not_the_active_step2_tab() {
    for active in ["BGEE", "BG2EE"] {
        let mut state = fixture();
        state.step1.game_install = "EET".to_string();
        state.step2.active_game_tab = active.to_string();
        state.step3.bg2ee_items = vec![item(&component("12", 0, true))];
        state.step3.bgee_items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
            summary_line: "Only BGEE question".to_string(),
            game_allow: vec!["bgee".to_string()],
            ..Default::default()
        }];
        state.step3.bg2ee_items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
            summary_line: "Only BG2EE question".to_string(),
            game_allow: vec!["bg2ee".to_string()],
            ..Default::default()
        }];
        assert!(open_missing(&mut state));
        let editor = state.step3.installer_answers.as_ref().unwrap();
        let ids: Vec<_> = editor
            .targets
            .iter()
            .map(|target| target.component_id.as_str())
            .collect();
        assert_eq!(ids, ["11", "12"], "active Step 2 tab: {active}");
    }
}

#[test]
fn imported_step3_plan_supplies_selected_component_predicates() {
    let mut state = fixture();
    state.step3.bgee_items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
        summary_line: "Selected quest component question".to_string(),
        condition: Some("MOD_IS_INSTALLED ~quest.tp2~ ~11~".to_string()),
        ..Default::default()
    }];
    assert_eq!(state.step2.bgee_mods, [] as [Step2ModState; 0]);
    assert!(open_missing(&mut state));
    assert_eq!(
        state
            .step3
            .installer_answers
            .as_ref()
            .unwrap()
            .targets
            .len(),
        1
    );
}

#[test]
fn an_empty_marker_is_missing_but_an_explicit_blank_enter_is_saved() {
    let mut state = fixture();
    state.step3.bgee_items[0]
        .raw_line
        .push_str(" // @wlb-inputs: ");
    assert!(open_missing(&mut state));
}

#[test]
fn game_file_conditions_use_each_queued_targets_own_game_folder() {
    let mut state = fixture();
    state.step1.game_install = "EET".to_string();
    state.step2.active_game_tab = "BGEE".to_string();
    state.step1.new_pre_eet_dir_enabled = false;
    state.step1.new_eet_dir_enabled = false;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bio_step3_prompt_dirs_{}_{unique}",
        std::process::id()
    ));
    let first = root.join("bgee");
    let second = root.join("bg2ee");
    std::fs::create_dir_all(first.join("override")).unwrap();
    std::fs::create_dir_all(second.join("override")).unwrap();
    std::fs::write(first.join("override/first.cre"), b"fixture").unwrap();
    std::fs::write(second.join("override/second.cre"), b"fixture").unwrap();
    state.step1.eet_bgee_game_folder = first.to_string_lossy().into_owned();
    state.step1.eet_bg2ee_game_folder = second.to_string_lossy().into_owned();
    state.step3.bg2ee_items = vec![item(&component("12", 0, true))];
    for (items, file) in [
        (&mut state.step3.bgee_items, "first.cre"),
        (&mut state.step3.bg2ee_items, "second.cre"),
    ] {
        items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
            summary_line: format!("Question for {file}"),
            condition: Some(format!("FILE_EXISTS_IN_GAME ~override/{file}~")),
            ..Default::default()
        }];
    }
    let opened = open_missing(&mut state);
    let ids = state.step3.installer_answers.as_ref().map(|editor| {
        editor
            .targets
            .iter()
            .map(|target| target.component_id.clone())
            .collect::<Vec<_>>()
    });
    std::fs::remove_file(first.join("override/first.cre")).unwrap();
    std::fs::remove_file(second.join("override/second.cre")).unwrap();
    std::fs::remove_dir(first.join("override")).unwrap();
    std::fs::remove_dir(second.join("override")).unwrap();
    std::fs::remove_dir(first).unwrap();
    std::fs::remove_dir(second).unwrap();
    std::fs::remove_dir(root).unwrap();
    assert!(opened);
    assert_eq!(ids.unwrap(), ["11", "12"]);
}

#[test]
fn changing_install_context_invalidates_an_existing_editor() {
    let mut state = fixture();
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    state.step1.game_install = "EET".to_string();
    let before = state.step3.bgee_items.clone();
    assert!(save_current(&mut state).is_err());
    assert_eq!(state.step3.bgee_items, before);
}
