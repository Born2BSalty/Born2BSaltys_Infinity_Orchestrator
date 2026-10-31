use super::*;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::settings::tool_validation::{ProbeTicket, ToolStatus};
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Scene {
    app: OrchestratorApp,
    urls: Vec<String>,
    _fixture_root: FixtureRoot,
}

struct FixtureRoot(PathBuf);

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture_root() -> FixtureRoot {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("bio_tools_page_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&root).unwrap();
    FixtureRoot(root)
}

fn preload(app: &mut OrchestratorApp, kind: ToolKind, value: &str, status: ToolStatus) {
    let now = Instant::now();
    let ticket: ProbeTicket = Arc::new(Mutex::new(Some(status)));
    for delta in [0, 201, 202] {
        app.settings_screen_state.tool_validation.observe_with(
            kind,
            value,
            now + Duration::from_millis(delta),
            |_, _| Ok(Arc::clone(&ticket)),
        );
    }
}

fn scene(size: egui::Vec2, wrong_tool: bool, empty: bool) -> Harness<'static, Scene> {
    let root = fixture_root();
    crate::platform_defaults::set_config_dir_override(Some(root.0.join("bootstrap")));
    let mut app = OrchestratorApp::new_isolated_for_test("tools_guidance");
    app.settings_screen_state.active_tab = SettingsTab::Tools;
    app.wizard_state.step1.weidu_binary.clear();
    app.wizard_state.step1.mod_installer_binary.clear();
    if !empty {
        let weidu = root.0.join("renamed-weidu.bin");
        let installer = root.0.join("renamed-installer.bin");
        std::fs::write(&weidu, b"fake weidu bytes; never executed").unwrap();
        std::fs::write(&installer, b"fake installer bytes; never executed").unwrap();
        let weidu = weidu.to_string_lossy().into_owned();
        let installer = installer.to_string_lossy().into_owned();
        app.wizard_state.step1.weidu_binary.clone_from(&weidu);
        app.wizard_state
            .step1
            .mod_installer_binary
            .clone_from(&installer);
        preload(
            &mut app,
            ToolKind::WeiDu,
            &weidu,
            ToolStatus::Detected("24900".to_string()),
        );
        preload(
            &mut app,
            ToolKind::ModInstaller,
            &installer,
            if wrong_tool {
                ToolStatus::Unrecognized
            } else {
                ToolStatus::Detected("15.0.1".to_string())
            },
        );
    }
    let mut fonts_ready = false;
    Harness::builder()
        .with_size(size)
        .with_max_steps(12)
        .build_state(
            move |ctx, scene| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(
                    scene.app.theme_palette,
                ));
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui::settings::page_settings::render(ui, &mut scene.app, ctx);
                });
                ctx.output(|output| {
                    for command in &output.commands {
                        if let egui::OutputCommand::OpenUrl(url) = command {
                            scene.urls.push(url.url.clone());
                        }
                    }
                });
            },
            Scene {
                app,
                urls: Vec::new(),
                _fixture_root: root,
            },
        )
}

