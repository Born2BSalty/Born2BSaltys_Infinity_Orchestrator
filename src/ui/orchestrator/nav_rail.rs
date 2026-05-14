// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_status::{NavStatusKind, PathValidationSummary};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_ITEM_GAP_PX, REDESIGN_NAV_ITEM_ICON_WIDTH_PX,
    REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX, REDESIGN_NAV_ITEM_PADDING_X_PX,
    REDESIGN_NAV_ITEM_PADDING_Y_PX, REDESIGN_NAV_ITEM_RADIUS_PX, REDESIGN_NAV_WIDTH_PX,
    REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette, redesign_accent, redesign_border_soft,
    redesign_border_strong, redesign_hover_overlay, redesign_pill_danger, redesign_rail_bg,
    redesign_shadow, redesign_status_dot, redesign_text_faint, redesign_text_muted,
    redesign_text_on_accent, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: &mut NavDestination,
    footer: &PathValidationSummary,
    rail_locked_tooltip: Option<&str>,
) {
    let rail_rect = ui.max_rect();
    ui.painter()
        .rect_filled(rail_rect, 0.0, redesign_rail_bg(palette));
    ui.painter().line_segment(
        [rail_rect.right_top(), rail_rect.right_bottom()],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    ui.set_width(REDESIGN_NAV_WIDTH_PX);
    ui.add_space(14.0);
    ui.vertical_centered_justified(|ui| {
        render_brand(ui, palette);
    });
    ui.add_space(12.0);

    for item in NavDestination::rail_items() {
        render_nav_item(ui, palette, current, item, rail_locked_tooltip);
        ui.add_space(4.0);
    }

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ui.available_height()),
        egui::Layout::bottom_up(egui::Align::Min),
        |ui| render_footer(ui, palette, footer),
    );
}

fn render_brand(ui: &mut egui::Ui, palette: ThemePalette) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mark_size = egui::vec2(36.0, 36.0);
                let (mark_rect, _) = ui.allocate_exact_size(mark_size, egui::Sense::hover());
                let shadow_rect = mark_rect.translate(egui::vec2(
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                ));
                ui.painter()
                    .rect_filled(shadow_rect, 6.0, redesign_shadow(palette));
                ui.painter().rect(
                    mark_rect,
                    6.0,
                    redesign_accent(palette),
                    egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    mark_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "∞",
                    egui::FontId::proportional(24.0),
                    egui::Color32::from_rgb(0x1a, 0x1a, 0x1a),
                );

                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Infinity")
                            .size(10.0)
                            .strong()
                            .color(redesign_text_primary(palette)),
                    );
                    ui.label(
                        egui::RichText::new("Orchestrator")
                            .size(9.0)
                            .color(redesign_text_faint(palette)),
                    );
                });
            });
        });

    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top() + 12.0;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 6.0, y),
            egui::pos2(rect.right() - 6.0, y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
    ui.add_space(12.0);
}

fn render_nav_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: &mut NavDestination,
    item: NavDestination,
    rail_locked_tooltip: Option<&str>,
) {
    let active = *current == item;
    let locked = rail_locked_tooltip.is_some();
    let available_width = ui.available_width();
    let item_height = REDESIGN_NAV_ITEM_PADDING_Y_PX.mul_add(2.0, 22.0);
    let sense = if locked {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(available_width, item_height), sense);
    let text_color = if locked {
        redesign_text_faint(palette)
    } else if active {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_primary(palette)
    };
    let fill = if locked {
        egui::Color32::TRANSPARENT
    } else if active {
        redesign_accent(palette)
    } else if response.hovered() {
        redesign_hover_overlay(palette)
    } else {
        egui::Color32::TRANSPARENT
    };
    let border = if active {
        redesign_border_strong(palette)
    } else {
        egui::Color32::TRANSPARENT
    };

    if active {
        let shadow_rect = rect.translate(egui::vec2(
            REDESIGN_SHADOW_OFFSET_BTN_PX,
            REDESIGN_SHADOW_OFFSET_BTN_PX,
        ));
        ui.painter().rect_filled(
            shadow_rect,
            REDESIGN_NAV_ITEM_RADIUS_PX,
            redesign_shadow(palette),
        );
    }
    ui.painter().rect(
        rect,
        REDESIGN_NAV_ITEM_RADIUS_PX,
        fill,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border),
        egui::StrokeKind::Inside,
    );

    let content_left = rect.left() + REDESIGN_NAV_ITEM_PADDING_X_PX;
    let icon_center = egui::pos2(
        content_left + (REDESIGN_NAV_ITEM_ICON_WIDTH_PX / 2.0),
        rect.center().y,
    );
    paint_nav_icon(ui.painter(), &item, icon_center, text_color);
    ui.painter().text(
        egui::pos2(
            content_left + REDESIGN_NAV_ITEM_ICON_WIDTH_PX + REDESIGN_NAV_ITEM_GAP_PX,
            rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        item.label(),
        egui::FontId::proportional(REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX),
        text_color,
    );

    if let Some(tooltip) = rail_locked_tooltip {
        response.on_hover_text(tooltip);
    } else if response.clicked() {
        *current = item;
    }
}

