// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use eframe::egui::{Color32, Frame, Id, Key, Margin, Order, Pos2, ScrollArea, Sense, Vec2};

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_border_soft, redesign_border_strong,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

const CONTENT_FRACTION: f32 = 0.6;
const CONTENT_MIN_PX: f32 = 380.0;
const CONTENT_MAX_PX: f32 = 920.0;
const FORM_FRACTION: f32 = 0.42;
const FORM_MIN_PX: f32 = 380.0;
const FORM_MAX_PX: f32 = 540.0;
const FOOTER_MIN_HEIGHT_PX: f32 = 50.0;
const FOOTER_MAX_HEIGHT_PX: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawerWidth {
    Content,
    Form,
}

#[must_use]
pub(crate) fn drawer_width(window_w: f32, width: DrawerWidth) -> f32 {
    match width {
        DrawerWidth::Content => (window_w * CONTENT_FRACTION).clamp(CONTENT_MIN_PX, CONTENT_MAX_PX),
        DrawerWidth::Form => (window_w * FORM_FRACTION).clamp(FORM_MIN_PX, FORM_MAX_PX),
    }
}

pub(crate) struct DrawerSpec<'a> {
    pub(crate) id_salt: &'a str,
    pub(crate) title: &'a str,
    pub(crate) subtitle: &'a str,
    pub(crate) width: DrawerWidth,
}

pub(crate) struct DrawerResponse<F> {
    pub(crate) close_requested: bool,
    pub(crate) footer: F,
}

pub(crate) fn render<F>(
    ctx: &egui::Context,
    palette: ThemePalette,
    spec: &DrawerSpec<'_>,
    body: impl FnOnce(&mut egui::Ui),
    footer: impl FnOnce(&mut egui::Ui) -> F,
) -> DrawerResponse<F> {
    let screen = ctx.screen_rect();
    let panel = egui::Rect::from_min_max(
        Pos2::new(screen.left(), screen.top() + REDESIGN_TITLEBAR_HEIGHT_PX),
        screen.max,
    );
    let w = drawer_width(screen.width(), spec.width);
    let popup_was_open = ctx.memory(egui::Memory::any_popup_open);
    let footer_h_id = Id::new(("drawer_footer_height", spec.id_salt));
    let footer_h = ctx
        .data(|d| d.get_temp::<f32>(footer_h_id))
        .unwrap_or(FOOTER_MIN_HEIGHT_PX)
        .clamp(FOOTER_MIN_HEIGHT_PX, FOOTER_MAX_HEIGHT_PX);

    let scrim_clicked = egui::Area::new(Id::new(("drawer_scrim", spec.id_salt)))
        .order(Order::Middle)
        .fixed_pos(panel.min)
        .interactable(true)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(panel, Sense::click());
            ui.painter()
                .rect_filled(panel, 0.0, Color32::from_black_alpha(148));
            response.clicked()
        })
        .inner;

    let mut head_close_clicked = false;
    let mut footer_out: Option<F> = None;

    let drawer_id = Id::new(("drawer", spec.id_salt));
    egui::Area::new(drawer_id)
        .order(Order::Foreground)
        .fixed_pos(Pos2::new(panel.right() - w, panel.top()))
        .show(ctx, |ui| {
            Frame::default()
                .fill(redesign_shell_bg(palette))
                .shadow(redesign_overlay_shadow(palette))
                .inner_margin(Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(w, panel.height()));
                    ui.set_max_size(Vec2::new(w, panel.height()));

                    let border_rect = ui.max_rect();
                    ui.painter().line_segment(
                        [border_rect.left_top(), border_rect.left_bottom()],
                        egui::Stroke::new(1.0_f32, redesign_border_strong(palette)),
                    );

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        head_close_clicked = render_head(ui, palette, spec);

                        let body_h = (ui.available_height() - footer_h).max(0.0);
                        ScrollArea::vertical()
                            .id_salt(("drawer_body", spec.id_salt))
                            .auto_shrink([false, false])
                            .max_height(body_h)
                            .show(ui, |ui| {
                                Frame::default()
                                    .inner_margin(Margin::symmetric(22, 18))
                                    .show(ui, body);
                            });

                        ui.painter().hline(
                            ui.max_rect().x_range(),
                            ui.cursor().top(),
                            egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
                        );

                        let footer_output = ScrollArea::vertical()
                            .id_salt(("drawer_footer", spec.id_salt))
                            .auto_shrink([false, false])
                            .max_height(footer_h)
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                Frame::default()
                                    .inner_margin(Margin {
                                        left: 22,
                                        right: 22,
                                        top: 12,
                                        bottom: 12,
                                    })
                                    .show(ui, |ui| ui.horizontal(footer).inner)
                                    .inner
                            });
                        let wanted_h = footer_output
                            .content_size
                            .y
                            .clamp(FOOTER_MIN_HEIGHT_PX, FOOTER_MAX_HEIGHT_PX);
                        if (wanted_h - footer_h).abs() > 0.5 {
                            ctx.data_mut(|d| d.insert_temp(footer_h_id, wanted_h));
                            ctx.request_repaint();
                        }
                        footer_out = Some(footer_output.inner);
                    });
                });
        });

    let escape_closes = ctx.input(|i| i.key_pressed(Key::Escape)) && !popup_was_open;

    DrawerResponse {
        close_requested: scrim_clicked || head_close_clicked || escape_closes,
        footer: footer_out.expect("drawer footer renders every frame"),
    }
}

fn render_head(ui: &mut egui::Ui, palette: ThemePalette, spec: &DrawerSpec<'_>) -> bool {
    let mut close_clicked = false;
    Frame::default()
        .inner_margin(Margin {
            left: 22,
            right: 22,
            top: 18,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(spec.title)
                            .size(17.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_primary(palette)),
                    );
                    ui.label(
                        egui::RichText::new(spec.subtitle)
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_muted(palette)),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if redesign_btn(
                        ui,
                        palette,
                        "\u{00D7} close",
                        BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        close_clicked = true;
                    }
                });
            });
        });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, redesign_border_soft(palette)),
    );
    close_clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_width_is_sixty_percent_clamped_to_380_and_920() {
        assert!((drawer_width(1024.0, DrawerWidth::Content) - 614.4).abs() < 0.01);
        assert!((drawer_width(2000.0, DrawerWidth::Content) - 920.0).abs() < 0.01);
        assert!((drawer_width(600.0, DrawerWidth::Content) - 380.0).abs() < 0.01);
    }

    #[test]
    fn form_width_is_forty_two_percent_clamped_to_380_and_540() {
        assert!((drawer_width(1024.0, DrawerWidth::Form) - 430.08).abs() < 0.01);
        assert!((drawer_width(2000.0, DrawerWidth::Form) - 540.0).abs() < 0.01);
        assert!((drawer_width(600.0, DrawerWidth::Form) - 380.0).abs() < 0.01);
    }
}
