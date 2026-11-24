// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_dot_background::paint_dot_background;
#[cfg(not(target_os = "windows"))]
use crate::ui::shared::redesign_tokens::REDESIGN_TITLEBAR_HEIGHT_PX;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, ThemePalette, redesign_border_strong,
    redesign_page_bg,
};
use crate::ui::shell::shell_statusbar::{self, RunningInstallStatus};
#[cfg(not(target_os = "windows"))]
use crate::ui::shell::shell_titlebar;

#[must_use]
pub fn render_shell<F: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    palette: ThemePalette,
    modlist_count: usize,
    running_install: Option<&RunningInstallStatus>,
    history_has_items: bool,
    history_open: bool,
    body: F,
) -> bool {
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    paint_dot_background(&bg_painter, ctx.screen_rect(), palette);

    #[cfg(not(target_os = "windows"))]
    egui::TopBottomPanel::top("redesign_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            shell_titlebar::render(ui, palette);
        });

    let mut history_clicked = false;
    egui::TopBottomPanel::bottom("redesign_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            if shell_statusbar::render(
                ui,
                palette,
                modlist_count,
                running_install,
                history_has_items,
                history_open,
            ) {
                history_clicked = true;
            }
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            body(ui);
        });

    #[cfg(not(target_os = "windows"))]
    paint_resize_handles(ctx);
    paint_window_border(ctx, palette);
    history_clicked
}

fn paint_window_border(ctx: &egui::Context, palette: ThemePalette) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("redesign_window_border"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_clip_rect(screen);
            ui.painter().rect_stroke(
                screen,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
        });
}

#[cfg(not(target_os = "windows"))]
fn paint_resize_handles(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let edge = 4.0;
    let corner = 8.0;

    let zones: [(egui::Rect, egui::ResizeDirection, egui::CursorIcon); 8] = [
        (
            egui::Rect::from_min_size(screen.min, egui::vec2(corner, corner)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.min.y),
                egui::pos2(screen.max.x - corner, screen.min.y + edge),
            ),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.min.y),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.max.x - edge, screen.min.y + corner),
                egui::pos2(screen.max.x, screen.max.y - corner),
            ),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.max.x - corner, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x + corner, screen.max.y - edge),
                egui::pos2(screen.max.x - corner, screen.max.y),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(screen.min.x, screen.max.y - corner),
                egui::vec2(corner, corner),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            egui::Rect::from_min_max(
                egui::pos2(screen.min.x, screen.min.y + corner),
                egui::pos2(screen.min.x + edge, screen.max.y - corner),
            ),
            egui::ResizeDirection::West,
            egui::CursorIcon::ResizeHorizontal,
        ),
    ];

    for (i, (rect, direction, cursor)) in zones.iter().enumerate() {
        egui::Area::new(egui::Id::new(("redesign_resize", i)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                let response = ui.allocate_response(rect.size(), egui::Sense::drag());
                if response.hovered() {
                    ui.ctx().set_cursor_icon(*cursor);
                }
                if response.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::BeginResize(*direction));
                }
            });
    }
}

#[cfg(all(test, target_os = "windows"))]
mod native_window_tests {
    use super::*;

