// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_status::{NavStatusKind, PathValidationSummary};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_BRAND_BOTTOM_GAP_PX,
    REDESIGN_NAV_BRAND_MARK_FONT_SIZE_PX, REDESIGN_NAV_BRAND_MARK_RADIUS_PX,
    REDESIGN_NAV_BRAND_MARK_SIZE_PX, REDESIGN_NAV_BRAND_NAME_FONT_SIZE_PX,
    REDESIGN_NAV_BRAND_PADDING_X_MARGIN, REDESIGN_NAV_BRAND_PADDING_Y_MARGIN,
    REDESIGN_NAV_BRAND_SUB_FONT_SIZE_PX, REDESIGN_NAV_BRAND_TEXT_GAP_PX,
    REDESIGN_NAV_FOOTER_DOT_GAP_PX, REDESIGN_NAV_FOOTER_DOT_SIZE_PX,
    REDESIGN_NAV_FOOTER_FONT_SIZE_PX, REDESIGN_NAV_FOOTER_TOP_PADDING_PX, REDESIGN_NAV_ITEM_GAP_PX,
    REDESIGN_NAV_ITEM_ICON_FONT_SIZE_PX, REDESIGN_NAV_ITEM_ICON_WIDTH_PX,
    REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX, REDESIGN_NAV_ITEM_PADDING_X_PX,
    REDESIGN_NAV_ITEM_PADDING_Y_PX, REDESIGN_NAV_ITEM_RADIUS_PX, REDESIGN_NAV_SEPARATOR_GAP_PX,
    REDESIGN_NAV_SEPARATOR_INSET_PX, REDESIGN_NAV_TOP_PADDING_PX, REDESIGN_NAV_WIDTH_PX,
    REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette, redesign_accent, redesign_border_soft,
    redesign_border_strong, redesign_font_medium, redesign_pill_danger, redesign_rail_bg,
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
    ui.add_space(REDESIGN_NAV_TOP_PADDING_PX);
    ui.vertical_centered_justified(|ui| {
        render_brand(ui, palette);
    });
    ui.add_space(REDESIGN_NAV_BRAND_BOTTOM_GAP_PX);

    for item in NavDestination::rail_items() {
        render_nav_item(ui, palette, current, item, rail_locked_tooltip);
        ui.add_space(REDESIGN_NAV_ITEM_GAP_PX);
    }

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ui.available_height()),
        egui::Layout::bottom_up(egui::Align::Min),
        |ui| render_footer(ui, palette, footer),
    );
}

fn render_brand(ui: &mut egui::Ui, palette: ThemePalette) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_NAV_BRAND_PADDING_X_MARGIN,
            REDESIGN_NAV_BRAND_PADDING_Y_MARGIN,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mark_size = egui::Vec2::splat(REDESIGN_NAV_BRAND_MARK_SIZE_PX);
                let (mark_rect, _) = ui.allocate_exact_size(mark_size, egui::Sense::hover());
                let shadow_rect = mark_rect.translate(egui::vec2(
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                ));
                ui.painter().rect_filled(
                    shadow_rect,
                    REDESIGN_NAV_BRAND_MARK_RADIUS_PX,
                    redesign_shadow(palette),
                );
                ui.painter().rect(
                    mark_rect,
                    REDESIGN_NAV_BRAND_MARK_RADIUS_PX,
                    redesign_accent(palette),
                    egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    mark_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "∞",
                    egui::FontId::new(REDESIGN_NAV_BRAND_MARK_FONT_SIZE_PX, redesign_font_medium()),
                    redesign_text_on_accent(palette),
                );

                ui.add_space(REDESIGN_NAV_BRAND_TEXT_GAP_PX);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("I N F I N I T Y")
                            .size(REDESIGN_NAV_BRAND_NAME_FONT_SIZE_PX)
                            .family(redesign_font_medium())
                            .color(redesign_text_primary(palette)),
                    );
                    ui.label(
                        egui::RichText::new("O R C H E S T R A T O R")
                            .size(REDESIGN_NAV_BRAND_SUB_FONT_SIZE_PX)
                            .color(redesign_text_faint(palette)),
                    );
                });
            });
        });

    let rect = ui.available_rect_before_wrap();
    let y = ui.cursor().top() + REDESIGN_NAV_SEPARATOR_GAP_PX;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + REDESIGN_NAV_SEPARATOR_INSET_PX, y),
            egui::pos2(rect.right() - REDESIGN_NAV_SEPARATOR_INSET_PX, y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
    ui.add_space(REDESIGN_NAV_SEPARATOR_GAP_PX);
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
    let item_height = REDESIGN_NAV_ITEM_PADDING_Y_PX.mul_add(
        2.0,
        REDESIGN_NAV_ITEM_ICON_FONT_SIZE_PX.max(REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX),
    );
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
    let fill = if active {
        redesign_accent(palette)
    } else {
        egui::Color32::TRANSPARENT
    };
    let border = if active {
        redesign_border_strong(palette)
    } else if response.hovered() {
        redesign_border_soft(palette)
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
    ui.painter().text(
        icon_center,
        egui::Align2::CENTER_CENTER,
        item.icon(),
        egui::FontId::monospace(REDESIGN_NAV_ITEM_ICON_FONT_SIZE_PX),
        text_color,
    );
    ui.painter().text(
        egui::pos2(
            content_left + REDESIGN_NAV_ITEM_ICON_WIDTH_PX + REDESIGN_NAV_ITEM_GAP_PX,
            rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        item.label(),
        egui::FontId::new(REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX, redesign_font_medium()),
        text_color,
    );

    if let Some(tooltip) = rail_locked_tooltip {
        response.on_hover_text(tooltip);
    } else if response.clicked() {
        *current = item;
    }
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette, footer: &PathValidationSummary) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + REDESIGN_NAV_SEPARATOR_INSET_PX, rect.top()),
            egui::pos2(rect.right() - REDESIGN_NAV_SEPARATOR_INSET_PX, rect.top()),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette)),
    );
    ui.add_space(REDESIGN_NAV_FOOTER_TOP_PADDING_PX);
    ui.horizontal(|ui| {
        let (dot_rect, _) = ui.allocate_exact_size(
            egui::Vec2::splat(REDESIGN_NAV_FOOTER_DOT_SIZE_PX),
            egui::Sense::hover(),
        );
        let dot_color = match footer.kind {
            NavStatusKind::Ok => redesign_status_dot(palette),
            NavStatusKind::Error => redesign_pill_danger(palette),
        };
        ui.painter().circle_filled(
            dot_rect.center(),
            REDESIGN_NAV_FOOTER_DOT_SIZE_PX / 2.0,
            dot_color,
        );
        ui.painter().circle_stroke(
            dot_rect.center(),
            REDESIGN_NAV_FOOTER_DOT_SIZE_PX / 2.0,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        );
        ui.add_space(REDESIGN_NAV_FOOTER_DOT_GAP_PX);
        ui.label(
            egui::RichText::new(&footer.text)
                .size(REDESIGN_NAV_FOOTER_FONT_SIZE_PX)
                .color(redesign_text_muted(palette)),
        );
    });
}
