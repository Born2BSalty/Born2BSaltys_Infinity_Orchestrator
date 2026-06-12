// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_status_dot, redesign_text_muted,
    redesign_text_primary,
};

const HISTORY_BTN_W: f32 = 26.0;

#[derive(Debug, Clone)]
pub struct RunningInstallStatus {
    pub modlist_name: String,
    pub elapsed: std::time::Duration,
}

#[must_use]
pub fn format_elapsed(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Renders the statusbar.
///
/// Returns `true` if the notification-history toggle button was clicked this frame.
#[must_use]
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    modlist_count: usize,
    running_install: Option<&RunningInstallStatus>,
    history_has_items: bool,
    history_open: bool,
) -> bool {
    let rect = ui.max_rect();
    let text_color = redesign_text_muted(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Proportional);

    // Allocate the history button's interactivity before borrowing the painter.
    let btn_right = rect.right() - 12.0;
    let btn_center_x = btn_right - HISTORY_BTN_W / 2.0;
    let btn_height = rect.height() * 0.75;
    let btn_rect = egui::Rect::from_center_size(
        egui::pos2(btn_center_x, rect.center().y),
        egui::vec2(HISTORY_BTN_W, btn_height),
    );
    let btn_resp = ui.allocate_rect(btn_rect, egui::Sense::click());
    let btn_hovered = btn_resp.hovered();
    let btn_clicked = btn_resp.clicked();

    let painter = ui.painter();

    painter.rect_filled(rect, 0.0, redesign_chrome_bg(palette));

    let top_y = REDESIGN_BORDER_WIDTH_PX.mul_add(0.5, rect.top());
    painter.line_segment(
        [
            egui::pos2(rect.left(), top_y),
            egui::pos2(rect.right(), top_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );

    let dot_x = rect.left() + 12.0 + 4.0;
    let dot_center = egui::pos2(dot_x, rect.center().y);
    painter.circle_filled(dot_center, 4.0, redesign_status_dot(palette));
    painter.circle_stroke(
        dot_center,
        4.0,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );

    let mut x = dot_center.x + 4.0 + 8.0;
    let mut segments = vec!["connected".to_string(), format!("{modlist_count} modlists")];
    if let Some(run) = running_install {
        segments.push("1 job running".to_string());
        segments.push(run.modlist_name.clone());
        segments.push(format_elapsed(run.elapsed));
    } else {
        segments.push("0 jobs running".to_string());
    }
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            let galley = painter.layout_no_wrap("·".to_string(), font.clone(), text_color);
            let pos = egui::pos2(x, galley.size().y.mul_add(-0.5, rect.center().y));
            painter.galley(pos, galley.clone(), text_color);
            x += galley.size().x + 8.0;
        }
        let galley = painter.layout_no_wrap(seg.clone(), font.clone(), text_color);
        let pos = egui::pos2(x, galley.size().y.mul_add(-0.5, rect.center().y));
        let w = galley.size().x;
        painter.galley(pos, galley, text_color);
        x += w + 8.0;
    }

    let version_text = format!("v{}", env!("CARGO_PKG_VERSION"));
    let version_galley = painter.layout_no_wrap(version_text, font, text_color);
    let version_x = rect.right() - 12.0 - version_galley.size().x - HISTORY_BTN_W - 4.0;
    let version_pos = egui::pos2(
        version_x,
        version_galley.size().y.mul_add(-0.5, rect.center().y),
    );
    painter.galley(version_pos, version_galley, text_color);

    if btn_hovered || history_open {
        painter.rect_filled(
            btn_rect,
            egui::CornerRadius::same(3),
            redesign_hover_overlay(palette),
        );
    }
    let bell_color = if history_has_items {
        redesign_text_primary(palette)
    } else {
        text_color
    };
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{F0A2}",
        egui::FontId::new(11.0, egui::FontFamily::Name("firacode_nerd".into())),
        bell_color,
    );

    btn_clicked
}

pub const HEIGHT_PX: f32 = REDESIGN_STATUSBAR_HEIGHT_PX;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn elapsed_under_an_hour_is_mm_ss_zero_padded() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "00:00");
        assert_eq!(format_elapsed(Duration::from_secs(9)), "00:09");
        assert_eq!(format_elapsed(Duration::from_secs(65)), "01:05");
        assert_eq!(format_elapsed(Duration::from_secs(59 * 60 + 59)), "59:59");
    }

    #[test]
    fn elapsed_over_an_hour_is_h_mm_ss() {
        assert_eq!(format_elapsed(Duration::from_hours(1)), "1:00:00");
        assert_eq!(
            format_elapsed(Duration::from_secs(3600 + 23 * 60 + 7)),
            "1:23:07"
        );
        assert_eq!(
            format_elapsed(Duration::from_secs(10 * 3600 + 5)),
            "10:00:05"
        );
    }

    #[test]
    fn sub_second_truncates_to_zero() {
        assert_eq!(format_elapsed(Duration::from_millis(1500)), "00:01");
    }
}
