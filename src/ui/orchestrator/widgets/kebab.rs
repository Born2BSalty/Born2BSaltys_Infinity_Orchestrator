// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_hover_overlay, redesign_pill_danger, redesign_shell_bg, redesign_text_primary,
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
    trigger_height: f32,
) -> egui::Response {
    let popup_id = ui.make_persistent_id(("orchestrator_kebab", id_salt));

    let trigger = trigger_button(ui, palette, trigger_height);
    if trigger.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    popup_below_widget_right_aligned(
        ui,
        popup_id,
        &trigger,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_max_width(DROPDOWN_MIN_WIDTH_PX);

            let chassis = egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                .inner_margin(egui::Margin::same(4));

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

fn popup_below_widget_right_aligned<R>(
    parent_ui: &egui::Ui,
    popup_id: egui::Id,
    widget_response: &egui::Response,
    close_behavior: egui::popup::PopupCloseBehavior,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    if !parent_ui.memory(|mem| mem.is_popup_open(popup_id)) {
        return None;
    }

    let mut pos = widget_response.rect.right_bottom();
    if let Some(to_global) = parent_ui
        .ctx()
        .layer_transform_to_global(parent_ui.layer_id())
    {
        pos = to_global * pos;
    }

    let frame = egui::Frame::popup(parent_ui.style());
    let response = egui::Area::new(popup_id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .pivot(egui::Align2::RIGHT_TOP)
        .show(parent_ui.ctx(), |ui| frame.show(ui, add_contents).inner);

    let should_close = match close_behavior {
        egui::popup::PopupCloseBehavior::CloseOnClick => widget_response.clicked_elsewhere(),
        egui::popup::PopupCloseBehavior::CloseOnClickOutside => {
            widget_response.clicked_elsewhere() && response.response.clicked_elsewhere()
        }
        egui::popup::PopupCloseBehavior::IgnoreClicks => false,
    };
    if parent_ui.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        parent_ui.memory_mut(egui::Memory::close_popup);
    }
    Some(response.inner)
}

fn trigger_button(ui: &mut egui::Ui, palette: ThemePalette, height: f32) -> egui::Response {
    let pad_x = 9.0;
    let label = "\u{00B7}\u{00B7}\u{00B7}";
    let text_color = redesign_text_primary(palette);
    let font = egui::FontId::new(15.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, height);
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
