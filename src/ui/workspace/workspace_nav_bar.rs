// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_soft, redesign_border_strong, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary, redesign_with_alpha,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;

const ARROW_BACK: &str = "\u{2190}";
const ARROW_FWD: &str = "\u{2192}";

const PREV_DISABLED_TOOLTIP: &str =
    "Disabled while install is running or after a successful install";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavBarOutcome {
    pub prev_clicked: bool,
    pub next_clicked: bool,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: WorkspaceStep,
    disable_prev: bool,
) -> NavBarOutcome {
    let mut outcome = NavBarOutcome::default();

    let is_last = current.next().is_none();

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

        let prev_disabled = disable_prev;
        let prev_resp = glyph_btn(
            ui,
            palette,
            GlyphSide::Leading(ARROW_BACK),
            "Previous",
            false,
            prev_disabled,
        );
        if prev_resp.clicked() && !prev_disabled {
            outcome.prev_clicked = true;
        }
        if prev_disabled {
            prev_resp.on_hover_text(PREV_DISABLED_TOOLTIP);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let resp = glyph_btn(
                ui,
                palette,
                GlyphSide::Trailing(ARROW_FWD),
                "Next",
                true,
                is_last,
            );
            if !is_last && resp.clicked() {
                outcome.next_clicked = true;
            }

            let hint = if is_last {
                "final step".to_string()
            } else {
                format!("next: {}", current.next().map_or("", WorkspaceStep::label))
            };
            ui.label(
                egui::RichText::new(hint)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
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
    let gap = 5.0;

    let visuals = GlyphButtonVisuals::new(palette, primary, disabled);

    let (glyph, leading) = match side {
        GlyphSide::Leading(g) => (g, true),
        GlyphSide::Trailing(g) => (g, false),
    };

    let glyph_galley = ui.painter().layout_no_wrap(
        glyph.to_string(),
        visuals.glyph_font.clone(),
        visuals.text_color,
    );
    let prose_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        visuals.prose_font.clone(),
        visuals.text_color,
    );

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
        paint_glyph_button(
            ui.painter(),
            &GlyphButtonPaint {
                rect,
                glyph,
                label,
                leading,
                gap,
                glyph_galley: &glyph_galley,
                prose_galley: &prose_galley,
            },
            &visuals,
        );
    }

    response
}

struct GlyphButtonVisuals {
    fill: egui::Color32,
    border: egui::Color32,
    shadow: egui::Color32,
    text_color: egui::Color32,
    glyph_font: egui::FontId,
    prose_font: egui::FontId,
    primary: bool,
}

impl GlyphButtonVisuals {
    fn new(palette: ThemePalette, primary: bool, disabled: bool) -> Self {
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
        Self {
            fill: button_alpha(fill, disabled),
            border: button_alpha(redesign_border_strong(palette), disabled),
            shadow: button_alpha(redesign_shadow(palette), disabled),
            text_color: button_alpha(text_color, disabled),
            glyph_font: egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
            prose_font: egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
            primary,
        }
    }
}

struct GlyphButtonPaint<'a> {
    rect: egui::Rect,
    glyph: &'a str,
    label: &'a str,
    leading: bool,
    gap: f32,
    glyph_galley: &'a std::sync::Arc<egui::Galley>,
    prose_galley: &'a std::sync::Arc<egui::Galley>,
}

fn paint_glyph_button(
    painter: &egui::Painter,
    paint: &GlyphButtonPaint<'_>,
    visuals: &GlyphButtonVisuals,
) {
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

    if visuals.primary {
        let shadow_rect = paint.rect.translate(egui::vec2(
            REDESIGN_SHADOW_OFFSET_BTN_PX,
            REDESIGN_SHADOW_OFFSET_BTN_PX,
        ));
        painter.rect_filled(shadow_rect, radius, visuals.shadow);
    }
    painter.rect_filled(paint.rect, radius, visuals.fill);
    painter.rect_stroke(
        paint.rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, visuals.border),
        egui::StrokeKind::Inside,
    );

    let total_w = paint.glyph_galley.size().x + paint.gap + paint.prose_galley.size().x;
    let start_x = paint.rect.center().x - total_w / 2.0;
    let cy = paint.rect.center().y;
    if paint.leading {
        paint_button_text(
            painter,
            &ButtonTextPaint {
                start_x,
                cy,
                first: paint.glyph,
                second: paint.label,
                gap: paint.gap,
                first_galley: paint.glyph_galley,
                leading_is_glyph: true,
            },
            visuals,
        );
    } else {
        paint_button_text(
            painter,
            &ButtonTextPaint {
                start_x,
                cy,
                first: paint.label,
                second: paint.glyph,
                gap: paint.gap,
                first_galley: paint.prose_galley,
                leading_is_glyph: false,
            },
            visuals,
        );
    }
}

