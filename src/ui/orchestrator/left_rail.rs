// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `LeftRail` — the persistent 200px left navigation rail.
//
// Per Phase 2 P2.T2: top section = 36×36px brand mark + wordmark stack;
// middle = 4 nav buttons (one per `NavDestination::rail_items`); bottom = a
// status dot + the path-validation summary text.
//
// 200px wide, `redesign_rail_bg` background, 1.5px solid right border in
// `redesign_border_strong`. Active nav item paints with `redesign_accent`
// fill + a 2px drop shadow; inactive items show a hover overlay.
//
// **No window-drag wiring**, **no rail-lock** (the `rail_locked: Option<&_>`
// parameter is reserved for Phase 7 C5 — Phase 2 callers pass `None`).
//
// SPEC: §2.1.

// rationale: `f32 as u8` casts are colour-channel / pixel roundings, correct
// by construction (Cat 2). The `0.7071` unit-vector literals are hand-tuned
// vector-icon geometry — substituting `FRAC_1_SQRT_2` would shift painted
// pixels, so `approx_constant` is NOT behavior-neutral; likewise `mul_add`
// changes float rounding. The icon-stepping `while x < end` loop, the
// readability-distinct match arms, and the `const fn` lint are intentional /
// churn. All suppressed without changing rendered output (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::approx_constant,
    clippy::suboptimal_flops,
    clippy::while_float,
    clippy::match_same_arms,
    clippy::missing_const_for_fn
)]

use eframe::egui;

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::nav_status::{PathValidationKind, PathValidationSummary};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_WIDTH_PX,
    REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette, redesign_accent, redesign_accent_deep,
    redesign_border_strong, redesign_hover_overlay, redesign_rail_bg, redesign_shadow,
    redesign_shell_bg, redesign_status_dot, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

/// Phase 7 C5 lock reason placeholder. Phase 2 callers pass `None`; the type
/// is defined here so the signature is stable across phases.
#[derive(Debug, Clone)]
pub enum RailLockReason {
    /// Install is in flight; navigation away is blocked until the install
    /// finishes (or is cancelled).
    InstallRunning { modlist_label: String },
}

/// Render the left rail and update `*current` when a rail item is clicked.
///
/// - `current`           — the active destination (mutated on click).
/// - `dev_mode`          — reserved for future dev-only rail affordances.
/// - `validation`        — path-validation summary for the bottom status row.
/// - `rail_locked`       — Phase 7 C5; Phase 2 passes `None`.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    current: &mut NavDestination,
    _dev_mode: bool,
    validation: &PathValidationSummary,
    _rail_locked: Option<&RailLockReason>,
) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    // 1. Background fill + right-side 1.5px border.
    painter.rect_filled(rect, 0.0, redesign_rail_bg(palette));
    let right_x = rect.right() - REDESIGN_BORDER_WIDTH_PX * 0.5;
    painter.line_segment(
        [
            egui::pos2(right_x, rect.top()),
            egui::pos2(right_x, rect.bottom()),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    // Carve out the inner content area, padded.
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
            // 2. Brand row: 36×36 accent square + wordmark stack.
            render_brand_row(ui, palette);

            ui.add_space(12.0);
            // Dashed bottom border under the brand row.
            let sep_top = ui.cursor().min.y;
            draw_dashed_horizontal(
                ui.painter(),
                sep_top,
                content_rect.left(),
                content_rect.right(),
                redesign_border_strong(palette),
            );
            ui.add_space(10.0);

            // 3. Nav items.
            for dest in NavDestination::rail_items() {
                let active = is_active(current, &dest);
                let clicked = render_nav_item(ui, palette, &dest, active);
                if clicked {
                    *current = dest;
                }
                ui.add_space(4.0);
            }

            // 4. Spacer pushes the status row to the bottom of the rail.
            let used_y = ui.cursor().min.y;
            let bottom_dashed_y = content_rect.bottom() - 32.0;
            if bottom_dashed_y > used_y {
                ui.add_space(bottom_dashed_y - used_y);
            }

            // Dashed top border above the status row.
            draw_dashed_horizontal(
                ui.painter(),
                bottom_dashed_y,
                content_rect.left(),
                content_rect.right(),
                redesign_border_strong(palette),
            );
            ui.add_space(8.0);

            // 5. Status dot + summary.
            render_status_row(ui, palette, validation);
        });
    });

    // Suppress unused-warning for the width const — keep the symbol alive.
    let _ = REDESIGN_NAV_WIDTH_PX;
}

