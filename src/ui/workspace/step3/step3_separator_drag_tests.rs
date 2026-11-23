use crate::app::state::Step3ItemState;
use crate::app::step3_separators::separator_row;
use crate::ui::workspace::state_workspace::{WorkspaceStep, WorkspaceViewState};
use eframe::egui;
use egui_kittest::Harness;

fn row(mod_name: &str, id: &str, is_parent: bool) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}.tp2"),
        component_id: if is_parent {
            "__PARENT__".to_string()
        } else {
            id.to_string()
        },
        mod_name: mod_name.to_string(),
        component_label: format!("{mod_name} part {id}"),
        raw_line: format!("~{mod_name}/{mod_name}.TP2~ #0 #{id} // part"),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: format!("{mod_name}::block0"),
        is_parent,
        parent_placeholder: false,
    }
}

fn one_mod(mod_name: &str) -> [Step3ItemState; 3] {
    [
        row(mod_name, "0", true),
        row(mod_name, "1", false),
        row(mod_name, "2", false),
    ]
}

fn sectioned_list() -> Vec<Step3ItemState> {
    let mut items = one_mod("Fixpack").to_vec();
    items.push(separator_row("Utilities", "SEPARATOR::1".to_string()));
    items.extend(one_mod("Eeex"));
    items.extend(one_mod("Spells"));
    items.push(separator_row("Quests", "SEPARATOR::2".to_string()));
    items.extend(one_mod("Heart"));
    items.extend(one_mod("Ruins"));
    items.push(separator_row("Tweaks", "SEPARATOR::3".to_string()));
    items.extend(one_mod("Klatu"));
    items
}

fn harness_for(
    items: Vec<Step3ItemState>,
    collapsed: Vec<String>,
) -> Harness<'static, crate::ui::orchestrator::orchestrator_app::OrchestratorApp> {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bootstrap = std::env::temp_dir().join(format!(
        "bio_step3_separator_drag_{}_{unique}",
        std::process::id()
    ));
    crate::platform_defaults::set_config_dir_override(Some(bootstrap));
    let mut app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
        "step3_separator_drag",
    );
    app.wizard_state.step1.game_install = "BGEE".to_string();
    app.wizard_state.step3.bgee_items = items;
    app.wizard_state.step3.bgee_collapsed_blocks = collapsed;
    app.workspace_view = WorkspaceViewState {
        current_step: WorkspaceStep::Step3,
        ..Default::default()
    };
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

fn shape_label(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Text(text) if text.galley.text() == label => {
            Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| shape_label(shape, label)),
        _ => None,
    }
}

fn label_rect<T>(harness: &Harness<'_, T>, label: &str) -> egui::Rect {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| shape_label(&shape.shape, label))
        .unwrap_or_else(|| panic!("no label {label} on screen"))
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

fn drag<T>(harness: &mut Harness<'_, T>, from: egui::Pos2, to: egui::Pos2) {
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(from));
    harness.run_steps(2);
    harness.input_mut().events.push(button(from, true));
    harness.run_steps(2);
    let steps = 24u16;
    for step in 1..=steps {
        let pos = from + (to - from) * (f32::from(step) / f32::from(steps));
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(pos));
        harness.run_steps(1);
    }
    harness.input_mut().events.push(button(to, false));
    harness.run_steps(2);
}

fn header_order(items: &[Step3ItemState]) -> Vec<&str> {
    items
        .iter()
        .filter(|item| item.is_parent)
        .map(|item| item.mod_name.as_str())
        .collect()
}

#[test]
fn a_closed_section_drags_from_anywhere_on_its_band_but_the_chevron() {
    let probe = harness_for(sectioned_list(), vec!["SEPARATOR::1".to_string()]);
    let name = label_rect(&probe, "Utilities");
    let target_y = label_rect(&probe, "Tweaks").center().y;
    drop(probe);

    for grab_x in [name.left() - 30.0, name.center().x, 1100.0] {
        let mut harness = harness_for(sectioned_list(), vec!["SEPARATOR::1".to_string()]);
        harness.run_steps(2);
        drag(
            &mut harness,
            egui::pos2(grab_x, name.center().y),
            egui::pos2(grab_x, target_y),
        );
        assert_eq!(
            header_order(&harness.state().wizard_state.step3.bgee_items),
            [
                "Fixpack",
                "Quests",
                "Heart",
                "Ruins",
                "Utilities",
                "Eeex",
                "Spells",
                "Tweaks",
                "Klatu"
            ],
            "grabbed at x={grab_x}"
        );
    }
}

fn header_band_right<T>(harness: &Harness<'_, T>, label: &str) -> f32 {
    let label = label_rect(harness, label);
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.rect.contains(label.center())
                    && rect.rect.height() < 3.0 * label.height() =>
            {
                Some(rect.rect.right())
            }
            _ => None,
        })
        .fold(0.0, f32::max)
}

#[test]
fn a_mod_drags_by_the_grip_on_the_right_of_its_header() {
    let mut harness = harness_for(sectioned_list(), Vec::new());
    harness.run_steps(2);
    let grip_x = header_band_right(&harness, "Heart (2)") - 14.0;
    let heart_y = label_rect(&harness, "Heart (2)").center().y;
    let klatu_y = label_rect(&harness, "Klatu (2)").center().y;

    drag(
        &mut harness,
        egui::pos2(grip_x, heart_y),
        egui::pos2(grip_x, klatu_y),
    );

    let order = header_order(&harness.state().wizard_state.step3.bgee_items);
    let position = |name: &str| order.iter().position(|item| *item == name).unwrap();
    assert!(position("Heart") > position("Ruins"), "order {order:?}");
}
