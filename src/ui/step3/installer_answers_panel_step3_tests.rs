use super::render;
use crate::app::state::{
    InstallerAnswerDraft, InstallerAnswerTarget, InstallerAnswersEditor, WizardState,
};
use crate::ui::orchestrator::widgets::drawer::{DrawerWidth, drawer_width};
use crate::ui::shared::redesign_tokens::{REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette};
use crate::ui::shared::redesign_visuals;
use eframe::egui;
use egui_kittest::Harness;

fn drawer_fixture() -> WizardState {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step3.installer_answers = Some(InstallerAnswersEditor {
        targets: vec![InstallerAnswerTarget {
            game_tab: "BGEE".to_string(),
            package_path: "A7-MULTIKITS/A7-MULTIKITS.TP2".to_string(),
            tp_file: "A7-MultiKits.tp2".to_string(),
            component_id: "11".to_string(),
            language: 0,
            row_index: 0,
            block_id: "a7-multikits".to_string(),
            original_line: "~A7-MULTIKITS/A7-MULTIKITS.TP2~ #0 #11 // Choose manually: 1.1".to_string(),
            title: "A7-MultiKits #11 — Multiclass kits for Fighter/Mage → Choose manually".to_string(),
            questions: "Please enter number of the kit to select (leave blank to proceed with the installation):\n\nPlease enter a new title for the selected kit (leave blank to keep current):\n\nAccept kit title '<new_kit_title>' ([A]ccept, [R]etry, [C]ancel)?\na = Accept\nr = Retry\nc = Cancel\n\nProceed with the installation ([Y]es, [N]o, [C]ancel)?\ny = Yes\nn = No\nc = Cancel\n\nNo kits were selected. Cancel the installation ([Y]es, [N]o)?\ny = Yes\nn = No\n\nUnselect kit '<kit_title>' ([Y]es, [N]o)?\ny = Yes\nn = No".to_string(),
            answers: vec![
                InstallerAnswerDraft { text: "1".to_string(), blank_enter: false },
                InstallerAnswerDraft { text: String::new(), blank_enter: true },
                InstallerAnswerDraft { text: "a".to_string(), blank_enter: false },
            ],
        }],
        active: 0,
        status: None,
        review_missing: false,
        game_install: "BGEE".to_string(),
    });
    attach_save_target(&mut state);
    state
}

fn attach_save_target(state: &mut WizardState) {
    state.step3.bgee_items = state
        .step3
        .installer_answers
        .as_ref()
        .unwrap()
        .targets
        .iter()
        .map(|target| crate::app::state::Step3ItemState {
            tp_file: target.tp_file.clone(),
            component_id: target.component_id.clone(),
            mod_name: "A7-MultiKits".to_string(),
            component_label: target.title.clone(),
            raw_line: target.original_line.clone(),
            prompt_summary: Some(target.questions.clone()),
            prompt_events: Vec::new(),
            selected_order: target.row_index + 1,
            block_id: target.block_id.clone(),
            is_parent: false,
            parent_placeholder: false,
        })
        .collect();
}

fn fixture_harness(size: egui::Vec2) -> Harness<'static, WizardState> {
    let mut fonts_installed = false;
    Harness::builder()
        .with_size(size)
        .with_max_steps(12)
        .build_state(
            move |ctx, state| {
                if !fonts_installed {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_installed = true;
                    ctx.request_repaint();
                    return;
                }
                ctx.set_visuals(redesign_visuals::build_for(ThemePalette::Dark));
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Step 3 — Reorder and Resolve");
                    let mut jump = false;
                    crate::ui::step3::list_step3::render(
                        ui,
                        state,
                        &mut jump,
                        &std::collections::HashMap::new(),
                    );
                });
                if render(ctx, state, ThemePalette::Dark) {
                    ctx.data_mut(|data| {
                        let id = egui::Id::new("fixture_save_notifications");
                        let count = data.get_temp::<usize>(id).unwrap_or(0);
                        data.insert_temp(id, count + 1);
                    });
                }
            },
            drawer_fixture(),
        )
}

fn shape_label(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Text(text) if text.galley.text() == label => {
            Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| shape_label(shape, label)),
        _ => None,
    }
}

fn painted_label(harness: &Harness<'_, WizardState>, label: &str) -> egui::Rect {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| shape_label(&shape.shape, label))
        .unwrap_or_else(|| panic!("painted label missing: {label}"))
}

fn click_label(harness: &mut Harness<'_, WizardState>, label: &str) {
    let pos = painted_label(harness, label).center();
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
}