fn is_active(current: &NavDestination, dest: &NavDestination) -> bool {
    match (current, dest) {
        (NavDestination::Home, NavDestination::Home) => true,
        (NavDestination::Install, NavDestination::Install) => true,
        (NavDestination::Create, NavDestination::Create) => true,
        (NavDestination::Settings, NavDestination::Settings) => true,
        // When the user is in a Workspace, the rail highlights **Create**.
        // Premise-checked against the canonical wireframe (the UI reference,
        // authoritative over prose): `app.jsx:37-40` `resumeBuild` does
        // `setActive("create")` and the workspace renders *inside* the
        // Create screen (`CreateScreen` short-circuits to `<WorkspaceView>`
        // when `resumedBuild` is set, `screens.jsx:3815`), so the rail's
        // active item is `create`. (The earlier `=> Home` arm cited "SPEC
        // §2.1", but §2.1's text only lists the 4 nav items + brand +
        // status — it says nothing about the workspace rail state; the
        // wireframe governs, and it shows Create.) The orchestrator routes
        // resume via `nav = Workspace { id }` (not a Create-local stage —
        // `state_create.rs`), so this arm maps that to the Create highlight
        // to match the wireframe.
        (NavDestination::Workspace { .. }, NavDestination::Create) => true,
        _ => false,
    }
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

    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
    // 2×2 drop shadow behind the mark.
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

    // ∞ glyph centered, dark ink for contrast against teal. FiraCode Nerd
    // is used here (not Poppins) because our shipped Poppins TTFs are a
    // Latin-only subset that does not include U+221E (see HANDOFF.md font
    // caveat). Nerd Font has full symbol coverage.
    painter.text(
        mark_square.center(),
        egui::Align2::CENTER_CENTER,
        "\u{221E}",
        egui::FontId::new(22.0, egui::FontFamily::Name("firacode_nerd".into())),
        egui::Color32::from_rgb(0x1a, 0x26, 0x38),
    );

    // Wordmark stack to the right of the mark.
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
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);

    if active {
        // Active: accent fill + 2×2 drop shadow + 1.5px border.
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
        // Hover overlay on inactive items.
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_filled(rect, radius, redesign_hover_overlay(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    } else {
        // Idle: no fill, no border (sketchy items only when active/hovered).
    }

    let text_color = if active {
        redesign_accent_deep(palette)
    } else {
        redesign_text_primary(palette)
    };

    // Icon (left) painted as vector shapes — see `paint_nav_icon` for why.
    let icon_center = egui::pos2(rect.left() + 20.0, rect.center().y);
    paint_nav_icon(painter, dest, icon_center, text_color);

    // Label.
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

/// Paint a nav-item icon as vector strokes. We paint rather than rely on a
/// font glyph because the wireframe icons (`⌂ ↓ ✎ ⚙`) are outside the Latin
/// subset of our shipped Poppins TTFs, and falling through to the default
/// font produces tofu boxes (see HANDOFF.md font caveat). Vectors decouple
/// the icon set from font coverage entirely.
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

    // One closed outline: bottom-left → up the left wall → roof apex →
    // down the right wall → (closed back along the floor). The walls meet
    // the roof eaves flush (no dangling 1.5px overhang) and `closed_line`
    // joins the corners cleanly, so the apex has no butt-cap notch — fixes
    // the prior "warped" look from independent, disconnected segments.
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
    // Pencil oriented from upper-right (eraser end) to lower-left (tip),
    // matching the wireframe's ✎ glyph axis. Drawn as three filled
    // parallelograms / triangles which together read as a pencil silhouette
    // at icon size:
    //   1. Eraser cap — short segment at the upper-right end.
    //   2. Body — longer segment after a small gap, the wooden barrel.
    //   3. Tip — triangle extending past the body's lower-left end.
    // The small gap between the eraser and body gives the visual notch
    // where the ferrule meets the wood, which is what reads as "pencil"
    // versus an undifferentiated diagonal blob.
    let along = egui::vec2(0.7071, -0.7071); // body axis, toward upper-right
    let perp = egui::vec2(0.7071, 0.7071); // perpendicular, toward lower-right
    let half_len = 5.0;
    let half_w = 1.6;

    // 1. Eraser cap.
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

    // 2. Body (after a ~0.7-unit gap).
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

    // 3. Triangular tip extending past the body's lower-left end.
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
        let inner = egui::pos2(center.x + angle.cos() * 6.0, center.y + angle.sin() * 6.0);
        let outer = egui::pos2(center.x + angle.cos() * 8.0, center.y + angle.sin() * 8.0);
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

    // 8×8 dot with 1px ring.
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
    while x < right {
        let x_end = (x + dash_w).min(right);
        painter.line_segment(
            [egui::pos2(x, y), egui::pos2(x_end, y)],
            egui::Stroke::new(1.0, color),
        );
        x += dash_w + gap_w;
    }
}
