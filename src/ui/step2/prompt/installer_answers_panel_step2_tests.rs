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
    state.step2.installer_answers = Some(InstallerAnswersEditor {
        targets: vec![InstallerAnswerTarget {
            game_tab: "BGEE".to_string(),
            package_path: "fixture/A7-MultiKits/A7-MultiKits.tp2".to_string(),
            tp_file: "A7-MultiKits.tp2".to_string(),
            component_id: "11".to_string(),
            language: 0,
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
    });
    attach_save_target(&mut state);
    state
}

fn attach_save_target(state: &mut WizardState) {
    let target = state.step2.installer_answers.as_ref().unwrap().targets[0].clone();
    state.step2.bgee_mods = vec![crate::app::state::Step2ModState {
        name: "A7-MultiKits".to_string(),
        tp_file: target.tp_file.clone(),
        tp2_path: target.package_path.clone(),
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
        components: vec![crate::app::state::Step2ComponentState {
            component_id: target.component_id.clone(),
            label: target.title.clone(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: target.original_line.clone(),
            prompt_summary: Some(target.questions),
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
        }],
    }];
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
                    ui.heading("Step 2 — Scan and Select");
                    ui.label(
                        "Component list and existing details pane remain behind this overlay.",
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
                .area_rect(egui::Id::new(("drawer", "step2_installer_answers")))
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
        for label in ["Save answers", "Answer during installation"] {
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
                .step2
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
                .save(directory.join(format!("step2-installer-answers-{name}.png")))
                .unwrap();
        }
    }
}

#[test]
fn deferring_closes_only_the_answer_overlay() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    let before = harness.state().step3.clone();
    click_label(&mut harness, "Answer during installation");
    assert_eq!(harness.state().step2.installer_answers, None);
    assert_eq!(harness.state().step3, before);
}

#[test]
fn successful_save_emits_one_workspace_dirty_notification() {
    let mut harness = fixture_harness(egui::vec2(1280.0, 820.0));
    click_label(&mut harness, "Save answers");
    let notifications = harness
        .ctx
        .data(|data| data.get_temp::<usize>(egui::Id::new("fixture_save_notifications")));
    assert_eq!(notifications, Some(1));
    let raw = &harness.state().step2.bgee_mods[0].components[0].raw_line;
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
    harness.state_mut().step2.bgee_mods.clear();
    harness.run_steps(1);
    assert_eq!(harness.state().step2.installer_answers, None);
}
