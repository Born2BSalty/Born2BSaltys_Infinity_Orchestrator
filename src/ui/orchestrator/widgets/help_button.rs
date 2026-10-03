// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::help_copy::{HelpBullet, help_text};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_page_bg, redesign_shell_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

pub(crate) use crate::ui::orchestrator::widgets::help_copy::HelpPage;

const WRENCH_SIZE: f32 = 22.0;
const CIRCLE_RADIUS: f32 = 9.5;
const CIRCLE_STROKE_W: f32 = 1.4;
const HANDLE_STROKE_W: f32 = 2.8;
const HEAD_RADIUS: f32 = 4.1;
const JAW_SLOT_W: f32 = 2.7;
const POPOVER_MAX_WIDTH: f32 = 540.0;
const POPOVER_TEXT_WIDTH: f32 = 500.0;
const POPOVER_BOTTOM_MARGIN: f32 = 40.0;
const POPOVER_MIN_BODY_HEIGHT: f32 = 120.0;
const GUTTER_W: f32 = 10.0;
const DOT_RADIUS: f32 = 1.6;
const BULLET_GAP: f32 = 9.0;
const DIAGNOSTICS_OFF_TEXT: &str =
    "Turn on diagnostic mode in Settings \u{2192} General to export diagnostics.";
const HOVER_TEXT: &str = "Help for this screen";

pub(crate) fn render(ui: &mut egui::Ui, palette: ThemePalette, page: HelpPage) {
    let anchor = wrench_button(ui, palette, page);
    render_popover(ui, palette, &anchor, page);
}

pub(crate) fn publish_diagnostic_mode(ctx: &egui::Context, on: bool) {
    ctx.memory_mut(|memory| memory.data.insert_temp(diagnostic_mode_id(), on));
}

#[must_use]
pub(crate) fn take_export_request(ctx: &egui::Context) -> bool {
    ctx.memory_mut(|memory| {
        std::mem::take(
            memory
                .data
                .get_temp_mut_or_default::<bool>(export_request_id()),
        )
    })
}

fn diagnostic_mode_id() -> egui::Id {
    egui::Id::new("orchestrator_help_diagnostic_mode")
}

fn export_request_id() -> egui::Id {
    egui::Id::new("orchestrator_help_export_request")
}

fn diagnostic_mode(ctx: &egui::Context) -> bool {
    ctx.memory_mut(|memory| {
        *memory
            .data
            .get_temp_mut_or_default::<bool>(diagnostic_mode_id())
    })
}

fn request_export(ctx: &egui::Context) {
    ctx.memory_mut(|memory| memory.data.insert_temp(export_request_id(), true));
}

fn popup_id(page: HelpPage) -> egui::Id {
    egui::Id::new("orchestrator_help_popover").with(page_salt(page))
}

const fn page_salt(page: HelpPage) -> &'static str {
    match page {
        HelpPage::Step2 { .. } => "step2",
        HelpPage::Step3 => "step3",
        HelpPage::Step4 => "step4",
        HelpPage::Step5 => "step5",
        HelpPage::Downloading { .. } => "downloading",
        HelpPage::Installing => "installing",
    }
}

fn wrench_button(ui: &mut egui::Ui, palette: ThemePalette, page: HelpPage) -> egui::Response {
    let size = egui::vec2(WRENCH_SIZE, WRENCH_SIZE);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg = redesign_page_bg(palette);
        let color = if response.hovered() {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        let painter = ui.painter();
        painter.circle_stroke(
            rect.center(),
            CIRCLE_RADIUS,
            egui::Stroke::new(CIRCLE_STROKE_W, color),
        );
        paint_wrench_glyph(painter, rect.center(), color, bg);
    }

    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id(page)));
    }

    response.on_hover_text(HOVER_TEXT)
}

fn paint_wrench_glyph(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
    bg: egui::Color32,
) {
    let diag = egui::vec2(
        std::f32::consts::FRAC_1_SQRT_2,
        -std::f32::consts::FRAC_1_SQRT_2,
    );
    let handle_start = center - diag * 5.2;
    let head_center = center + diag * 3.0;

    painter.line_segment(
        [handle_start, head_center],
        egui::Stroke::new(HANDLE_STROKE_W, color),
    );
    painter.circle_filled(handle_start, HANDLE_STROKE_W / 2.0, color);
    painter.circle_filled(head_center, HEAD_RADIUS, color);
    painter.line_segment(
        [center + diag * 3.4, center + diag * 8.0],
        egui::Stroke::new(JAW_SLOT_W, bg),
    );
}

