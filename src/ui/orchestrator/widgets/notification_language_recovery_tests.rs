use super::*;
use crate::registry::workspace_model::ComponentRef;
use egui_kittest::{Harness, kittest::Queryable};

fn fixture_notice(eligible: usize, other: usize) -> LanguageRecoveryNotice {
    let rows = (0..eligible + other)
        .map(|index| LanguageRecoveryRow {
            tab: "BGEE".to_string(),
            original: ComponentRef {
                tp2: format!("LONG_MOD_{index}/SETUP_LONG_MOD_{index}.TP2"),
                id: i64::try_from(index).unwrap(),
                language: 0,
                wlb_inputs: (index == 0).then(|| "yes".to_string()),
            },
            available_language: (index < eligible).then_some(1),
            reason: "mod no longer present".to_string(),
        })
        .collect();
    LanguageRecoveryNotice {
        owner: "RECOVERY".to_string(),
        generation: 4,
        rows,
    }
}

fn popup(
    size: egui::Vec2,
    palette: ThemePalette,
    notice: Option<LanguageRecoveryNotice>,
) -> Harness<'static, NotificationManager> {
    let mut manager = NotificationManager::new();
    if let Some(notice) = notice {
        manager.warn_language_recovery(notice.clone());
        manager.pending.clear();
        manager.recovery_replaced = false;
        manager.view_all = Some(ViewAllRequest {
            kind: ToastKind::Warning,
            text: notice.text(),
            recovery: Some(notice),
        });
    } else {
        manager.warn_persistent("Some saved selections use a different language");
        manager.pending.clear();
        manager.view_all = Some(ViewAllRequest {
            kind: ToastKind::Warning,
            text: "Some saved selections use a different language".to_string(),
            recovery: None,
        });
    }
    let mut fonts_ready = false;
    let mut versions_app = None;
    Harness::builder()
        .with_size(size)
        .with_max_steps(64)
        .build_state(
            move |ctx, state| {
                if !fonts_ready {
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                    fonts_ready = true;
                    ctx.request_repaint();
                    return;
                }
                ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(palette));
                egui::CentralPanel::default().show(ctx, |_| {});
                if ctx.data(|data| data.get_temp::<bool>(egui::Id::new("recovery-test-versions"))).unwrap_or(false) {
                    state.guard_language_recovery_escape(ctx);
                    let app = versions_app.get_or_insert_with(|| {
                        let mut app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test("recovery-overlay");
                        app.wizard_state.step2.update_selected_popup_open = true;
                        app
                    });
                    crate::ui::workspace::versions::versions_drawer::render(ctx, app, &mut None, palette);
                    ctx.data_mut(|data| data.insert_temp(egui::Id::new("recovery-test-versions-open"), app.wizard_state.step2.update_selected_popup_open));
                }
                let (owner, generation, scanning) = ctx
                    .data(|data| {
                        data.get_temp::<(String, u64, bool)>(egui::Id::new("recovery-test-status"))
                    })
                    .unwrap_or_else(|| ("RECOVERY".to_string(), 4, false));
                if ctx.data(|data| data.get_temp::<bool>(egui::Id::new("recovery-test-guard"))).unwrap_or(false) {
                    state.guard_language_recovery_escape(ctx);
                }
                state.set_language_recovery_enabled(&owner, generation, scanning);
                state.show(ctx, palette);
                state.render_history_popup(ctx, palette, false);
                let copies = ctx.output(|output| {
                    output
                        .commands
                        .iter()
                        .filter_map(|command| match command {
                            egui::OutputCommand::CopyText(text) => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                });
                ctx.data_mut(|data| {
                    data.get_temp_mut_or_default::<Vec<String>>(egui::Id::new(
                        "recovery-test-copies",
                    ))
                    .extend(copies);
                });
            },
            manager,
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

fn painted(harness: &Harness<'_, NotificationManager>, label: &str) -> egui::Rect {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| shape_label(&shape.shape, label))
        .unwrap_or_else(|| panic!("Missing {label}"))
}

fn click(harness: &mut Harness<'_, NotificationManager>, label: &str) {
    if label == "View all" {
        harness.get_by_label(label).click();
        harness.run();
        return;
    }
    let pos = painted(harness, label).center();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(pos));
    harness.run();
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
}

fn capture(harness: &mut Harness<'_, NotificationManager>, name: &str) {
    if let Some(directory) = std::env::var_os("BIO_LANGUAGE_RECOVERY_SCREENSHOT_DIR") {
        let directory = std::path::PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        harness
            .render()
            .unwrap()
            .save(directory.join(format!("recovery-{name}.png")))
            .unwrap();
    }
}

#[test]
fn language_recovery_drawer_restore_keep_copy_and_ordinary_warning() {
    let size = egui::vec2(1000.0, 800.0);
    let expected = fixture_notice(1, 0);
    let mut harness = popup(size, ThemePalette::Dark, Some(expected.clone()));
    harness.run();
    click(&mut harness, "Copy");
    assert!(
        harness
            .ctx
            .data(|data| data.get_temp::<Vec<String>>(egui::Id::new("recovery-test-copies")))
            .unwrap_or_default()
            .iter()
            .any(|text| text == &expected.text())
    );
    assert!(harness.state().view_all.is_some());
    click(&mut harness, "Keep unresolved for now");
    assert!(harness.state().view_all.is_none());
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
    let mut harness = popup(size, ThemePalette::Dark, Some(expected.clone()));
    harness.run();
    click(&mut harness, "Restore selections");
    assert_eq!(
        harness.state_mut().take_language_recovery_action(),
        Some(expected)
    );
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
    let mut ordinary = popup(size, ThemePalette::Dark, None);
    ordinary.run();
    assert!(ordinary.state().history.back().unwrap().recovery.is_none());
    assert!(
        ordinary
            .ctx
            .memory(|memory| memory.area_rect(egui::Id::new("notification_view_all_window")))
            .is_some()
    );
    click(&mut ordinary, "Copy");
    assert!(ordinary.state().view_all.is_some());
    click(&mut ordinary, "Close");
    assert!(
        ordinary
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
}

#[test]
fn language_recovery_drawer_no_eligible_disables_restore() {
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(fixture_notice(0, 2)),
    );
    harness.run();
    click(&mut harness, "Restore selections");
    assert!(harness.state().view_all.is_some());
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
}

#[test]
fn language_recovery_drawer_snapshots_actual_widget_and_footer_bounds() {
    for (name, palette, size, eligible, other) in [
        (
            "mismatch-dark",
            ThemePalette::Dark,
            egui::vec2(1000.0, 800.0),
            2,
            0,
        ),
        (
            "mismatch-light",
            ThemePalette::Light,
            egui::vec2(1000.0, 800.0),
            2,
            0,
        ),
        ("mixed", ThemePalette::Dark, egui::vec2(1000.0, 800.0), 1, 2),
        (
            "no-eligible",
            ThemePalette::Dark,
            egui::vec2(1000.0, 800.0),
            0,
            2,
        ),
        (
            "remaining",
            ThemePalette::Light,
            egui::vec2(1000.0, 800.0),
            0,
            1,
        ),
        (
            "long-narrow",
            ThemePalette::Dark,
            egui::vec2(640.0, 560.0),
            24,
            8,
        ),
    ] {
        let mut harness = popup(size, palette, Some(fixture_notice(eligible, other)));
        harness.run();
        for label in ["Keep unresolved for now", "Copy", "Restore selections"] {
            let rect = painted(&harness, label);
            assert!(
                egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(rect),
                "{name} footer outside viewport: {label} {rect:?}"
            );
        }
        capture(&mut harness, name);
        if name == "long-narrow" {
            let pos = visible_row(&harness, 0)
                .expect("Initial row visible")
                .center();
            harness.input_mut().events.extend([
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -5000.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            harness.run();
            harness.run_steps(12);
            assert!(visible_row(&harness, 0).is_none());
            assert!(visible_row(&harness, 31).is_some());
            capture(&mut harness, "long-narrow-scrolled");
        }
    }
}

#[test]
fn language_recovery_drawer_footer_normal_row_geometry() {
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(fixture_notice(2, 0)),
    );
    harness.run();
    let bounds = drawer_rect(&harness);
    assert!(
        (bounds.max.x - 1000.0).abs() <= 1.0 && bounds.min.x >= 380.0,
        "Recovery drawer must align with the right viewport edge: {bounds:?}"
    );
    let restore = button_rect(&harness, "Restore selections");
    let copy = button_rect(&harness, "Copy");
    let keep = button_rect(&harness, "Keep unresolved for now");
    println!(
        "Measured normal drawer {bounds:?}, width={}, footer={restore:?}/{copy:?}/{keep:?}",
        bounds.width()
    );
    assert!(
        (restore.min.y - copy.min.y).abs() <= 1.0 && (copy.min.y - keep.min.y).abs() <= 1.0,
        "Footer buttons must share one row: restore={restore:?}, copy={copy:?}, keep={keep:?}"
    );
    assert!((restore.height() - copy.height()).abs() <= 1.0);
    assert!((copy.height() - keep.height()).abs() <= 1.0);
    assert!(restore.max.x < copy.min.x && copy.max.x < keep.min.x);
    assert!((copy.min.x - restore.max.x - 8.0).abs() <= 1.0);
    assert!((keep.min.x - copy.max.x - 8.0).abs() <= 1.0);
}

fn enclosing_button(shape: &egui::Shape, label: egui::Rect) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Rect(rect) if rect.rect.contains_rect(label) => Some(rect.rect),
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .filter_map(|shape| enclosing_button(shape, label))
            .min_by(|left, right| left.area().total_cmp(&right.area())),
        _ => None,
    }
}

