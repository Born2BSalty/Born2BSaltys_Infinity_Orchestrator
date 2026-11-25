// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use egui_kittest::Harness;

use crate::app::mod_display_name::{mod_display_label, set_display_mod_names};
use crate::app::mod_rename::user_mod_renames;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

const TP_FILE: &str = "setup-bio_rename_dialog_fixture.tp2";

struct NamesShown;

impl NamesShown {
    fn on() -> Self {
        set_display_mod_names(true);
        Self
    }
}

impl Drop for NamesShown {
    fn drop(&mut self) {
        set_display_mod_names(false);
    }
}

struct Scene {
    app: OrchestratorApp,
    fonts_ready: bool,
}

#[test]
fn rename_dialog_saves_into_the_user_file() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _names = NamesShown::on();
    let mut app = OrchestratorApp::new_isolated_for_test("rename-dialog");
    app.open_rename_mod(TP_FILE);
    let opened = app.rename_mod.as_ref().expect("dialog state is open");
    assert_eq!(opened.tp2, "bio_rename_dialog_fixture");
    assert_eq!(opened.buffer, "");
    assert_eq!(mod_display_label(TP_FILE), "bio_rename_dialog_fixture");

    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .with_max_steps(64)
        .build_state(
            |ctx, scene: &mut Scene| {
                if !scene.fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    scene.fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                scene.app.render_rename_mod_dialog(ctx);
            },
            Scene {
                app,
                fonts_ready: false,
            },
        );
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("My Tweaks".to_string()));
    harness.run();
    assert_eq!(
        harness
            .state()
            .app
            .rename_mod
            .as_ref()
            .map(|state| state.buffer.as_str()),
        Some("My Tweaks")
    );
    harness.press_key(egui::Key::Enter);
    harness.run();

    assert!(harness.state().app.rename_mod.is_none());
    assert_eq!(
        user_mod_renames()
            .get("bio_rename_dialog_fixture")
            .map(String::as_str),
        Some("My Tweaks")
    );
    assert_eq!(mod_display_label(TP_FILE), "My Tweaks");
}
