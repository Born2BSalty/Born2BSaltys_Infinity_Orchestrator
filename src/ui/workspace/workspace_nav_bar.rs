// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_nav_bar` — the bottom back/next nav row for the workspace.
//
// Mirrors `wireframe-preview/screens.jsx::WorkspaceNavBar` (line 3358-3387):
//   container: marginTop 20, paddingTop 14, borderTop "1.5px dashed
//              var(--border-soft)", flex row, alignItems center, gap 12.
//     <Btn small disabled={disablePrev}>← Previous</Btn>
//     <div marginLeft:auto gap:10>
//       <Label hand color:var(--text-faint)>
//          {isLast ? "final step" : `next: ${nextLabel}`}
//       </Label>
//       <Btn primary disabled={isLast}>Next →</Btn>
//     </div>
//
// **Nav step-indicator REMOVED (deliberate user-directed wireframe
// deviation — SPEC §2.2 records it as intentional).** The wireframe also
// drew a `<Label hand color:text-faint marginLeft:14>on {kicker} · {label}
// · step {i} of {total}</Label>` between `← Previous` and the right
// cluster; the user directed its removal on all 4 steps (final authority).
// The progress bar above already shows the current step + number, so the
// line was redundant chrome. Everything else is wireframe-faithful; the
// right cluster's `next: <label>` / `final step` hint is kept (it is the
// only forward-looking affordance, NOT the removed indicator).
//
// **Symbol-glyph rule (HANDOFF caveat).** `←` U+2190 / `→` U+2192 are
// base-FiraCode glyphs the bundled full FiraCode Nerd build covers
// (cmap-verified) but the Latin-only Poppins subset tofus. The shared
// `redesign_btn` hardcodes `poppins_medium`, so — following the established
// convention (`install/sub_flow_footer.rs`, `home/toast.rs`: glyph in
// `firacode_nerd`, prose in `poppins_medium`, side by side) — this module
// paints its own button chassis, pixel-identical to `redesign_btn`'s small
// variant (sketchy border, 2×2 primary shadow, active-press transform,
// theme-invariant `#1a2638` primary text), with the arrow in `firacode_nerd`.
//
// **`← Previous` is enabled on the first workspace step** (Step 2): it
// routes back to Home (SPEC §2.2 / P6.T4 — the user entered the workspace
// via a Home `resume`/`open`, so first-step Previous closes that loop
// rather than being a dead control; intentional affordance-forward
// deviation from the wireframe's former first-step *disabled* state,
// recorded SPEC §2.2 + overview 2026-05-16). The caller (`workspace_view`)
// interprets a first-step `prev_clicked` as the Home route. `← Previous` is
// force-disabled **only** by `disable_prev` — the Phase-7 install-running /
// post-install gate (wireframe `disablePrev`; `false` until Phase 7).
//
// SPEC: §2.2 (workspace nav bar).

// rationale: f32→u8 channel/alpha roundings + an intentional pixel-stepping
// dashed-rule loop — correct by construction (Cat 2 / Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::while_float
)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_soft, redesign_border_strong, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;

const ARROW_BACK: &str = "\u{2190}"; // ←
const ARROW_FWD: &str = "\u{2192}"; // →

/// What the nav bar did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavBarOutcome {
    /// `← Previous` clicked (only ever `true` when enabled).
    pub prev_clicked: bool,
    /// `Next →` clicked (only ever `true` when enabled).
    pub next_clicked: bool,
}

/// Render the workspace nav bar for `current`. `disable_prev` force-disables
/// `← Previous` regardless of step (the Phase-7 install-running / post-
/// install gate; `false` in Run 1).
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: WorkspaceStep,
    disable_prev: bool,
) -> NavBarOutcome {
    let mut outcome = NavBarOutcome::default();

    let is_last = current.next().is_none();

    // Wireframe: marginTop 20, paddingTop 14, 1.5px dashed top rule.
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

        // ← Previous (small). **Enabled on the first workspace step** so it
        // can route back to Home (SPEC §2.2 / P6.T4): the user reached the
        // workspace via a Home `resume`/`open`, so first-step Previous
        // closes that loop rather than being a dead control. The caller
        // (`workspace_view`) interprets a first-step `prev_clicked` as a
        // Home route. `← Previous` is force-disabled **only** by
        // `disable_prev` — the Phase-7 install-running / post-install gate
        // (wireframe `disablePrev`; `false` until Phase 7).
        let prev_disabled = disable_prev;
        if glyph_btn(
            ui,
            palette,
            GlyphSide::Leading(ARROW_BACK),
            "Previous",
            false,
            prev_disabled,
        )
        .clicked()
            && !prev_disabled
        {
            outcome.prev_clicked = true;
        }

        // **Nav step-indicator REMOVED on all 4 steps — deliberate
        // user-directed wireframe deviation (SPEC §2.2 records it as
        // intentional so a future review does not restore it).** The
        // wireframe (`screens.jsx:3376-3378`) draws a faint
        // `on <Step N> · <Label> · step <i> of <total>` Label here; the
        // user directed its removal (final authority on the directed
        // deviation). The progress bar above already shows the current
        // step + its number, so this line was redundant chrome. The
        // forward `next: <label>` / `final step` hint (right cluster) is
        // kept — it is the only forward-looking affordance and is NOT the
        // removed indicator. (The right cluster's last-step logic uses
        // `is_last`/`current.next()`, not the removed index/total.)

        // Right cluster (marginLeft auto): next-hint + `Next →` primary.
        // `right_to_left` lays the trailing edge first, so add the primary
        // first, the hint second → on-screen order `[hint] [Next →]`.
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

/// Render a primary forward button styled **exactly** like this nav bar's
/// `Next →` (the same `glyph_btn` chassis: sketchy border, 2×2 primary
/// shadow, active-press transform, `firacode_nerd` arrow + `poppins_medium`
/// prose, theme-invariant `#1a2638` primary text). Exposed so Create's
/// single `Start →` CTA reuses the workspace forward-button styling verbatim
/// (the dispatch-brief mandate — one styling source, no fourth copy of the
/// glyph-button chassis). `label` is the prose; the trailing glyph is the
/// shared `→` (`ARROW_FWD`, cmap-present in `firacode_nerd`).
pub(crate) fn forward_primary_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
) -> egui::Response {
    glyph_btn(
        ui,
        palette,
        GlyphSide::Trailing(ARROW_FWD),
        label,
        true,  // primary (accent fill + 2×2 shadow) — same as `Next →`
        false, // never disabled here (the choose CTA is always actionable)
    )
}

/// Which side the arrow glyph sits on (mirrors `sub_flow_footer::GlyphSide`).
enum GlyphSide {
    /// `← label`.
    Leading(&'static str),
    /// `label →`.
    Trailing(&'static str),
}

/// Paint a sketchy button whose label is `glyph + prose` — glyph in
/// `firacode_nerd`, prose in `poppins_medium`. Chassis matches
/// `redesign_btn`'s small variant (10×4 pad, 12px, 2×2 primary shadow,
/// active-press transform) for visual consistency with every other redesign
/// button. (Pattern duplicated from `install/sub_flow_footer::glyph_btn`,
/// which is module-private — the project convention is each module paints
/// its own glyph-button chassis.)
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
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);

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

/// Paint a 1.5px dashed horizontal rule (wireframe `borderTop: 1.5px
/// dashed`). Mirrors `sub_flow_footer::paint_dashed_hline`.
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

/// Apply an alpha multiplier (0.0..=1.0) on top of an existing `Color32`.
fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (f32::from(c.a()) * alpha).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
