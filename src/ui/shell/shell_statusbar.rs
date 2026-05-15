// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign shell statusbar (Infinity Orchestrator binary).
//
// Per Phase 1 P1.T5 — paints the 26px footer:
//   - 1.5px top border in `redesign_border_strong`,
//   - `redesign_chrome_bg` background,
//   - left-aligned segments `● connected · <N> modlists · <J> jobs running`
//     separated by ` · `,
//   - right-aligned `v<crate version>`,
//   - status dot is `8×8px` filled in `redesign_status_dot` with a 1px
//     `redesign_border_strong` ring.
//
// SPEC: §1.2 (26px footer status bar always visible), wireframe
// `index.html:175-184`, `app.jsx:148-155`.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_border_strong, redesign_chrome_bg, redesign_status_dot, redesign_text_muted,
    ThemePalette, REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX,
};

/// Paint the redesign statusbar inside the given `ui` (caller is expected to
/// have allocated a 26px-tall strip — e.g., via `egui::TopBottomPanel::bottom`).
///
/// `modlist_count` and `jobs_running` are caller-provided. Phase 3 / Phase 7
/// wire real sources; Phase 1 callers just pass zero.
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, modlist_count: usize, jobs_running: usize) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    // Background fill.
    painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));

    // 1.5px top border.
    let top_y = rect.top() + REDESIGN_BORDER_WIDTH_PX * 0.5;
    painter.line_segment(
        [
            egui::pos2(rect.left(), top_y),
            egui::pos2(rect.right(), top_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    let text_color = redesign_text_muted(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Proportional);

    // Status dot — 8×8px filled in `redesign_status_dot` with a 1px ring.
    let dot_x = rect.left() + 12.0 + 4.0;
    let dot_center = egui::pos2(dot_x, rect.center().y);
    painter.circle_filled(dot_center, 4.0, redesign_status_dot(palette));
    painter.circle_stroke(
        dot_center,
        4.0,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );

    // Left-aligned text segments after the dot.
    let mut x = dot_center.x + 4.0 + 8.0;
    let segments = [
        "connected".to_string(),
        format!("{modlist_count} modlists"),
        format!("{jobs_running} jobs running"),
    ];
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            let galley = painter.layout_no_wrap("·".to_string(), font.clone(), text_color);
            let pos = egui::pos2(x, rect.center().y - galley.size().y * 0.5);
            painter.galley(pos, galley.clone(), text_color);
            x += galley.size().x + 8.0;
        }
        let galley = painter.layout_no_wrap(seg.clone(), font.clone(), text_color);
        let pos = egui::pos2(x, rect.center().y - galley.size().y * 0.5);
        let w = galley.size().x;
        painter.galley(pos, galley, text_color);
        x += w + 8.0;
    }

    // Right-aligned crate version.
    let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
    let galley = painter.layout_no_wrap(version_text, font, text_color);
    let pos = egui::pos2(
        rect.right() - 12.0 - galley.size().x,
        rect.center().y - galley.size().y * 0.5,
    );
    painter.galley(pos, galley, text_color);
}

/// Convenience: the natural pixel height of the statusbar strip
/// (matches the wireframe `.sk-statusbar` height: 26px).
pub const HEIGHT_PX: f32 = REDESIGN_STATUSBAR_HEIGHT_PX;