#[test]
fn wide_overlay_is_right_anchored_and_footer_remains_reachable() {
    for (name, size) in [
        ("normal", egui::vec2(1280.0, 820.0)),
        ("narrow", egui::vec2(960.0, 720.0)),
    ] {
        let mut harness = fixture_harness(size);
        let rect = harness.ctx.memory(|memory| {
            memory
                .area_rect(egui::Id::new(("drawer", "step3_installer_answers")))
                .unwrap()
        });
        assert!(
            (rect.right() - size.x).abs() < 1.0,
            "drawer exceeds right edge: {rect:?}"
        );
        assert!((rect.width() - drawer_width(size.x, DrawerWidth::Wide)).abs() < 1.0);
        assert!((rect.top() - REDESIGN_TITLEBAR_HEIGHT_PX).abs() < 1.0);
        assert!(
            rect.bottom() <= size.y + 1.0,
            "drawer exceeds bottom edge: {rect:?}"
        );
        for label in ["Save answers", "Close"] {
            let bounds = painted_label(&harness, label);
            assert!(bounds.left() >= rect.left());
            assert!(bounds.right() <= size.x);
            assert!(bounds.top() >= rect.top());
            assert!(
                bounds.bottom() <= size.y,
                "{label} is outside viewport: {bounds:?}"
            );
        }
        let first_answer = painted_label(&harness, "1");
        let footer = painted_label(&harness, "Save answers");
        assert!(first_answer.top() >= rect.top());
        assert!(
            first_answer.bottom() < footer.top(),
            "first answer must be visible before scrolling: {first_answer:?}"
        );
        if name == "normal" {
            let questions = &harness
                .state()
                .step3
                .installer_answers
                .as_ref()
                .unwrap()
                .targets[0]
                .questions;
            let question_bounds = painted_label(&harness, questions);
            let answers_heading = painted_label(&harness, "Answers to send, in order");
            assert!(
                question_bounds.right() + 24.0 <= answers_heading.left(),
                "question text must leave a clear gutter before answers: {question_bounds:?}, {answers_heading:?}"
            );
        }
        if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            harness
                .render()
                .unwrap()
                .save(directory.join(format!("step3-installer-answers-{name}.png")))
                .unwrap();
        }
    }
}

#[test]
fn closing_discards_only_the_answer_overlay() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    let before = harness.state().step3.bgee_items.clone();
    click_label(&mut harness, "Close");
    assert_eq!(harness.state().step3.installer_answers, None);
    assert_eq!(harness.state().step3.bgee_items, before);
}

#[test]
fn successful_save_emits_one_workspace_dirty_notification() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    click_label(&mut harness, "Save answers");
    let notifications = harness
        .ctx
        .data(|data| data.get_temp::<usize>(egui::Id::new("fixture_save_notifications")));
    assert_eq!(notifications, Some(1));
    assert_eq!(harness.state().step3.installer_answers, None);
    assert_eq!(harness.state().step3.installer_answers_workspace, None);
    let raw = &harness.state().step3.bgee_items[0].raw_line;
    let values =
        crate::app::wlb_inputs::decode_answers(crate::app::wlb_inputs::marker_spec(raw).unwrap())
            .unwrap();
    assert_eq!(values, ["1", "", "a"]);
    harness.run();
    assert_eq!(
        harness
            .ctx
            .data(|data| data.get_temp::<usize>(egui::Id::new("fixture_save_notifications"))),
        Some(1)
    );
}

#[test]
fn stale_editor_is_hidden_after_restoring_an_unrelated_workspace() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    harness.state_mut().step3.bgee_items.clear();
    harness.run_steps(1);
    assert_eq!(harness.state().step3.installer_answers, None);
}

#[test]
fn a_real_step3_component_prompt_pill_opens_the_answer_editor() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    harness.state_mut().step3.installer_answers = None;
    harness.run();
    click_label(&mut harness, "PROMPT");
    let editor = harness.state().step3.installer_answers.as_ref().unwrap();
    assert_eq!(editor.targets[editor.active].component_id, "11");
    assert!(!editor.review_missing);
    assert!(harness.state().step3.bgee_items[0].raw_line.contains("#11"));
}

