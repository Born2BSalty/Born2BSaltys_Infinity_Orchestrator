use super::request_step3_next;
use crate::app::state::{InstallerAnswerDraft, Step3ItemState, WizardState};
use crate::app::wlb_inputs;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::workspace::state_workspace::{WorkspaceStep, WorkspaceViewState};
use eframe::egui;
use egui_kittest::Harness;

fn row(id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: "quest.tp2".to_string(),
        component_id: id.to_string(),
        mod_name: "Quest".to_string(),
        component_label: format!("Choose kit {id}"),
        raw_line: format!("~QUEST/QUEST.TP2~ #0 #{id} // Choose a kit"),
        prompt_summary: Some("Choose a kit:\n1) Yes\n2) No".to_string()),
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: "quest".to_string(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn fixture() -> (WizardState, WorkspaceViewState) {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step3.bgee_items = vec![row("11"), row("12")];
    let workspace = WorkspaceViewState {
        current_step: WorkspaceStep::Step3,
        ..Default::default()
    };
    (state, workspace)
}

#[test]
fn next_with_missing_inputs_opens_editor_without_completing_step3() {
    let (mut state, mut workspace) = fixture();
    assert!(!request_step3_next(&mut state, &mut workspace));
    assert_eq!(workspace.current_step, WorkspaceStep::Step3);
    assert!(!workspace.completed_steps.contains(&WorkspaceStep::Step3));
    assert_eq!(
        state
            .step3
            .installer_answers
            .as_ref()
            .unwrap()
            .targets
            .len(),
        2
    );
}

#[test]
fn next_with_saved_inputs_proceeds_to_step4_without_opening_editor() {
    let (mut state, mut workspace) = fixture();
    for item in &mut state.step3.bgee_items {
        let spec = wlb_inputs::encode_answers(&[String::new()])
            .unwrap()
            .unwrap();
        item.raw_line = wlb_inputs::replace_marker(&item.raw_line, &spec).unwrap();
    }
    assert!(request_step3_next(&mut state, &mut workspace));
    assert_eq!(workspace.current_step, WorkspaceStep::Step4);
    assert!(workspace.completed_steps.contains(&WorkspaceStep::Step3));
    assert_eq!(state.step3.installer_answers, None);
}

#[test]
fn closing_an_unanswered_queue_does_not_allow_the_next_attempt_through() {
    let (mut state, mut workspace) = fixture();
    assert!(!request_step3_next(&mut state, &mut workspace));
    state.step3.installer_answers = None;
    assert!(!request_step3_next(&mut state, &mut workspace));
    assert_eq!(workspace.current_step, WorkspaceStep::Step3);
    assert!(!workspace.completed_steps.contains(&WorkspaceStep::Step3));
}

#[test]
fn an_unrelated_game_tree_does_not_block_next() {
    let (mut state, mut workspace) = fixture();
    state.step1.game_install = "BG2EE".to_string();
    assert!(request_step3_next(&mut state, &mut workspace));
    assert_eq!(workspace.current_step, WorkspaceStep::Step4);
}

struct Scene {
    state: WizardState,
    workspace: WorkspaceViewState,
    save_notifications: usize,
}

fn scene_harness() -> Harness<'static, Scene> {
    let (state, workspace) = fixture();
    let mut fonts_ready = false;
    Harness::builder()
        .with_size(egui::vec2(1280.0, 820.0))
        .with_max_steps(12)
        .build_state(
            move |ctx, scene| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Step 3 — Reorder and Resolve");
                    ui.allocate_ui(egui::vec2(ui.available_width(), 350.0), |ui| {
                        let mut jump = false;
                        crate::ui::step3::list_step3::render(
                            ui,
                            &mut scene.state,
                            &mut jump,
                            &std::collections::HashMap::new(),
                        );
                    });
                    let outcome = crate::ui::workspace::workspace_nav_bar::render(
                        ui,
                        ThemePalette::Dark,
                        scene.workspace.current_step,
                        false,
                        None,
                    );
                    if outcome.next_clicked {
                        request_step3_next(&mut scene.state, &mut scene.workspace);
                    }
                });
                if crate::ui::step3::installer_answers_panel_step3::render(
                    ctx,
                    &mut scene.state,
                    ThemePalette::Dark,
                ) {
                    scene.save_notifications += 1;
                }
            },
            Scene {
                state,
                workspace,
                save_notifications: 0,
            },
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

