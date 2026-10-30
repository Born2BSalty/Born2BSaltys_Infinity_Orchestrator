use super::{bind_workspace, checked_snapshot, open_newly_selected, open_selected, save_current};
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
        tp2_path: "fixture/quest/setup-quest.tp2".to_string(),
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

fn fixture() -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step2.bgee_mods = vec![mod_state(vec![component("11", 3, true)])];
    state
}

fn selection(state: &WizardState) -> Step2Selection {
    let mod_state = &state.step2.bgee_mods[0];
    Step2Selection::Component {
        game_tab: state.step2.active_game_tab.clone(),
        tp_file: mod_state.tp_file.clone(),
        component_id: mod_state.components[0].component_id.clone(),
        component_key: mod_state.components[0].raw_line.clone(),
    }
}

fn open_fixture(state: &mut WizardState) {
    let target = selection(state);
    assert!(open_selected(state, &target));
}

fn answers(state: &mut WizardState, values: &[(&str, bool)]) {
    state.step2.installer_answers.as_mut().unwrap().targets[0].answers = values
        .iter()
        .map(|(text, blank_enter)| InstallerAnswerDraft {
            text: (*text).to_string(),
            blank_enter: *blank_enter,
        })
        .collect();
}

fn step3_item(source: &Step2ComponentState) -> Step3ItemState {
    Step3ItemState {
        tp_file: "setup-quest.tp2".to_string(),
        component_id: source.component_id.clone(),
        mod_name: "Quest".to_string(),
        component_label: source.label.clone(),
        raw_line: source.raw_line.clone(),
        prompt_summary: source.prompt_summary.clone(),
        prompt_events: Vec::new(),
        selected_order: 7,
        block_id: "quest".to_string(),
        is_parent: false,
        parent_placeholder: false,
    }
}

#[test]
fn existing_selection_does_not_automatically_open_the_editor() {
    let mut state = fixture();
    let before = checked_snapshot(&state);
    open_newly_selected(&mut state, &before);
    assert_eq!(state.step2.installer_answers, None);
}

#[test]
fn bulk_selection_queues_only_newly_selected_enabled_prompted_components() {
    let mut state = fixture();
    state.step2.bgee_mods[0].components.extend([
        component("12", 3, false),
        component("13", 3, false),
        component("14", 3, false),
        component("15", 3, false),
    ]);
    state.step2.bgee_mods[0].components[3].prompt_summary = None;
    state.step2.bgee_mods[0].components[4].disabled = true;
    let before = checked_snapshot(&state);
    for value in &mut state.step2.bgee_mods[0].components[1..] {
        value.checked = true;
    }
    open_newly_selected(&mut state, &before);
    let editor = state.step2.installer_answers.as_ref().unwrap();
    let ids: Vec<_> = editor
        .targets
        .iter()
        .map(|target| target.component_id.as_str())
        .collect();
    assert_eq!(ids, ["12", "13"]);
    assert_eq!(editor.active, 0);
}

#[test]
fn reopening_saved_answers_and_saving_preserves_exact_values_and_identity() {
    let mut state = fixture();
    let expected = vec!["1".to_string(), String::new(), "  Élan,kit  ".to_string()];
    let spec = wlb_inputs::encode_answers(&expected).unwrap().unwrap();
    let original = state.step2.bgee_mods[0].components[0].raw_line.clone();
    state.step2.bgee_mods[0].components[0].raw_line =
        wlb_inputs::replace_marker(&original, &spec).unwrap();
    state.step3.bgee_items = vec![step3_item(&state.step2.bgee_mods[0].components[0])];
    state.step2.selected = Some(selection(&state));
    open_fixture(&mut state);
    let draft = &state.step2.installer_answers.as_ref().unwrap().targets[0];
    assert_eq!(draft.language, 3);
    assert!(draft.answers[1].blank_enter);
    assert_eq!(draft.answers[2].text, "  Élan,kit  ");
    answers(
        &mut state,
        &[("2", false), ("", true), ("  Élan,kit  ", false)],
    );
    save_current(&mut state).unwrap();
    let raw = &state.step2.bgee_mods[0].components[0].raw_line;
    assert_eq!(wlb_inputs::parse_identity(raw).unwrap().language, 3);
    assert_eq!(
        wlb_inputs::decode_answers(wlb_inputs::marker_spec(raw).unwrap()).unwrap(),
        ["2", "", "  Élan,kit  "]
    );
    assert_eq!(state.step3.bgee_items[0].raw_line, *raw);
    assert_eq!(state.step3.bgee_items[0].selected_order, 7);
    let Some(Step2Selection::Component { component_key, .. }) = &state.step2.selected else {
        panic!("component selection must remain selected");
    };
    assert_eq!(component_key, raw);
}