fn paint_nav_icon(
    painter: &egui::Painter,
    item: &NavDestination,
    center: egui::Pos2,
    color: egui::Color32,
) {
    match item {
        NavDestination::Home => paint_home_icon(painter, center, color),
        NavDestination::Install => paint_install_icon(painter, center, color),
        NavDestination::Create => paint_create_icon(painter, center, color),
        NavDestination::Settings => paint_settings_icon(painter, center, color),
        NavDestination::Workspace { .. } => {}
    }
}

fn icon_stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.8, color)
}

fn paint_home_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    let left = center.x - 6.0;
    let right = center.x + 6.0;
    let roof_y = center.y - 1.0;
    let top = center.y - 7.0;
    let bottom = center.y + 7.0;

    painter.line_segment(
        [egui::pos2(left, roof_y), egui::pos2(center.x, top)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x, top), egui::pos2(right, roof_y)],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(left + 1.5, roof_y),
            egui::pos2(left + 1.5, bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(right - 1.5, roof_y),
            egui::pos2(right - 1.5, bottom),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(left + 1.5, bottom),
            egui::pos2(right - 1.5, bottom),
        ],
        stroke,
    );
}

fn paint_install_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 8.0),
            egui::pos2(center.x, center.y + 6.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x - 6.0, center.y),
            egui::pos2(center.x, center.y + 6.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x + 6.0, center.y),
            egui::pos2(center.x, center.y + 6.0),
        ],
        stroke,
    );
}

fn paint_create_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(3.0, color);
    painter.line_segment(
        [
            egui::pos2(center.x - 5.0, center.y - 6.0),
            egui::pos2(center.x + 6.0, center.y + 5.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x - 7.0, center.y - 8.0),
            egui::pos2(center.x - 4.0, center.y - 5.0),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            egui::pos2(center.x + 4.0, center.y + 7.0),
            egui::pos2(center.x + 8.0, center.y + 3.0),
        ],
        egui::Stroke::new(1.6, color),
    );
}

fn paint_settings_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = icon_stroke(color);
    painter.circle_stroke(center, 4.2, stroke);
    painter.circle_filled(center, 1.4, color);

    for angle in [
        0.0_f32,
        std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::FRAC_PI_4 * 3.0,
        std::f32::consts::PI,
        std::f32::consts::FRAC_PI_4 * 5.0,
        std::f32::consts::PI * 1.5,
        std::f32::consts::FRAC_PI_4 * 7.0,
    ] {
        let inner = egui::pos2(center.x + angle.cos() * 6.0, center.y + angle.sin() * 6.0);
        let outer = egui::pos2(center.x + angle.cos() * 8.0, center.y + angle.sin() * 8.0);
        painter.line_segment([inner, outer], stroke);
    }
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette, footer: &PathValidationSummary) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 6.0, rect.top()),
            egui::pos2(rect.right() - 6.0, rect.top()),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        let dot_color = match footer.kind {
            NavStatusKind::Ok => redesign_status_dot(palette),
            NavStatusKind::Error => redesign_pill_danger(palette),
        };
        ui.painter()
            .circle_filled(dot_rect.center(), 4.0, dot_color);
        ui.painter().circle_stroke(
            dot_rect.center(),
            4.0,
            egui::Stroke::new(1.0, redesign_border_strong(palette)),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(&footer.text)
                .size(11.0)
                .color(redesign_text_muted(palette)),
        );
    });
}
