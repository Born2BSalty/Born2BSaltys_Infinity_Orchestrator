// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use egui_kittest::Harness;

use super::{ParentRowInput, render_parent_row};
use crate::app::mod_display_name::set_display_mod_names;
use crate::app::state::Step2ModState;
use crate::ui::shared::redesign_tokens::ThemePalette;

const TP_FILE: &str = "setup-bio_rename_fixture.tp2";

struct Isolation(PathBuf);

impl Isolation {
    fn new(show_names: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "bio_tree_rename_menu_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::platform_defaults::set_config_dir_override(Some(root.clone()));
        set_display_mod_names(show_names);
        Self(root)
    }
}

impl Drop for Isolation {
    fn drop(&mut self) {
        set_display_mod_names(false);
        crate::platform_defaults::clear_config_dir_override_if(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Scene {
    mod_state: Step2ModState,
    next_selection_order: usize,
    jump_to_selected_requested: bool,
    rename_mod: Option<String>,
    fonts_ready: bool,
}

fn fixture_mod() -> Step2ModState {
    Step2ModState {
        name: "Fixture".to_string(),
        tp_file: TP_FILE.to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: false,
        hidden_components: Vec::new(),
        components: Vec::new(),
    }
}

fn scene() -> Harness<'static, Scene> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 200.0))
        .with_max_steps(64)
        .build_ui_state(
            |ui, scene: &mut Scene| {
                if !scene.fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ui.ctx());
                    scene.fonts_ready = true;
                    ui.ctx().request_repaint();
                    return;
                }
                let result = render_parent_row(
                    ui,
                    &mut scene.mod_state,
                    ParentRowInput {
                        active_tab: "BGEE",
                        selected: None,
                        next_selection_order: &mut scene.next_selection_order,
                        jump_to_selected_requested: &mut scene.jump_to_selected_requested,
                        palette: ThemePalette::Dark,
                    },
                );
                if result.rename_mod.is_some() {
                    scene.rename_mod = result.rename_mod;
                }
            },
            Scene {
                mod_state: fixture_mod(),
                next_selection_order: 0,
                jump_to_selected_requested: false,
                rename_mod: None,
                fonts_ready: false,
            },
        )
}

fn collect_text(shape: &egui::Shape, prefix: &str, found: &mut Option<egui::Rect>) {
    match shape {
        egui::Shape::Text(text) if found.is_none() && text.galley.text().starts_with(prefix) => {
            *found = Some(egui::Rect::from_min_size(text.pos, text.galley.size()));
        }
        egui::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text(shape, prefix, found);
            }
        }
        _ => {}
    }
}

fn painted_text(harness: &Harness<'_, Scene>, prefix: &str) -> Option<egui::Rect> {
    let mut found = None;
    for clipped in &harness.output().shapes {
        collect_text(&clipped.shape, prefix, &mut found);
    }
    found
}

fn press(harness: &mut Harness<'_, Scene>, pos: egui::Pos2, button: egui::PointerButton) {
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
}

fn right_click_the_mod(harness: &mut Harness<'_, Scene>) {
    harness.run();
    let label = painted_text(harness, "bio_rename_fixture").expect("mod label is painted");
    press(harness, label.center(), egui::PointerButton::Secondary);
}

#[test]
fn rename_menu_is_shown_only_with_mod_names_on() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    {
        let _isolation = Isolation::new(false);
        let mut harness = scene();
        right_click_the_mod(&mut harness);
        assert!(painted_text(&harness, "Rename").is_none());
    }
    let _isolation = Isolation::new(true);
    let mut harness = scene();
    right_click_the_mod(&mut harness);
    let rename = painted_text(&harness, "Rename").expect("Rename item is shown");
    press(&mut harness, rename.center(), egui::PointerButton::Primary);
    assert_eq!(harness.state().rename_mod.as_deref(), Some(TP_FILE));
}
