// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

pub use crate::install_runtime::rail_lock_reason::RailLockReason;
use crate::install_runtime::rail_lock_reason::rail_lock_tooltip;
use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_status::{PathValidationKind, PathValidationSummary};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_WIDTH_PX,
    REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_hover_overlay, redesign_rail_bg, redesign_shadow, redesign_shell_bg,
    redesign_status_dot, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: &mut NavDestination,
    _dev_mode: bool,
    validation: &PathValidationSummary,
    rail_locked: Option<&RailLockReason>,
) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    painter.rect_filled(rect, 0.0, redesign_rail_bg(palette));
    let right_x = REDESIGN_BORDER_WIDTH_PX.mul_add(-0.5, rect.right());
    painter.line_segment(
        [
            egui::pos2(right_x, rect.top()),
            egui::pos2(right_x, rect.bottom()),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    let pad_x = 14.0;
    let pad_top = 14.0;
    let pad_bottom = 14.0;
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad_x, rect.top() + pad_top),
        egui::pos2(
            rect.right() - REDESIGN_BORDER_WIDTH_PX - pad_x,
            rect.bottom() - pad_bottom,
        ),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        ui.set_clip_rect(content_rect);
        ui.vertical(|ui| {
            render_brand_row(ui, palette);

            ui.add_space(12.0);
            let sep_top = ui.cursor().min.y;
            draw_dashed_horizontal(
                ui.painter(),
                sep_top,
                content_rect.left(),
                content_rect.right(),
                redesign_border_strong(palette),
            );
            ui.add_space(10.0);

            let lock_tooltip = rail_locked.map(|reason| match reason {
                RailLockReason::InstallRunning { modlist_label, .. } => {
                    rail_lock_tooltip(modlist_label)
                }
            });
            for dest in NavDestination::rail_items() {
                if let Some(tip) = lock_tooltip.as_deref() {
                    render_nav_item_locked(ui, palette, &dest, tip);
                } else {
                    let active = is_active(current, &dest);
                    let clicked = render_nav_item(ui, palette, &dest, active);
                    if clicked {
                        *current = dest;
                    }
                }
                ui.add_space(4.0);
            }

            let used_y = ui.cursor().min.y;
            let bottom_dashed_y = content_rect.bottom() - 32.0;
            if bottom_dashed_y > used_y {
                ui.add_space(bottom_dashed_y - used_y);
            }

            draw_dashed_horizontal(
                ui.painter(),
                bottom_dashed_y,
                content_rect.left(),
                content_rect.right(),
                redesign_border_strong(palette),
            );
            ui.add_space(8.0);

            render_status_row(ui, palette, validation);
        });
    });

    let _ = REDESIGN_NAV_WIDTH_PX;
}

const fn is_active(current: &NavDestination, dest: &NavDestination) -> bool {
    matches!(
        (current, dest),
        (NavDestination::Home, NavDestination::Home)
            | (NavDestination::Install, NavDestination::Install)
            | (
                NavDestination::Create | NavDestination::Workspace { .. },
                NavDestination::Create
            )
            | (NavDestination::Settings, NavDestination::Settings)
    )
}

fn render_brand_row(ui: &mut egui::Ui, palette: ThemePalette) {
    let painter = ui.painter().clone();
    let brand_mark_size = 36.0;
    let (mark_rect, _) = ui.allocate_exact_size(
        egui::vec2(brand_mark_size + 8.0 + 100.0, brand_mark_size),
        egui::Sense::hover(),
    );
    let mark_square =
        egui::Rect::from_min_size(mark_rect.min, egui::vec2(brand_mark_size, brand_mark_size));

    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    let shadow_rect = mark_square.translate(egui::vec2(
        REDESIGN_SHADOW_OFFSET_BTN_PX,
        REDESIGN_SHADOW_OFFSET_BTN_PX,
    ));
    painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));

    painter.rect_filled(mark_square, radius, redesign_accent(palette));
    painter.rect_stroke(
        mark_square,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );

    painter.text(
        mark_square.center(),
        egui::Align2::CENTER_CENTER,
        "\u{221E}",
        egui::FontId::new(22.0, egui::FontFamily::Name("firacode_nerd".into())),
        egui::Color32::from_rgb(0x1a, 0x26, 0x38),
    );

    let text_left = mark_square.right() + 10.0;
    let text_top = mark_square.top() + 2.0;

    painter.text(
        egui::pos2(text_left, text_top),
        egui::Align2::LEFT_TOP,
        "I N F I N I T Y",
        egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into())),
        redesign_text_primary(palette),
    );
    painter.text(
        egui::pos2(text_left, text_top + 14.0),
        egui::Align2::LEFT_TOP,
        "O R C H E S T R A T O R",
        egui::FontId::new(9.0, egui::FontFamily::Proportional),
        redesign_text_faint(palette),
    );
}