struct ButtonTextPaint<'a> {
    start_x: f32,
    cy: f32,
    first: &'a str,
    second: &'a str,
    gap: f32,
    first_galley: &'a std::sync::Arc<egui::Galley>,
    leading_is_glyph: bool,
}

/// Picks the `(first_font, second_font)` pair for the two text pieces in
/// a glyph+prose button. `leading_is_glyph == true` means the first piece
/// is the icon glyph (the rendering font must be the `FiraCode` Nerd one
/// that carries the arrow PUA range); the second piece is the prose
/// label. `false` flips it. The Latin prose font carries no arrow glyphs,
/// so a swap drops the arrow to the missing-glyph `?` fallback.
fn pick_button_fonts(
    visuals: &GlyphButtonVisuals,
    leading_is_glyph: bool,
) -> (egui::FontId, egui::FontId) {
    if leading_is_glyph {
        (visuals.glyph_font.clone(), visuals.prose_font.clone())
    } else {
        (visuals.prose_font.clone(), visuals.glyph_font.clone())
    }
}

fn paint_button_text(
    painter: &egui::Painter,
    paint: &ButtonTextPaint<'_>,
    visuals: &GlyphButtonVisuals,
) {
    let (first_font, second_font) = pick_button_fonts(visuals, paint.leading_is_glyph);
    painter.text(
        egui::pos2(paint.start_x, paint.cy),
        egui::Align2::LEFT_CENTER,
        paint.first,
        first_font,
        visuals.text_color,
    );
    painter.text(
        egui::pos2(
            paint.start_x + paint.first_galley.size().x + paint.gap,
            paint.cy,
        ),
        egui::Align2::LEFT_CENTER,
        paint.second,
        second_font,
        visuals.text_color,
    );
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

fn button_alpha(c: egui::Color32, disabled: bool) -> egui::Color32 {
    if disabled {
        redesign_with_alpha(c, 1, 2)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::{GlyphButtonVisuals, pick_button_fonts};
    use crate::ui::shared::redesign_tokens::ThemePalette;
    use eframe::egui;

    fn visuals() -> GlyphButtonVisuals {
        GlyphButtonVisuals::new(ThemePalette::Dark, false, false)
    }

    fn family_name(font: &egui::FontId) -> String {
        match &font.family {
            egui::FontFamily::Name(name) => name.as_ref().to_string(),
            egui::FontFamily::Proportional => "proportional".to_string(),
            egui::FontFamily::Monospace => "monospace".to_string(),
        }
    }

    #[test]
    fn pick_button_fonts_leading_glyph_routes_arrow_to_firacode_nerd() {
        let v = visuals();
        let (first, second) = pick_button_fonts(&v, true);
        assert_eq!(
            family_name(&first),
            "firacode_nerd",
            "Previous-style button: leading piece is the arrow glyph and \
             MUST use the FiraCode Nerd family that carries the arrow PUA \
             range; got `{}`",
            family_name(&first)
        );
        assert_eq!(family_name(&second), "poppins_medium");
    }

    #[test]
    fn pick_button_fonts_trailing_glyph_routes_arrow_to_firacode_nerd() {
        let v = visuals();
        let (first, second) = pick_button_fonts(&v, false);
        assert_eq!(family_name(&first), "poppins_medium");
        assert_eq!(
            family_name(&second),
            "firacode_nerd",
            "Next-style button: trailing piece is the arrow glyph and MUST \
             use FiraCode Nerd — Poppins is a Latin subset that drops the \
             arrow to the missing-glyph `?` fallback; got `{}`",
            family_name(&second)
        );
    }
}