fn click<T>(harness: &mut Harness<'_, T>, label: &str) {
    let pos = harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| shape_label(&shape.shape, label))
        .unwrap_or_else(|| panic!("painted button missing: {label}"))
        .center();
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
fn real_next_and_editor_save_buttons_review_two_components_before_step4() {
    let mut harness = scene_harness();
    click(&mut harness, "Next");
    assert_eq!(harness.state().workspace.current_step, WorkspaceStep::Step3);
    assert!(
        !harness
            .state()
            .workspace
            .completed_steps
            .contains(&WorkspaceStep::Step3)
    );
    assert_eq!(
        harness
            .state()
            .state
            .step3
            .installer_answers
            .as_ref()
            .unwrap()
            .targets
            .len(),
        2
    );
    if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join("step3-next-missing-answers.png"))
            .unwrap();
    }
    for (number, expected) in [("1", "11"), ("2", "12")] {
        let editor = harness
            .state_mut()
            .state
            .step3
            .installer_answers
            .as_mut()
            .unwrap();
        assert_eq!(editor.targets[editor.active].component_id, expected);
        editor.targets[editor.active].answers = vec![InstallerAnswerDraft {
            text: number.to_string(),
            blank_enter: false,
        }];
        harness.run();
        click(&mut harness, "Save answers");
        assert_eq!(harness.state().state.step3.installer_answers, None);
        assert_eq!(harness.state().workspace.current_step, WorkspaceStep::Step3);
        if expected == "11" {
            click(&mut harness, "Next");
            let editor = harness
                .state()
                .state
                .step3
                .installer_answers
                .as_ref()
                .unwrap();
            assert_eq!(editor.targets.len(), 1);
            assert_eq!(editor.targets[editor.active].component_id, "12");
        }
    }
    assert_eq!(harness.state().save_notifications, 2);
    assert_eq!(harness.state().state.step3.installer_answers, None);
    assert_eq!(harness.state().workspace.current_step, WorkspaceStep::Step3);
    click(&mut harness, "Next");
    assert_eq!(harness.state().workspace.current_step, WorkspaceStep::Step4);
    assert!(
        harness
            .state()
            .workspace
            .completed_steps
            .contains(&WorkspaceStep::Step3)
    );
}

#[test]
fn real_close_keeps_step3_and_leaves_saved_lines_untouched() {
    let mut harness = scene_harness();
    let before = harness.state().state.step3.bgee_items.clone();
    click(&mut harness, "Next");
    click(&mut harness, "Close");
    assert_eq!(harness.state().state.step3.installer_answers, None);
    assert_eq!(harness.state().state.step3.bgee_items, before);
    assert_eq!(harness.state().workspace.current_step, WorkspaceStep::Step3);
    click(&mut harness, "Next");
    assert!(harness.state().state.step3.installer_answers.is_some());
}

#[test]
fn scanning_or_running_install_cannot_advance_from_step3() {
    for exact_log in [false, true] {
        for installing in [false, true] {
            let (mut state, mut workspace) = fixture();
            if exact_log {
                state.step1.install_mode =
                    crate::app::state::Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
            }
            if installing {
                state.step5.install_running = true;
            } else {
                state.step2.is_scanning = true;
            }
            assert!(!request_step3_next(&mut state, &mut workspace));
            assert_eq!(workspace.current_step, WorkspaceStep::Step3);
            assert!(!workspace.completed_steps.contains(&WorkspaceStep::Step3));
            assert_eq!(state.step3.installer_answers, None);
        }
    }
}

#[test]
fn ordinary_exact_log_navigation_preserves_the_immutable_plan() {
    let (mut state, mut workspace) = fixture();
    state.step1.install_mode =
        crate::app::state::Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
    let original = state.step3.bgee_items.clone();
    assert!(request_step3_next(&mut state, &mut workspace));
    assert_eq!(workspace.current_step, WorkspaceStep::Step4);
    assert!(workspace.completed_steps.contains(&WorkspaceStep::Step3));
    assert_eq!(state.step3.installer_answers, None);
    assert_eq!(state.step3.bgee_items, original);
}

#[test]
fn next_binds_queue_to_workspace_before_a_later_render_can_switch_owner() {
    let (mut state, mut workspace) = fixture();
    workspace.loaded_workspace_id = Some("first-workspace".to_string());
    assert!(!request_step3_next(&mut state, &mut workspace));
    assert_eq!(
        state.step3.installer_answers_workspace.as_deref(),
        Some("first-workspace")
    );
    crate::ui::step3::installer_answers_step3::bind_workspace(&mut state, Some("second-workspace"));
    assert_eq!(state.step3.installer_answers, None);
}

