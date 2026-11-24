use super::render_prompt_popup;
use crate::app::state::{PromptPopupMode, WizardState};
use eframe::egui;
use egui_kittest::Harness;

fn has_text(shape: &egui::Shape, expected: &str) -> bool {
    match shape {
        egui::Shape::Text(text) => text.galley.text() == expected,
        egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_text(shape, expected)),
        _ => false,
    }
}

#[test]
fn planned_preview_popup_renders_live_component_and_mod_text_without_mutation() {
    for (index, title, raw) in [
        (
            0,
            "EET.tp2 #0",
            "Component: 0 - EET\n\nDetected %bgee_dir%?\n1 = Yes\n2 = No",
        ),
        (
            1,
            "EET.tp2",
            "Répertoire détecté : %bgee_dir% ?\n1 = Oui\n2 = Non",
        ),
    ] {
        let mut state = WizardState::default();
        state.step1.game_install = "EET".to_string();
        state.step1.new_pre_eet_dir_enabled = true;
        state.step1.eet_pre_dir = "planned/Pré EET".to_string();
        state.step2.prompt_popup_open = true;
        state.step2.prompt_popup_mode = PromptPopupMode::Text;
        state.step2.prompt_popup_title = title.to_string();
        state.step2.prompt_popup_text = raw.to_string();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 820.0))
            .build_state(
                |ctx, state| {
                    egui::CentralPanel::default().show(ctx, |ui| render_prompt_popup(ui, state));
                },
                state,
            );
        let expected = raw.replace("%bgee_dir%", "planned/Pré EET");
        assert!(
            harness
                .output()
                .shapes
                .iter()
                .any(|shape| has_text(&shape.shape, &expected))
        );
        assert_eq!(harness.state().step2.prompt_popup_text, raw);
        harness.state_mut().step1.eet_pre_dir = "changed/BGEE".to_string();
        harness.run();
        let expected = raw.replace("%bgee_dir%", "changed/BGEE");
        assert!(
            harness
                .output()
                .shapes
                .iter()
                .any(|shape| has_text(&shape.shape, &expected))
        );
        assert_eq!(harness.state().step2.prompt_popup_text, raw);
        if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            harness
                .render()
                .unwrap()
                .save(directory.join(format!("eet-step2-preview-{index}.png")))
                .unwrap();
        }
    }
}
