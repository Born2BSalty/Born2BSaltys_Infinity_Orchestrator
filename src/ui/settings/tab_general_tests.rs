use super::*;
use crate::settings::redesign_fields::UiLanguage;
use egui_kittest::{Harness, kittest::Queryable};

struct LanguageScanFixture(std::path::PathBuf);

impl Drop for LanguageScanFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn combination_red_settings_click_home_resume_uses_memory_language() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("settings-resume");
    app.wizard_state.step1.language = "en_US".to_string();
    let fixture = LanguageScanFixture(app.settings_store.path().with_extension("scan"));
    std::fs::create_dir_all(fixture.0.join("empty-mods")).unwrap();
    let mut persisted = crate::settings::model::AppSettings::default();
    persisted.step1.language = "en_US".to_string();
    persisted.step1.bgee_game_folder = fixture.0.to_string_lossy().into_owned();
    app.settings_store.save(&persisted).unwrap();
    app.registry
        .entries
        .push(crate::registry::model::ModlistEntry {
            id: "LANGUAGE-RESUME".to_string(),
            name: "Language resume".to_string(),
            ..Default::default()
        });
    app.workspace_state.insert(
        "LANGUAGE-RESUME".to_string(),
        crate::registry::workspace_model::ModlistWorkspaceState::default(),
    );
    let mut harness = language_resume_scene(app);
    for (locale, label, warm) in [
        ("de_DE", "German", false),
        ("en_US", "English", false),
        ("de_DE", "German", true),
        ("en_US", "English", true),
    ] {
        harness.state_mut().nav =
            crate::ui::orchestrator::nav_destination::NavDestination::Settings;
        harness.run();
        if harness.query_by_label(label).is_none() {
            harness.get_by_role(egui::accesskit::Role::ComboBox).click();
            harness.run();
        }
        harness
            .query_by_label(label)
            .or_else(|| {
                harness.query_by_label(if label == "German" {
                    "Deutsch"
                } else {
                    "English"
                })
            })
            .unwrap()
            .click();
        harness.run();
        harness.state_mut().nav = crate::ui::orchestrator::nav_destination::NavDestination::Home;
        harness.run();
        if warm {
            harness.state_mut().workspace_view.loaded_workspace_id =
                Some("LANGUAGE-RESUME".to_string());
        }
        harness.state_mut().nav =
            crate::ui::orchestrator::nav_destination::NavDestination::Workspace {
                modlist_id: Some("LANGUAGE-RESUME".to_string()),
            };
        harness.run();
        assert_eq!(
            harness
                .state()
                .settings_store
                .load()
                .unwrap()
                .step1
                .language,
            "en_US"
        );
        assert_eq!(harness.state().wizard_state.step1.language, locale);
        harness.state_mut().wizard_state.step1.mods_folder =
            fixture.0.join("empty-mods").to_string_lossy().into_owned();
        let (sender, _receiver) = std::sync::mpsc::channel();
        let (_, _, _, report) = crate::app::scan::worker::scan_impl(
            &harness.state().wizard_state.step1,
            &sender,
            &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();
        assert_eq!(report.preferred_locale, locale);
        assert_eq!(
            crate::app::step5::command_config::build_install_command_config(
                &harness.state().wizard_state.step1
            )
            .language,
            locale
        );
    }
}

fn scene(language: &str, general: UiLanguage) -> Harness<'static, OrchestratorApp> {
    scene_with_size(language, general, egui::vec2(1280.0, 820.0))
}

fn scene_with_size(
    language: &str,
    general: UiLanguage,
    size: egui::Vec2,
) -> Harness<'static, OrchestratorApp> {
    let mut app = OrchestratorApp::new_isolated_for_test("mod_language_general");
    app.wizard_state.step1.language = language.to_string();
    app.redesign_settings.language = general;
    let mut fonts_ready = false;
    Harness::builder()
        .with_size(size)
        .with_max_steps(12)
        .build_state(
            move |ctx, app| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(
                    app.theme_palette,
                ));
                egui::CentralPanel::default().show(ctx, |ui| render(ui, app));
            },
            app,
        )
}

#[test]
fn mod_language_general_actual_combo_updates_step1_only() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut harness = scene("en_US", UiLanguage::English);
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    let option = harness
        .query_by_label("German")
        .or_else(|| harness.query_by_label("Deutsch"))
        .unwrap();
    option.click();
    harness.run();
    assert_eq!(harness.state().wizard_state.step1.language, "de_DE");
    assert_eq!(
        harness.state().redesign_settings.language,
        UiLanguage::English
    );
}

#[test]
fn mod_language_general_displays_step1_and_actionable_wording_without_mutation() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut harness = scene("en_US", UiLanguage::German);
    harness.get_by_label("Mod installation language");
    assert!(painted_text(&harness).contains("English"));
    harness.get_by_label("controls component scans and mod installation");
    harness.get_by_label("Existing lists: use Rescan Mods after a change. Unavailable mod languages fall back to English or another available language.");
    assert!(harness.query_by_label("(coming soon)").is_none());
    assert!(
        harness
            .query_by_label("language used across the BIO app")
            .is_none()
    );
    harness.run_steps(3);
    assert_eq!(harness.state().wizard_state.step1.language, "en_US");
    assert_eq!(
        harness.state().redesign_settings.language,
        UiLanguage::German
    );
}