fn workspace_harness()
-> Harness<'static, crate::ui::orchestrator::orchestrator_app::OrchestratorApp> {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bootstrap = std::env::temp_dir().join(format!(
        "bio_step3_answer_bootstrap_{}_{unique}",
        std::process::id()
    ));
    crate::platform_defaults::set_config_dir_override(Some(bootstrap));
    let mut app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
        "step3_answers_workspace",
    );
    let (state, mut workspace) = fixture();
    workspace.loaded_workspace_id = Some("isolated-answer-workspace".to_string());
    app.wizard_state = state;
    app.workspace_view = workspace;
    let mut fonts_ready = false;
    Harness::builder()
        .with_size(egui::vec2(1280.0, 820.0))
        .with_max_steps(12)
        .build_state(
            move |ctx, app| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui::workspace::step3::workspace_step3::render(ui, app);
                });
            },
            app,
        )
}

#[test]
fn redesigned_workspace_prompt_pill_and_save_mark_actual_workspace_dirty() {
    let mut harness = workspace_harness();
    assert!(!harness.state().workspace_state_dirty);
    click(&mut harness, "PROMPT");
    assert_eq!(
        harness
            .state()
            .wizard_state
            .step3
            .installer_answers_workspace
            .as_deref(),
        Some("isolated-answer-workspace"),
    );
    let editor = harness
        .state_mut()
        .wizard_state
        .step3
        .installer_answers
        .as_mut()
        .unwrap();
    assert_eq!(editor.targets[editor.active].component_id, "11");
    editor.targets[editor.active].answers = vec![InstallerAnswerDraft {
        text: "2".to_string(),
        blank_enter: false,
    }];
    harness.run();
    click(&mut harness, "Save answers");
    assert!(harness.state().workspace_state_dirty);
    assert_eq!(harness.state().wizard_state.step3.installer_answers, None);
    let state = &harness.state().wizard_state;
    let workspace =
        crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
            state,
            &crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
    let json = serde_json::to_string(&workspace).unwrap();
    let restored: crate::registry::workspace_model::ModlistWorkspaceState =
        serde_json::from_str(&json).unwrap();
    assert_eq!(restored.order_bgee[0].id, 11);
    assert_eq!(
        wlb_inputs::decode_answers(restored.order_bgee[0].wlb_inputs.as_deref().unwrap()).unwrap(),
        ["2"],
    );
    if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join("step3-workspace-prompt-answers.png"))
            .unwrap();
    }
}

#[test]
fn redesigned_workspace_prompt_pill_uses_its_game_instead_of_the_step2_tab() {
    for allowed_game in ["bg2ee", "eet"] {
        let mut harness = workspace_harness();
        let state = &mut harness.state_mut().wizard_state;
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
        click(&mut harness, "PROMPT");
        let editor = harness
            .state()
            .wizard_state
            .step3
            .installer_answers
            .as_ref()
            .unwrap();
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
fn removing_answers_in_actual_workspace_marks_dirty_and_persists_cleared_input() {
    let mut harness = workspace_harness();
    let state = &mut harness.state_mut().wizard_state;
    let base = state.step3.bgee_items[0].raw_line.clone();
    state.step3.bgee_items[0].raw_line = wlb_inputs::replace_marker(&base, "2").unwrap();
    harness.run();
    click(&mut harness, "PROMPT");
    click(&mut harness, "Remove");
    click(&mut harness, "Save answers");
    assert!(harness.state().workspace_state_dirty);
    assert_eq!(harness.state().wizard_state.step3.installer_answers, None);
    assert_eq!(
        harness.state().wizard_state.step3.bgee_items[0].raw_line,
        base
    );
    let workspace =
        crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
            &harness.state().wizard_state,
            &crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
    let json = serde_json::to_string(&workspace).unwrap();
    let restored: crate::registry::workspace_model::ModlistWorkspaceState =
        serde_json::from_str(&json).unwrap();
    assert_eq!(restored.order_bgee[0].wlb_inputs, None);
    let state = &mut harness.state_mut().wizard_state;
    assert!(crate::ui::step3::installer_answers_step3::open_missing(
        state
    ));
    assert_eq!(
        state.step3.installer_answers.as_ref().unwrap().targets[0].component_id,
        "11"
    );
}