fn render_nav_item(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    dest: &NavDestination,
    active: bool,
) -> bool {
    let height = 36.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

    if active {
        let shadow_rect = rect.translate(egui::vec2(
            REDESIGN_SHADOW_OFFSET_BTN_PX,
            REDESIGN_SHADOW_OFFSET_BTN_PX,
        ));
        painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));
        painter.rect_filled(rect, radius, redesign_accent(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    } else if response.hovered() {
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_filled(rect, radius, redesign_hover_overlay(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }

    let text_color = redesign_text_primary(palette);

    let icon_center = egui::pos2(rect.left() + 20.0, rect.center().y);
    paint_nav_icon(painter, dest, icon_center, text_color);

    let label_x = rect.left() + 38.0;
    painter.text(
        egui::pos2(label_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        dest.label(),
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
        text_color,
    );

    response.clicked()
}

fn render_nav_item_locked(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    dest: &NavDestination,
    tooltip: &str,
) {
    let height = 36.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );

    let text_color = redesign_text_faint(palette);
    let painter = ui.painter();
    let icon_center = egui::pos2(rect.left() + 20.0, rect.center().y);
    paint_nav_icon(painter, dest, icon_center, text_color);
    let label_x = rect.left() + 38.0;
    painter.text(
        egui::pos2(label_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        dest.label(),
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
        text_color,
    );

    response.on_hover_text(tooltip);
}

fn paint_nav_icon(
    painter: &egui::Painter,
    dest: &NavDestination,
    center: egui::Pos2,
    color: egui::Color32,
) {
    match dest {
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

    painter.add(egui::Shape::closed_line(
        vec![
            egui::pos2(left, bottom),
            egui::pos2(left, roof_y),
            egui::pos2(center.x, top),
            egui::pos2(right, roof_y),
            egui::pos2(right, bottom),
        ],
        stroke,
    ));
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
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    let along = egui::vec2(diagonal, -diagonal);
    let perp = egui::vec2(diagonal, diagonal);
    let half_len = 5.0;
    let half_w = 1.6;

    let cap_far = center + along * half_len;
    let cap_near = center + along * (half_len - 2.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            cap_far + perp * half_w,
            cap_far - perp * half_w,
            cap_near - perp * half_w,
            cap_near + perp * half_w,
        ],
        color,
        egui::Stroke::NONE,
    ));

    let body_top = center + along * (half_len - 2.7);
    let body_bot = center - along * half_len;
    let body_c = body_bot - perp * half_w;
    let body_d = body_bot + perp * half_w;
    painter.add(egui::Shape::convex_polygon(
        vec![
            body_top + perp * half_w,
            body_top - perp * half_w,
            body_c,
            body_d,
        ],
        color,
        egui::Stroke::NONE,
    ));

    let tip_apex = body_bot - along * 3.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip_apex, body_c, body_d],
        color,
        egui::Stroke::NONE,
    ));
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
        let inner = egui::pos2(
            angle.cos().mul_add(6.0, center.x),
            angle.sin().mul_add(6.0, center.y),
        );
        let outer = egui::pos2(
            angle.cos().mul_add(8.0, center.x),
            angle.sin().mul_add(8.0, center.y),
        );
        painter.line_segment([inner, outer], stroke);
    }
}

fn render_status_row(ui: &mut egui::Ui, palette: ThemePalette, validation: &PathValidationSummary) {
    let dot_color = match validation.kind {
        PathValidationKind::Ok => redesign_status_dot(palette),
        PathValidationKind::Err(_) => egui::Color32::from_rgb(0xE0, 0x6C, 0x6C),
    };
    let text_color = redesign_text_muted(palette);

    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    let painter = ui.painter();

    let dot_center = egui::pos2(rect.left() + 4.0, rect.center().y);
    painter.circle_filled(dot_center, 4.0, dot_color);
    painter.circle_stroke(
        dot_center,
        4.0,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );

    let text_pos = egui::pos2(dot_center.x + 12.0, rect.center().y);
    painter.text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        &validation.text,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
        text_color,
    );
}

fn draw_dashed_horizontal(
    painter: &egui::Painter,
    y: f32,
    left: f32,
    right: f32,
    color: egui::Color32,
) {
    let dash_w = 4.0;
    let gap_w = 4.0;
    let mut x = left;
    loop {
        if x >= right {
            break;
        }
        let x_end = (x + dash_w).min(right);
        painter.line_segment(
            [egui::pos2(x, y), egui::pos2(x_end, y)],
            egui::Stroke::new(1.0, color),
        );
        x += dash_w + gap_w;
    }
}