fn button_rect(harness: &Harness<'_, NotificationManager>, label: &str) -> egui::Rect {
    let text = painted(harness, label);
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|shape| enclosing_button(&shape.shape, text))
        .min_by(|left, right| left.area().total_cmp(&right.area()))
        .unwrap_or_else(|| panic!("Missing button rectangle for {label}"))
}

fn drawer_rect(harness: &Harness<'_, NotificationManager>) -> egui::Rect {
    harness.ctx.memory(|memory| {
        memory
            .area_rect(egui::Id::new("language_recovery_drawer"))
            .unwrap()
    })
}

fn footer_rects(harness: &Harness<'_, NotificationManager>) -> [egui::Rect; 3] {
    ["Restore selections", "Copy", "Keep unresolved for now"]
        .map(|label| button_rect(harness, label))
}

fn visible_row(harness: &Harness<'_, NotificationManager>, index: usize) -> Option<egui::Rect> {
    let needle = format!("LONG_MOD_{index}/SETUP_LONG_MOD_{index}.TP2");
    harness.output().shapes.iter().find_map(|shape| {
        if let egui::Shape::Text(text) = &shape.shape
            && text.galley.text().contains(&needle)
        {
            let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
            return shape
                .clip_rect
                .intersect(rect)
                .is_positive()
                .then_some(rect);
        }
        None
    })
}