#[test]
fn invalid_answers_leave_both_component_stores_untouched() {
    for invalid in ["", "a\nb", "a\rb", "a\0b"] {
        let mut state = fixture();
        state.step3.bgee_items = vec![step3_item(&state.step2.bgee_mods[0].components[0])];
        open_fixture(&mut state);
        answers(&mut state, &[(invalid, false)]);
        let before = (
            state.step2.bgee_mods.clone(),
            state.step3.bgee_items.clone(),
        );
        assert!(
            save_current(&mut state).is_err(),
            "invalid answer {invalid:?} was accepted"
        );
        assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
    }
}

#[test]
fn stale_disabled_and_deselected_targets_cannot_overwrite_saved_data() {
    for scenario in 0..3 {
        let mut state = fixture();
        state.step3.bgee_items = vec![step3_item(&state.step2.bgee_mods[0].components[0])];
        open_fixture(&mut state);
        answers(&mut state, &[("2", false)]);
        let target = &mut state.step2.bgee_mods[0].components[0];
        match scenario {
            0 => target.raw_line.push_str(" // changed elsewhere"),
            1 => target.disabled = true,
            _ => target.checked = false,
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
fn ambiguous_step3_mirror_rejects_before_step2_is_changed() {
    let mut state = fixture();
    let item = step3_item(&state.step2.bgee_mods[0].components[0]);
    state.step3.bgee_items = vec![item.clone(), item];
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    let before = (
        state.step2.bgee_mods.clone(),
        state.step3.bgee_items.clone(),
    );
    assert!(save_current(&mut state).is_err());
    assert_eq!((state.step2.bgee_mods, state.step3.bgee_items), before);
}

#[test]
fn duplicated_package_target_rejects_without_partial_writes() {
    let mut state = fixture();
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    state.step2.bgee_mods.push(state.step2.bgee_mods[0].clone());
    let before = state.step2.bgee_mods.clone();
    assert!(save_current(&mut state).is_err());
    assert_eq!(state.step2.bgee_mods, before);
}

#[test]
fn saving_is_isolated_by_game_slot_and_language() {
    let mut state = fixture();
    state.step1.game_install = "EET".to_string();
    state.step2.bg2ee_mods = state.step2.bgee_mods.clone();
    let other_language = component("11", 4, true);
    state.step3.bgee_items = vec![
        step3_item(&state.step2.bgee_mods[0].components[0]),
        step3_item(&other_language),
    ];
    state.step3.bg2ee_items = state.step3.bgee_items.clone();
    let second_before = (
        state.step2.bg2ee_mods.clone(),
        state.step3.bg2ee_items.clone(),
    );
    let language_before = state.step3.bgee_items[1].clone();
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    save_current(&mut state).unwrap();
    assert_eq!(
        (state.step2.bg2ee_mods, state.step3.bg2ee_items),
        second_before
    );
    assert_eq!(state.step3.bgee_items[1], language_before);
    assert!(wlb_inputs::marker_spec(&state.step2.bgee_mods[0].components[0].raw_line).is_some());
}

#[test]
fn iwdee_answers_save_to_the_first_game_slot() {
    let mut state = fixture();
    state.step1.game_install = "IWDEE".to_string();
    state.step2.active_game_tab = "IWDEE".to_string();
    open_fixture(&mut state);
    answers(&mut state, &[("1", false)]);
    save_current(&mut state).unwrap();
    let raw = &state.step2.bgee_mods[0].components[0].raw_line;
    assert_eq!(wlb_inputs::marker_spec(raw), Some("1"));
    assert_eq!(state.step2.bg2ee_mods, []);
}

#[test]
fn scanning_locks_answer_saving_without_erasing_existing_values() {
    let mut state = fixture();
    open_fixture(&mut state);
    answers(&mut state, &[("2", false)]);
    state.step2.is_scanning = true;
    let before = state.step2.bgee_mods.clone();
    assert!(save_current(&mut state).is_err());
    assert_eq!(state.step2.bgee_mods, before);
}

#[test]
fn switching_workspace_clears_drafts_even_when_components_are_identical() {
    let mut state = fixture();
    open_fixture(&mut state);
    bind_workspace(&mut state, Some("first-workspace"));
    answers(&mut state, &[("2", false)]);
    let before = state.step2.bgee_mods.clone();
    bind_workspace(&mut state, Some("second-workspace"));
    assert_eq!(state.step2.installer_answers, None);
    assert_eq!(state.step2.bgee_mods, before);
    assert!(save_current(&mut state).is_err());
}