#[test]
fn real_legacy_prompt_pill_uses_its_game_instead_of_the_step2_tab() {
    for allowed_game in ["bg2ee", "eet"] {
        let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
        let state = harness.state_mut();
        state.step3.installer_answers = None;
        state.step1.game_install = "EET".to_string();
        state.step2.active_game_tab = "BGEE".to_string();
        state.step3.active_game_tab = "BG2EE".to_string();
        state.step3.bg2ee_items = std::mem::take(&mut state.step3.bgee_items);
        state.step3.bg2ee_items[0].prompt_events = vec![crate::parser::PromptSummaryEvent {
            summary_line: "A conditional second-game question".to_string(),
            game_allow: vec![allowed_game.to_string()],
            ..Default::default()
        }];
        if allowed_game == "eet" {
            state
                .step3
                .bg2ee_items
                .push(crate::app::state::Step3ItemState {
                    tp_file: "EET.tp2".to_string(),
                    component_id: "0".to_string(),
                    mod_name: "EET".to_string(),
                    raw_line: "~EET/EET.TP2~ #0 #0".to_string(),
                    block_id: "eet".to_string(),
                    component_label: "EET core".to_string(),
                    prompt_summary: None,
                    prompt_events: Vec::new(),
                    selected_order: 2,
                    is_parent: false,
                    parent_placeholder: false,
                });
        }
        harness.run();
        click_label(&mut harness, "PROMPT");
        let editor = harness.state().step3.installer_answers.as_ref().unwrap();
        assert_eq!(editor.targets[editor.active].game_tab, "BG2EE");
        assert_eq!(editor.targets[editor.active].component_id, "11");
        assert!(
            editor.targets[editor.active]
                .questions
                .contains("conditional second-game")
        );
    }
}

#[test]
fn removing_every_answer_and_saving_closes_drawer_and_clears_saved_marker() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    let state = harness.state_mut();
    let base = state.step3.bgee_items[0].raw_line.clone();
    let spec =
        crate::app::wlb_inputs::encode_answers(&["1".to_string(), String::new(), "a".to_string()])
            .unwrap()
            .unwrap();
    let saved = crate::app::wlb_inputs::replace_marker(&base, &spec).unwrap();
    state.step3.bgee_items[0].raw_line.clone_from(&saved);
    state.step3.installer_answers.as_mut().unwrap().targets[0].original_line = saved;
    harness.run();
    for _ in 0..3 {
        click_label(&mut harness, "Remove");
    }
    assert_eq!(
        harness
            .state()
            .step3
            .installer_answers
            .as_ref()
            .unwrap()
            .targets[0]
            .answers,
        [] as [InstallerAnswerDraft; 0]
    );
    click_label(&mut harness, "Save answers");
    assert_eq!(harness.state().step3.installer_answers, None);
    assert_eq!(harness.state().step3.bgee_items[0].raw_line, base);
    assert_eq!(
        harness
            .ctx
            .data(|data| data.get_temp::<usize>(egui::Id::new("fixture_save_notifications"))),
        Some(1)
    );
    if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join("step3-removed-answers-closed.png"))
            .unwrap();
    }
}

#[test]
fn moving_blank_enter_to_first_row_preserves_all_answers_through_real_controls() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    click_label(&mut harness, "a");
    let modifiers = egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    };
    harness.input_mut().events.extend([
        egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        },
        egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        },
        egui::Event::Text("test".to_string()),
    ]);
    harness.run();
    click_label(&mut harness, "Save answers");
    assert!(harness.state().step3.installer_answers.is_none());
    let first_line = harness.state().step3.bgee_items[0].raw_line.clone();
    let first_spec = crate::app::wlb_inputs::marker_spec(&first_line).unwrap();
    let first_answers = crate::app::wlb_inputs::decode_answers(first_spec).unwrap();
    assert_eq!(first_answers, ["1", "", "test"]);
    assert_eq!(first_spec, "1,,test");

    click_label(&mut harness, "PROMPT");
    for index in [0, 1] {
        let bounds = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| shape_label(&shape.shape, "Send Enter without text"))
            .collect::<Vec<_>>();
        assert_eq!(bounds.len(), 3);
        let pos = bounds[index].center();
        harness.input_mut().events.extend([
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        harness.run();
    }
    click_label(&mut harness, "Answer");
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("1".to_string()));
    harness.run();
    click_label(&mut harness, "Save answers");
    assert!(harness.state().step3.installer_answers.is_none());
    let second_line = harness.state().step3.bgee_items[0].raw_line.clone();
    let second_spec = crate::app::wlb_inputs::marker_spec(&second_line).unwrap();
    let second_answers = crate::app::wlb_inputs::decode_answers(second_spec).unwrap();
    assert_eq!(second_answers, ["", "1", "test"]);
    assert_eq!(second_spec, ",1,test");
    eprintln!(
        "LEADING_BLANK_DIAGNOSTIC first_spec={first_spec:?} first_answers={first_answers:?} second_spec={second_spec:?} second_answers={second_answers:?}"
    );

    click_label(&mut harness, "PROMPT");
    let target = &harness
        .state()
        .step3
        .installer_answers
        .as_ref()
        .unwrap()
        .targets[0];
    assert!(target.answers[0].blank_enter);
    assert!(!target.answers[1].blank_enter);
    assert_eq!(target.answers[1].text, "1");
    assert_eq!(target.answers[2].text, "test");
}