fn assert_drawer_bounds(harness: &Harness<'_, NotificationManager>, size: egui::Vec2) {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let bounds = drawer_rect(harness);
    assert!(
        viewport.contains_rect(bounds),
        "Drawer {bounds:?} outside {viewport:?}"
    );
    assert!(
        (bounds.max.x - size.x).abs() <= 1.0,
        "Drawer {bounds:?} at viewport {size:?}"
    );
    let footer = footer_rects(harness);
    for rect in footer {
        assert!(
            bounds.contains_rect(rect),
            "Footer {rect:?} outside {bounds:?}"
        );
    }
    for pair in footer.windows(2) {
        assert!(!pair[0].intersects(pair[1]));
    }
    let header = painted(harness, "Some saved selections use a different language");
    assert!(bounds.contains_rect(header));
    assert!(header.max.y < footer[0].min.y);
}

#[test]
fn language_recovery_drawer_snapshots_true_narrow_scroll_and_resize_back() {
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(fixture_notice(24, 8)),
    );
    harness.run();
    for size in [
        egui::vec2(1000.0, 800.0),
        egui::vec2(640.0, 560.0),
        egui::vec2(320.0, 560.0),
        egui::vec2(1000.0, 800.0),
    ] {
        harness.set_size(size);
        harness.step();
        assert_drawer_bounds(&harness, size);
        harness.run();
        assert_drawer_bounds(&harness, size);
    }
    harness.set_size(egui::vec2(320.0, 560.0));
    harness.run();
    let bounds = drawer_rect(&harness);
    let before = footer_rects(&harness);
    let header = painted(&harness, "Some saved selections use a different language");
    let first = visible_row(&harness, 0).expect("First row is visible initially");
    assert!(first.min.y > header.max.y && first.max.y < before[0].min.y);
    assert!(visible_row(&harness, 31).is_none());
    capture(&mut harness, "true-320-long-before-scroll");
    let pos = egui::pos2(
        bounds.center().x,
        f32::midpoint(first.min.y, before[0].min.y),
    );
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -5000.0),
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    harness.run();
    harness.run_steps(12);
    assert!(visible_row(&harness, 0).is_none());
    let last = visible_row(&harness, 31).expect("Last row must become visible after scrolling");
    assert!(last.max.y <= before[0].min.y);
    assert_eq!(
        header,
        painted(&harness, "Some saved selections use a different language")
    );
    assert_eq!(before, footer_rects(&harness));
    capture(&mut harness, "true-320-long-after-scroll");
}