    fn painted_text(shape: &egui::Shape, expected: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text().contains(expected),
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| painted_text(shape, expected)),
            _ => false,
        }
    }

    #[test]
    fn native_window_shell_uses_full_client_area_without_custom_edge_actions() {
        for size in [egui::vec2(1024.0, 700.0), egui::vec2(1280.0, 820.0)] {
            for palette in [ThemePalette::Light, ThemePalette::Dark] {
                let edge_positions = [
                    egui::pos2(1.0, 1.0),
                    egui::pos2(size.x / 2.0, 1.0),
                    egui::pos2(size.x - 1.0, 1.0),
                    egui::pos2(size.x - 1.0, size.y / 2.0),
                    egui::pos2(size.x - 1.0, size.y - 1.0),
                    egui::pos2(size.x / 2.0, size.y - 1.0),
                    egui::pos2(1.0, size.y - 1.0),
                    egui::pos2(1.0, size.y / 2.0),
                ];
                for pos in edge_positions {
                    let ctx = egui::Context::default();
                    crate::ui::shared::redesign_fonts::install_redesign_fonts(&ctx);
                    let mut body_rect = egui::Rect::NOTHING;
                    for events in [
                        vec![egui::Event::PointerMoved(pos)],
                        vec![egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        }],
                        vec![egui::Event::PointerMoved(pos + egui::vec2(12.0, 12.0))],
                    ] {
                        let output = ctx.run(
                            egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    size,
                                )),
                                events,
                                ..Default::default()
                            },
                            |ctx| {
                                assert!(!render_shell(ctx, palette, 3, None, false, false, |ui| {
                                    body_rect = ui.max_rect();
                                    ui.label("Client content");
                                }));
                            },
                        );
                        assert!(body_rect.top().abs() < 0.01);
                        assert!((body_rect.width() - size.x).abs() < 0.01);
                        assert!(
                            (body_rect.bottom() - (size.y - REDESIGN_STATUSBAR_HEIGHT_PX)).abs()
                                < 0.01
                        );
                        assert!(
                            output
                                .shapes
                                .iter()
                                .any(|shape| painted_text(&shape.shape, "3 modlists"))
                        );
                        assert!(
                            output
                                .shapes
                                .iter()
                                .any(|shape| painted_text(&shape.shape, "0 jobs running"))
                        );
                        assert!(output.viewport_output.values().all(|viewport| {
                            viewport.commands.iter().all(|command| {
                                !matches!(
                                    command,
                                    egui::ViewportCommand::StartDrag
                                        | egui::ViewportCommand::BeginResize(_)
                                )
                            })
                        }));
                    }
                }
            }
        }
    }

    #[test]
    fn native_window_shell_palette_snapshots() {
        let Some(directory) = std::env::var_os("BIO_TEST_SCREENSHOT_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        assert!(directory.is_absolute());
        let directory = directory
            .canonicalize()
            .expect("existing external snapshot directory");
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        assert!(!directory.starts_with(source));
        for (theme, palette) in [("light", ThemePalette::Light), ("dark", ThemePalette::Dark)] {
            for (layout, size) in [
                ("minimum", egui::vec2(1024.0, 700.0)),
                ("normal", egui::vec2(1280.0, 820.0)),
            ] {
                let mut fonts_installed = false;
                let mut harness = egui_kittest::Harness::builder().with_size(size).build_state(
                    move |ctx, state| {
                        if !fonts_installed {
                            crate::ui::shared::redesign_fonts::install_redesign_fonts(ctx);
                            fonts_installed = true;
                            ctx.request_repaint();
                            return;
                        }
                        ctx.set_visuals(crate::ui::shared::redesign_visuals::build_for(palette));
                        let _ = render_shell(ctx, palette, 3, None, false, false, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                crate::ui::step1::content_step1::render_game_selection_content(ui, state);
                                ui.add_space(20.0);
                                crate::ui::step1::content_step1::render_advanced_options_content(ui, state, false);
                                ui.add_space(20.0);
                                crate::ui::step1::content_step1::render_flags_content(ui, state);
                            });
                        });
                    },
                    crate::app::state::Step1State::default(),
                );
                harness.run();
                for expected in [
                    "Game Selection",
                    "Advanced Options",
                    "3 modlists",
                    "0 jobs running",
                ] {
                    assert!(
                        harness
                            .output()
                            .shapes
                            .iter()
                            .any(|shape| painted_text(&shape.shape, expected))
                    );
                }
                harness
                    .render()
                    .unwrap()
                    .save(directory.join(format!("native-window-{theme}-{layout}.png")))
                    .unwrap();
            }
        }
    }
}