fn click(harness: &mut Harness<'_, Scene>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

#[test]
fn tools_links_are_available_with_empty_paths_and_emit_exact_urls() {
    let mut harness = scene(egui::vec2(1280.0, 820.0), false, true);
    harness.get_by_label(guidance(ToolKind::WeiDu, ToolPlatform::current()));
    harness.get_by_label(guidance(ToolKind::ModInstaller, ToolPlatform::current()));
    click(&mut harness, "Download WeiDU");
    click(&mut harness, "Download Mod Installer");
    assert_eq!(
        harness.state().urls,
        [
            "https://github.com/WeiDUorg/weidu/releases",
            "https://github.com/dark0dave/mod_installer/releases"
        ]
    );
    assert_eq!(harness.state().app.wizard_state.step1.weidu_binary, "");
    assert_eq!(
        harness.state().app.wizard_state.step1.mod_installer_binary,
        ""
    );
}

#[test]
fn recognized_results_keep_paths_and_settings_clean() {
    let mut harness = scene(egui::vec2(1280.0, 820.0), false, false);
    let before = harness.state().app.wizard_state.step1.clone();
    harness.get_by_label("Detected WeiDU — version 24900");
    harness.get_by_label("Detected Mod Installer — version 15.0.1");
    harness.run_steps(3);
    let app = &harness.state().app;
    assert_eq!(app.wizard_state.step1.weidu_binary, before.weidu_binary);
    assert_eq!(
        app.wizard_state.step1.mod_installer_binary,
        before.mod_installer_binary
    );
    assert!(app.settings_screen_state.path_edit_debounce.is_empty());
}

#[test]
fn wrong_existing_executable_shows_identity_error_and_download_help() {
    let mut harness = scene(egui::vec2(960.0, 720.0), true, false);
    harness.get_by_label("This file exists, but it isn't recognized as Mod Installer.");
    harness.get_by_label("Download Mod Installer");
    click(&mut harness, "Download Mod Installer");
    assert_eq!(
        harness.state().urls.last().map(String::as_str),
        Some("https://github.com/dark0dave/mod_installer/releases")
    );
    assert!(
        harness
            .state()
            .app
            .settings_screen_state
            .path_edit_debounce
            .is_empty()
    );
}

#[test]
fn editing_a_path_immediately_replaces_the_old_detected_result() {
    let mut harness = scene(egui::vec2(1280.0, 820.0), false, false);
    let original = harness.state().app.wizard_state.step1.weidu_binary.clone();
    let missing = Path::new(&original)
        .with_file_name("missing.exe")
        .to_string_lossy()
        .into_owned();
    harness.get_all_by_value(&original).next().unwrap().focus();
    harness.run();
    harness.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness
        .input_mut()
        .events
        .push(egui::Event::Text(missing.clone()));
    harness.run();
    assert_eq!(harness.state().app.wizard_state.step1.weidu_binary, missing);
    assert!(
        harness
            .state()
            .app
            .settings_screen_state
            .path_edit_debounce
            .contains_key(validate_now::FIELD_WEIDU_BINARY)
    );
    let mut validation = harness
        .state()
        .app
        .settings_screen_state
        .tool_validation
        .clone();
    assert!(matches!(
        validation.observe_with(ToolKind::WeiDu, &missing, Instant::now(), |_, _| panic!(
            "missing file"
        )),
        ToolStatus::InvalidPath(_)
    ));
}

#[test]
fn tools_guidance_production_render_fits_normal_and_narrow_views() {
    for (name, size, wrong) in [
        ("normal", egui::vec2(1280.0, 820.0), false),
        ("narrow", egui::vec2(960.0, 720.0), false),
        ("wrong-tool", egui::vec2(960.0, 720.0), true),
    ] {
        let mut harness = scene(size, wrong, false);
        harness.get_by_label("Download WeiDU");
        harness.get_by_label("Download Mod Installer");
        if let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            harness
                .render()
                .unwrap()
                .save(directory.join(format!("tools-guidance-{name}.png")))
                .unwrap();
        }
    }
}

fn link_shape<'a>(shape: &'a egui::Shape, label: &str) -> Option<&'a egui::epaint::TextShape> {
    match shape {
        egui::Shape::Text(text) if text.galley.text().starts_with(label) => Some(text),
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| link_shape(shape, label)),
        _ => None,
    }
}

#[test]
fn tool_download_links_keep_teal_icon_and_click_target_under_production_theme() {
    for size in [egui::vec2(1280.0, 820.0), egui::vec2(960.0, 720.0)] {
        let mut harness = scene(size, false, true);
        let expected =
            crate::ui::shared::redesign_tokens::redesign_accent(harness.state().app.theme_palette);
        for (label, url) in [
            (
                "Download WeiDU",
                "https://github.com/WeiDUorg/weidu/releases",
            ),
            (
                "Download Mod Installer",
                "https://github.com/dark0dave/mod_installer/releases",
            ),
        ] {
            harness.get_by_label(label);
            let shape = harness
                .output()
                .shapes
                .iter()
                .find_map(|shape| link_shape(&shape.shape, label))
                .unwrap();
            assert!(shape.galley.job.sections.iter().all(|section| {
                shape.override_text_color.unwrap_or(section.format.color) == expected
            }));
            let icon = shape
                .galley
                .job
                .sections
                .iter()
                .find(|section| {
                    shape.galley.job.text[section.byte_range.clone()].contains('\u{F08E}')
                })
                .unwrap();
            assert_eq!(
                icon.format.font_id.family,
                egui::FontFamily::Name("firacode_nerd".into())
            );
            let bounds = egui::Rect::from_min_size(shape.pos, shape.galley.size());
            let pos = egui::pos2(bounds.right() - 3.0, bounds.center().y);
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
            assert_eq!(harness.state().urls.last().map(String::as_str), Some(url));
        }
    }
}
