// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_pill_danger, redesign_shadow, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmOutcome {
    #[default]
    Pending,
    Confirmed,
    Cancelled,
}

pub struct ConfirmDialog<'a> {
    pub id_salt: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub confirm_label: &'a str,
    pub danger: bool,
}

const MAX_WIDTH_PX: f32 = 460.0;

#[must_use]
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    dialog: &ConfirmDialog<'_>,
) -> ConfirmOutcome {
    let mut outcome = ConfirmOutcome::Pending;

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new(dialog.title)
        .id(egui::Id::new((
            "orchestrator_confirm_dialog",
            dialog.id_salt,
        )))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            ui.label(
                egui::RichText::new(dialog.title)
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new(dialog.body)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.add_space(16.0);

            let footer_h = 30.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), footer_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    let confirm = if dialog.danger {
                        danger_primary_btn(ui, palette, dialog.confirm_label)
                    } else {
                        redesign_btn(
                            ui,
                            palette,
                            dialog.confirm_label,
                            BtnOpts {
                                small: true,
                                primary: true,
                                ..Default::default()
                            },
                        )
                    };
                    if confirm.clicked() {
                        outcome = ConfirmOutcome::Confirmed;
                    }

                    if redesign_btn(
                        ui,
                        palette,
                        "Cancel",
                        BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        outcome = ConfirmOutcome::Cancelled;
                    }
                },
            );
        });

    outcome
}

fn danger_primary_btn(ui: &mut egui::Ui, palette: ThemePalette, label: &str) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let fill = redesign_pill_danger(palette);
    let text_color = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

    let font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        let shadow_rect = rect.translate(egui::vec2(2.0, 2.0));
        painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    response
}