fn painted_text(harness: &Harness<'_, OrchestratorApp>) -> String {
    fn collect(shape: &egui::Shape, result: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                result.push_str(&text.galley.job.text);
                result.push('\n');
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, result);
                }
            }
            _ => {}
        }
    }
    let mut text = String::new();
    for shape in &harness.output().shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

#[test]
fn mod_language_general_unknown_and_automatic_are_visible_without_mutation() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (value, label) in [
        (" xX-Custom ", " xX-Custom "),
        ("", "Automatic (game language)"),
    ] {
        let mut harness = scene(value, UiLanguage::German);
        harness.run_steps(3);
        assert!(painted_text(&harness).contains(label));
        assert_eq!(harness.state().wizard_state.step1.language, value);
        assert_eq!(
            harness.state().redesign_settings.language,
            UiLanguage::German
        );
    }
}

#[test]
fn mod_language_general_active_install_disables_real_combo() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut harness = scene("en_US", UiLanguage::English);
    harness.state_mut().wizard_state.step5.install_running = true;
    harness.run_steps(2);
    harness.get_by_label("Available after installation finishes");
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    assert!(harness.query_by_label("German").is_none());
    assert_eq!(harness.state().wizard_state.step1.language, "en_US");
    harness.state_mut().wizard_state.step5.install_running = false;
}

#[test]
fn mod_language_general_snapshot() {
    let Some(output) = std::env::var_os("BIO_MOD_LANGUAGE_SCREENSHOT_DIR") else {
        return;
    };
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir_all(&output).unwrap();
    for (palette, theme) in [
        (
            crate::ui::shared::redesign_tokens::ThemePalette::Light,
            "light",
        ),
        (
            crate::ui::shared::redesign_tokens::ThemePalette::Dark,
            "dark",
        ),
    ] {
        for width in [960.0, 1280.0] {
            let mut harness =
                scene_with_size("de_DE", UiLanguage::English, egui::vec2(width, 820.0));
            harness.state_mut().theme_palette = palette;
            harness.state_mut().redesign_settings.theme_palette = match palette {
                crate::ui::shared::redesign_tokens::ThemePalette::Light => ThemeChoice::Light,
                crate::ui::shared::redesign_tokens::ThemePalette::Dark => ThemeChoice::Dark,
            };
            harness.run_steps(3);
            harness
                .render()
                .unwrap()
                .save(output.join(format!("general-language-{theme}-{width:.0}.png")))
                .unwrap();
            harness.get_by_role(egui::accesskit::Role::ComboBox).click();
            harness.run();
            harness
                .render()
                .unwrap()
                .save(output.join(format!("general-language-{theme}-{width:.0}-open.png")))
                .unwrap();
        }
    }
}

#[test]
fn mod_language_general_helper_stays_inside_language_column_below_combo() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for width in [960.0, 1280.0] {
        let harness = scene_with_size("de_DE", UiLanguage::English, egui::vec2(width, 820.0));
        let title = harness
            .get_by_label("Mod installation language")
            .bounding_box()
            .unwrap();
        let combo = harness
            .get_by_role(egui::accesskit::Role::ComboBox)
            .bounding_box()
            .unwrap();
        let helper = harness.get_by_label("Existing lists: use Rescan Mods after a change. Unavailable mod languages fall back to English or another available language.").bounding_box().unwrap();
        assert!(
            helper.x0 >= title.x0 - 1.0,
            "helper overlaps theme column: {helper:?}"
        );
        assert!(
            helper.x1 <= f64::from(width) - 7.0,
            "helper escapes screen: {helper:?}"
        );
        assert!(
            helper.y0 >= combo.y1,
            "helper overlaps combo: {helper:?} {combo:?}"
        );
    }
}

#[test]
fn mod_language_general_settings_page_english_german_english_round_trip() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut app = OrchestratorApp::new_isolated_for_test("locale-page-round-trip");
    app.wizard_state.step1.language = "en_US".to_string();
    app.redesign_settings.language = UiLanguage::German;
    let mut fonts_ready = false;
    let mut harness = Harness::builder()
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
                    crate::ui::settings::page_settings::render(ui, app, ctx);
                });
            },
            app,
        );
    for (locale, label) in [
        ("en_US", "English"),
        ("de_DE", "German"),
        ("en_US", "English"),
    ] {
        if harness.state().wizard_state.step1.language != locale {
            if harness.query_by_label(label).is_none() {
                harness.get_by_role(egui::accesskit::Role::ComboBox).click();
                harness.run();
            }
            harness.get_by_label(label).click();
            harness.run();
        }
        assert!(painted_text(&harness).contains(label));
        assert_eq!(harness.state().wizard_state.step1.language, locale);
        assert_eq!(
            harness.state().redesign_settings.language,
            UiLanguage::German
        );
    }
}

fn language_resume_scene(app: OrchestratorApp) -> Harness<'static, OrchestratorApp> {
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
                    if matches!(
                        app.nav,
                        crate::ui::orchestrator::nav_destination::NavDestination::Settings
                    ) {
                        render(ui, app);
                    } else {
                        crate::ui::orchestrator::page_router::render(ui, app, ctx);
                    }
                });
            },
            app,
        )
}