#[test]
fn language_recovery_drawer_stale_and_scanning_restore_disabled() {
    for status in [
        ("RECOVERY", 4_u64, true),
        ("OTHER", 4, false),
        ("RECOVERY", 5, false),
    ] {
        let mut harness = popup(
            egui::vec2(1000.0, 800.0),
            ThemePalette::Dark,
            Some(fixture_notice(1, 0)),
        );
        harness.ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("recovery-test-status"),
                (status.0.to_string(), status.1, status.2),
            );
        });
        harness.run();
        click(&mut harness, "Restore selections");
        assert!(harness.state().view_all.is_some());
        assert!(
            harness
                .state_mut()
                .take_language_recovery_action()
                .is_none()
        );
        click(&mut harness, "Copy");
        assert!(harness.state().view_all.is_some());
    }
}

#[test]
fn language_recovery_drawer_close_escape_and_scrim_dismiss_only() {
    for action in ["close", "escape", "scrim"] {
        let mut harness = popup(
            egui::vec2(1000.0, 800.0),
            ThemePalette::Dark,
            Some(fixture_notice(1, 0)),
        );
        harness.run();
        match action {
            "close" => {
                let bounds = drawer_rect(&harness);
                let pos = egui::pos2(bounds.max.x - 31.0, bounds.min.y + 27.0);
                println!(
                    "Close target {pos:?}, shell {bounds:?}, title {:?}",
                    painted(&harness, "Some saved selections use a different language")
                );
                capture(&mut harness, "close-target");
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
            "escape" => {
                harness.input_mut().events.push(egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                harness.run();
            }
            _ => {
                let pos = egui::pos2(100.0, 400.0);
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
        }
        assert!(harness.state().view_all.is_none(), "{action}");
        assert!(
            harness
                .state_mut()
                .take_language_recovery_action()
                .is_none()
        );
        assert_eq!(harness.state().history.len(), 1);
    }
}

#[test]
fn language_recovery_drawer_over_actual_versions_and_single_escape() {
    let _ambient_lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(fixture_notice(2, 0)),
    );
    harness
        .ctx
        .data_mut(|data| data.insert_temp(egui::Id::new("recovery-test-versions"), true));
    harness.run();
    let footer = button_rect(&harness, "Keep unresolved for now");
    assert_eq!(
        harness.ctx.layer_id_at(footer.center()),
        Some(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("language_recovery_drawer")
        ))
    );
    click(&mut harness, "Copy");
    assert!(harness.state().view_all.is_some());
    capture(&mut harness, "over-versions");
    harness.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert!(harness.state().view_all.is_none());
    assert!(
        harness
            .ctx
            .data(|data| data.get_temp::<bool>(egui::Id::new("recovery-test-versions-open")))
            .unwrap()
    );
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
    harness.state_mut().history_open = true;
    harness.run();
    harness.run_steps(2);
    let history_link = painted(&harness, "View all");
    println!(
        "History expected {:?}, versions {:?}",
        egui::Id::new("notification_history_popup"),
        egui::Id::new(("drawer", "versions_drawer"))
    );
    println!(
        "History link {history_link:?}, top layer {:?}",
        harness.ctx.layer_id_at(history_link.center())
    );
    capture(&mut harness, "history-entry");
    click(&mut harness, "View all");
    assert!(harness.state().view_all.is_some());
    assert!(!harness.state().history_open);
    let pos = egui::pos2(200.0, 100.0);
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
    assert!(harness.state().view_all.is_none());
    assert!(
        harness
            .ctx
            .data(|data| data.get_temp::<bool>(egui::Id::new("recovery-test-versions-open")))
            .unwrap()
    );
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
}

#[test]
fn language_recovery_drawer_popup_suppresses_escape() {
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(fixture_notice(1, 0)),
    );
    harness.run();
    harness
        .ctx
        .memory_mut(|memory| memory.open_popup(egui::Id::new("recovery-test-popup")));
    harness.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert!(harness.state().view_all.is_some());
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
}

