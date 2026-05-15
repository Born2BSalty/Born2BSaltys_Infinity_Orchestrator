// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `sub_flow_footer` — the linear sub-flow footer used by every Install
// Modlist stage (and, later, the Create fork sub-flow).
//
// Mirrors `wireframe-preview/screens.jsx::SubFlowFooter` (line 3494-3510):
//   <div marginTop:20 paddingTop:14 borderTop:"1.5px dashed var(--border-soft)"
//        display:flex alignItems:center gap:12 flexShrink:0>
//     {onBack && <Btn small onClick={onBack}>← {backLabel}</Btn>}
//     {hint   && <Label hand color:var(--text-faint) marginLeft:6>{hint}</Label>}
//     <div marginLeft:auto>
//       <Btn primary onClick={onPrimary}>{primaryLabel}</Btn>
//     </div>
//   </div>
//
// **Symbol-glyph handling (HANDOFF caveat).** The wireframe's Back button is
// `← {backLabel}` and the primary CTAs carry a trailing `→` (`Preview →`,
// `Continue Install →`). The shipped Poppins TTFs are a Latin-only subset:
// `←` U+2190 and `→` U+2192 tofu in any `poppins_*` family. The shared
// `redesign_btn` hardcodes `poppins_medium` for its single centered label, so
// it cannot faithfully render a glyph+prose label. Following the established
// project convention (see `home/toast.rs`: glyph in `firacode_nerd`, prose in
// Poppins, side by side), this module paints its own button chassis —
// pixel-identical to `redesign_btn` (sketchy border, 2×2 primary shadow,
// active-press transform, theme-invariant `#1a2638` primary text) — but lays
// the arrow glyph in `firacode_nerd` and the word in `poppins_medium`.
//
// `render` takes three optional pieces (back / hint / primary) so each stage
// supplies only what it needs:
//   - `back`    → `Some(BackBtn { label, ... })` renders the `← label` button.
//   - `hint`    → `Some(&str)` renders the faint hand-style hint.
//   - `primary` → the right-aligned primary CTA + whether it is disabled.
//
// SPEC: §4 (the SubFlowFooter pattern is used by every Install stage).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_soft, redesign_border_strong, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

/// The trailing/leading arrow glyph rendered in `firacode_nerd`. Kept as
/// constants so the symbol-glyph rule is visible at the call sites.
const ARROW_BACK: &str = "\u{2190}"; // ←
const ARROW_FWD: &str = "\u{2192}"; // →

/// Total vertical footprint of the footer: 20px top margin + 14px padding
/// above the controls + a ~30px control row (small button = 12px text +
/// 2×4 pad ≈ 20px, rounded up for the dashed rule + breathing room). Stages
/// that bottom-pin the footer (wireframe `<div flex:1 />` spacer) reserve
/// this much height for it so it never overflows the visible content area.
pub const FOOTER_HEIGHT_PX: f32 = 64.0;

/// What the footer did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FooterOutcome {
    /// `← Back`-style button clicked.
    pub back_clicked: bool,
    /// Primary CTA clicked (only ever `true` when the primary is enabled).
    pub primary_clicked: bool,
}

/// The Back-button spec (left side). `label` is the prose only — the leading
/// `←` is painted separately in `firacode_nerd`.
pub struct BackBtn<'a> {
    pub label: &'a str,
}

/// The primary CTA spec (right side). `label` is the prose only — the
/// trailing `→` is painted separately in `firacode_nerd`. `disabled` greys
/// it out and suppresses clicks (used by stage 1's empty-import-code state).
pub struct PrimaryBtn<'a> {
    pub label: &'a str,
    pub disabled: bool,
}