fn render_popover(ui: &egui::Ui, palette: ThemePalette, anchor: &egui::Response, page: HelpPage) {
    let popup_id = popup_id(page);
    if !ui.memory(|memory| memory.is_popup_open(popup_id)) {
        return;
    }

    let mut anchor_pos = anchor.rect.right_bottom();
    if let Some(to_global) = ui.ctx().layer_transform_to_global(ui.layer_id()) {
        anchor_pos = to_global * anchor_pos;
    }
    let constrain_to = ui.ctx().screen_rect();
    let popup_frame = egui::Frame::popup(ui.style());
    let body_height =
        (constrain_to.bottom() - anchor_pos.y - POPOVER_BOTTOM_MARGIN).max(POPOVER_MIN_BODY_HEIGHT);

    let area_response = egui::Area::new(popup_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(anchor_pos)
        .pivot(egui::Align2::RIGHT_TOP)
        .constrain_to(constrain_to)
        .default_size(egui::vec2(
            POPOVER_MAX_WIDTH,
            body_height + POPOVER_BOTTOM_MARGIN,
        ))
        .show(ui.ctx(), |ui| {
            ui.set_max_width(POPOVER_MAX_WIDTH);
            popup_frame.show(ui, |ui| {
                egui::Frame::default()
                    .fill(redesign_shell_bg(palette))
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    ))
                    .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        let ctx = ui.ctx().clone();
                        egui::ScrollArea::vertical()
                            .max_height(body_height)
                            .min_scrolled_height(body_height)
                            .auto_shrink([true, true])
                            .show(ui, |ui| {
                                ui.set_max_width(POPOVER_TEXT_WIDTH);
                                render_popover_body(ui, &ctx, palette, page);
                            });
                    });
            });
        });

    let should_close = anchor.clicked_elsewhere() && area_response.response.clicked_elsewhere();
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
        ui.memory_mut(egui::Memory::close_popup);
    }
}

fn render_popover_body(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    palette: ThemePalette,
    page: HelpPage,
) {
    let text = help_text(page);
    ui.label(
        egui::RichText::new(text.title)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(BULLET_GAP);
    for (index, bullet) in text.bullets.iter().enumerate() {
        if index > 0 {
            ui.add_space(BULLET_GAP);
        }
        render_bullet(ui, palette, bullet);
    }
    render_diagnostics_line(ui, ctx, palette);
}

fn render_bullet(ui: &mut egui::Ui, palette: ThemePalette, bullet: &HelpBullet) {
    let wrap_width = (ui.available_width().min(POPOVER_TEXT_WIDTH) - GUTTER_W).max(0.0);
    let galley = ui.fonts(|fonts| fonts.layout_job(bullet_job(palette, bullet, wrap_width)));
    let row_height = galley
        .rows
        .first()
        .map_or_else(|| galley.size().y, eframe::epaint::text::Row::height);

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(GUTTER_W + wrap_width, galley.size().y),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        let dot_center = egui::pos2(rect.min.x + GUTTER_W / 2.0, rect.min.y + row_height / 2.0);
        let painter = ui.painter();
        painter.circle_filled(dot_center, DOT_RADIUS, redesign_text_muted(palette));
        painter.galley(
            egui::pos2(rect.min.x + GUTTER_W, rect.min.y),
            galley,
            redesign_text_muted(palette),
        );
    }
}

fn bullet_job(
    palette: ThemePalette,
    bullet: &HelpBullet,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    if let Some(lead) = bullet.lead {
        job.append(
            lead,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
                color: redesign_text_primary(palette),
                ..Default::default()
            },
        );
        job.append(
            " ",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into())),
                color: redesign_text_muted(palette),
                ..Default::default()
            },
        );
    }
    job.append(
        bullet.body,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into())),
            color: redesign_text_muted(palette),
            ..Default::default()
        },
    );
    job
}

fn render_diagnostics_line(ui: &mut egui::Ui, ctx: &egui::Context, palette: ThemePalette) {
    ui.add_space(10.0);
    if diagnostic_mode(ctx) {
        let response = redesign_btn(
            ui,
            palette,
            "Export diagnostics",
            BtnOpts {
                small: true,
                ..Default::default()
            },
        );
        if response.clicked() {
            request_export(ctx);
            ui.memory_mut(egui::Memory::close_popup);
        }
    } else {
        ui.label(
            egui::RichText::new(DIAGNOSTICS_OFF_TEXT)
                .size(12.5)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_faint(palette)),
        );
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{
        HelpPage, diagnostic_mode, publish_diagnostic_mode, request_export, take_export_request,
    };

    #[test]
    fn export_request_is_taken_once() {
        let ctx = egui::Context::default();
        assert!(!take_export_request(&ctx));
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            request_export(ctx);
        });
        assert!(take_export_request(&ctx));
        assert!(!take_export_request(&ctx));
    }

    #[test]
    fn diagnostic_mode_round_trips() {
        let ctx = egui::Context::default();
        assert!(!diagnostic_mode(&ctx));
        publish_diagnostic_mode(&ctx, true);
        assert!(diagnostic_mode(&ctx));
        publish_diagnostic_mode(&ctx, false);
        assert!(!diagnostic_mode(&ctx));
    }

    #[test]
    fn help_page_variants_are_distinguishable() {
        assert_ne!(
            HelpPage::Step2 {
                created_from_mods: false
            },
            HelpPage::Step2 {
                created_from_mods: true
            }
        );
        assert_ne!(HelpPage::Step3, HelpPage::Step4);
    }
}