#[test]
fn language_recovery_drawer_toast_entry_keeps_typed_notice() {
    let expected = fixture_notice(1, 2);
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(expected.clone()),
    );
    harness.state_mut().view_all = None;
    harness.state_mut().warn_language_recovery(expected.clone());
    harness.run();
    capture(&mut harness, "toast-entry");
    click(&mut harness, "View all");
    harness.run_steps(2);
    assert_eq!(
        harness.state().view_all.as_ref().unwrap().recovery,
        Some(expected)
    );
    assert_drawer_bounds(&harness, egui::vec2(1000.0, 800.0));
}

fn live_recovery() -> Harness<'static, NotificationManager> {
    let mut harness = popup(egui::vec2(1000.0, 800.0), ThemePalette::Dark, None);
    let manager = harness.state_mut();
    manager.view_all = None;
    manager.pending.clear();
    manager.history.clear();
    manager.warn_language_recovery(fixture_notice(1, 2));
    harness.run();
    assert_eq!(recovery_toast_count(&harness), 1);
    assert!(
        harness
            .output()
            .shapes
            .iter()
            .any(|shape| shape_label(&shape.shape, "View all").is_some())
    );
    click(&mut harness, "View all");
    harness.run_steps(2);
    assert!(harness.state().view_all.is_some());
    harness
}

fn recovery_toast_count(harness: &Harness<'_, NotificationManager>) -> usize {
    harness
        .ctx
        .data(|data| data.get_temp::<Vec<Toast>>(toasts_id()))
        .unwrap_or_default()
        .iter()
        .filter(|toast| toast.kind == ToastKind::Custom(KIND_LANGUAGE_RECOVERY))
        .count()
}

#[test]
fn language_recovery_visible_toast_dismissal_retains_history() {
    let mut harness = live_recovery();
    click(&mut harness, "Keep unresolved for now");
    harness.run_steps(32);
    assert!(harness.state().view_all.is_none());
    assert_eq!(
        harness.state().history.back().unwrap().recovery,
        Some(fixture_notice(1, 2))
    );
    assert_eq!(
        recovery_toast_count(&harness),
        0,
        "Closing recovery drawer must dismiss its already visible toast"
    );
    harness.state_mut().history_open = true;
    harness.run();
    click(&mut harness, "View all");
    harness.run_steps(2);
    assert_eq!(
        harness.state().view_all.as_ref().unwrap().recovery,
        Some(fixture_notice(1, 2))
    );
    assert_eq!(recovery_toast_count(&harness), 0);
}

fn recovery_escape(harness: &mut Harness<'_, NotificationManager>) {
    harness.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
}

fn recovery_pointer(harness: &mut Harness<'_, NotificationManager>, pos: egui::Pos2) {
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(pos));
    harness.run();
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
}

#[test]
fn language_recovery_visible_toast_all_close_paths_preserve_ordinary_warning() {
    for action in [
        "Restore selections",
        "Keep unresolved for now",
        "close",
        "escape",
        "guard",
        "scrim",
    ] {
        let mut harness = live_recovery();
        harness
            .state_mut()
            .warn_persistent("Ordinary persistent warning");
        harness.run();
        match action {
            "Restore selections" | "Keep unresolved for now" => click(&mut harness, action),
            "escape" => recovery_escape(&mut harness),
            "guard" => {
                harness
                    .ctx
                    .data_mut(|data| data.insert_temp(egui::Id::new("recovery-test-guard"), true));
                recovery_escape(&mut harness);
            }
            "close" => {
                let bounds = drawer_rect(&harness);
                recovery_pointer(
                    &mut harness,
                    egui::pos2(bounds.max.x - 31.0, bounds.min.y + 27.0),
                );
            }
            _ => recovery_pointer(&mut harness, egui::pos2(100.0, 400.0)),
        }
        harness.run_steps(32);
        assert!(harness.state().view_all.is_none(), "{action}");
        assert_eq!(recovery_toast_count(&harness), 0, "{action}");
        assert_eq!(
            harness.state_mut().take_language_recovery_action(),
            (action == "Restore selections").then(|| fixture_notice(1, 2)),
            "{action}"
        );
        assert!(
            harness
                .state_mut()
                .take_language_recovery_action()
                .is_none()
        );
        assert_eq!(harness.state().recovery, Some(fixture_notice(1, 2)));
        assert!(
            harness
                .state()
                .history
                .iter()
                .any(|record| record.recovery == Some(fixture_notice(1, 2)))
        );
        assert!(
            harness
                .ctx
                .data(|data| data.get_temp::<Vec<Toast>>(toasts_id()))
                .unwrap()
                .iter()
                .any(|toast| toast.kind == ToastKind::Warning
                    && toast.text.text() == "Ordinary persistent warning")
        );
        painted(&harness, "Ordinary persistent warning");
        if action == "Keep unresolved for now" {
            capture(&mut harness, "dismissed-ordinary-remains");
        }
    }
}

