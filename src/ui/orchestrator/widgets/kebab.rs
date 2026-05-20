// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_I8,
    ThemePalette, redesign_border_strong, redesign_hover_overlay, redesign_pill_danger,
    redesign_shadow, redesign_shell_bg, redesign_text_primary,
};

pub struct KebabItem<'a> {
    pub label: &'a str,
    pub on_click: Box<dyn FnMut() + 'a>,
    pub danger: bool,
}

impl<'a> KebabItem<'a> {
    pub fn new(label: &'a str, on_click: impl FnMut() + 'a) -> Self {
        Self {
            label,
            on_click: Box::new(on_click),
            danger: false,
        }
    }

    pub fn danger(label: &'a str, on_click: impl FnMut() + 'a) -> Self {
        Self {
            label,
            on_click: Box::new(on_click),
            danger: true,
        }
    }
}

const DROPDOWN_MIN_WIDTH_PX: f32 = 180.0;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    id_salt: &str,
    items: &mut [KebabItem<'_>],
) -> egui::Response {
    let popup_id = ui.make_persistent_id(("orchestrator_kebab", id_salt));

    let trigger = trigger_button(ui, palette);
    if trigger.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &trigger,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(DROPDOWN_MIN_WIDTH_PX);

            let chassis = egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                .inner_margin(egui::Margin::same(4))
                .shadow(egui::epaint::Shadow {
                    offset: [
                        REDESIGN_SHADOW_OFFSET_BTN_I8 + 1,
                        REDESIGN_SHADOW_OFFSET_BTN_I8 + 1,
                    ],
                    blur: 0,
                    spread: 0,
                    color: redesign_shadow(palette),
                });

            chassis.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for item in items.iter_mut() {
                    if menu_item(ui, palette, item.label, item.danger) {
                        (item.on_click)();
                        ui.memory_mut(egui::Memory::close_popup);
                    }
                }
            });
        },
    );

    trigger
}

fn trigger_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 9.0;
    let pad_y = 3.0;
    let label = "\u{00B7}\u{00B7}\u{00B7}";
    let text_color = redesign_text_primary(palette);
    let font = egui::FontId::new(15.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
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

    response.on_hover_text("More actions")
}

fn menu_item(ui: &mut egui::Ui, palette: ThemePalette, label: &str, danger: bool) -> bool {
    let text_color = if danger {
        redesign_pill_danger(palette)
    } else {
        redesign_text_primary(palette)
    };
    let font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into()));

    let pad_x = 10.0;
    let pad_y = 6.0;
    let row_width = ui.available_width().max(DROPDOWN_MIN_WIDTH_PX - 8.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let row_height = galley.size().y + pad_y * 2.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
                redesign_hover_overlay(palette),
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            text_color,
        );
    }

    response.clicked()
}
