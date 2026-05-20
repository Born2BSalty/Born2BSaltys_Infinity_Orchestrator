// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_soft, redesign_border_strong, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary, redesign_with_alpha,
};

const ARROW_BACK: &str = "\u{2190}";
const ARROW_FWD: &str = "\u{2192}";

pub const FOOTER_HEIGHT_PX: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FooterOutcome {
    pub back_clicked: bool,
    pub secondary_clicked: bool,
    pub primary_clicked: bool,
}

#[derive(Clone, Copy)]
pub struct BackBtn<'a> {
    pub label: &'a str,
}

#[derive(Clone, Copy)]
pub struct SecondaryBtn<'a> {
    pub label: &'a str,
}

#[derive(Clone, Copy)]
pub struct PrimaryBtn<'a> {
    pub label: &'a str,
    pub disabled: bool,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    back: Option<BackBtn<'_>>,
    secondary: Option<SecondaryBtn<'_>>,
    hint: Option<&str>,
    primary: PrimaryBtn<'_>,
) -> FooterOutcome {
    let mut outcome = FooterOutcome::default();

    ui.add_space(20.0);
    let top_y = ui.cursor().top();
    let full_w = ui.available_width();
    paint_dashed_hline(
        ui,
        egui::pos2(ui.cursor().left(), top_y),
        full_w,
        redesign_border_soft(palette),
    );
    ui.add_space(14.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        if let Some(b) = back
            && glyph_btn(
                ui,
                palette,
                GlyphSide::Leading(ARROW_BACK),
                b.label,
                false,
                false,
            )
            .clicked()
        {
            outcome.back_clicked = true;
        }

        if let Some(h) = hint {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(h)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = glyph_btn(
                ui,
                palette,
                GlyphSide::Trailing(ARROW_FWD),
                primary.label,
                true,
                primary.disabled,
            );
            if !primary.disabled && resp.clicked() {
                outcome.primary_clicked = true;
            }

            if let Some(s) = secondary {
                let sresp = glyph_btn(
                    ui,
                    palette,
                    GlyphSide::Trailing(ARROW_FWD),
                    s.label,
                    false,
                    false,
                );
                if sresp.clicked() {
                    outcome.secondary_clicked = true;
                }
            }
        });
    });

    outcome
}

#[derive(Clone, Copy)]
enum GlyphSide {
    Leading(&'static str),
    Trailing(&'static str),
}

fn glyph_btn(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    side: GlyphSide,
    label: &str,
    primary: bool,
    disabled: bool,
) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let gap = 5.0;

    let fill = if primary {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let text_color = if primary {
        egui::Color32::from_rgb(0x1a, 0x26, 0x38)
    } else {
        redesign_text_primary(palette)
    };

    let glyph_font = egui::FontId::new(font_size, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));

    let (glyph, leading) = match side {
        GlyphSide::Leading(g) => (g, true),
        GlyphSide::Trailing(g) => (g, false),
    };

    let glyph_galley =
        ui.painter()
            .layout_no_wrap(glyph.to_string(), glyph_font.clone(), text_color);
    let prose_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), prose_font.clone(), text_color);

    let content_w = glyph_galley.size().x + gap + prose_galley.size().x;
    let content_h = glyph_galley.size().y.max(prose_galley.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);

    let sense = if disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(desired, sense);

    let pressed = !disabled && response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let alpha = if disabled { 0.5 } else { 1.0 };
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

        if primary {
            let shadow_rect = rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            ));
            painter.rect_filled(
                shadow_rect,
                radius,
                with_alpha(redesign_shadow(palette), alpha),
            );
        }
        painter.rect_filled(rect, radius, with_alpha(fill, alpha));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                with_alpha(redesign_border_strong(palette), alpha),
            ),
            egui::StrokeKind::Inside,
        );

        let total_w = glyph_galley.size().x + gap + prose_galley.size().x;
        let start_x = rect.center().x - total_w / 2.0;
        let cy = rect.center().y;
        let col = with_alpha(text_color, alpha);

        paint_glyph_text(
            painter,
            GlyphTextPaint {
                start_x,
                cy,
                glyph,
                label,
                glyph_font,
                prose_font,
                glyph_w: glyph_galley.size().x,
                prose_w: prose_galley.size().x,
                gap,
                leading,
                color: col,
            },
        );
    }

    response
}

struct GlyphTextPaint<'a> {
    start_x: f32,
    cy: f32,
    glyph: &'a str,
    label: &'a str,
    glyph_font: egui::FontId,
    prose_font: egui::FontId,
    glyph_w: f32,
    prose_w: f32,
    gap: f32,
    leading: bool,
    color: egui::Color32,
}

fn paint_glyph_text(painter: &egui::Painter, paint: GlyphTextPaint<'_>) {
    let GlyphTextPaint {
        start_x,
        cy,
        glyph,
        label,
        glyph_font,
        prose_font,
        glyph_w,
        prose_w,
        gap,
        leading,
        color,
    } = paint;

    if leading {
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            glyph,
            glyph_font,
            color,
        );
        painter.text(
            egui::pos2(start_x + glyph_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            prose_font,
            color,
        );
    } else {
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            label,
            prose_font,
            color,
        );
        painter.text(
            egui::pos2(start_x + prose_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            glyph,
            glyph_font,
            color,
        );
    }
}

fn paint_dashed_hline(ui: &egui::Ui, start: egui::Pos2, width: f32, color: egui::Color32) {
    let dash = 5.0;
    let gap = 4.0;
    let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, color);
    let painter = ui.painter();
    let mut x = start.x;
    let end_x = start.x + width;
    loop {
        if x >= end_x {
            break;
        }
        let seg_end = (x + dash).min(end_x);
        painter.line_segment(
            [egui::pos2(x, start.y), egui::pos2(seg_end, start.y)],
            stroke,
        );
        x += dash + gap;
    }
}

fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    if alpha < 1.0 {
        redesign_with_alpha(c, 1, 2)
    } else {
        c
    }
}
