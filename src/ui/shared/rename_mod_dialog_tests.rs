// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use eframe::egui;
use egui_kittest::Harness;

use super::RenameTarget;
use crate::app::mod_display_name::{invalidate_names, mod_display_label, set_display_mod_names};
use crate::app::mod_downloads::{
    active_modlist_dir, active_modlist_downloads_path, mod_downloads_user_path,
    set_active_modlist_dir,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;

const TP_FILE: &str = "setup-bio_rename_dialog_fixture.tp2";
const TP2: &str = "bio_rename_dialog_fixture";

struct NamesShown(Option<PathBuf>);

impl NamesShown {
    fn on() -> Self {
        set_display_mod_names(true);
        Self(active_modlist_dir())
    }
}

impl Drop for NamesShown {
    fn drop(&mut self) {
        set_display_mod_names(false);
        set_active_modlist_dir(self.0.take());
        invalidate_names();
    }
}

struct Scene {
    app: OrchestratorApp,
    fonts_ready: bool,
}

fn open_list_under_isolated_config() -> PathBuf {
    let dir = crate::platform_defaults::app_config_dir()
        .expect("isolated config dir")
        .join("modlists")
        .join("rename-dialog-list");
    set_active_modlist_dir(Some(dir));
    invalidate_names();
    active_modlist_downloads_path().expect("active list path")
}

fn save_through_dialog(app: OrchestratorApp, typed: &str) -> Harness<'static, Scene> {
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
        .push(egui::Event::Text(typed.to_string()));
    harness.run();
    assert_eq!(
        harness
            .state()
            .app
            .rename_mod
            .as_ref()
            .map(|state| state.buffer.as_str()),
        Some(typed)
    );
    harness.press_key(egui::Key::Enter);
    harness.run();
    harness
}

fn names_in(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    toml::from_str::<toml::Table>(&text)
        .unwrap()
        .get("mods")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .filter(|entry| entry.get("tp2").and_then(toml::Value::as_str) == Some(TP2))
        .filter_map(|entry| entry.get("name").and_then(toml::Value::as_str))
        .map(str::to_string)
        .collect()
}

#[test]
fn a_catalog_rename_saves_to_the_user_file() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _names = NamesShown::on();
    let mut app = OrchestratorApp::new_isolated_for_test("rename-dialog");
    let list = open_list_under_isolated_config();
    app.open_rename_mod(TP_FILE, RenameTarget::UserFile);
    let opened = app.rename_mod.as_ref().expect("dialog state is open");
    assert_eq!(opened.tp2, TP2);
    assert_eq!(opened.buffer, "");
    assert_eq!(mod_display_label(TP_FILE), TP2);

    let harness = save_through_dialog(app, "My Tweaks");

    assert!(harness.state().app.rename_mod.is_none());
    assert_eq!(names_in(&mod_downloads_user_path()), vec!["My Tweaks"]);
    assert!(!list.exists());
    assert_eq!(mod_display_label(TP_FILE), "My Tweaks");
}

#[test]
fn a_step2_rename_saves_to_the_open_list_only() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _names = NamesShown::on();
    let mut app = OrchestratorApp::new_isolated_for_test("rename-dialog-step2");
    let list = open_list_under_isolated_config();
    crate::ui::workspace::step_action_dispatch::dispatch_step2(
        Step2Action::RenameMod {
            tp_file: TP_FILE.to_string(),
        },
        &mut app,
    );
    assert_eq!(
        app.rename_mod.as_ref().map(|state| state.target),
        Some(RenameTarget::ActiveModlist)
    );

    let harness = save_through_dialog(app, "List Name");

    assert!(harness.state().app.rename_mod.is_none());
    assert_eq!(names_in(&list), vec!["List Name"]);
    assert_eq!(names_in(&mod_downloads_user_path()).len(), 0);
    assert_eq!(mod_display_label(TP_FILE), "List Name");
}

#[test]
fn a_catalog_rename_prefill_ignores_the_active_list_name() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _names = NamesShown::on();
    let mut app = OrchestratorApp::new_isolated_for_test("rename-dialog-prefill");
    set_active_modlist_dir(None);
    invalidate_names();
    crate::app::mod_rename::set_mod_name(&mod_downloads_user_path(), TP_FILE, "User Name").unwrap();
    let list = open_list_under_isolated_config();
    std::fs::create_dir_all(list.parent().unwrap()).unwrap();
    std::fs::write(
        &list,
        format!("format = 2\n\n[[mods]]\nname = \"List Name\"\ntp2 = \"{TP2}\"\n"),
    )
    .unwrap();
    invalidate_names();
    assert_eq!(mod_display_label(TP_FILE), "List Name");

    app.open_rename_mod(TP_FILE, RenameTarget::UserFile);
    assert_eq!(
        app.rename_mod.as_ref().map(|state| state.buffer.as_str()),
        Some("User Name")
    );
    app.open_rename_mod(TP_FILE, RenameTarget::ActiveModlist);
    assert_eq!(
        app.rename_mod.as_ref().map(|state| state.buffer.as_str()),
        Some("List Name")
    );
}

#[test]
fn a_step2_rename_without_an_open_list_writes_nothing() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _names = NamesShown::on();
    let mut app = OrchestratorApp::new_isolated_for_test("rename-dialog-nolist");
    set_active_modlist_dir(None);
    invalidate_names();
    app.open_rename_mod(TP_FILE, RenameTarget::ActiveModlist);

    let harness = save_through_dialog(app, "List Name");

    let state = harness.state().app.rename_mod.as_ref();
    assert!(state.and_then(|state| state.error.as_deref()).is_some());
    assert_eq!(names_in(&mod_downloads_user_path()).len(), 0);
    assert_eq!(mod_display_label(TP_FILE), TP2);
}