#[test]
fn language_recovery_visible_toast_copy_and_popup_escape_remain_open() {
    let mut harness = live_recovery();
    click(&mut harness, "Copy");
    assert!(harness.state().view_all.is_some());
    assert_eq!(recovery_toast_count(&harness), 1);
    for guarded in [false, true] {
        harness
            .ctx
            .data_mut(|data| data.insert_temp(egui::Id::new("recovery-test-guard"), guarded));
        harness
            .ctx
            .memory_mut(|memory| memory.open_popup(egui::Id::new("recovery-test-popup")));
        recovery_escape(&mut harness);
        assert!(harness.state().view_all.is_some());
        assert_eq!(recovery_toast_count(&harness), 1);
        assert!(
            harness
                .state_mut()
                .take_language_recovery_action()
                .is_none()
        );
    }
}

#[test]
fn language_recovery_partial_restore_updates_history_without_reannouncing() {
    let mut harness = live_recovery();
    click(&mut harness, "Restore selections");
    assert_eq!(
        harness.state_mut().take_language_recovery_action(),
        Some(fixture_notice(1, 2))
    );
    let mut remaining = fixture_notice(1, 2);
    remaining.rows.remove(0);
    harness
        .state_mut()
        .warn_language_recovery(remaining.clone());
    harness.run_steps(32);
    assert_eq!(recovery_toast_count(&harness), 0);
    assert_eq!(harness.state().recovery, Some(remaining.clone()));
    assert_eq!(
        harness.state().history.back().unwrap().recovery,
        Some(remaining.clone())
    );
    assert_eq!(
        harness.state().history.back().unwrap().text,
        remaining.text()
    );
    assert!(
        harness
            .state_mut()
            .take_language_recovery_action()
            .is_none()
    );
    harness.state_mut().history_open = true;
    harness.run();
    harness.run_steps(2);
    harness.get_all_by_label("View all").next().unwrap().click();
    harness.run();
    harness.run_steps(2);
    assert_eq!(
        harness.state().view_all.as_ref().unwrap().recovery,
        Some(remaining)
    );
    assert_eq!(recovery_toast_count(&harness), 0);
    capture(&mut harness, "partial-restore-history-reopened");
}

#[test]
fn language_recovery_different_owner_or_generation_announces_again() {
    for change_owner in [false, true] {
        let mut harness = live_recovery();
        click(&mut harness, "Keep unresolved for now");
        harness.run_steps(32);
        assert_eq!(recovery_toast_count(&harness), 0);
        let mut notice = fixture_notice(1, 2);
        if change_owner {
            notice.owner = "OTHER".to_string();
        } else {
            notice.generation += 1;
        }
        harness.state_mut().warn_language_recovery(notice.clone());
        harness.run_steps(4);
        assert_eq!(recovery_toast_count(&harness), 1);
        assert_eq!(
            harness.state().history.back().unwrap().recovery,
            Some(notice)
        );
    }
}

#[test]
fn language_recovery_pending_toast_dismissal_prevents_first_display() {
    let notice = fixture_notice(1, 2);
    let mut harness = popup(
        egui::vec2(1000.0, 800.0),
        ThemePalette::Dark,
        Some(notice.clone()),
    );
    harness.state_mut().warn_language_recovery(notice.clone());
    let ctx = harness.ctx.clone();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| harness.state_mut().guard_language_recovery_escape(ctx),
    );
    assert!(harness.state().view_all.is_none());
    harness.run_steps(32);
    assert_eq!(recovery_toast_count(&harness), 0);
    assert_eq!(
        harness.state().history.back().unwrap().recovery,
        Some(notice)
    );
}

#[test]
fn language_recovery_ordinary_view_all_close_preserves_visible_warning() {
    let mut harness = popup(egui::vec2(1000.0, 800.0), ThemePalette::Dark, None);
    harness
        .state_mut()
        .warn_persistent("Ordinary persistent warning");
    harness.run();
    click(&mut harness, "Close");
    harness.run_steps(32);
    assert!(harness.state().view_all.is_none());
    painted(&harness, "Ordinary persistent warning");
}