/// Render the sub-flow footer at the current cursor. Returns which control
/// was activated this frame.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    back: Option<BackBtn<'_>>,
    hint: Option<&str>,
    primary: PrimaryBtn<'_>,
) -> FooterOutcome {
    let mut outcome = FooterOutcome::default();

    // Wireframe: `marginTop:20; paddingTop:14; borderTop:1.5px dashed`.
    ui.add_space(20.0);
    let top_y = ui.cursor().top();
    let full_w = ui.available_width();
    // 1.5px dashed top rule across the footer width (wireframe `borderTop`).
    paint_dashed_hline(
        ui,
        egui::pos2(ui.cursor().left(), top_y),
        full_w,
        redesign_border_soft(palette),
    );
    ui.add_space(14.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        // ── Back (left). Glyph in firacode_nerd, prose in poppins_medium. ──
        if let Some(b) = back {
            if glyph_btn(ui, palette, GlyphSide::Leading(ARROW_BACK), b.label, false, false)
                .clicked()
            {
                outcome.back_clicked = true;
            }
        }

        // ── Hint (faint hand-style, marginLeft:6). ──
        if let Some(h) = hint {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(h)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
        }

        // ── Primary CTA pushed flush-right (wireframe `marginLeft:auto`). ──
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
        });
    });

    outcome
}

/// Which side the arrow glyph sits on.
enum GlyphSide {
    /// `← label` (Back).
    Leading(&'static str),
    /// `label →` (primary CTA).
    Trailing(&'static str),
}

/// Paint a sketchy button whose label is `glyph + prose`, the glyph in
/// `firacode_nerd` and the prose in `poppins_medium`. Chassis matches
/// `redesign_btn` (small variant: 10×4 padding, 12px) so the footer stays
/// visually consistent with every other redesign button.
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
    // Wireframe primary text is the theme-invariant `#1a2638` (same as
    // `redesign_btn`'s primary branch); non-primary uses the theme text.
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

    // `redesign_btn`'s active-press transform (shift down-right 1px).
    let pressed = !disabled && response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let alpha = if disabled { 0.5 } else { 1.0 };
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);

        if primary {
            let shadow_rect = rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            ));
            painter.rect_filled(shadow_rect, radius, with_alpha(redesign_shadow(palette), alpha));
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

        // Lay glyph + prose horizontally, centered as a unit in the rect.
        let total_w = glyph_galley.size().x + gap + prose_galley.size().x;
        let start_x = rect.center().x - total_w / 2.0;
        let cy = rect.center().y;
        let col = with_alpha(text_color, alpha);

        if leading {
            painter.text(
                egui::pos2(start_x, cy),
                egui::Align2::LEFT_CENTER,
                glyph,
                glyph_font,
                col,
            );
            painter.text(
                egui::pos2(start_x + glyph_galley.size().x + gap, cy),
                egui::Align2::LEFT_CENTER,
                label,
                prose_font,
                col,
            );
        } else {
            painter.text(
                egui::pos2(start_x, cy),
                egui::Align2::LEFT_CENTER,
                label,
                prose_font,
                col,
            );
            painter.text(
                egui::pos2(start_x + prose_galley.size().x + gap, cy),
                egui::Align2::LEFT_CENTER,
                glyph,
                glyph_font,
                col,
            );
        }
    }

    response
}

/// Paint a 1.5px dashed horizontal rule (wireframe `borderTop: 1.5px dashed`).
fn paint_dashed_hline(ui: &egui::Ui, start: egui::Pos2, width: f32, color: egui::Color32) {
    let dash = 5.0;
    let gap = 4.0;
    let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, color);
    let painter = ui.painter();
    let mut x = start.x;
    let end_x = start.x + width;
    while x < end_x {
        let seg_end = (x + dash).min(end_x);
        painter.line_segment(
            [egui::pos2(x, start.y), egui::pos2(seg_end, start.y)],
            stroke,
        );
        x += dash + gap;
    }
}

/// Apply an alpha multiplier (0.0..=1.0) on top of an existing `Color32`
/// (mirrors `redesign_btn::with_alpha`).
fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (c.a() as f32 * alpha).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
